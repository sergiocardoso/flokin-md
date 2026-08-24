use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    time::{Duration, Instant},
};

use rusqlite::{
    types::{ToSqlOutput, Value, ValueRef},
    Connection, ToSql,
};

use crate::{Collection, Document, PropertyValue};

pub const DEFAULT_RESULT_LIMIT: usize = 1_000;
const READ_ONLY_ERROR: &str = "MDB-009 permite apenas consultas de leitura.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlCatalog {
    pub tables: Vec<SqlTable>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlTable {
    pub name: String,
    pub collection_id: String,
    pub display_name: String,
    pub columns: Vec<SqlColumn>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlColumn {
    pub name: String,
    pub source_property: Option<String>,
    pub value_type: SqlColumnType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlColumnType {
    Text,
    Integer,
    Real,
    Boolean,
    Json,
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SqlQueryResult {
    pub columns: Vec<SqlResultColumn>,
    pub rows: Vec<Vec<SqlValue>>,
    pub elapsed: Duration,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlResultColumn {
    pub name: String,
    pub value_type: Option<SqlColumnType>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SqlValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlError {
    pub message: String,
}

#[derive(Debug)]
pub struct SqlProjection {
    connection: Connection,
    catalog: SqlCatalog,
}

impl SqlProjection {
    pub fn build(documents: &[Document], collections: &[Collection]) -> Result<Self, SqlError> {
        let catalog = build_catalog(documents, collections);
        let connection = Connection::open_in_memory().map_err(SqlError::from)?;
        connection
            .execute_batch("PRAGMA trusted_schema = OFF;")
            .map_err(SqlError::from)?;

        for table in &catalog.tables {
            create_table(&connection, table)?;
            insert_documents(&connection, table, documents)?;
        }
        connection
            .execute_batch("PRAGMA query_only = ON;")
            .map_err(SqlError::from)?;

        Ok(Self {
            connection,
            catalog,
        })
    }

    pub const fn catalog(&self) -> &SqlCatalog {
        &self.catalog
    }

    pub fn execute_read(&self, query: &str, limit: usize) -> Result<SqlQueryResult, SqlError> {
        let sql = query.trim();
        if sql.is_empty() {
            return Err(SqlError::new("Digite uma consulta SQL."));
        }

        let started_at = Instant::now();
        let statement_sql = single_statement(sql)?;
        if !starts_with_read_query(statement_sql) {
            return Err(SqlError::new(READ_ONLY_ERROR));
        }
        let mut statement = self
            .connection
            .prepare(statement_sql)
            .map_err(SqlError::from)?;
        if !statement.readonly() {
            return Err(SqlError::new(READ_ONLY_ERROR));
        }

        let column_names = statement
            .column_names()
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let column_types = column_names
            .iter()
            .map(|name| self.catalog.result_column_type(name))
            .collect::<Vec<_>>();
        let mut rows = statement.query([]).map_err(SqlError::from)?;
        let mut result_rows = Vec::new();
        let mut truncated = false;

        while let Some(row) = rows.next().map_err(SqlError::from)? {
            if result_rows.len() >= limit {
                truncated = true;
                break;
            }

            let mut values = Vec::with_capacity(column_names.len());
            for index in 0..column_names.len() {
                values.push(sql_value(row.get_ref(index).map_err(SqlError::from)?));
            }
            result_rows.push(values);
        }

        Ok(SqlQueryResult {
            columns: column_names
                .into_iter()
                .zip(column_types)
                .map(|(name, value_type)| SqlResultColumn { name, value_type })
                .collect(),
            rows: result_rows,
            elapsed: started_at.elapsed(),
            truncated,
        })
    }
}

impl SqlCatalog {
    pub fn table_for_collection(&self, collection_id: &str) -> Option<&SqlTable> {
        self.tables
            .iter()
            .find(|table| table.collection_id == collection_id)
    }

    fn result_column_type(&self, name: &str) -> Option<SqlColumnType> {
        let normalized = normalize_identifier(name);
        self.tables
            .iter()
            .flat_map(|table| table.columns.iter())
            .find(|column| column.name == name || column.name == normalized)
            .map(|column| column.value_type)
    }
}

impl SqlColumnType {
    pub const fn sql_type(self) -> &'static str {
        match self {
            Self::Text | Self::Json | Self::Null => "TEXT",
            Self::Integer | Self::Boolean => "INTEGER",
            Self::Real => "REAL",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Text => "TEXT",
            Self::Integer => "INTEGER",
            Self::Real => "REAL",
            Self::Boolean => "BOOLEAN",
            Self::Json => "JSON",
            Self::Null => "NULL",
        }
    }
}

impl SqlValue {
    pub fn display_value(&self, value_type: Option<SqlColumnType>) -> String {
        match (self, value_type) {
            (Self::Null, _) => String::from("—"),
            (Self::Integer(1), Some(SqlColumnType::Boolean)) => String::from("✓"),
            (Self::Integer(0), Some(SqlColumnType::Boolean)) => String::from("✕"),
            (Self::Integer(value), _) => value.to_string(),
            (Self::Real(value), _) => value.to_string(),
            (Self::Text(value), _) => value.clone(),
        }
    }
}

impl SqlError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SqlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SqlError {}

impl From<rusqlite::Error> for SqlError {
    fn from(error: rusqlite::Error) -> Self {
        Self::new(user_sql_error(error))
    }
}

fn build_catalog(documents: &[Document], collections: &[Collection]) -> SqlCatalog {
    let mut table_names = BTreeSet::new();
    let tables = collections
        .iter()
        .filter(|collection| {
            documents
                .iter()
                .any(|document| document.collection_id == collection.id)
        })
        .map(|collection| {
            let table_name = unique_identifier(&collection.display_name, &mut table_names);
            let collection_documents = documents
                .iter()
                .filter(|document| document.collection_id == collection.id)
                .collect::<Vec<_>>();
            SqlTable {
                name: table_name,
                collection_id: collection.id.clone(),
                display_name: collection.display_name.clone(),
                columns: discover_columns(&collection_documents),
            }
        })
        .collect();

    SqlCatalog { tables }
}

fn discover_columns(documents: &[&Document]) -> Vec<SqlColumn> {
    let mut columns = vec![
        SqlColumn {
            name: String::from("title"),
            source_property: None,
            value_type: SqlColumnType::Text,
        },
        SqlColumn {
            name: String::from("_path"),
            source_property: None,
            value_type: SqlColumnType::Text,
        },
        SqlColumn {
            name: String::from("_file_name"),
            source_property: None,
            value_type: SqlColumnType::Text,
        },
    ];

    let mut stats = BTreeMap::<String, PropertyStats>::new();
    for document in documents {
        for (property, value) in &document.properties {
            if is_redundant_property(property) {
                continue;
            }

            let stat = stats.entry(property.clone()).or_default();
            stat.seen += 1;
            stat.value_types.insert(value_type(value));
        }
    }

    let mut used_names = columns
        .iter()
        .map(|column| column.name.clone())
        .collect::<BTreeSet<_>>();
    for (property, stat) in stats {
        columns.push(SqlColumn {
            name: unique_identifier(&property, &mut used_names),
            source_property: Some(property),
            value_type: stat.inferred_type(),
        });
    }

    columns
}

#[derive(Debug, Default)]
struct PropertyStats {
    seen: usize,
    value_types: BTreeSet<SqlColumnType>,
}

impl PropertyStats {
    fn inferred_type(&self) -> SqlColumnType {
        let mut types = self
            .value_types
            .iter()
            .copied()
            .filter(|value_type| *value_type != SqlColumnType::Null)
            .collect::<BTreeSet<_>>();

        if types.is_empty() {
            return SqlColumnType::Null;
        }

        if types.len() == 1 {
            return types.pop_first().unwrap_or(SqlColumnType::Text);
        }

        if types == BTreeSet::from([SqlColumnType::Integer, SqlColumnType::Real]) {
            return SqlColumnType::Real;
        }

        if types
            .iter()
            .any(|value_type| matches!(value_type, SqlColumnType::Json))
        {
            SqlColumnType::Json
        } else {
            SqlColumnType::Text
        }
    }
}

impl Ord for SqlColumnType {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (*self as u8).cmp(&(*other as u8))
    }
}

impl PartialOrd for SqlColumnType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn create_table(connection: &Connection, table: &SqlTable) -> Result<(), SqlError> {
    let columns = table
        .columns
        .iter()
        .map(|column| {
            format!(
                "{} {}",
                quote_identifier(column.name.as_str()),
                column.value_type.sql_type()
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    connection
        .execute(
            &format!("CREATE TABLE {} ({columns})", quote_identifier(&table.name)),
            [],
        )
        .map(|_| ())
        .map_err(SqlError::from)
}

fn insert_documents(
    connection: &Connection,
    table: &SqlTable,
    documents: &[Document],
) -> Result<(), SqlError> {
    let placeholders = std::iter::repeat_n("?", table.columns.len())
        .collect::<Vec<_>>()
        .join(", ");
    let columns = table
        .columns
        .iter()
        .map(|column| quote_identifier(&column.name))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "INSERT INTO {} ({columns}) VALUES ({placeholders})",
        quote_identifier(&table.name)
    );
    let mut statement = connection.prepare(&sql).map_err(SqlError::from)?;

    for document in documents
        .iter()
        .filter(|document| document.collection_id == table.collection_id)
    {
        let values = table
            .columns
            .iter()
            .map(|column| projected_value(document, column))
            .collect::<Vec<_>>();
        let params = values
            .iter()
            .map(|value| value as &dyn ToSql)
            .collect::<Vec<_>>();
        statement
            .execute(params.as_slice())
            .map_err(SqlError::from)?;
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
enum ProjectedValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
}

impl ToSql for ProjectedValue {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(match self {
            Self::Null => ToSqlOutput::Owned(Value::Null),
            Self::Integer(value) => ToSqlOutput::Owned(Value::Integer(*value)),
            Self::Real(value) => ToSqlOutput::Owned(Value::Real(*value)),
            Self::Text(value) => ToSqlOutput::Owned(Value::Text(value.clone())),
        })
    }
}

fn projected_value(document: &Document, column: &SqlColumn) -> ProjectedValue {
    match column.name.as_str() {
        "title" if column.source_property.is_none() => ProjectedValue::Text(document.title.clone()),
        "_path" => ProjectedValue::Text(document.relative_path.display().to_string()),
        "_file_name" => ProjectedValue::Text(document.file_name.to_string_lossy().into_owned()),
        _ => column
            .source_property
            .as_ref()
            .and_then(|property| document.properties.get(property))
            .map(|value| projected_property_value(value, column.value_type))
            .unwrap_or(ProjectedValue::Null),
    }
}

fn projected_property_value(value: &PropertyValue, column_type: SqlColumnType) -> ProjectedValue {
    match (value, column_type) {
        (PropertyValue::Null, _) => ProjectedValue::Null,
        (PropertyValue::Bool(value), _) => ProjectedValue::Integer(i64::from(*value)),
        (PropertyValue::Number(value), SqlColumnType::Integer) => value
            .parse::<i64>()
            .map(ProjectedValue::Integer)
            .unwrap_or_else(|_| ProjectedValue::Text(value.clone())),
        (PropertyValue::Number(value), SqlColumnType::Real) => value
            .parse::<f64>()
            .map(ProjectedValue::Real)
            .unwrap_or_else(|_| ProjectedValue::Text(value.clone())),
        (PropertyValue::Number(value), _) => ProjectedValue::Text(value.clone()),
        (PropertyValue::String(value), _) => ProjectedValue::Text(value.clone()),
        (PropertyValue::Array(_) | PropertyValue::Object(_), _) => {
            ProjectedValue::Text(json_value(value).to_string())
        }
    }
}

fn value_type(value: &PropertyValue) -> SqlColumnType {
    match value {
        PropertyValue::Null => SqlColumnType::Null,
        PropertyValue::Bool(_) => SqlColumnType::Boolean,
        PropertyValue::Number(value) if value.parse::<i64>().is_ok() => SqlColumnType::Integer,
        PropertyValue::Number(value) if value.parse::<f64>().is_ok() => SqlColumnType::Real,
        PropertyValue::Number(_) | PropertyValue::String(_) => SqlColumnType::Text,
        PropertyValue::Array(_) | PropertyValue::Object(_) => SqlColumnType::Json,
    }
}

fn json_value(value: &PropertyValue) -> serde_json::Value {
    match value {
        PropertyValue::Null => serde_json::Value::Null,
        PropertyValue::Bool(value) => serde_json::Value::Bool(*value),
        PropertyValue::Number(value) => value
            .parse::<i64>()
            .map(serde_json::Value::from)
            .or_else(|_| value.parse::<f64>().map(serde_json::Value::from))
            .unwrap_or_else(|_| serde_json::Value::String(value.clone())),
        PropertyValue::String(value) => serde_json::Value::String(value.clone()),
        PropertyValue::Array(values) => {
            serde_json::Value::Array(values.iter().map(json_value).collect())
        }
        PropertyValue::Object(values) => serde_json::Value::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), json_value(value)))
                .collect(),
        ),
    }
}

fn sql_value(value: ValueRef<'_>) -> SqlValue {
    match value {
        ValueRef::Null => SqlValue::Null,
        ValueRef::Integer(value) => SqlValue::Integer(value),
        ValueRef::Real(value) => SqlValue::Real(value),
        ValueRef::Text(value) => SqlValue::Text(String::from_utf8_lossy(value).into_owned()),
        ValueRef::Blob(value) => SqlValue::Text(format!("<{} bytes>", value.len())),
    }
}

pub fn normalize_identifier(value: &str) -> String {
    let mut normalized = String::new();
    let mut previous_was_separator = false;

    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            normalized.push(character);
            previous_was_separator = false;
        } else if !previous_was_separator && !normalized.is_empty() {
            normalized.push('_');
            previous_was_separator = true;
        }
    }

    while normalized.ends_with('_') {
        normalized.pop();
    }

    if normalized.is_empty() {
        normalized.push_str("column");
    }

    if normalized
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        normalized.insert(0, '_');
    }

    normalized
}

fn unique_identifier(value: &str, used: &mut BTreeSet<String>) -> String {
    let base = normalize_identifier(value);
    let mut candidate = base.clone();
    let mut suffix = 2;
    while used.contains(&candidate) {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }
    used.insert(candidate.clone());
    candidate
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn single_statement(sql: &str) -> Result<&str, SqlError> {
    let mut remaining = sql.trim();
    let mut statements = 0;
    let mut first = "";

    while !remaining.is_empty() {
        let Some(end) = sqlite_statement_end(remaining) else {
            statements += 1;
            if first.is_empty() {
                first = remaining.trim();
            }
            break;
        };

        let statement = remaining[..end].trim();
        if !statement.is_empty() {
            statements += 1;
            if first.is_empty() {
                first = statement;
            }
        }
        remaining = remaining[end + 1..].trim();
    }

    if statements == 1 {
        Ok(first)
    } else {
        Err(SqlError::new(
            "MDB-009 aceita apenas uma statement SQL por execução.",
        ))
    }
}

fn sqlite_statement_end(sql: &str) -> Option<usize> {
    let mut single_quote = false;
    let mut double_quote = false;
    let mut line_comment = false;
    let mut block_comment = false;
    let mut chars = sql.char_indices().peekable();

    while let Some((index, character)) = chars.next() {
        if line_comment {
            if character == '\n' {
                line_comment = false;
            }
            continue;
        }
        if block_comment {
            if character == '*' && chars.peek().is_some_and(|(_, next)| *next == '/') {
                chars.next();
                block_comment = false;
            }
            continue;
        }
        if single_quote {
            if character == '\'' {
                if chars.peek().is_some_and(|(_, next)| *next == '\'') {
                    chars.next();
                } else {
                    single_quote = false;
                }
            }
            continue;
        }
        if double_quote {
            if character == '"' {
                if chars.peek().is_some_and(|(_, next)| *next == '"') {
                    chars.next();
                } else {
                    double_quote = false;
                }
            }
            continue;
        }

        match character {
            '\'' => single_quote = true,
            '"' => double_quote = true,
            '-' if chars.peek().is_some_and(|(_, next)| *next == '-') => {
                chars.next();
                line_comment = true;
            }
            '/' if chars.peek().is_some_and(|(_, next)| *next == '*') => {
                chars.next();
                block_comment = true;
            }
            ';' => return Some(index),
            _ => {}
        }
    }

    None
}

fn starts_with_read_query(sql: &str) -> bool {
    matches!(first_keyword(sql).as_deref(), Some("select" | "with"))
}

fn first_keyword(sql: &str) -> Option<String> {
    let mut remaining = sql.trim_start();

    loop {
        if let Some(after_comment) = remaining.strip_prefix("--") {
            let (_, after_line) = after_comment.split_once('\n')?;
            remaining = after_line.trim_start();
            continue;
        }

        if let Some(after_open) = remaining.strip_prefix("/*") {
            let (_, after_comment) = after_open.split_once("*/")?;
            remaining = after_comment.trim_start();
            continue;
        }

        break;
    }

    let keyword = remaining
        .chars()
        .take_while(|character| character.is_ascii_alphabetic())
        .collect::<String>()
        .to_ascii_lowercase();

    if keyword.is_empty() {
        None
    } else {
        Some(keyword)
    }
}

fn is_redundant_property(property: &str) -> bool {
    matches!(property, "title" | "type")
}

fn user_sql_error(error: rusqlite::Error) -> String {
    let message = error.to_string();
    if message.contains("attempt to write a readonly database")
        || message.contains("not authorized")
        || message.contains("readonly")
    {
        String::from(READ_ONLY_ERROR)
    } else {
        message
    }
}

pub fn default_query(catalog: &SqlCatalog, selected_collection: Option<&str>) -> String {
    let table = selected_collection
        .and_then(|collection_id| catalog.table_for_collection(collection_id))
        .or_else(|| catalog.tables.first());

    table.map_or_else(
        || {
            String::from(
                "-- Abra uma pasta com Collections para consultar a projeção SQLite descartável.",
            )
        },
        |table| {
            format!(
                "SELECT *\nFROM {}\nLIMIT 100;",
                quote_identifier(table.name.as_str())
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Collection, Document, DocumentMetadata};
    use std::{ffi::OsString, path::PathBuf};

    #[test]
    fn collection_becomes_table_and_standard_columns_exist() {
        let documents = vec![doc("Projects/carf.md", "CARF", "project", [])];
        let projection = projection(&documents);
        let table = &projection.catalog().tables[0];

        assert_eq!(table.name, "projects");
        assert_eq!(column_names(table), ["title", "_path", "_file_name"]);
    }

    #[test]
    fn normalizes_table_and_column_names() {
        assert_eq!(normalize_identifier("Meeting Notes"), "meeting_notes");
        assert_eq!(normalize_identifier("created-at"), "created_at");
        assert_eq!(normalize_identifier("Repository URL"), "repository_url");
        assert_eq!(normalize_identifier("123 value"), "_123_value");
    }

    #[test]
    fn table_name_collisions_are_deterministic() {
        let documents = vec![
            doc_with_collection("a.md", "A", "meeting_notes", "Meeting Notes", []),
            doc_with_collection("b.md", "B", "meeting-notes", "Meeting Notes", []),
        ];
        let collections = collections_from_documents(&documents);
        let projection = SqlProjection::build(&documents, &collections).unwrap();

        assert_eq!(
            projection
                .catalog()
                .tables
                .iter()
                .map(|table| table.name.as_str())
                .collect::<Vec<_>>(),
            ["meeting_notes", "meeting_notes_2"]
        );
    }

    #[test]
    fn property_columns_normalize_and_collide_deterministically() {
        let documents = vec![doc(
            "Projects/carf.md",
            "CARF",
            "project",
            [
                ("Repository URL", PropertyValue::String(String::from("a"))),
                ("repository-url", PropertyValue::String(String::from("b"))),
            ],
        )];
        let projection = projection(&documents);

        assert_eq!(
            column_names(&projection.catalog().tables[0]),
            [
                "title",
                "_path",
                "_file_name",
                "repository_url",
                "repository_url_2"
            ]
        );
    }

    #[test]
    fn projects_types_and_nulls_into_sql() {
        let documents = vec![
            doc(
                "Projects/carf.md",
                "CARF",
                "project",
                [
                    ("status", PropertyValue::String(String::from("active"))),
                    ("priority", PropertyValue::Number(String::from("42"))),
                    ("score", PropertyValue::Number(String::from("7.5"))),
                    ("published", PropertyValue::Bool(true)),
                    (
                        "tags",
                        PropertyValue::Array(vec![
                            PropertyValue::String(String::from("rust")),
                            PropertyValue::String(String::from("typescript")),
                        ]),
                    ),
                    (
                        "meta",
                        PropertyValue::Object(BTreeMap::from([(
                            String::from("owner"),
                            PropertyValue::String(String::from("Sergio")),
                        )])),
                    ),
                ],
            ),
            doc(
                "Projects/cvm.md",
                "CVM",
                "project",
                [("published", PropertyValue::Bool(false))],
            ),
        ];
        let projection = projection(&documents);
        let result = projection
            .execute_read(
                "SELECT title, status, priority, score, published, tags, meta, _path, _file_name
                 FROM projects
                 ORDER BY title",
                DEFAULT_RESULT_LIMIT,
            )
            .unwrap();

        assert_eq!(result.rows.len(), 2);
        assert_eq!(result.rows[0][0], SqlValue::Text(String::from("CARF")));
        assert_eq!(result.rows[0][2], SqlValue::Integer(42));
        assert_eq!(result.rows[0][3], SqlValue::Real(7.5));
        assert_eq!(result.rows[0][4], SqlValue::Integer(1));
        assert_eq!(
            result.rows[0][5],
            SqlValue::Text(String::from("[\"rust\",\"typescript\"]"))
        );
        assert_eq!(
            result.rows[0][6],
            SqlValue::Text(String::from("{\"owner\":\"Sergio\"}"))
        );
        assert_eq!(
            result.rows[0][7],
            SqlValue::Text(String::from("Projects/carf.md"))
        );
        assert_eq!(result.rows[0][8], SqlValue::Text(String::from("carf.md")));
        assert_eq!(result.rows[1][1], SqlValue::Null);
        assert_eq!(result.rows[1][4], SqlValue::Integer(0));
    }

    #[test]
    fn select_where_limit_count_group_by_join_and_alias_work() {
        let documents = vec![
            doc(
                "Projects/carf.md",
                "CARF",
                "project",
                [("status", PropertyValue::String(String::from("active")))],
            ),
            doc(
                "Projects/cvm.md",
                "CVM",
                "project",
                [("status", PropertyValue::String(String::from("active")))],
            ),
            doc(
                "Meetings/m1.md",
                "Revisao",
                "meeting",
                [("project", PropertyValue::String(String::from("CARF")))],
            ),
        ];
        let projection = projection(&documents);

        assert_eq!(
            projection
                .execute_read(
                    "SELECT title FROM projects WHERE status = 'active' LIMIT 1",
                    DEFAULT_RESULT_LIMIT
                )
                .unwrap()
                .rows
                .len(),
            1
        );
        assert_eq!(
            projection
                .execute_read(
                    "SELECT COUNT(*) AS total FROM projects",
                    DEFAULT_RESULT_LIMIT
                )
                .unwrap()
                .rows[0][0],
            SqlValue::Integer(2)
        );
        assert_eq!(
            projection
                .execute_read(
                    "SELECT status, COUNT(*) AS total FROM projects GROUP BY status",
                    DEFAULT_RESULT_LIMIT
                )
                .unwrap()
                .rows[0][1],
            SqlValue::Integer(2)
        );
        assert_eq!(
            projection
                .execute_read(
                    "SELECT p.title AS project_title, m.title AS meeting_title
                     FROM projects p
                     JOIN meetings m ON m.project = p.title",
                    DEFAULT_RESULT_LIMIT
                )
                .unwrap()
                .rows[0],
            vec![
                SqlValue::Text(String::from("CARF")),
                SqlValue::Text(String::from("Revisao"))
            ]
        );
    }

    #[test]
    fn numeric_order_by_is_numeric() {
        let documents = [1, 2, 10, 100]
            .into_iter()
            .rev()
            .map(|priority| {
                doc(
                    format!("Tasks/task-{priority}.md"),
                    format!("Task {priority}"),
                    "task",
                    [("priority", PropertyValue::Number(priority.to_string()))],
                )
            })
            .collect::<Vec<_>>();
        let projection = projection(&documents);

        let values = projection
            .execute_read(
                "SELECT priority FROM tasks ORDER BY priority",
                DEFAULT_RESULT_LIMIT,
            )
            .unwrap()
            .rows
            .into_iter()
            .map(|row| row[0].clone())
            .collect::<Vec<_>>();

        assert_eq!(
            values,
            vec![
                SqlValue::Integer(1),
                SqlValue::Integer(2),
                SqlValue::Integer(10),
                SqlValue::Integer(100)
            ]
        );
    }

    #[test]
    fn missing_properties_are_null() {
        let documents = vec![
            doc("Projects/carf.md", "CARF", "project", []),
            doc(
                "Projects/cvm.md",
                "CVM",
                "project",
                [("status", PropertyValue::String(String::from("active")))],
            ),
        ];
        let projection = projection(&documents);

        let result = projection
            .execute_read(
                "SELECT title FROM projects WHERE status IS NULL",
                DEFAULT_RESULT_LIMIT,
            )
            .unwrap();

        assert_eq!(result.rows[0][0], SqlValue::Text(String::from("CARF")));
    }

    #[test]
    fn invalid_column_returns_friendly_error() {
        let documents = vec![doc("Projects/carf.md", "CARF", "project", [])];
        let projection = projection(&documents);

        let error = projection
            .execute_read("SELECT banana FROM projects", DEFAULT_RESULT_LIMIT)
            .unwrap_err();

        assert!(error.message.contains("no such column: banana"));
    }

    #[test]
    fn writes_and_multi_statement_are_rejected() {
        let documents = vec![doc("Projects/carf.md", "CARF", "project", [])];
        let projection = projection(&documents);

        for sql in [
            "UPDATE projects SET title = 'x'",
            "DELETE FROM projects",
            "INSERT INTO projects(title) VALUES ('x')",
            "DROP TABLE projects",
            "VACUUM",
            "PRAGMA query_only = OFF",
        ] {
            assert_eq!(
                projection
                    .execute_read(sql, DEFAULT_RESULT_LIMIT)
                    .unwrap_err()
                    .message,
                READ_ONLY_ERROR
            );
        }
        assert_eq!(
            projection
                .execute_read(
                    "SELECT * FROM projects; DROP TABLE projects;",
                    DEFAULT_RESULT_LIMIT
                )
                .unwrap_err()
                .message,
            "MDB-009 aceita apenas uma statement SQL por execução."
        );
    }

    #[test]
    fn with_select_is_allowed_but_write_cte_fails_closed() {
        let documents = vec![doc("Projects/carf.md", "CARF", "project", [])];
        let projection = projection(&documents);

        assert_eq!(
            projection
                .execute_read(
                    "WITH selected AS (SELECT title FROM projects) SELECT * FROM selected",
                    DEFAULT_RESULT_LIMIT
                )
                .unwrap()
                .rows[0][0],
            SqlValue::Text(String::from("CARF"))
        );
    }

    #[test]
    fn rebuild_updates_and_workspace_change_drops_old_state() {
        let old = vec![doc(
            "Tasks/task-42.md",
            "Task 42",
            "task",
            [("priority", PropertyValue::Number(String::from("42")))],
        )];
        let new = vec![doc(
            "Tasks/task-42.md",
            "Task 42",
            "task",
            [("priority", PropertyValue::Number(String::from("999")))],
        )];
        let other_workspace = vec![doc("People/sergio.md", "Sergio", "person", [])];

        assert_eq!(
            projection(&new)
                .execute_read(
                    "SELECT priority FROM tasks WHERE title = 'Task 42'",
                    DEFAULT_RESULT_LIMIT
                )
                .unwrap()
                .rows[0][0],
            SqlValue::Integer(999)
        );
        assert!(projection(&other_workspace)
            .execute_read("SELECT * FROM tasks", DEFAULT_RESULT_LIMIT)
            .is_err());
        assert_eq!(
            projection(&old)
                .execute_read("SELECT priority FROM tasks", DEFAULT_RESULT_LIMIT)
                .unwrap()
                .rows[0][0],
            SqlValue::Integer(42)
        );
    }

    #[test]
    fn unicode_values_survive_and_identifiers_become_safe_ascii() {
        let documents = vec![doc(
            "Projetos/visao.md",
            "Visão",
            "projeto",
            [("responsável", PropertyValue::String(String::from("Sérgio")))],
        )];
        let projection = projection(&documents);

        assert_eq!(projection.catalog().tables[0].name, "projetos");
        assert_eq!(
            projection
                .execute_read("SELECT respons_vel FROM projetos", DEFAULT_RESULT_LIMIT)
                .unwrap()
                .rows[0][0],
            SqlValue::Text(String::from("Sérgio"))
        );
    }

    #[test]
    fn handles_one_thousand_documents_and_result_truncation() {
        let documents = (0..1_000)
            .map(|index| {
                doc(
                    format!("Tasks/task-{index}.md"),
                    format!("Task {index}"),
                    "task",
                    [("priority", PropertyValue::Number(index.to_string()))],
                )
            })
            .collect::<Vec<_>>();
        let projection = projection(&documents);

        assert_eq!(
            projection
                .execute_read("SELECT COUNT(*) FROM tasks", DEFAULT_RESULT_LIMIT)
                .unwrap()
                .rows[0][0],
            SqlValue::Integer(1_000)
        );
        let result = projection.execute_read("SELECT * FROM tasks", 10).unwrap();
        assert_eq!(result.rows.len(), 10);
        assert!(result.truncated);
    }

    fn projection(documents: &[Document]) -> SqlProjection {
        SqlProjection::build(documents, &collections_from_documents(documents)).unwrap()
    }

    fn doc<const N: usize>(
        path: impl Into<String>,
        title: impl Into<String>,
        collection_id: impl Into<String>,
        properties: [(&str, PropertyValue); N],
    ) -> Document {
        let collection_id = collection_id.into();
        let display_name = format!("{collection_id}s");
        doc_with_collection(path, title, collection_id, display_name, properties)
    }

    fn doc_with_collection<const N: usize>(
        path: impl Into<String>,
        title: impl Into<String>,
        collection_id: impl Into<String>,
        document_type: impl Into<String>,
        properties: [(&str, PropertyValue); N],
    ) -> Document {
        let relative_path = PathBuf::from(path.into());
        let file_name = relative_path
            .file_name()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| OsString::from("document.md"));
        Document {
            path: PathBuf::from("/workspace").join(&relative_path),
            relative_path,
            file_name,
            metadata: DocumentMetadata {
                file_size: None,
                modified: None,
            },
            title: title.into(),
            source_content: Some(String::new()),
            markdown_content: String::new(),
            properties: properties
                .into_iter()
                .map(|(key, value)| (String::from(key), value))
                .collect(),
            document_type: Some(document_type.into()),
            collection_id: collection_id.into(),
            warnings: Vec::new(),
        }
    }

    fn collections_from_documents(documents: &[Document]) -> Vec<Collection> {
        let mut collections = BTreeMap::<String, String>::new();
        for document in documents {
            collections.insert(
                document.collection_id.clone(),
                document
                    .document_type
                    .clone()
                    .unwrap_or_else(|| document.collection_id.clone()),
            );
        }
        collections
            .into_iter()
            .map(|(id, display_name)| Collection {
                id,
                display_name,
                document_count: 1,
            })
            .collect()
    }

    fn column_names(table: &SqlTable) -> Vec<&str> {
        table
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect()
    }
}
