use std::collections::{BTreeMap, BTreeSet};

use crate::{SqlCatalog, SqlColumnType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlCompletionContext {
    pub cursor: usize,
    pub replacement_start: usize,
    pub replacement_end: usize,
    pub prefix: String,
    pub qualifier: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlCompletionItem {
    pub label: String,
    pub insert_text: String,
    pub kind: SqlCompletionKind,
    pub detail: String,
    pub replacement_start: usize,
    pub replacement_end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SqlCompletionKind {
    Keyword,
    Table,
    Column,
    Alias,
    Function,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MatchRank {
    ExactPrefix,
    CaseInsensitivePrefix,
    Contains,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionScope {
    General,
    Table,
    Column,
}

const KEYWORDS: &[&str] = &[
    "SELECT",
    "FROM",
    "WHERE",
    "JOIN",
    "LEFT JOIN",
    "INNER JOIN",
    "ON",
    "AS",
    "AND",
    "OR",
    "ORDER BY",
    "GROUP BY",
    "HAVING",
    "LIMIT",
    "OFFSET",
    "DISTINCT",
    "NULL",
    "IS",
    "NOT",
    "IN",
    "LIKE",
    "ASC",
    "DESC",
    "WITH",
];

const FUNCTIONS: &[&str] = &[
    "COUNT", "SUM", "AVG", "MIN", "MAX", "COALESCE", "LOWER", "UPPER", "LENGTH", "ROUND",
];

pub const DEFAULT_SQL_COMPLETION_LIMIT: usize = 50;

pub fn complete_sql(
    catalog: &SqlCatalog,
    sql: &str,
    cursor: usize,
    limit: usize,
) -> Vec<SqlCompletionItem> {
    let context = completion_context(sql, cursor.min(sql.len()));
    let aliases = table_aliases(catalog, sql);
    let scope = completion_scope(sql, context.replacement_start);
    let mut candidates = Vec::new();

    if let Some(qualifier) = context.qualifier.as_deref() {
        if let Some(table_name) = aliases
            .get(&qualifier.to_ascii_lowercase())
            .or_else(|| catalog.table_named(qualifier).map(|table| &table.name))
        {
            if let Some(table) = catalog.table_named(table_name) {
                for column in &table.columns {
                    candidates.push(SqlCompletionItem {
                        label: format!("{qualifier}.{}", column.name),
                        insert_text: format!(
                            "{}.{}",
                            quote_identifier_if_needed(qualifier),
                            quote_identifier_if_needed(&column.name)
                        ),
                        kind: SqlCompletionKind::Column,
                        detail: column.value_type.label().to_owned(),
                        replacement_start: context.replacement_start,
                        replacement_end: context.replacement_end,
                    });
                }
            }
        }
        return ranked(candidates, &context.prefix, limit);
    }

    match scope {
        CompletionScope::Table => {
            add_tables(catalog, &context, &mut candidates);
        }
        CompletionScope::Column => {
            add_columns(catalog, &aliases, &context, &mut candidates);
            add_functions(&context, &mut candidates);
            add_aliases(&aliases, &context, &mut candidates);
            add_keywords(&context, &mut candidates);
        }
        CompletionScope::General => {
            add_keywords(&context, &mut candidates);
            add_tables(catalog, &context, &mut candidates);
            add_columns(catalog, &aliases, &context, &mut candidates);
            add_functions(&context, &mut candidates);
            add_aliases(&aliases, &context, &mut candidates);
        }
    }

    ranked(candidates, &context.prefix, limit)
}

pub fn completion_context(sql: &str, cursor: usize) -> SqlCompletionContext {
    let cursor = cursor.min(sql.len());
    let before = &sql[..cursor];
    let mut token_start = cursor;
    for (index, character) in before.char_indices().rev() {
        if is_identifier_character(character) || character == '.' {
            token_start = index;
        } else {
            break;
        }
    }

    let token = &sql[token_start..cursor];
    if let Some(dot) = token.rfind('.') {
        SqlCompletionContext {
            cursor,
            replacement_start: token_start,
            replacement_end: cursor,
            prefix: token[dot + 1..].to_owned(),
            qualifier: Some(token[..dot].to_owned()),
        }
    } else {
        SqlCompletionContext {
            cursor,
            replacement_start: token_start,
            replacement_end: cursor,
            prefix: token.to_owned(),
            qualifier: None,
        }
    }
}

pub fn replace_sql_completion(sql: &str, item: &SqlCompletionItem) -> String {
    let mut updated = String::with_capacity(sql.len() + item.insert_text.len());
    updated.push_str(&sql[..item.replacement_start]);
    updated.push_str(&item.insert_text);
    updated.push_str(&sql[item.replacement_end..]);
    updated
}

fn add_keywords(context: &SqlCompletionContext, candidates: &mut Vec<SqlCompletionItem>) {
    for keyword in KEYWORDS {
        candidates.push(SqlCompletionItem {
            label: (*keyword).to_owned(),
            insert_text: (*keyword).to_owned(),
            kind: SqlCompletionKind::Keyword,
            detail: String::from("KEYWORD"),
            replacement_start: context.replacement_start,
            replacement_end: context.replacement_end,
        });
    }
}

fn add_tables(
    catalog: &SqlCatalog,
    context: &SqlCompletionContext,
    candidates: &mut Vec<SqlCompletionItem>,
) {
    for table in &catalog.tables {
        candidates.push(SqlCompletionItem {
            label: table.name.clone(),
            insert_text: quote_identifier_if_needed(&table.name),
            kind: SqlCompletionKind::Table,
            detail: String::from("TABLE"),
            replacement_start: context.replacement_start,
            replacement_end: context.replacement_end,
        });
    }
}

fn add_columns(
    catalog: &SqlCatalog,
    aliases: &BTreeMap<String, String>,
    context: &SqlCompletionContext,
    candidates: &mut Vec<SqlCompletionItem>,
) {
    if aliases.is_empty() {
        let mut seen_columns = BTreeSet::new();
        for table in &catalog.tables {
            for column in &table.columns {
                if !seen_columns.insert(column.name.clone()) {
                    continue;
                }
                candidates.push(SqlCompletionItem {
                    label: column.name.clone(),
                    insert_text: quote_identifier_if_needed(&column.name),
                    kind: SqlCompletionKind::Column,
                    detail: column.value_type.label().to_owned(),
                    replacement_start: context.replacement_start,
                    replacement_end: context.replacement_end,
                });
            }
        }
        return;
    }

    let tables = referenced_tables(catalog, aliases);
    let unique_table_names = tables
        .iter()
        .map(|(_, table_name)| table_name.as_str())
        .collect::<BTreeSet<_>>();
    if unique_table_names.len() == 1 {
        if let Some(table) = unique_table_names
            .iter()
            .next()
            .and_then(|table_name| catalog.table_named(table_name))
        {
            for column in &table.columns {
                candidates.push(SqlCompletionItem {
                    label: column.name.clone(),
                    insert_text: quote_identifier_if_needed(&column.name),
                    kind: SqlCompletionKind::Column,
                    detail: column.value_type.label().to_owned(),
                    replacement_start: context.replacement_start,
                    replacement_end: context.replacement_end,
                });
            }
        }
        return;
    }

    for (alias, table_name) in tables {
        if alias == table_name.to_ascii_lowercase() {
            continue;
        }
        if let Some(table) = catalog.table_named(&table_name) {
            for column in &table.columns {
                candidates.push(SqlCompletionItem {
                    label: format!("{alias}.{}", column.name),
                    insert_text: format!(
                        "{}.{}",
                        quote_identifier_if_needed(&alias),
                        quote_identifier_if_needed(&column.name)
                    ),
                    kind: SqlCompletionKind::Column,
                    detail: column.value_type.label().to_owned(),
                    replacement_start: context.replacement_start,
                    replacement_end: context.replacement_end,
                });
            }
        }
    }
}

fn add_aliases(
    aliases: &BTreeMap<String, String>,
    context: &SqlCompletionContext,
    candidates: &mut Vec<SqlCompletionItem>,
) {
    for alias in aliases.keys() {
        candidates.push(SqlCompletionItem {
            label: alias.clone(),
            insert_text: quote_identifier_if_needed(alias),
            kind: SqlCompletionKind::Alias,
            detail: String::from("ALIAS"),
            replacement_start: context.replacement_start,
            replacement_end: context.replacement_end,
        });
    }
}

fn add_functions(context: &SqlCompletionContext, candidates: &mut Vec<SqlCompletionItem>) {
    for function in FUNCTIONS {
        candidates.push(SqlCompletionItem {
            label: (*function).to_owned(),
            insert_text: format!("{function}("),
            kind: SqlCompletionKind::Function,
            detail: String::from("FUNCTION"),
            replacement_start: context.replacement_start,
            replacement_end: context.replacement_end,
        });
    }
}

fn ranked(
    candidates: Vec<SqlCompletionItem>,
    prefix: &str,
    limit: usize,
) -> Vec<SqlCompletionItem> {
    let mut seen = BTreeSet::new();
    let mut matches = candidates
        .into_iter()
        .filter_map(|item| {
            let key = (
                item.kind,
                item.label.to_ascii_lowercase(),
                item.insert_text.clone(),
            );
            if !seen.insert(key) {
                return None;
            }
            match_rank(&item.label, prefix).map(|rank| (rank, item.kind, item.label.clone(), item))
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| {
                left.2
                    .to_ascii_lowercase()
                    .cmp(&right.2.to_ascii_lowercase())
            })
            .then_with(|| left.2.cmp(&right.2))
    });
    matches
        .into_iter()
        .take(limit)
        .map(|(_, _, _, item)| item)
        .collect()
}

fn match_rank(label: &str, prefix: &str) -> Option<MatchRank> {
    if prefix.is_empty() {
        return Some(MatchRank::CaseInsensitivePrefix);
    }
    if label.starts_with(prefix) {
        Some(MatchRank::ExactPrefix)
    } else if label
        .to_ascii_lowercase()
        .starts_with(&prefix.to_ascii_lowercase())
    {
        Some(MatchRank::CaseInsensitivePrefix)
    } else if label
        .to_ascii_lowercase()
        .contains(&prefix.to_ascii_lowercase())
    {
        Some(MatchRank::Contains)
    } else {
        None
    }
}

fn completion_scope(sql: &str, cursor: usize) -> CompletionScope {
    let tokens = tokens(&sql[..cursor]);
    let significant = tokens
        .iter()
        .filter(|token| token.text != "," && token.text != "(")
        .collect::<Vec<_>>();
    let last = significant.last().map(|token| token.lower.as_str());
    let previous = significant
        .iter()
        .rev()
        .nth(1)
        .map(|token| token.lower.as_str());

    if matches!(last, Some("from" | "join")) {
        return CompletionScope::Table;
    }

    if matches!(last, Some("by")) && matches!(previous, Some("order" | "group")) {
        return CompletionScope::Column;
    }

    if matches!(
        last,
        Some("select" | "where" | "on" | "having" | "and" | "or" | "distinct")
    ) {
        return CompletionScope::Column;
    }

    let mut scope = CompletionScope::General;
    for token in significant {
        match token.lower.as_str() {
            "from" | "join" => scope = CompletionScope::Table,
            "select" | "where" | "on" | "having" => scope = CompletionScope::Column,
            "order" | "group" => scope = CompletionScope::Column,
            _ => {}
        }
    }
    scope
}

fn table_aliases(catalog: &SqlCatalog, sql: &str) -> BTreeMap<String, String> {
    let tokens = tokens(sql);
    let mut aliases = BTreeMap::new();
    let mut index = 0;
    while index < tokens.len() {
        if !matches!(tokens[index].lower.as_str(), "from" | "join") {
            index += 1;
            continue;
        }

        let Some(table_token) = tokens.get(index + 1) else {
            break;
        };
        let table_name = table_token.text.trim_matches('"');
        if catalog.table_named(table_name).is_none() {
            index += 1;
            continue;
        }

        aliases.insert(table_name.to_ascii_lowercase(), table_name.to_owned());
        let alias_token = if tokens
            .get(index + 2)
            .is_some_and(|token| token.lower == "as")
        {
            tokens.get(index + 3)
        } else {
            tokens.get(index + 2)
        };

        if let Some(alias_token) = alias_token {
            if is_alias_boundary(&alias_token.lower) {
                index += 2;
                continue;
            }
            aliases.insert(
                alias_token.text.trim_matches('"').to_ascii_lowercase(),
                table_name.to_owned(),
            );
        }
        index += 2;
    }
    aliases
}

fn referenced_tables(
    catalog: &SqlCatalog,
    aliases: &BTreeMap<String, String>,
) -> Vec<(String, String)> {
    if aliases.is_empty() {
        return catalog
            .tables
            .iter()
            .map(|table| (table.name.clone(), table.name.clone()))
            .collect();
    }

    aliases
        .iter()
        .filter(|(alias, table)| alias.as_str() != table.to_ascii_lowercase())
        .map(|(alias, table)| (alias.clone(), table.clone()))
        .collect::<Vec<_>>()
        .into_iter()
        .chain(
            aliases
                .iter()
                .filter(|(alias, table)| alias.as_str() == table.to_ascii_lowercase())
                .map(|(alias, table)| (alias.clone(), table.clone())),
        )
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    text: String,
    lower: String,
}

fn tokens(sql: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for character in sql.chars() {
        if is_identifier_character(character) || character == '"' {
            current.push(character);
        } else {
            if !current.is_empty() {
                tokens.push(Token {
                    lower: current.trim_matches('"').to_ascii_lowercase(),
                    text: std::mem::take(&mut current),
                });
            }
            if matches!(character, ',' | '(' | ')') {
                tokens.push(Token {
                    text: character.to_string(),
                    lower: character.to_string(),
                });
            }
        }
    }
    if !current.is_empty() {
        tokens.push(Token {
            lower: current.trim_matches('"').to_ascii_lowercase(),
            text: current,
        });
    }
    tokens
}

fn is_alias_boundary(token: &str) -> bool {
    matches!(
        token,
        "where"
            | "join"
            | "left"
            | "inner"
            | "on"
            | "order"
            | "group"
            | "having"
            | "limit"
            | "offset"
            | ","
            | ")"
    )
}

fn is_identifier_character(character: char) -> bool {
    character == '_' || character.is_alphanumeric()
}

pub fn quote_identifier_if_needed(identifier: &str) -> String {
    if is_bare_identifier(identifier) {
        identifier.to_owned()
    } else {
        format!("\"{}\"", identifier.replace('"', "\"\""))
    }
}

fn is_bare_identifier(identifier: &str) -> bool {
    let mut chars = identifier.chars();
    chars
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

trait CatalogCompletionExt {
    fn table_named(&self, name: &str) -> Option<&crate::SqlTable>;
}

impl CatalogCompletionExt for SqlCatalog {
    fn table_named(&self, name: &str) -> Option<&crate::SqlTable> {
        self.tables
            .iter()
            .find(|table| table.name.eq_ignore_ascii_case(name))
    }
}

#[allow(dead_code)]
fn _assert_column_type_copy(value_type: SqlColumnType) -> SqlColumnType {
    value_type
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqlColumn, SqlTable};

    fn catalog() -> SqlCatalog {
        SqlCatalog {
            tables: vec![
                table(
                    "meetings",
                    &[
                        ("title", SqlColumnType::Text),
                        ("attendees", SqlColumnType::Json),
                        ("remote", SqlColumnType::Boolean),
                    ],
                ),
                table(
                    "project_archive",
                    &[
                        ("title", SqlColumnType::Text),
                        ("priority", SqlColumnType::Integer),
                    ],
                ),
                table(
                    "projects",
                    &[
                        ("title", SqlColumnType::Text),
                        ("status", SqlColumnType::Text),
                        ("priority", SqlColumnType::Integer),
                        ("published", SqlColumnType::Boolean),
                        ("_path", SqlColumnType::Text),
                    ],
                ),
            ],
        }
    }

    fn table(name: &str, columns: &[(&str, SqlColumnType)]) -> SqlTable {
        SqlTable {
            name: name.to_owned(),
            collection_id: name.to_owned(),
            display_name: name.to_owned(),
            columns: columns
                .iter()
                .map(|(name, value_type)| SqlColumn {
                    name: (*name).to_owned(),
                    source_property: None,
                    value_type: *value_type,
                })
                .collect(),
        }
    }

    fn labels(sql: &str) -> Vec<String> {
        labels_at(sql, sql.len())
    }

    fn labels_at(sql: &str, cursor: usize) -> Vec<String> {
        complete_sql(&catalog(), sql, cursor, DEFAULT_SQL_COMPLETION_LIMIT)
            .into_iter()
            .map(|item| item.label)
            .collect()
    }

    fn items(sql: &str) -> Vec<SqlCompletionItem> {
        items_at(sql, sql.len())
    }

    fn items_at(sql: &str, cursor: usize) -> Vec<SqlCompletionItem> {
        complete_sql(&catalog(), sql, cursor, DEFAULT_SQL_COMPLETION_LIMIT)
    }

    #[test]
    fn completes_read_only_keywords() {
        assert_eq!(labels("SEL").first().map(String::as_str), Some("SELECT"));
        assert!(labels("WH").contains(&String::from("WHERE")));

        let forbidden = [
            "INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "CREATE", "REPLACE",
        ];
        let suggestions = labels("");
        for keyword in forbidden {
            assert!(!suggestions.contains(&keyword.to_owned()));
        }
    }

    #[test]
    fn from_and_join_suggest_tables_with_prefix_filtering() {
        assert_eq!(
            labels("SELECT * FROM pro"),
            vec![String::from("project_archive"), String::from("projects")]
        );
        assert_eq!(
            labels("SELECT * FROM projects p JOIN mee"),
            vec![String::from("meetings")]
        );
    }

    #[test]
    fn column_contexts_suggest_single_table_columns() {
        let sql = "SELECT pri\nFROM projects";
        assert!(labels_at(sql, "SELECT pri".len()).contains(&String::from("priority")));
        let sql = "SELECT * FROM projects WHERE pub";
        assert!(labels_at(sql, sql.len()).contains(&String::from("published")));
        assert!(labels("SELECT * FROM projects ORDER BY sta").contains(&String::from("status")));
        assert!(labels("SELECT * FROM projects GROUP BY sta").contains(&String::from("status")));
    }

    #[test]
    fn aliases_and_dot_completion_use_only_the_target_table() {
        let sql = "SELECT p.\nFROM projects p";
        let dot = labels_at(sql, "SELECT p.".len());
        assert!(dot.contains(&String::from("p.title")));
        assert!(dot.contains(&String::from("p.priority")));
        assert!(!dot.contains(&String::from("p.attendees")));

        let sql = "SELECT p.pr\nFROM projects p";
        assert_eq!(
            labels_at(sql, "SELECT p.pr".len()),
            vec![String::from("p.priority")]
        );
        let sql = "SELECT p.pr\nFROM projects AS p";
        assert_eq!(
            labels_at(sql, "SELECT p.pr".len()),
            vec![String::from("p.priority")]
        );
    }

    #[test]
    fn join_aliases_and_multiple_tables_are_qualified() {
        let sql = "SELECT m.\nFROM projects p JOIN meetings m ON p.title = m.";
        let join = labels_at(sql, "SELECT m.".len());
        assert!(join.contains(&String::from("m.attendees")));
        assert!(!join.contains(&String::from("m.priority")));

        let sql = "SELECT tit\nFROM projects p JOIN meetings m ON p.title = m.title";
        let multiple = labels_at(sql, "SELECT tit".len());
        assert!(multiple.contains(&String::from("p.title")));
        assert!(multiple.contains(&String::from("m.title")));
        assert!(!multiple.contains(&String::from("title")));
    }

    #[test]
    fn type_metadata_is_attached() {
        let sql = "SELECT pri\nFROM projects";
        let priority = items_at(sql, "SELECT pri".len())
            .into_iter()
            .find(|item| item.label == "priority")
            .unwrap();
        assert_eq!(priority.detail, "INTEGER");

        let table = items("SELECT * FROM pro")
            .into_iter()
            .find(|item| item.label == "projects")
            .unwrap();
        assert_eq!(table.detail, "TABLE");

        let keyword = items("SEL").into_iter().next().unwrap();
        assert_eq!(keyword.detail, "KEYWORD");
    }

    #[test]
    fn matching_is_case_insensitive_and_deterministic() {
        let sql = "select PRI\nfrom projects";
        assert_eq!(
            labels_at(sql, "select PRI".len()),
            vec![String::from("priority")]
        );
        assert_eq!(
            labels("SELECT * FROM pro"),
            vec![String::from("project_archive"), String::from("projects")]
        );
    }

    #[test]
    fn replacement_replaces_only_current_fragment() {
        let query = "SELECT pri";
        let item = items(query)
            .into_iter()
            .find(|item| item.label == "priority")
            .unwrap();
        assert_eq!(item.replacement_start, "SELECT ".len());
        assert_eq!(replace_sql_completion(query, &item), "SELECT priority");
    }

    #[test]
    fn uses_real_catalog_names_and_collision_suffixes() {
        let catalog = SqlCatalog {
            tables: vec![table("documents", &[]), table("documents_2", &[])],
        };
        let labels = complete_sql(&catalog, "FROM doc", "FROM doc".len(), 50)
            .into_iter()
            .map(|item| item.label)
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            vec![String::from("documents"), String::from("documents_2")]
        );
    }

    #[test]
    fn schema_updates_are_reflected_by_the_catalog_argument() {
        let before = SqlCatalog {
            tables: vec![table("projects", &[("priority", SqlColumnType::Integer)])],
        };
        let after = SqlCatalog {
            tables: vec![table("projects", &[("budget", SqlColumnType::Integer)])],
        };
        let sql = "SELECT bu FROM projects";
        assert!(complete_sql(&before, sql, "SELECT bu".len(), 50).is_empty());
        assert_eq!(
            complete_sql(&after, sql, "SELECT bu".len(), 50)[0].label,
            "budget"
        );
        assert!(
            complete_sql(&after, "SELECT pri FROM projects", "SELECT pri".len(), 50).is_empty()
        );
    }

    #[test]
    fn quotes_identifiers_when_catalog_contains_non_bare_names() {
        let catalog = SqlCatalog {
            tables: vec![table("projetos", &[("ação", SqlColumnType::Text)])],
        };
        let item = complete_sql(&catalog, "SELECT a FROM projetos", "SELECT a".len(), 50)
            .into_iter()
            .find(|item| item.label == "ação")
            .unwrap();
        assert_eq!(item.insert_text, "\"ação\"");
    }

    #[test]
    fn suggests_functions_in_column_context() {
        let suggestions = items("SELECT CO");
        let function = suggestions
            .into_iter()
            .find(|item| item.label == "COALESCE")
            .unwrap();
        assert_eq!(function.kind, SqlCompletionKind::Function);
        assert_eq!(function.insert_text, "COALESCE(");
    }

    #[test]
    fn no_match_returns_empty_results() {
        let sql = "SELECT zzz FROM projects";
        assert!(labels_at(sql, "SELECT zzz".len()).is_empty());
    }

    #[test]
    fn result_limit_is_enforced() {
        let many = SqlCatalog {
            tables: (0..80)
                .map(|index| table(&format!("projects_{index:02}"), &[]))
                .collect(),
        };
        let suggestions = complete_sql(&many, "FROM projects_", "FROM projects_".len(), 20);
        assert_eq!(suggestions.len(), 20);
    }
}
