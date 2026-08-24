use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::PathBuf,
    time::{Duration, Instant},
};

use rusqlite::{
    types::{ToSqlOutput, Value, ValueRef},
    Connection, ToSql,
};
use sqlparser::{
    ast::{AssignmentTarget, Expr, ObjectName, Statement, TableFactor},
    dialect::SQLiteDialect,
    parser::Parser,
};

use crate::{
    content_fingerprint, patch_frontmatter_properties, BulkEditChangeStatus, BulkEditFileChange,
    BulkEditPlan, BulkEditValue, FrontmatterPatchOutcome, SchemaCatalog, SchemaType,
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
pub struct SqlWritePlan {
    pub sql: String,
    pub table: String,
    pub collection_id: String,
    pub collection_display_name: String,
    pub matched_rows: usize,
    pub affected_rows: usize,
    pub no_change_rows: usize,
    pub warnings: Vec<String>,
    pub mutation_plan: BulkEditPlan,
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
            if starts_with_update_query(statement_sql) {
                return Err(SqlError::new(
                    "Consultas de escrita exigem o modo Atualização.",
                ));
            }
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

    pub fn preview_update(
        query: &str,
        documents: &[Document],
        collections: &[Collection],
        editor: &crate::EditorState,
        schema_catalog: &SchemaCatalog,
    ) -> Result<SqlWritePlan, SqlError> {
        let sql = query.trim();
        if sql.is_empty() {
            return Err(SqlError::new("Digite uma atualização SQL."));
        }
        let statement_sql = single_statement_with_message(sql, "Execute uma atualização por vez.")?;
        let parsed = parse_update_statement(statement_sql)?;
        let projection = SqlProjection::build_writable(documents, collections)?;
        projection.simulate_update(statement_sql, parsed, documents, editor, schema_catalog)
    }

    fn build_writable(
        documents: &[Document],
        collections: &[Collection],
    ) -> Result<Self, SqlError> {
        let catalog = build_catalog(documents, collections);
        let connection = Connection::open_in_memory().map_err(SqlError::from)?;
        connection
            .execute_batch("PRAGMA trusted_schema = OFF;")
            .map_err(SqlError::from)?;

        for table in &catalog.tables {
            create_table(&connection, table)?;
            insert_documents(&connection, table, documents)?;
        }

        Ok(Self {
            connection,
            catalog,
        })
    }

    fn simulate_update(
        &self,
        sql: &str,
        parsed: ParsedUpdate,
        documents: &[Document],
        editor: &crate::EditorState,
        schema_catalog: &SchemaCatalog,
    ) -> Result<SqlWritePlan, SqlError> {
        let Some(table) = self.catalog.table_by_name(&parsed.table) else {
            return Err(SqlError::new(format!(
                "Tabela '{}' não corresponde a uma Collection.",
                parsed.table
            )));
        };
        let target_columns = validate_update_columns(table, &parsed.columns)?;
        let before_rows = select_rows(
            &self.connection,
            table,
            &target_columns,
            parsed.selection.as_ref(),
        )?;

        self.connection
            .execute_batch("SAVEPOINT flokinmd_sql_update_preview;")
            .map_err(SqlError::from)?;
        let simulation_result = (|| {
            self.connection.execute(sql, []).map_err(SqlError::from)?;
            let matched_paths = before_rows
                .iter()
                .map(|row| row.path.clone())
                .collect::<BTreeSet<_>>();
            select_rows(&self.connection, table, &target_columns, None).map(|rows| {
                rows.into_iter()
                    .filter(|row| matched_paths.contains(&row.path))
                    .collect::<Vec<_>>()
            })
        })();
        let rollback_result = self.connection.execute_batch(
            "ROLLBACK TO flokinmd_sql_update_preview; RELEASE flokinmd_sql_update_preview;",
        );
        if let Err(error) = rollback_result {
            return Err(SqlError::from(error));
        }
        let after_rows = simulation_result?;

        build_sql_write_plan(SqlWritePlanBuild {
            sql,
            table,
            columns: target_columns.as_slice(),
            before_rows,
            after_rows,
            documents,
            editor,
            schema_catalog,
            no_where: parsed.selection.is_none(),
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

    pub fn table_by_name(&self, name: &str) -> Option<&SqlTable> {
        self.tables.iter().find(|table| table.name == name)
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
    single_statement_with_message(sql, "MDB-009 aceita apenas uma statement SQL por execução.")
}

fn single_statement_with_message<'a>(sql: &'a str, message: &str) -> Result<&'a str, SqlError> {
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
        Err(SqlError::new(message))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedUpdate {
    table: String,
    columns: Vec<String>,
    selection: Option<String>,
}

fn parse_update_statement(sql: &str) -> Result<ParsedUpdate, SqlError> {
    let dialect = SQLiteDialect {};
    let statements = Parser::parse_sql(&dialect, sql)
        .map_err(|error| SqlError::new(format!("Erro de sintaxe SQL: {error}")))?;
    let [statement] = statements.as_slice() else {
        return Err(SqlError::new("Execute uma atualização por vez."));
    };

    match statement {
        Statement::Update {
            table,
            assignments,
            selection,
            returning,
            ..
        } => {
            if returning.as_ref().is_some_and(|items| !items.is_empty()) {
                return Err(SqlError::new(
                    "UPDATE ... RETURNING ainda não é suportado. Use o Preview para revisar o resultado.",
                ));
            }
            let table_name = update_table_name(table)?;
            if assignments.is_empty() {
                return Err(SqlError::new("Informe pelo menos uma coluna em SET."));
            }
            let mut columns = Vec::new();
            for assignment in assignments {
                match &assignment.target {
                    AssignmentTarget::ColumnName(name) => {
                        columns.push(object_name_last_part(name)?);
                    }
                    AssignmentTarget::Tuple(_) => {
                        return Err(SqlError::new(
                            "SET com tupla ainda não é suportado nesta milestone.",
                        ));
                    }
                }
            }
            columns.sort();
            columns.dedup();
            Ok(ParsedUpdate {
                table: table_name,
                columns,
                selection: selection.as_ref().map(Expr::to_string),
            })
        }
        Statement::Query(_) => Err(SqlError::new(
            "Modo Atualização aceita apenas UPDATE nesta milestone.",
        )),
        _ => Err(SqlError::new(
            "Somente UPDATE é suportado no modo Atualização nesta milestone.",
        )),
    }
}

fn update_table_name(table: &sqlparser::ast::TableWithJoins) -> Result<String, SqlError> {
    if !table.joins.is_empty() {
        return Err(SqlError::new(
            "UPDATE com JOIN ainda não é suportado nesta milestone.",
        ));
    }
    match &table.relation {
        TableFactor::Table { name, .. } => {
            if name.0.len() != 1 {
                return Err(SqlError::new(
                    "Use o nome simples da tabela da Collection no UPDATE.",
                ));
            }
            object_name_last_part(name)
        }
        _ => Err(SqlError::new(
            "Somente UPDATE direto em tabela de Collection é suportado.",
        )),
    }
}

fn object_name_last_part(name: &ObjectName) -> Result<String, SqlError> {
    name.0
        .last()
        .map(|ident| ident.value.clone())
        .ok_or_else(|| SqlError::new("Identificador SQL inválido."))
}

#[derive(Debug, Clone, PartialEq)]
struct SimulatedRow {
    path: String,
    values: BTreeMap<String, SqlValue>,
}

fn validate_update_columns<'a>(
    table: &'a SqlTable,
    columns: &[String],
) -> Result<Vec<&'a SqlColumn>, SqlError> {
    let mut validated = Vec::new();
    for column_name in columns {
        let Some(column) = table
            .columns
            .iter()
            .find(|column| column.name == *column_name)
        else {
            return Err(SqlError::new(format!(
                "Coluna '{}' não existe em {}.",
                column_name, table.display_name
            )));
        };
        if column.name == "_path" || column.name == "_file_name" {
            return Err(SqlError::new(format!(
                "{} é uma coluna interna e não pode ser modificada.",
                column.name
            )));
        }
        if column.name == "title" && column.source_property.is_none() {
            return Err(SqlError::new(
                "title possui semântica estrutural e ainda não pode ser alterado via SQL.",
            ));
        }
        if column.source_property.is_none() {
            return Err(SqlError::new(format!(
                "{} é uma coluna interna e não pode ser modificada.",
                column.name
            )));
        }
        if column.value_type == SqlColumnType::Json {
            return Err(SqlError::new(
                "Atualização SQL de Array/Object ainda não é suportada.",
            ));
        }
        validated.push(column);
    }
    Ok(validated)
}

fn select_rows(
    connection: &Connection,
    table: &SqlTable,
    columns: &[&SqlColumn],
    selection: Option<&String>,
) -> Result<Vec<SimulatedRow>, SqlError> {
    let mut select_columns = vec![quote_identifier("_path")];
    select_columns.extend(columns.iter().map(|column| quote_identifier(&column.name)));
    let mut sql = format!(
        "SELECT {} FROM {}",
        select_columns.join(", "),
        quote_identifier(&table.name)
    );
    if let Some(selection) = selection {
        sql.push_str(" WHERE ");
        sql.push_str(selection);
    }
    sql.push_str(" ORDER BY ");
    sql.push_str(quote_identifier("_path").as_str());

    let mut statement = connection.prepare(&sql).map_err(SqlError::from)?;
    let rows = statement
        .query_map([], |row| {
            let path = match row.get_ref(0)? {
                ValueRef::Text(value) => String::from_utf8_lossy(value).into_owned(),
                other => sql_value(other).display_value(None),
            };
            let mut values = BTreeMap::new();
            for (index, column) in columns.iter().enumerate() {
                values.insert(column.name.clone(), sql_value(row.get_ref(index + 1)?));
            }
            Ok(SimulatedRow { path, values })
        })
        .map_err(SqlError::from)?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(SqlError::from)?);
    }
    Ok(result)
}

struct SqlWritePlanBuild<'a> {
    sql: &'a str,
    table: &'a SqlTable,
    columns: &'a [&'a SqlColumn],
    before_rows: Vec<SimulatedRow>,
    after_rows: Vec<SimulatedRow>,
    documents: &'a [Document],
    editor: &'a crate::EditorState,
    schema_catalog: &'a SchemaCatalog,
    no_where: bool,
}

fn build_sql_write_plan(input: SqlWritePlanBuild<'_>) -> Result<SqlWritePlan, SqlError> {
    let SqlWritePlanBuild {
        sql,
        table,
        columns,
        before_rows,
        after_rows,
        documents,
        editor,
        schema_catalog,
        no_where,
    } = input;
    let documents_by_relative = documents
        .iter()
        .map(|document| (document.relative_path.display().to_string(), document))
        .collect::<BTreeMap<_, _>>();
    let after_by_path = after_rows
        .into_iter()
        .map(|row| (row.path.clone(), row))
        .collect::<BTreeMap<_, _>>();
    let mut changes = Vec::new();

    for before in before_rows {
        let Some(after) = after_by_path.get(&before.path) else {
            continue;
        };
        let Some(document) = documents_by_relative.get(&before.path) else {
            changes.push(sql_blocked_change(
                PathBuf::from(&before.path),
                PathBuf::from(&before.path),
                0,
                "Documento ausente.",
            ));
            continue;
        };
        let Some(source) = document.source_content.as_deref() else {
            changes.push(sql_blocked_change(
                document.path.clone(),
                document.relative_path.clone(),
                0,
                "Não foi possível ler o conteúdo do arquivo.",
            ));
            continue;
        };
        let fingerprint = content_fingerprint(source);
        if let Some(tab) = editor.tab(&document.path) {
            if tab.dirty {
                changes.push(sql_blocked_change(
                    document.path.clone(),
                    document.relative_path.clone(),
                    fingerprint,
                    "Arquivo possui alterações não salvas.",
                ));
                continue;
            }
            if tab.external_conflict.is_some() {
                changes.push(sql_blocked_change(
                    document.path.clone(),
                    document.relative_path.clone(),
                    fingerprint,
                    "Arquivo possui conflito com alteração externa.",
                ));
                continue;
            }
        }

        let mut frontmatter_changes = Vec::new();
        for column in columns {
            let before_value = before.values.get(&column.name).unwrap_or(&SqlValue::Null);
            let after_value = after.values.get(&column.name).unwrap_or(&SqlValue::Null);
            if before_value == after_value {
                continue;
            }
            let property = column
                .source_property
                .as_ref()
                .expect("validated source property");
            let value = markdown_value_for_sql(table, column, after_value, schema_catalog)?;
            frontmatter_changes.push((property.clone(), Some(value)));
        }

        if frontmatter_changes.is_empty() {
            changes.push(BulkEditFileChange {
                path: document.path.clone(),
                relative_path: document.relative_path.clone(),
                original_fingerprint: fingerprint,
                before: None,
                after: None,
                property_changes: Vec::new(),
                status: BulkEditChangeStatus::NoChange,
                reason: Some(String::from("Sem alteração necessária.")),
                new_content: None,
            });
            continue;
        }

        match patch_frontmatter_properties(source, frontmatter_changes.as_slice()) {
            Ok(FrontmatterPatchOutcome::Changed {
                property_changes,
                content,
            }) => changes.push(BulkEditFileChange {
                path: document.path.clone(),
                relative_path: document.relative_path.clone(),
                original_fingerprint: fingerprint,
                before: property_changes
                    .first()
                    .and_then(|change| change.before.clone()),
                after: property_changes
                    .first()
                    .and_then(|change| change.after.clone()),
                property_changes,
                status: BulkEditChangeStatus::Changed,
                reason: None,
                new_content: Some(content),
            }),
            Ok(FrontmatterPatchOutcome::NoChange { property_changes }) => {
                changes.push(BulkEditFileChange {
                    path: document.path.clone(),
                    relative_path: document.relative_path.clone(),
                    original_fingerprint: fingerprint,
                    before: None,
                    after: None,
                    property_changes,
                    status: BulkEditChangeStatus::NoChange,
                    reason: Some(String::from("Sem alteração necessária.")),
                    new_content: None,
                });
            }
            Err(message) => changes.push(sql_unsupported_change(
                document.path.clone(),
                document.relative_path.clone(),
                fingerprint,
                &message,
            )),
        }
    }

    let matched_rows = changes.len();
    let affected_rows = changes
        .iter()
        .filter(|change| change.status == BulkEditChangeStatus::Changed)
        .count();
    let no_change_rows = changes
        .iter()
        .filter(|change| change.status == BulkEditChangeStatus::NoChange)
        .count();
    let mut warnings = Vec::new();
    if no_where && matched_rows > 0 {
        warnings.push(format!(
            "Esta atualização afeta todos os documentos da Collection {}.",
            table.display_name
        ));
    }
    let mutation_plan = BulkEditPlan {
        collection_id: table.collection_id.clone(),
        operation: crate::BulkEditOperation::SetProperty {
            property: String::from("__sql_update__"),
            value: BulkEditValue::Null,
        },
        changes,
        warnings: warnings.clone(),
    };
    Ok(SqlWritePlan {
        sql: sql.to_owned(),
        table: table.name.clone(),
        collection_id: table.collection_id.clone(),
        collection_display_name: table.display_name.clone(),
        matched_rows,
        affected_rows,
        no_change_rows,
        warnings,
        mutation_plan,
    })
}

fn sql_blocked_change(
    path: PathBuf,
    relative_path: PathBuf,
    original_fingerprint: u64,
    reason: &str,
) -> BulkEditFileChange {
    BulkEditFileChange {
        path,
        relative_path,
        original_fingerprint,
        before: None,
        after: None,
        property_changes: Vec::new(),
        status: BulkEditChangeStatus::Blocked,
        reason: Some(reason.to_owned()),
        new_content: None,
    }
}

fn sql_unsupported_change(
    path: PathBuf,
    relative_path: PathBuf,
    original_fingerprint: u64,
    reason: &str,
) -> BulkEditFileChange {
    BulkEditFileChange {
        path,
        relative_path,
        original_fingerprint,
        before: None,
        after: None,
        property_changes: Vec::new(),
        status: BulkEditChangeStatus::Unsupported,
        reason: Some(reason.to_owned()),
        new_content: None,
    }
}

fn markdown_value_for_sql(
    table: &SqlTable,
    column: &SqlColumn,
    value: &SqlValue,
    schema_catalog: &SchemaCatalog,
) -> Result<BulkEditValue, SqlError> {
    let property = column.source_property.as_deref().unwrap_or(&column.name);
    let schema_field = schema_catalog
        .collection(&table.collection_id)
        .and_then(|schema| schema.fields.iter().find(|field| field.name == property));
    let expected_type = schema_field.map(|field| field.field_type);
    if matches!(expected_type, Some(SchemaType::Array | SchemaType::Object)) {
        return Err(SqlError::new(
            "Atualização SQL de Array/Object ainda não é suportada.",
        ));
    }
    if matches!(expected_type, Some(SchemaType::Mixed | SchemaType::Unknown)) {
        return Err(SqlError::new(
            "Atualização SQL de campo Mixed ainda não é segura nesta milestone.",
        ));
    }

    let inferred = sql_column_to_schema_type(column.value_type);
    let target_type = expected_type.unwrap_or(inferred);
    let converted = match (target_type, value) {
        (_, SqlValue::Null) => BulkEditValue::Null,
        (SchemaType::String, SqlValue::Text(value)) => BulkEditValue::String(value.clone()),
        (SchemaType::Integer, SqlValue::Integer(value)) => {
            BulkEditValue::Integer(value.to_string())
        }
        (SchemaType::Float, SqlValue::Integer(value)) => BulkEditValue::Float(value.to_string()),
        (SchemaType::Float, SqlValue::Real(value)) => BulkEditValue::Float(value.to_string()),
        (SchemaType::Boolean, SqlValue::Integer(0)) => BulkEditValue::Boolean(false),
        (SchemaType::Boolean, SqlValue::Integer(1)) => BulkEditValue::Boolean(true),
        (SchemaType::Relation, SqlValue::Text(value)) if is_relation_literal(value) => {
            BulkEditValue::String(value.clone())
        }
        (SchemaType::String, SqlValue::Integer(value))
            if column.value_type == SqlColumnType::Text =>
        {
            BulkEditValue::String(value.to_string())
        }
        (SchemaType::String, SqlValue::Real(value)) if column.value_type == SqlColumnType::Text => {
            BulkEditValue::String(value.to_string())
        }
        (expected, actual) => {
            return Err(SqlError::new(format!(
                "{}.{} espera {}, mas a atualização produziria {}.",
                table.display_name,
                property,
                expected.label(),
                sql_value_schema_type(actual).label()
            )));
        }
    };

    if let Some(field) = schema_field {
        if field.declared
            && !matches!(converted, BulkEditValue::Null)
            && !schema_type_accepts_for_sql(field.field_type, converted.schema_type())
        {
            return Err(SqlError::new(format!(
                "{}.{} espera {}, mas a atualização produziria {}.",
                table.display_name,
                property,
                field.field_type.label(),
                converted.schema_type().label()
            )));
        }
        if field.declared
            && field.required
            && matches!(converted, BulkEditValue::Null)
            && !field.nullable
        {
            return Err(SqlError::new(format!(
                "{} é obrigatório no schema explícito.",
                property
            )));
        }
    }

    Ok(converted)
}

fn sql_column_to_schema_type(column_type: SqlColumnType) -> SchemaType {
    match column_type {
        SqlColumnType::Text => SchemaType::String,
        SqlColumnType::Integer => SchemaType::Integer,
        SqlColumnType::Real => SchemaType::Float,
        SqlColumnType::Boolean => SchemaType::Boolean,
        SqlColumnType::Json => SchemaType::Object,
        SqlColumnType::Null => SchemaType::Null,
    }
}

fn sql_value_schema_type(value: &SqlValue) -> SchemaType {
    match value {
        SqlValue::Null => SchemaType::Null,
        SqlValue::Integer(_) => SchemaType::Integer,
        SqlValue::Real(_) => SchemaType::Float,
        SqlValue::Text(_) => SchemaType::String,
    }
}

fn schema_type_accepts_for_sql(expected: SchemaType, actual: SchemaType) -> bool {
    matches!(
        (expected, actual),
        (SchemaType::String, SchemaType::String)
            | (SchemaType::Integer, SchemaType::Integer)
            | (SchemaType::Float, SchemaType::Float)
            | (SchemaType::Boolean, SchemaType::Boolean)
            | (SchemaType::Relation, SchemaType::String)
            | (SchemaType::Relation, SchemaType::Relation)
            | (SchemaType::Float, SchemaType::Integer)
            | (_, SchemaType::Null)
    )
}

fn is_relation_literal(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.starts_with("[[") && trimmed.ends_with("]]") && trimmed.len() > 4
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

fn starts_with_update_query(sql: &str) -> bool {
    matches!(first_keyword(sql).as_deref(), Some("update"))
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
    use crate::{
        apply_bulk_edit_plan, Collection, Document, DocumentMetadata, EditorState, RelationIndex,
        SchemaCatalog,
    };
    use std::{ffi::OsString, fs, path::PathBuf};

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

        assert_eq!(
            projection
                .execute_read("UPDATE projects SET title = 'x'", DEFAULT_RESULT_LIMIT)
                .unwrap_err()
                .message,
            "Consultas de escrita exigem o modo Atualização."
        );
        for sql in [
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
    fn update_mode_parser_and_validation_rejects_unsupported_statements() {
        let documents = vec![doc_with_source(
            "Projects/carf.md",
            "CARF",
            "project",
            [("status", PropertyValue::String(String::from("active")))],
            "---\nstatus: active\n---\n# CARF\n",
        )];
        let collections = collections_from_documents(&documents);
        let schema = schema_catalog(&documents, &collections);

        assert!(SqlProjection::preview_update(
            "UPDATE projects SET status = 'archived'",
            &documents,
            &collections,
            &EditorState::default(),
            &schema,
        )
        .is_ok());
        assert_eq!(
            SqlProjection::preview_update(
                "UPDATE projects SET status = 'a'; UPDATE projects SET status = 'b';",
                &documents,
                &collections,
                &EditorState::default(),
                &schema,
            )
            .unwrap_err()
            .message,
            "Execute uma atualização por vez."
        );
        for sql in [
            "SELECT * FROM projects",
            "INSERT INTO projects(status) VALUES ('x')",
            "DELETE FROM projects",
            "ALTER TABLE projects ADD COLUMN x TEXT",
        ] {
            assert!(SqlProjection::preview_update(
                sql,
                &documents,
                &collections,
                &EditorState::default(),
                &schema,
            )
            .is_err());
        }
        assert!(SqlProjection::preview_update(
            "UPDATE projects SET _path = 'x'",
            &documents,
            &collections,
            &EditorState::default(),
            &schema,
        )
        .unwrap_err()
        .message
        .contains("_path é uma coluna interna"));
        assert!(SqlProjection::preview_update(
            "UPDATE projects SET title = 'x'",
            &documents,
            &collections,
            &EditorState::default(),
            &schema,
        )
        .unwrap_err()
        .message
        .contains("title possui semântica estrutural"));
        assert!(SqlProjection::preview_update(
            "UPDATE missing SET status = 'x'",
            &documents,
            &collections,
            &EditorState::default(),
            &schema,
        )
        .unwrap_err()
        .message
        .contains("não corresponde a uma Collection"));
        assert!(SqlProjection::preview_update(
            "UPDATE projects SET missing = 'x'",
            &documents,
            &collections,
            &EditorState::default(),
            &schema,
        )
        .unwrap_err()
        .message
        .contains("Coluna 'missing' não existe"));
    }

    #[test]
    fn update_preview_simulates_where_arithmetic_multiple_sets_and_rolls_back() {
        let documents = vec![
            doc_with_source(
                "Projects/carf.md",
                "CARF",
                "project",
                [
                    ("status", PropertyValue::String(String::from("active"))),
                    ("priority", PropertyValue::Number(String::from("10"))),
                ],
                "---\nstatus: active\npriority: 10\n---\n# CARF\n",
            ),
            doc_with_source(
                "Projects/cvm.md",
                "CVM",
                "project",
                [
                    ("status", PropertyValue::String(String::from("active"))),
                    ("priority", PropertyValue::Number(String::from("15"))),
                ],
                "---\nstatus: active\npriority: 15\n---\n# CVM\n",
            ),
            doc_with_source(
                "Projects/done.md",
                "Done",
                "project",
                [
                    ("status", PropertyValue::String(String::from("archived"))),
                    ("priority", PropertyValue::Number(String::from("5"))),
                ],
                "---\nstatus: archived\npriority: 5\n---\n# Done\n",
            ),
        ];
        let collections = collections_from_documents(&documents);
        let schema = schema_catalog(&documents, &collections);
        let plan = SqlProjection::preview_update(
            "UPDATE projects SET status = 'archived', priority = priority + 10 WHERE status = 'active'",
            &documents,
            &collections,
            &EditorState::default(),
            &schema,
        )
        .unwrap();

        assert_eq!(plan.matched_rows, 2);
        assert_eq!(plan.affected_rows, 2);
        assert_eq!(plan.mutation_plan.summary().changed, 2);
        assert_eq!(
            plan.mutation_plan.changes[0]
                .property_changes
                .iter()
                .map(|change| change.after.as_deref().unwrap_or(""))
                .collect::<Vec<_>>(),
            vec!["priority: 20", "status: archived"]
        );
        assert_eq!(
            projection(&documents)
                .execute_read(
                    "SELECT status, priority FROM projects WHERE _path = 'Projects/carf.md'",
                    DEFAULT_RESULT_LIMIT
                )
                .unwrap()
                .rows[0],
            vec![
                SqlValue::Text(String::from("active")),
                SqlValue::Integer(10)
            ]
        );
    }

    #[test]
    fn update_preview_handles_no_where_noop_zero_matches_boolean_null_and_relation() {
        let documents = vec![doc_with_source(
            "People/sergio.md",
            "Sergio",
            "person",
            [
                ("active", PropertyValue::Bool(true)),
                ("owner", PropertyValue::String(String::from("[[Ana]]"))),
                (
                    "reviewed_at",
                    PropertyValue::String(String::from("2026-01-01")),
                ),
            ],
            "---\nactive: true\nowner: \"[[Ana]]\"\nreviewed_at: 2026-01-01\n---\n# Sergio\n",
        )];
        let collections = collections_from_documents(&documents);
        let schema = schema_catalog(&documents, &collections);

        let no_where = SqlProjection::preview_update(
            "UPDATE persons SET active = 0",
            &documents,
            &collections,
            &EditorState::default(),
            &schema,
        )
        .unwrap();
        assert_eq!(no_where.warnings.len(), 1);
        assert_eq!(
            no_where.mutation_plan.changes[0].property_changes[0].after,
            Some(String::from("active: false"))
        );

        let null_plan = SqlProjection::preview_update(
            "UPDATE persons SET reviewed_at = NULL WHERE active = 1",
            &documents,
            &collections,
            &EditorState::default(),
            &schema,
        )
        .unwrap();
        assert_eq!(
            null_plan.mutation_plan.changes[0].property_changes[0].after,
            Some(String::from("reviewed_at: null"))
        );

        let relation = SqlProjection::preview_update(
            "UPDATE persons SET owner = '[[Sergio]]'",
            &documents,
            &collections,
            &EditorState::default(),
            &schema,
        )
        .unwrap();
        assert_eq!(
            relation.mutation_plan.changes[0].property_changes[0].after,
            Some(String::from("owner: \"[[Sergio]]\""))
        );

        let noop = SqlProjection::preview_update(
            "UPDATE persons SET active = 1 WHERE active = 1",
            &documents,
            &collections,
            &EditorState::default(),
            &schema,
        )
        .unwrap();
        assert_eq!(noop.matched_rows, 1);
        assert_eq!(noop.affected_rows, 0);
        assert!(!noop.mutation_plan.can_apply());

        let zero = SqlProjection::preview_update(
            "UPDATE persons SET active = 0 WHERE active IS NULL",
            &documents,
            &collections,
            &EditorState::default(),
            &schema,
        )
        .unwrap();
        assert_eq!(zero.matched_rows, 0);
        assert!(!zero.mutation_plan.can_apply());
    }

    #[test]
    fn unsafe_types_are_blocked() {
        let documents = vec![
            doc_with_source(
                "Projects/carf.md",
                "CARF",
                "project",
                [(
                    "tags",
                    PropertyValue::Array(vec![PropertyValue::String(String::from("a"))]),
                )],
                "---\ntags:\n  - a\n---\n# CARF\n",
            ),
            doc_with_source(
                "Projects/cvm.md",
                "CVM",
                "project",
                [("mixed", PropertyValue::String(String::from("x")))],
                "---\nmixed: x\n---\n# CVM\n",
            ),
            doc_with_source(
                "Projects/num.md",
                "Num",
                "project",
                [("mixed", PropertyValue::Number(String::from("10")))],
                "---\nmixed: 10\n---\n# Num\n",
            ),
        ];
        let collections = collections_from_documents(&documents);
        let schema = schema_catalog(&documents, &collections);

        assert!(SqlProjection::preview_update(
            "UPDATE projects SET tags = '[]'",
            &documents,
            &collections,
            &EditorState::default(),
            &schema,
        )
        .unwrap_err()
        .message
        .contains("Array/Object"));
        assert!(SqlProjection::preview_update(
            "UPDATE projects SET mixed = 'y'",
            &documents,
            &collections,
            &EditorState::default(),
            &schema,
        )
        .unwrap_err()
        .message
        .contains("Mixed"));
    }

    #[test]
    fn preview_does_not_write_and_apply_updates_files_for_rebuild() {
        let workspace = temp_workspace();
        workspace.write("Projects/carf.md", "---\nstatus: active\n---\n# CARF\n");
        let path = workspace.path().join("Projects/carf.md");
        let mut document = doc_with_source(
            "Projects/carf.md",
            "CARF",
            "project",
            [("status", PropertyValue::String(String::from("active")))],
            "---\nstatus: active\n---\n# CARF\n",
        );
        document.path = path.clone();
        let documents = vec![document];
        let collections = collections_from_documents(&documents);
        let schema = schema_catalog(&documents, &collections);
        let plan = SqlProjection::preview_update(
            "UPDATE projects SET status = 'archived'",
            &documents,
            &collections,
            &EditorState::default(),
            &schema,
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "---\nstatus: active\n---\n# CARF\n"
        );
        apply_bulk_edit_plan(&plan.mutation_plan).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "---\nstatus: archived\n---\n# CARF\n"
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

    fn schema_catalog(documents: &[Document], collections: &[Collection]) -> SchemaCatalog {
        SchemaCatalog::build(
            documents,
            collections,
            &RelationIndex::build(documents),
            crate::ExplicitSchemaState::Absent,
        )
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

    fn doc_with_source<const N: usize>(
        path: impl Into<String>,
        title: impl Into<String>,
        collection_id: impl Into<String>,
        properties: [(&str, PropertyValue); N],
        source: impl Into<String>,
    ) -> Document {
        let mut document = doc(path, title, collection_id, properties);
        document.source_content = Some(source.into());
        document
    }

    struct TempWorkspace {
        root: PathBuf,
    }

    impl TempWorkspace {
        fn path(&self) -> &std::path::Path {
            &self.root
        }

        fn write(&self, relative: &str, content: &str) {
            let path = self.root.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, content).unwrap();
        }
    }

    impl Drop for TempWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn temp_workspace() -> TempWorkspace {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("flokinmd-sql-test-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        TempWorkspace { root }
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
