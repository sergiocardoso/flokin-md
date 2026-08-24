use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use crate::{Document, PropertyValue};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RelationIndex {
    relations: Vec<Relation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relation {
    pub source_document: PathBuf,
    pub source_title: String,
    pub source_relative_path: PathBuf,
    pub property: String,
    pub target: RelationTarget,
    pub status: RelationStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationTarget {
    pub raw: String,
    pub display: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationStatus {
    Resolved(RelationDocument),
    Unresolved,
    Ambiguous(Vec<RelationDocument>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationDocument {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub title: String,
}

impl RelationIndex {
    #[cfg(test)]
    pub(crate) fn from_relations(relations: Vec<Relation>) -> Self {
        Self { relations }
    }

    pub fn build(documents: &[Document]) -> Self {
        let mut title_index = BTreeMap::<String, Vec<RelationDocument>>::new();
        let mut path_index = BTreeMap::<String, RelationDocument>::new();

        for document in documents {
            let relation_document = RelationDocument {
                path: document.path.clone(),
                relative_path: document.relative_path.clone(),
                title: document.title.clone(),
            };
            title_index
                .entry(document.title.clone())
                .or_default()
                .push(relation_document.clone());
            path_index.insert(path_key(&document.relative_path), relation_document);
        }

        for candidates in title_index.values_mut() {
            candidates
                .sort_by(|left, right| compare_paths(&left.relative_path, &right.relative_path));
        }

        let mut relations = Vec::new();
        for document in documents {
            for (property, value) in &document.properties {
                collect_property_relations(
                    document,
                    property,
                    value,
                    &title_index,
                    &path_index,
                    &mut relations,
                );
            }
        }

        relations.sort_by(|left, right| {
            compare_paths(&left.source_relative_path, &right.source_relative_path)
                .then_with(|| left.property.cmp(&right.property))
                .then_with(|| left.target.raw.cmp(&right.target.raw))
        });

        Self { relations }
    }

    pub fn all(&self) -> &[Relation] {
        &self.relations
    }

    pub fn outgoing(&self, source_document: &Path) -> Vec<&Relation> {
        self.relations
            .iter()
            .filter(|relation| relation.source_document == source_document)
            .collect()
    }

    pub fn incoming(&self, target_document: &Path) -> Vec<&Relation> {
        self.relations
            .iter()
            .filter(|relation| {
                matches!(
                    &relation.status,
                    RelationStatus::Resolved(target) if target.path == target_document
                )
            })
            .collect()
    }
}

fn collect_property_relations(
    document: &Document,
    property: &str,
    value: &PropertyValue,
    title_index: &BTreeMap<String, Vec<RelationDocument>>,
    path_index: &BTreeMap<String, RelationDocument>,
    relations: &mut Vec<Relation>,
) {
    match value {
        PropertyValue::String(value) => {
            if let Some(target) = parse_wikilink(value) {
                relations.push(relation(
                    document,
                    property,
                    target,
                    title_index,
                    path_index,
                ));
            }
        }
        PropertyValue::Array(values) => {
            for value in values {
                if let PropertyValue::String(value) = value {
                    if let Some(target) = parse_wikilink(value) {
                        relations.push(relation(
                            document,
                            property,
                            target,
                            title_index,
                            path_index,
                        ));
                    }
                }
            }
        }
        PropertyValue::Null
        | PropertyValue::Bool(_)
        | PropertyValue::Number(_)
        | PropertyValue::Object(_) => {}
    }
}

fn relation(
    document: &Document,
    property: &str,
    target: RelationTarget,
    title_index: &BTreeMap<String, Vec<RelationDocument>>,
    path_index: &BTreeMap<String, RelationDocument>,
) -> Relation {
    Relation {
        source_document: document.path.clone(),
        source_title: document.title.clone(),
        source_relative_path: document.relative_path.clone(),
        property: property.to_owned(),
        status: resolve_target(&target.raw, title_index, path_index),
        target,
    }
}

fn resolve_target(
    raw: &str,
    title_index: &BTreeMap<String, Vec<RelationDocument>>,
    path_index: &BTreeMap<String, RelationDocument>,
) -> RelationStatus {
    if looks_like_path(raw) {
        return path_index
            .get(&path_key(Path::new(raw)))
            .cloned()
            .map(RelationStatus::Resolved)
            .unwrap_or(RelationStatus::Unresolved);
    }

    match title_index.get(raw).map(Vec::as_slice) {
        Some([target]) => RelationStatus::Resolved(target.clone()),
        Some(candidates) if !candidates.is_empty() => {
            RelationStatus::Ambiguous(candidates.to_vec())
        }
        _ => RelationStatus::Unresolved,
    }
}

pub fn parse_wikilink(value: &str) -> Option<RelationTarget> {
    let value = value.trim();
    let inner = value
        .strip_prefix("[[")
        .and_then(|value| value.strip_suffix("]]"))?;
    let inner = inner.trim();
    if inner.is_empty() {
        return None;
    }

    let (raw, display) = inner
        .split_once('|')
        .map_or((inner, inner), |(raw, display)| {
            let raw = raw.trim();
            let display = display.trim();
            (raw, if display.is_empty() { raw } else { display })
        });
    if raw.is_empty() {
        return None;
    }

    Some(RelationTarget {
        raw: raw.to_owned(),
        display: display.to_owned(),
    })
}

pub fn display_relation_value(value: &PropertyValue, index: &RelationIndex) -> Option<String> {
    let target = match value {
        PropertyValue::String(value) => parse_wikilink(value),
        _ => None,
    }?;
    index
        .relations
        .iter()
        .find(|relation| relation.target.raw == target.raw)
        .filter(|relation| matches!(relation.status, RelationStatus::Resolved(_)))
        .map(|_| format!("{} ↗", target.display))
}

fn looks_like_path(raw: &str) -> bool {
    raw.contains('/') || raw.ends_with(".md") || raw.ends_with(".markdown")
}

fn path_key(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn compare_paths(left: &Path, right: &Path) -> std::cmp::Ordering {
    left.to_string_lossy()
        .to_lowercase()
        .cmp(&right.to_string_lossy().to_lowercase())
        .then_with(|| left.cmp(right))
}

pub fn relation_display_property(property: &str) -> String {
    let mut chars = property.chars();
    match chars.next() {
        Some(first) => format!(
            "{}{}",
            first.to_uppercase().collect::<String>(),
            chars.as_str()
        ),
        None => String::new(),
    }
}

#[allow(dead_code)]
fn _assert_no_duplicate_docs(candidates: &[RelationDocument]) -> BTreeSet<&PathBuf> {
    candidates.iter().map(|candidate| &candidate.path).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DocumentMetadata, PropertyValue};

    use std::{collections::BTreeMap, ffi::OsString};

    #[test]
    fn common_string_is_not_a_relation() {
        let index = RelationIndex::build(&[document(
            "meetings/carf.md",
            "CARF Daily",
            [("project", PropertyValue::String(String::from("CARF")))],
        )]);

        assert!(index.all().is_empty());
    }

    #[test]
    fn wikilink_becomes_relation_and_resolves_unique_title() {
        let documents = vec![
            document("projects/carf.md", "CARF", []),
            document(
                "meetings/carf.md",
                "CARF Daily",
                [("project", string("[[CARF]]"))],
            ),
        ];

        let index = RelationIndex::build(&documents);
        let relation = &index.all()[0];

        assert_eq!(relation.property, "project");
        assert_eq!(relation.target.raw, "CARF");
        assert_eq!(relation.target.display, "CARF");
        assert!(matches!(
            &relation.status,
            RelationStatus::Resolved(target) if target.relative_path == Path::new("projects/carf.md")
        ));
    }

    #[test]
    fn unknown_title_is_unresolved() {
        let index = RelationIndex::build(&[document(
            "meetings/carf.md",
            "CARF Daily",
            [("owner", string("[[Pessoa Que Não Existe]]"))],
        )]);

        assert!(matches!(index.all()[0].status, RelationStatus::Unresolved));
    }

    #[test]
    fn duplicated_title_is_ambiguous_and_never_chosen() {
        let documents = vec![
            document("projects/carf.md", "CARF", []),
            document("archive/carf.md", "CARF", []),
            document(
                "meetings/carf.md",
                "CARF Daily",
                [("project", string("[[CARF]]"))],
            ),
        ];

        let index = RelationIndex::build(&documents);

        assert!(matches!(
            &index.all()[0].status,
            RelationStatus::Ambiguous(candidates) if candidates.len() == 2
        ));
    }

    #[test]
    fn relative_path_resolution_has_precedence() {
        let documents = vec![
            document("projects/carf.md", "Duplicated", []),
            document("archive/carf.md", "Duplicated", []),
            document(
                "meetings/carf.md",
                "CARF Daily",
                [("project", string("[[projects/carf.md]]"))],
            ),
        ];

        let index = RelationIndex::build(&documents);

        assert!(matches!(
            &index.all()[0].status,
            RelationStatus::Resolved(target) if target.relative_path == Path::new("projects/carf.md")
        ));
    }

    #[test]
    fn display_label_is_parsed_when_present() {
        let target = parse_wikilink("[[projects/carf.md|CARF]]").unwrap();

        assert_eq!(target.raw, "projects/carf.md");
        assert_eq!(target.display, "CARF");
    }

    #[test]
    fn arrays_and_mixed_arrays_only_emit_wikilinks() {
        let documents = vec![
            document("people/sergio.md", "Sergio", []),
            document("people/maria.md", "Maria", []),
            document(
                "meetings/carf.md",
                "CARF Daily",
                [(
                    "participants",
                    PropertyValue::Array(vec![
                        string("[[Sergio]]"),
                        string("visitante"),
                        string("[[Maria]]"),
                    ]),
                )],
            ),
        ];

        let index = RelationIndex::build(&documents);

        assert_eq!(index.all().len(), 2);
        assert!(index
            .all()
            .iter()
            .all(|relation| relation.property == "participants"));
    }

    #[test]
    fn normal_array_is_not_a_relation() {
        let index = RelationIndex::build(&[document(
            "projects/carf.md",
            "CARF",
            [(
                "tags",
                PropertyValue::Array(vec![string("rust"), string("markdown")]),
            )],
        )]);

        assert!(index.all().is_empty());
    }

    #[test]
    fn outgoing_and_incoming_are_queryable() {
        let documents = vec![
            document("projects/carf.md", "CARF", []),
            document(
                "meetings/daily.md",
                "CARF Daily",
                [("project", string("[[CARF]]"))],
            ),
            document(
                "meetings/weekly.md",
                "CARF Weekly",
                [("related_project", string("[[CARF]]"))],
            ),
        ];
        let index = RelationIndex::build(&documents);
        let source = PathBuf::from("/workspace/meetings/daily.md");
        let target = PathBuf::from("/workspace/projects/carf.md");

        assert_eq!(index.outgoing(&source).len(), 1);
        assert_eq!(index.incoming(&target).len(), 2);
    }

    #[test]
    fn unicode_title_resolves_exactly() {
        let documents = vec![
            document("people/sergio.md", "Sérgio", []),
            document(
                "projects/carf.md",
                "CARF",
                [("owner", string("[[Sérgio]]"))],
            ),
        ];

        let index = RelationIndex::build(&documents);

        assert!(matches!(index.all()[0].status, RelationStatus::Resolved(_)));
    }

    #[test]
    fn cycles_and_self_relations_do_not_recurse_or_panic() {
        let documents = vec![
            document(
                "a.md",
                "A",
                [("next", string("[[B]]")), ("self", string("[[A]]"))],
            ),
            document("b.md", "B", [("next", string("[[A]]"))]),
        ];

        let index = RelationIndex::build(&documents);

        assert_eq!(index.all().len(), 3);
        assert_eq!(index.incoming(&PathBuf::from("/workspace/a.md")).len(), 2);
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
            metadata: DocumentMetadata {
                file_size: None,
                modified: None,
            },
            title: title.to_owned(),
            source_content: Some(String::new()),
            markdown_content: String::new(),
            properties: properties
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect::<BTreeMap<_, _>>(),
            document_type: None,
            collection_id: String::from("documents"),
            warnings: Vec::new(),
        }
    }

    fn string(value: &str) -> PropertyValue {
        PropertyValue::String(value.to_owned())
    }
}
