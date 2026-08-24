use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use crate::{display_relation_value, Document, PropertyValue, RelationIndex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableModel {
    pub columns: Vec<TableColumn>,
    pub rows: Vec<TableRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableColumn {
    pub id: String,
    pub label: String,
    pub inferred_type: TableValueType,
    pub width: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TableValueType {
    Title,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Null,
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRow {
    pub document_path: PathBuf,
    pub cells: Vec<TableCell>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableCell {
    Missing,
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Relation(String),
    Array(Vec<String>),
    Object,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableSort {
    pub column_id: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl TableModel {
    pub fn collection(
        collection_id: &str,
        documents: &[Document],
        sort: Option<&TableSort>,
    ) -> Self {
        Self::collection_with_relations(collection_id, documents, sort, None)
    }

    pub fn collection_with_relations(
        collection_id: &str,
        documents: &[Document],
        sort: Option<&TableSort>,
        relation_index: Option<&RelationIndex>,
    ) -> Self {
        let documents = documents
            .iter()
            .filter(|document| document.collection_id == collection_id)
            .collect::<Vec<_>>();
        let columns = discover_columns(&documents);
        let mut rows = documents
            .iter()
            .map(|document| table_row(document, &columns, relation_index))
            .collect::<Vec<_>>();

        rows.sort_by(|left, right| compare_rows(left, right, &columns, sort));

        Self { columns, rows }
    }
}

impl TableCell {
    pub fn display_value(&self) -> String {
        match self {
            Self::Missing | Self::Null => String::from("—"),
            Self::Bool(true) => String::from("✓"),
            Self::Bool(false) => String::from("✕"),
            Self::Number(value) | Self::String(value) | Self::Relation(value) => value.clone(),
            Self::Array(values) => {
                if values.is_empty() {
                    String::from("-")
                } else {
                    values.join(", ")
                }
            }
            Self::Object => String::from("{...}"),
        }
    }

    fn sort_rank(&self) -> u8 {
        match self {
            Self::Missing | Self::Null => 1,
            _ => 0,
        }
    }
}

fn discover_columns(documents: &[&Document]) -> Vec<TableColumn> {
    let mut property_stats = BTreeMap::<String, PropertyStats>::new();

    for document in documents {
        for (property, value) in &document.properties {
            if is_redundant_property(property) {
                continue;
            }

            let stats = property_stats.entry(property.clone()).or_default();
            stats.count += 1;
            stats.value_types.insert(value_type(value));
        }
    }

    let mut properties = property_stats.into_iter().collect::<Vec<_>>();
    properties.sort_by(|(left_id, left), (right_id, right)| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left_id.to_lowercase().cmp(&right_id.to_lowercase()))
            .then_with(|| left_id.cmp(right_id))
    });

    let mut columns = vec![TableColumn {
        id: String::from("title"),
        label: String::from("Title"),
        inferred_type: TableValueType::Title,
        width: 260,
    }];

    columns.extend(properties.into_iter().map(|(id, stats)| TableColumn {
        label: humanize_property(&id),
        width: column_width(stats.inferred_type()),
        inferred_type: stats.inferred_type(),
        id,
    }));

    columns
}

fn table_row(
    document: &Document,
    columns: &[TableColumn],
    relation_index: Option<&RelationIndex>,
) -> TableRow {
    let cells = columns
        .iter()
        .map(|column| {
            if column.id == "title" {
                TableCell::String(document.title.clone())
            } else {
                document
                    .properties
                    .get(&column.id)
                    .map(|value| table_cell(value, relation_index))
                    .unwrap_or(TableCell::Missing)
            }
        })
        .collect();

    TableRow {
        document_path: document.path.clone(),
        cells,
    }
}

fn compare_rows(
    left: &TableRow,
    right: &TableRow,
    columns: &[TableColumn],
    sort: Option<&TableSort>,
) -> Ordering {
    let ordering = sort
        .and_then(|sort| {
            columns
                .iter()
                .position(|column| column.id == sort.column_id)
                .map(|column_index| {
                    compare_sorted_cells(
                        &left.cells[column_index],
                        &right.cells[column_index],
                        sort.direction,
                    )
                    .unwrap_or(Ordering::Equal)
                })
        })
        .unwrap_or(Ordering::Equal);

    ordering.then_with(|| {
        left.document_path
            .to_string_lossy()
            .to_lowercase()
            .cmp(&right.document_path.to_string_lossy().to_lowercase())
    })
}

fn compare_sorted_cells(
    left: &TableCell,
    right: &TableCell,
    direction: SortDirection,
) -> Option<Ordering> {
    let rank_order = left.sort_rank().cmp(&right.sort_rank());
    if rank_order != Ordering::Equal {
        return Some(rank_order);
    }

    let ordering = match (left, right) {
        (TableCell::Missing | TableCell::Null, TableCell::Missing | TableCell::Null) => {
            Some(Ordering::Equal)
        }
        (TableCell::Number(left), TableCell::Number(right)) => {
            match (parse_number(left), parse_number(right)) {
                (Some(left), Some(right)) => left.partial_cmp(&right),
                _ => Some(left.cmp(right)),
            }
        }
        (TableCell::Bool(left), TableCell::Bool(right)) => Some(left.cmp(right)),
        _ => Some(
            left.display_value()
                .to_lowercase()
                .cmp(&right.display_value().to_lowercase()),
        ),
    }?;

    Some(match direction {
        SortDirection::Ascending => ordering,
        SortDirection::Descending => ordering.reverse(),
    })
}

fn table_cell(value: &PropertyValue, relation_index: Option<&RelationIndex>) -> TableCell {
    if let Some(relation_value) =
        relation_index.and_then(|index| display_relation_value(value, index))
    {
        return TableCell::Relation(relation_value);
    }

    match value {
        PropertyValue::Null => TableCell::Null,
        PropertyValue::Bool(value) => TableCell::Bool(*value),
        PropertyValue::Number(value) => TableCell::Number(value.clone()),
        PropertyValue::String(value) => TableCell::String(value.clone()),
        PropertyValue::Array(values) => {
            TableCell::Array(values.iter().map(compact_property_value).collect())
        }
        PropertyValue::Object(_) => TableCell::Object,
    }
}

fn compact_property_value(value: &PropertyValue) -> String {
    match value {
        PropertyValue::Null => String::from("—"),
        PropertyValue::Bool(true) => String::from("✓"),
        PropertyValue::Bool(false) => String::from("✕"),
        PropertyValue::Number(value) | PropertyValue::String(value) => value.clone(),
        PropertyValue::Array(_) => String::from("[...]"),
        PropertyValue::Object(_) => String::from("{...}"),
    }
}

fn is_redundant_property(property: &str) -> bool {
    matches!(property, "title" | "type")
}

fn humanize_property(property: &str) -> String {
    property
        .split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!(
                    "{}{}",
                    first.to_uppercase().collect::<String>(),
                    chars.as_str()
                ),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn column_width(value_type: TableValueType) -> u16 {
    match value_type {
        TableValueType::Boolean => 96,
        TableValueType::Number => 112,
        TableValueType::Null => 96,
        TableValueType::Array => 220,
        TableValueType::Object => 120,
        TableValueType::String | TableValueType::Mixed => 180,
        TableValueType::Title => 260,
    }
}

fn value_type(value: &PropertyValue) -> TableValueType {
    match value {
        PropertyValue::Null => TableValueType::Null,
        PropertyValue::Bool(_) => TableValueType::Boolean,
        PropertyValue::Number(_) => TableValueType::Number,
        PropertyValue::String(_) => TableValueType::String,
        PropertyValue::Array(_) => TableValueType::Array,
        PropertyValue::Object(_) => TableValueType::Object,
    }
}

fn parse_number(value: &str) -> Option<f64> {
    value.parse::<f64>().ok().filter(|value| value.is_finite())
}

#[derive(Debug, Default)]
struct PropertyStats {
    count: usize,
    value_types: BTreeSet<TableValueType>,
}

impl PropertyStats {
    fn inferred_type(&self) -> TableValueType {
        let non_null = self
            .value_types
            .iter()
            .copied()
            .filter(|value_type| *value_type != TableValueType::Null)
            .collect::<BTreeSet<_>>();

        match non_null.len() {
            0 => TableValueType::Null,
            1 => non_null.into_iter().next().unwrap_or(TableValueType::Null),
            _ => TableValueType::Mixed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::ffi::OsString;

    #[test]
    fn collection_with_two_documents_discovers_columns_and_rows() {
        let documents = vec![
            document(
                "projects/carf.md",
                "CARF",
                [
                    ("title", PropertyValue::String(String::from("Ignored"))),
                    ("status", PropertyValue::String(String::from("active"))),
                    ("priority", PropertyValue::Number(String::from("1"))),
                    ("published", PropertyValue::Bool(true)),
                ],
            ),
            document(
                "projects/cvm.md",
                "CVM",
                [("status", PropertyValue::String(String::from("paused")))],
            ),
        ];

        let table = TableModel::collection("project", &documents, None);

        assert_eq!(
            column_ids(&table),
            vec!["title", "status", "priority", "published"]
        );
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].cells[0].display_value(), "CARF");
        assert_eq!(table.rows[1].cells[0].display_value(), "CVM");
    }

    #[test]
    fn title_is_always_first_and_type_is_not_duplicated() {
        let documents = vec![document(
            "projects/carf.md",
            "CARF",
            [
                ("title", PropertyValue::String(String::from("CARF"))),
                ("type", PropertyValue::String(String::from("project"))),
                ("owner", PropertyValue::String(String::from("Sergio"))),
            ],
        )];

        let table = TableModel::collection("project", &documents, None);

        assert_eq!(column_ids(&table), vec!["title", "owner"]);
        assert_eq!(table.columns[0].label, "Title");
    }

    #[test]
    fn property_present_in_one_document_still_generates_column_and_missing_cell() {
        let documents = vec![
            document(
                "projects/carf.md",
                "CARF",
                [("owner", PropertyValue::String(String::from("Sergio")))],
            ),
            document("projects/cvm.md", "CVM", []),
        ];

        let table = TableModel::collection("project", &documents, None);

        assert_eq!(column_ids(&table), vec!["title", "owner"]);
        assert_eq!(table.rows[1].cells[1], TableCell::Missing);
        assert_eq!(table.rows[1].cells[1].display_value(), "—");
    }

    #[test]
    fn renders_strings_numbers_booleans_arrays_null_and_objects() {
        let documents = vec![document(
            "projects/carf.md",
            "CARF",
            [
                ("name", PropertyValue::String(String::from("Sergio"))),
                ("score", PropertyValue::Number(String::from("42"))),
                ("published", PropertyValue::Bool(true)),
                ("archived", PropertyValue::Bool(false)),
                (
                    "tags",
                    PropertyValue::Array(vec![
                        PropertyValue::String(String::from("rust")),
                        PropertyValue::Bool(false),
                        PropertyValue::String(String::from("typescript")),
                    ]),
                ),
                ("empty", PropertyValue::Null),
                ("meta", PropertyValue::Object(BTreeMap::new())),
            ],
        )];

        let table = TableModel::collection("project", &documents, None);
        let row = &table.rows[0];

        assert_eq!(cell(&table, row, "name").display_value(), "Sergio");
        assert_eq!(cell(&table, row, "score").display_value(), "42");
        assert_eq!(cell(&table, row, "published").display_value(), "✓");
        assert_eq!(cell(&table, row, "archived").display_value(), "✕");
        assert_eq!(
            cell(&table, row, "tags").display_value(),
            "rust, ✕, typescript"
        );
        assert_eq!(cell(&table, row, "empty").display_value(), "—");
        assert_eq!(cell(&table, row, "meta").display_value(), "{...}");
    }

    #[test]
    fn boolean_false_is_distinct_from_true_missing_and_null() {
        let documents = vec![
            document(
                "projects/true.md",
                "True",
                [("published", PropertyValue::Bool(true))],
            ),
            document(
                "projects/false.md",
                "False",
                [("published", PropertyValue::Bool(false))],
            ),
            document("projects/missing.md", "Missing", []),
            document(
                "projects/null.md",
                "Null",
                [("published", PropertyValue::Null)],
            ),
        ];

        let table = TableModel::collection("project", &documents, None);
        let true_cell = cell(&table, &table.rows[3], "published");
        let false_cell = cell(&table, &table.rows[0], "published");
        let missing_cell = cell(&table, &table.rows[1], "published");
        let null_cell = cell(&table, &table.rows[2], "published");

        assert_eq!(true_cell.display_value(), "✓");
        assert_eq!(false_cell.display_value(), "✕");
        assert_ne!(true_cell, false_cell);
        assert_ne!(false_cell, missing_cell);
        assert_eq!(missing_cell.display_value(), "—");
        assert_eq!(null_cell.display_value(), "—");
    }

    #[test]
    fn infers_mixed_type_without_breaking() {
        let documents = vec![
            document(
                "projects/a.md",
                "A",
                [("foo", PropertyValue::Number(String::from("10")))],
            ),
            document(
                "projects/b.md",
                "B",
                [("foo", PropertyValue::String(String::from("dez")))],
            ),
        ];

        let table = TableModel::collection("project", &documents, None);

        assert_eq!(column(&table, "foo").inferred_type, TableValueType::Mixed);
    }

    #[test]
    fn column_order_is_deterministic_by_coverage_then_name() {
        let documents = vec![
            document(
                "projects/a.md",
                "A",
                [
                    ("zeta", PropertyValue::String(String::from("z"))),
                    ("alpha", PropertyValue::String(String::from("a"))),
                ],
            ),
            document(
                "projects/b.md",
                "B",
                [("zeta", PropertyValue::String(String::from("z")))],
            ),
        ];

        let table = TableModel::collection("project", &documents, None);

        assert_eq!(column_ids(&table), vec!["title", "zeta", "alpha"]);
    }

    #[test]
    fn rows_are_deterministic_by_path_without_sort() {
        let documents = vec![
            document("projects/b.md", "B", []),
            document("projects/a.md", "A", []),
        ];

        let table = TableModel::collection("project", &documents, None);

        assert_eq!(row_titles(&table), vec!["A", "B"]);
    }

    #[test]
    fn sorts_string_ascending_and_descending_case_insensitively() {
        let documents = vec![
            document("projects/b.md", "beta", []),
            document("projects/a.md", "Alpha", []),
        ];

        let asc = TableSort {
            column_id: String::from("title"),
            direction: SortDirection::Ascending,
        };
        let desc = TableSort {
            column_id: String::from("title"),
            direction: SortDirection::Descending,
        };

        assert_eq!(
            row_titles(&TableModel::collection("project", &documents, Some(&asc))),
            vec!["Alpha", "beta"]
        );
        assert_eq!(
            row_titles(&TableModel::collection("project", &documents, Some(&desc))),
            vec!["beta", "Alpha"]
        );
    }

    #[test]
    fn sorts_numbers_numerically_ascending_and_descending() {
        let documents = vec![
            document(
                "projects/ten.md",
                "Ten",
                [("priority", PropertyValue::Number(String::from("10")))],
            ),
            document(
                "projects/two.md",
                "Two",
                [("priority", PropertyValue::Number(String::from("2")))],
            ),
        ];
        let asc = TableSort {
            column_id: String::from("priority"),
            direction: SortDirection::Ascending,
        };
        let desc = TableSort {
            column_id: String::from("priority"),
            direction: SortDirection::Descending,
        };

        assert_eq!(
            row_titles(&TableModel::collection("project", &documents, Some(&asc))),
            vec!["Two", "Ten"]
        );
        assert_eq!(
            row_titles(&TableModel::collection("project", &documents, Some(&desc))),
            vec!["Ten", "Two"]
        );
    }

    #[test]
    fn sorting_keeps_missing_values_at_the_end() {
        let documents = vec![
            document("projects/a.md", "Missing", []),
            document(
                "projects/b.md",
                "One",
                [("priority", PropertyValue::Number(String::from("1")))],
            ),
            document(
                "projects/c.md",
                "Two",
                [("priority", PropertyValue::Number(String::from("2")))],
            ),
        ];
        let desc = TableSort {
            column_id: String::from("priority"),
            direction: SortDirection::Descending,
        };

        assert_eq!(
            row_titles(&TableModel::collection("project", &documents, Some(&desc))),
            vec!["Two", "One", "Missing"]
        );
    }

    #[test]
    fn sorts_booleans_deterministically_ascending_and_descending() {
        let documents = vec![
            document("projects/a.md", "Missing", []),
            document(
                "projects/b.md",
                "True",
                [("published", PropertyValue::Bool(true))],
            ),
            document(
                "projects/c.md",
                "False",
                [("published", PropertyValue::Bool(false))],
            ),
        ];
        let asc = TableSort {
            column_id: String::from("published"),
            direction: SortDirection::Ascending,
        };
        let desc = TableSort {
            column_id: String::from("published"),
            direction: SortDirection::Descending,
        };

        assert_eq!(
            row_titles(&TableModel::collection("project", &documents, Some(&asc))),
            vec!["False", "True", "Missing"]
        );
        assert_eq!(
            row_titles(&TableModel::collection("project", &documents, Some(&desc))),
            vec!["True", "False", "Missing"]
        );
    }

    #[test]
    fn supports_unicode_values_and_labels() {
        let documents = vec![document(
            "projects/acao.md",
            "Ação Pública",
            [("responsável", PropertyValue::String(String::from("Sérgio")))],
        )];

        let table = TableModel::collection("project", &documents, None);

        assert_eq!(column(&table, "responsável").label, "Responsável");
        assert_eq!(table.rows[0].cells[0].display_value(), "Ação Pública");
        assert_eq!(
            cell(&table, &table.rows[0], "responsável").display_value(),
            "Sérgio"
        );
    }

    #[test]
    fn handles_more_than_one_hundred_documents() {
        let documents = (0..150)
            .map(|index| {
                document(
                    &format!("projects/{index:03}.md"),
                    &format!("Project {index:03}"),
                    [("priority", PropertyValue::Number(index.to_string()))],
                )
            })
            .collect::<Vec<_>>();

        let table = TableModel::collection("project", &documents, None);

        assert_eq!(table.rows.len(), 150);
        assert_eq!(table.columns.len(), 2);
    }

    #[test]
    fn selected_row_path_points_to_the_document() {
        let documents = vec![document("projects/carf.md", "CARF", [])];

        let table = TableModel::collection("project", &documents, None);

        assert_eq!(
            table.rows[0].document_path,
            PathBuf::from("/workspace/projects/carf.md")
        );
    }

    fn document<const N: usize>(
        relative_path: &str,
        title: &str,
        properties: [(&str, PropertyValue); N],
    ) -> Document {
        let relative_path = PathBuf::from(relative_path);
        let path = PathBuf::from("/workspace").join(&relative_path);

        Document {
            path,
            relative_path: relative_path.clone(),
            file_name: relative_path
                .file_name()
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| OsString::from("document.md")),
            metadata: crate::DocumentMetadata {
                file_size: None,
                modified: None,
            },
            title: title.to_owned(),
            source_content: Some(String::new()),
            markdown_content: String::new(),
            properties: properties
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
            document_type: Some(String::from("project")),
            collection_id: String::from("project"),
            warnings: Vec::new(),
        }
    }

    fn column_ids(table: &TableModel) -> Vec<&str> {
        table
            .columns
            .iter()
            .map(|column| column.id.as_str())
            .collect()
    }

    fn row_titles(table: &TableModel) -> Vec<String> {
        table
            .rows
            .iter()
            .map(|row| row.cells[0].display_value())
            .collect()
    }

    fn column<'a>(table: &'a TableModel, id: &str) -> &'a TableColumn {
        table.columns.iter().find(|column| column.id == id).unwrap()
    }

    fn cell<'a>(table: &'a TableModel, row: &'a TableRow, id: &str) -> &'a TableCell {
        let index = table
            .columns
            .iter()
            .position(|column| column.id == id)
            .unwrap();
        &row.cells[index]
    }
}
