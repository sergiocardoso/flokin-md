use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use crate::{
    schema_type_for_property_value, Document, ExplicitSchemaState, RelationIndex, RelationStatus,
    ScanError, SchemaCatalog, SchemaField, SchemaSource, SchemaType,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DatabaseHealth {
    pub summary: HealthSummary,
    pub collection_summaries: Vec<CollectionHealthSummary>,
    pub issues: Vec<HealthIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HealthSummary {
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
    pub healthy_documents: usize,
    pub total_documents: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionHealthSummary {
    pub collection_id: String,
    pub display_name: String,
    pub document_count: usize,
    pub issue_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthIssue {
    pub id: String,
    pub severity: HealthSeverity,
    pub category: HealthCategory,
    pub kind: HealthIssueKind,
    pub document_path: Option<PathBuf>,
    pub relative_path: Option<PathBuf>,
    pub collection_id: Option<String>,
    pub property: Option<String>,
    pub message: String,
    pub details: Vec<String>,
    pub expected: Option<SchemaType>,
    pub found: Option<SchemaType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealthSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealthCategory {
    Parsing,
    Schema,
    Relations,
    Workspace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthIssueKind {
    InvalidFrontmatter,
    FileReadError,
    WorkspaceScanError,
    ExplicitSchemaInvalid,
    RequiredFieldMissing,
    TypeMismatch,
    UndeclaredField,
    MixedObservedTypes,
    RelationUnresolved,
    RelationAmbiguous,
}

impl HealthSeverity {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Error => "Error",
            Self::Warning => "Warning",
            Self::Info => "Info",
        }
    }
}

impl HealthCategory {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Parsing => "Parsing",
            Self::Schema => "Schema",
            Self::Relations => "Relations",
            Self::Workspace => "Workspace",
        }
    }
}

pub fn build_health(
    documents: &[Document],
    workspace_errors: &[ScanError],
    schema_catalog: &SchemaCatalog,
    relation_index: &RelationIndex,
) -> DatabaseHealth {
    let mut issues = Vec::new();
    collect_parsing_issues(documents, workspace_errors, &mut issues);
    collect_explicit_schema_issues(schema_catalog, &mut issues);
    collect_schema_issues(documents, schema_catalog, relation_index, &mut issues);
    collect_relation_issues(relation_index, &mut issues);
    issues.sort_by(compare_issues);

    let unhealthy_documents = issues
        .iter()
        .filter(|issue| {
            matches!(
                issue.severity,
                HealthSeverity::Error | HealthSeverity::Warning
            )
        })
        .filter_map(|issue| issue.document_path.as_ref())
        .collect::<BTreeSet<_>>();
    let summary = HealthSummary {
        errors: issues
            .iter()
            .filter(|issue| issue.severity == HealthSeverity::Error)
            .count(),
        warnings: issues
            .iter()
            .filter(|issue| issue.severity == HealthSeverity::Warning)
            .count(),
        info: issues
            .iter()
            .filter(|issue| issue.severity == HealthSeverity::Info)
            .count(),
        healthy_documents: documents.len().saturating_sub(unhealthy_documents.len()),
        total_documents: documents.len(),
    };

    let collection_summaries = schema_catalog
        .collections
        .iter()
        .map(|collection| CollectionHealthSummary {
            collection_id: collection.collection_id.clone(),
            display_name: collection.display_name.clone(),
            document_count: collection.document_count,
            issue_count: issues
                .iter()
                .filter(|issue| issue.collection_id.as_ref() == Some(&collection.collection_id))
                .count(),
        })
        .collect();

    DatabaseHealth {
        summary,
        collection_summaries,
        issues,
    }
}

fn collect_parsing_issues(
    documents: &[Document],
    workspace_errors: &[ScanError],
    issues: &mut Vec<HealthIssue>,
) {
    for document in documents {
        for warning in &document.warnings {
            let (kind, message) = if warning.message.to_lowercase().contains("yaml") {
                (
                    HealthIssueKind::InvalidFrontmatter,
                    String::from("Frontmatter YAML inválido."),
                )
            } else {
                (
                    HealthIssueKind::FileReadError,
                    String::from("Não foi possível ler o arquivo."),
                )
            };
            issues.push(HealthIssue {
                id: issue_id(
                    HealthCategory::Parsing,
                    &kind,
                    Some(&document.relative_path),
                    None,
                    None,
                ),
                severity: HealthSeverity::Error,
                category: HealthCategory::Parsing,
                kind,
                document_path: Some(document.path.clone()),
                relative_path: Some(document.relative_path.clone()),
                collection_id: Some(document.collection_id.clone()),
                property: None,
                message,
                details: vec![warning.message.clone()],
                expected: None,
                found: None,
            });
        }
    }

    for error in workspace_errors {
        let kind = HealthIssueKind::WorkspaceScanError;
        issues.push(HealthIssue {
            id: issue_id(
                HealthCategory::Workspace,
                &kind,
                Some(&error.path),
                None,
                None,
            ),
            severity: HealthSeverity::Error,
            category: HealthCategory::Workspace,
            kind,
            document_path: None,
            relative_path: Some(error.path.clone()),
            collection_id: None,
            property: None,
            message: String::from("Erro ao processar o workspace."),
            details: vec![error.message.clone()],
            expected: None,
            found: None,
        });
    }
}

fn collect_explicit_schema_issues(schema_catalog: &SchemaCatalog, issues: &mut Vec<HealthIssue>) {
    if let ExplicitSchemaState::Invalid(warning) = &schema_catalog.explicit_schema {
        let kind = HealthIssueKind::ExplicitSchemaInvalid;
        issues.push(HealthIssue {
            id: issue_id(
                HealthCategory::Schema,
                &kind,
                Some(&warning.path),
                None,
                None,
            ),
            severity: HealthSeverity::Error,
            category: HealthCategory::Schema,
            kind,
            document_path: None,
            relative_path: Some(warning.path.clone()),
            collection_id: None,
            property: None,
            message: String::from("Schema explícito inválido."),
            details: vec![warning.message.clone()],
            expected: None,
            found: None,
        });
    }
}

fn collect_schema_issues(
    documents: &[Document],
    schema_catalog: &SchemaCatalog,
    relation_index: &RelationIndex,
    issues: &mut Vec<HealthIssue>,
) {
    let documents_by_collection = documents.iter().fold(
        BTreeMap::<String, Vec<&Document>>::new(),
        |mut grouped, document| {
            grouped
                .entry(document.collection_id.clone())
                .or_default()
                .push(document);
            grouped
        },
    );

    for collection in &schema_catalog.collections {
        if collection.source == SchemaSource::Inferred {
            for field in &collection.fields {
                if field.inferred_type == SchemaType::Mixed {
                    issues.push(mixed_issue(collection.collection_id.as_str(), field));
                }
            }
            continue;
        }

        let declared_fields = collection
            .fields
            .iter()
            .filter(|field| field.declared)
            .map(|field| (field.name.as_str(), field))
            .collect::<BTreeMap<_, _>>();
        let Some(documents) = documents_by_collection.get(&collection.collection_id) else {
            continue;
        };

        for document in documents {
            for field in declared_fields.values() {
                if field.structural {
                    continue;
                }
                match document.properties.get(&field.name) {
                    Some(value) => {
                        let found = schema_type_for_property_value(
                            document,
                            &field.name,
                            value,
                            relation_index,
                        );
                        if found != SchemaType::Null && found != field.field_type {
                            issues.push(type_mismatch_issue(document, field, found));
                        }
                    }
                    None if field.required => issues.push(required_missing_issue(document, field)),
                    None => {}
                }
            }

            for property in document.properties.keys() {
                if property == "title" || property == "type" {
                    continue;
                }
                if !declared_fields.contains_key(property.as_str()) {
                    issues.push(undeclared_field_issue(document, property));
                }
            }
        }
    }
}

fn collect_relation_issues(relation_index: &RelationIndex, issues: &mut Vec<HealthIssue>) {
    for relation in relation_index.all() {
        match &relation.status {
            RelationStatus::Resolved(_) => {}
            RelationStatus::Unresolved => {
                let kind = HealthIssueKind::RelationUnresolved;
                issues.push(HealthIssue {
                    id: issue_id(
                        HealthCategory::Relations,
                        &kind,
                        Some(&relation.source_relative_path),
                        Some(&relation.property),
                        Some(&relation.target.raw),
                    ),
                    severity: HealthSeverity::Warning,
                    category: HealthCategory::Relations,
                    kind,
                    document_path: Some(relation.source_document.clone()),
                    relative_path: Some(relation.source_relative_path.clone()),
                    collection_id: None,
                    property: Some(relation.property.clone()),
                    message: format!("Relação não resolvida: {}.", relation.target.raw),
                    details: vec![format!("Target: {}", relation.target.raw)],
                    expected: None,
                    found: None,
                });
            }
            RelationStatus::Ambiguous(candidates) => {
                let kind = HealthIssueKind::RelationAmbiguous;
                issues.push(HealthIssue {
                    id: issue_id(
                        HealthCategory::Relations,
                        &kind,
                        Some(&relation.source_relative_path),
                        Some(&relation.property),
                        Some(&relation.target.raw),
                    ),
                    severity: HealthSeverity::Error,
                    category: HealthCategory::Relations,
                    kind,
                    document_path: Some(relation.source_document.clone()),
                    relative_path: Some(relation.source_relative_path.clone()),
                    collection_id: None,
                    property: Some(relation.property.clone()),
                    message: format!(
                        "Relação ambígua: {} documentos correspondem.",
                        candidates.len()
                    ),
                    details: candidates
                        .iter()
                        .map(|candidate| candidate.relative_path.display().to_string())
                        .collect(),
                    expected: None,
                    found: None,
                });
            }
        }
    }
}

fn mixed_issue(collection_id: &str, field: &SchemaField) -> HealthIssue {
    let kind = HealthIssueKind::MixedObservedTypes;
    HealthIssue {
        id: issue_id(
            HealthCategory::Schema,
            &kind,
            None,
            Some(&field.name),
            Some(collection_id),
        ),
        severity: HealthSeverity::Warning,
        category: HealthCategory::Schema,
        kind,
        document_path: None,
        relative_path: None,
        collection_id: Some(collection_id.to_owned()),
        property: Some(field.name.clone()),
        message: String::from("Tipos inconsistentes observados."),
        details: field
            .observed_types
            .iter()
            .map(|observed| {
                format!(
                    "{}: {} documentos",
                    observed.field_type.label(),
                    observed.count
                )
            })
            .collect(),
        expected: None,
        found: Some(SchemaType::Mixed),
    }
}

fn required_missing_issue(document: &Document, field: &SchemaField) -> HealthIssue {
    let kind = HealthIssueKind::RequiredFieldMissing;
    HealthIssue {
        id: issue_id(
            HealthCategory::Schema,
            &kind,
            Some(&document.relative_path),
            Some(&field.name),
            None,
        ),
        severity: HealthSeverity::Error,
        category: HealthCategory::Schema,
        kind,
        document_path: Some(document.path.clone()),
        relative_path: Some(document.relative_path.clone()),
        collection_id: Some(document.collection_id.clone()),
        property: Some(field.name.clone()),
        message: String::from("Campo obrigatório ausente."),
        details: Vec::new(),
        expected: Some(field.field_type),
        found: None,
    }
}

fn type_mismatch_issue(document: &Document, field: &SchemaField, found: SchemaType) -> HealthIssue {
    let kind = HealthIssueKind::TypeMismatch;
    HealthIssue {
        id: issue_id(
            HealthCategory::Schema,
            &kind,
            Some(&document.relative_path),
            Some(&field.name),
            None,
        ),
        severity: HealthSeverity::Error,
        category: HealthCategory::Schema,
        kind,
        document_path: Some(document.path.clone()),
        relative_path: Some(document.relative_path.clone()),
        collection_id: Some(document.collection_id.clone()),
        property: Some(field.name.clone()),
        message: format!(
            "Esperado {}, encontrado {}.",
            field.field_type.label(),
            found.label()
        ),
        details: Vec::new(),
        expected: Some(field.field_type),
        found: Some(found),
    }
}

fn undeclared_field_issue(document: &Document, property: &str) -> HealthIssue {
    let kind = HealthIssueKind::UndeclaredField;
    HealthIssue {
        id: issue_id(
            HealthCategory::Schema,
            &kind,
            Some(&document.relative_path),
            Some(property),
            None,
        ),
        severity: HealthSeverity::Warning,
        category: HealthCategory::Schema,
        kind,
        document_path: Some(document.path.clone()),
        relative_path: Some(document.relative_path.clone()),
        collection_id: Some(document.collection_id.clone()),
        property: Some(property.to_owned()),
        message: String::from("Campo não declarado no schema explícito."),
        details: Vec::new(),
        expected: None,
        found: None,
    }
}

fn issue_id(
    category: HealthCategory,
    kind: &HealthIssueKind,
    path: Option<&PathBuf>,
    property: Option<&str>,
    extra: Option<&str>,
) -> String {
    format!(
        "{category:?}:{kind:?}:{}:{}:{}",
        path.map(|path| path.display().to_string())
            .unwrap_or_default(),
        property.unwrap_or_default(),
        extra.unwrap_or_default()
    )
}

fn compare_issues(left: &HealthIssue, right: &HealthIssue) -> std::cmp::Ordering {
    left.severity
        .cmp(&right.severity)
        .then_with(|| left.relative_path.cmp(&right.relative_path))
        .then_with(|| left.category.cmp(&right.category))
        .then_with(|| left.property.cmp(&right.property))
        .then_with(|| left.id.cmp(&right.id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Collection, DocumentMetadata, ExplicitCollectionSchema, ExplicitFieldSchema,
        ExplicitSchema, PropertyValue, RelationIndex,
    };
    use std::ffi::OsString;

    fn doc(
        collection: &str,
        relative_path: &str,
        title: &str,
        properties: &[(&str, PropertyValue)],
    ) -> Document {
        let relative_path = PathBuf::from(relative_path);
        Document {
            path: PathBuf::from("/workspace").join(&relative_path),
            relative_path: relative_path.clone(),
            file_name: relative_path
                .file_name()
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| OsString::from("doc.md")),
            metadata: DocumentMetadata {
                file_size: None,
                modified: None,
            },
            title: title.to_owned(),
            source_content: Some(String::new()),
            markdown_content: String::new(),
            properties: properties
                .iter()
                .map(|(key, value)| ((*key).to_owned(), value.clone()))
                .collect(),
            document_type: Some(collection.to_owned()),
            collection_id: collection.to_owned(),
            warnings: Vec::new(),
        }
    }

    fn collection(id: &str, count: usize) -> Collection {
        Collection {
            id: id.to_owned(),
            display_name: id.to_owned(),
            document_count: count,
        }
    }

    fn explicit(field_type: SchemaType, required: bool) -> ExplicitSchemaState {
        ExplicitSchemaState::Loaded(ExplicitSchema {
            version: 1,
            collections: BTreeMap::from([(
                String::from("project"),
                ExplicitCollectionSchema {
                    fields: BTreeMap::from([(
                        String::from("priority"),
                        ExplicitFieldSchema {
                            field_type,
                            required,
                            target: None,
                        },
                    )]),
                },
            )]),
        })
    }

    fn health(documents: &[Document], explicit_schema: ExplicitSchemaState) -> DatabaseHealth {
        let collections = vec![collection("project", documents.len())];
        let relations = RelationIndex::build(documents);
        let schema = SchemaCatalog::build(documents, &collections, &relations, explicit_schema);
        build_health(documents, &[], &schema, &relations)
    }

    #[test]
    fn explicit_schema_required_type_undeclared_and_correction() {
        let documents = vec![
            doc(
                "project",
                "projects/a.md",
                "A",
                &[
                    ("priority", PropertyValue::String(String::from("high"))),
                    ("budget", PropertyValue::Number(String::from("100"))),
                ],
            ),
            doc("project", "projects/b.md", "B", &[]),
        ];
        let database_health = health(&documents, explicit(SchemaType::Integer, true));

        assert!(database_health
            .issues
            .iter()
            .any(|issue| issue.kind == HealthIssueKind::TypeMismatch));
        assert!(database_health
            .issues
            .iter()
            .any(|issue| issue.kind == HealthIssueKind::RequiredFieldMissing));
        assert!(database_health
            .issues
            .iter()
            .any(|issue| issue.kind == HealthIssueKind::UndeclaredField));
        assert_eq!(database_health.summary.errors, 2);
        assert_eq!(database_health.summary.warnings, 1);
        assert_eq!(database_health.summary.healthy_documents, 0);

        let corrected = vec![
            doc(
                "project",
                "projects/a.md",
                "A",
                &[("priority", PropertyValue::Number(String::from("10")))],
            ),
            doc(
                "project",
                "projects/b.md",
                "B",
                &[("priority", PropertyValue::Number(String::from("20")))],
            ),
        ];
        let database_health = health(&corrected, explicit(SchemaType::Integer, true));
        assert!(database_health.issues.is_empty());
        assert_eq!(database_health.summary.healthy_documents, 2);
    }

    #[test]
    fn inferred_mixed_type_is_collection_warning() {
        let documents = vec![
            doc(
                "project",
                "projects/a.md",
                "A",
                &[("priority", PropertyValue::Number(String::from("10")))],
            ),
            doc(
                "project",
                "projects/b.md",
                "B",
                &[("priority", PropertyValue::String(String::from("high")))],
            ),
        ];
        let health = health(&documents, ExplicitSchemaState::Absent);

        assert_eq!(health.summary.errors, 0);
        assert_eq!(health.summary.warnings, 1);
        assert_eq!(health.summary.healthy_documents, 2);
        assert_eq!(health.issues[0].kind, HealthIssueKind::MixedObservedTypes);
    }

    #[test]
    fn explicit_schema_invalid_and_scan_errors_are_health_issues() {
        let documents = vec![doc("project", "projects/a.md", "A", &[])];
        let relations = RelationIndex::build(&documents);
        let schema = SchemaCatalog::build(
            &documents,
            &[collection("project", 1)],
            &relations,
            ExplicitSchemaState::Invalid(crate::SchemaWarning {
                path: PathBuf::from("/workspace/flokin.schema.yaml"),
                message: String::from("versão 999 incompatível"),
            }),
        );
        let health = build_health(
            &documents,
            &[ScanError {
                path: PathBuf::from("/workspace/missing.md"),
                message: String::from("permission denied"),
            }],
            &schema,
            &relations,
        );

        assert!(health
            .issues
            .iter()
            .any(|issue| issue.kind == HealthIssueKind::ExplicitSchemaInvalid));
        assert!(health
            .issues
            .iter()
            .any(|issue| issue.kind == HealthIssueKind::WorkspaceScanError));
    }

    #[test]
    fn parsing_warning_becomes_parsing_issue() {
        let mut document = doc("project", "projects/broken.md", "Broken", &[]);
        document.warnings.push(crate::DocumentWarning {
            path: document.path.clone(),
            message: String::from("YAML frontmatter inválido: bad"),
        });
        let health = health(&[document], ExplicitSchemaState::Absent);

        assert_eq!(health.summary.errors, 1);
        assert_eq!(health.summary.healthy_documents, 0);
        assert_eq!(health.issues[0].kind, HealthIssueKind::InvalidFrontmatter);
    }

    #[test]
    fn relation_issues_follow_relation_index_without_self_or_cycle_warnings() {
        let documents = vec![
            doc(
                "project",
                "projects/a.md",
                "A",
                &[
                    ("owner", PropertyValue::String(String::from("[[Maria]]"))),
                    ("self", PropertyValue::String(String::from("[[A]]"))),
                    ("next", PropertyValue::String(String::from("[[B]]"))),
                ],
            ),
            doc(
                "project",
                "projects/b.md",
                "B",
                &[("next", PropertyValue::String(String::from("[[A]]")))],
            ),
        ];
        let database_health = health(&documents, ExplicitSchemaState::Absent);
        assert_eq!(database_health.summary.warnings, 1);
        assert_eq!(
            database_health.issues[0].kind,
            HealthIssueKind::RelationUnresolved
        );

        let mut resolved = documents.clone();
        resolved.push(doc("project", "people/maria.md", "Maria", &[]));
        let database_health = health(&resolved, ExplicitSchemaState::Absent);
        assert!(!database_health
            .issues
            .iter()
            .any(|issue| matches!(issue.kind, HealthIssueKind::RelationUnresolved)));
    }

    #[test]
    fn ambiguous_and_broken_path_relations_are_reported() {
        let documents = vec![
            doc(
                "project",
                "projects/a.md",
                "A",
                &[
                    ("related", PropertyValue::String(String::from("[[CARF]]"))),
                    (
                        "path",
                        PropertyValue::String(String::from("[[projects/missing.md]]")),
                    ),
                ],
            ),
            doc("project", "projects/carf.md", "CARF", &[]),
            doc("project", "archive/carf.md", "CARF", &[]),
        ];
        let health = health(&documents, ExplicitSchemaState::Absent);

        assert!(health
            .issues
            .iter()
            .any(|issue| issue.kind == HealthIssueKind::RelationAmbiguous));
        assert!(health
            .issues
            .iter()
            .any(|issue| issue.kind == HealthIssueKind::RelationUnresolved));
    }

    #[test]
    fn summary_counts_document_once_and_order_is_deterministic() {
        let mut document = doc(
            "project",
            "projects/a.md",
            "A",
            &[
                ("priority", PropertyValue::String(String::from("high"))),
                ("extra", PropertyValue::Bool(true)),
            ],
        );
        document.warnings.push(crate::DocumentWarning {
            path: document.path.clone(),
            message: String::from("YAML frontmatter inválido: bad"),
        });
        let health = health(&[document], explicit(SchemaType::Integer, true));

        assert_eq!(health.summary.total_documents, 1);
        assert_eq!(health.summary.healthy_documents, 0);
        assert_eq!(health.summary.errors, 2);
        assert_eq!(health.summary.warnings, 1);
        assert_eq!(health.issues[0].severity, HealthSeverity::Error);
        assert!(health
            .issues
            .windows(2)
            .all(|pair| compare_issues(&pair[0], &pair[1]) != std::cmp::Ordering::Greater));
    }
}
