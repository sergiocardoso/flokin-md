use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};

use crate::{
    CollectionSchema, Document, EditorState, ExplicitSchemaState, SchemaCatalog, SchemaSource,
    SchemaType,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkEditSelection {
    pub collection_id: String,
    pub document_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BulkEditOperation {
    SetProperty {
        property: String,
        value: BulkEditValue,
    },
    RemoveProperty {
        property: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BulkEditValue {
    String(String),
    Integer(String),
    Float(String),
    Boolean(bool),
    Null,
    Relation(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkEditPlan {
    pub collection_id: String,
    pub operation: BulkEditOperation,
    pub changes: Vec<BulkEditFileChange>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkEditFileChange {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub original_fingerprint: u64,
    pub before: Option<String>,
    pub after: Option<String>,
    pub status: BulkEditChangeStatus,
    pub reason: Option<String>,
    pub new_content: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkEditChangeStatus {
    Changed,
    NoChange,
    Blocked,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkEditSummary {
    pub selected: usize,
    pub changed: usize,
    pub no_change: usize,
    pub blocked: usize,
    pub unsupported: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BulkEditApplyError {
    StalePreview {
        path: PathBuf,
    },
    Preflight {
        path: PathBuf,
        message: String,
    },
    Stage {
        path: PathBuf,
        message: String,
    },
    Commit {
        path: PathBuf,
        message: String,
        rollback_failed: Vec<PathBuf>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkEditResult {
    pub changed_paths: Vec<PathBuf>,
}

impl BulkEditSelection {
    pub fn new(collection_id: String, mut document_paths: Vec<PathBuf>) -> Self {
        document_paths.sort();
        document_paths.dedup();
        Self {
            collection_id,
            document_paths,
        }
    }
}

impl BulkEditPlan {
    pub fn summary(&self) -> BulkEditSummary {
        let mut summary = BulkEditSummary {
            selected: self.changes.len(),
            changed: 0,
            no_change: 0,
            blocked: 0,
            unsupported: 0,
        };
        for change in &self.changes {
            match change.status {
                BulkEditChangeStatus::Changed => summary.changed += 1,
                BulkEditChangeStatus::NoChange => summary.no_change += 1,
                BulkEditChangeStatus::Blocked => summary.blocked += 1,
                BulkEditChangeStatus::Unsupported => summary.unsupported += 1,
            }
        }
        summary
    }

    pub fn can_apply(&self) -> bool {
        let summary = self.summary();
        summary.changed > 0 && summary.blocked == 0 && summary.unsupported == 0
    }
}

impl BulkEditValue {
    pub fn schema_type(&self) -> SchemaType {
        match self {
            Self::String(_) => SchemaType::String,
            Self::Integer(_) => SchemaType::Integer,
            Self::Float(_) => SchemaType::Float,
            Self::Boolean(_) => SchemaType::Boolean,
            Self::Null => SchemaType::Null,
            Self::Relation(_) => SchemaType::Relation,
        }
    }

    fn yaml_scalar(&self) -> String {
        match self {
            Self::String(value) => quote_yaml_string(value),
            Self::Integer(value) | Self::Float(value) => value.trim().to_owned(),
            Self::Boolean(true) => String::from("true"),
            Self::Boolean(false) => String::from("false"),
            Self::Null => String::from("null"),
            Self::Relation(target) => {
                let trimmed = target.trim();
                let wikilink = if trimmed.starts_with("[[") && trimmed.ends_with("]]") {
                    trimmed.to_owned()
                } else {
                    format!("[[{trimmed}]]")
                };
                quote_yaml_string(&wikilink)
            }
        }
    }
}

pub fn validate_bulk_edit_operation(
    collection_id: &str,
    operation: &BulkEditOperation,
    schema_catalog: &SchemaCatalog,
) -> Result<Vec<String>, String> {
    let Some(schema) = schema_catalog.collection(collection_id) else {
        return Ok(Vec::new());
    };
    let property = operation.property();
    let field = schema.fields.iter().find(|field| field.name == property);
    let mut warnings = Vec::new();

    match operation {
        BulkEditOperation::SetProperty { value, .. } => {
            if let Some(field) = field {
                if field.declared {
                    let expected = field.field_type;
                    if !schema_type_accepts(expected, value.schema_type()) {
                        return Err(format!(
                            "{}.{property} espera {}.",
                            schema.display_name,
                            expected.label()
                        ));
                    }
                }
                if matches!(field.field_type, SchemaType::Array | SchemaType::Object) {
                    return Err(String::from("Bulk edit deste tipo ainda não é suportado."));
                }
            } else if schema.source == SchemaSource::Explicit {
                warnings.push(String::from("Campo não declarado no schema explícito."));
            }
        }
        BulkEditOperation::RemoveProperty { .. } => {
            if let Some(field) = field {
                if field.declared && field.required {
                    return Err(format!("{property} é obrigatório no schema explícito."));
                }
                if matches!(field.field_type, SchemaType::Array | SchemaType::Object) {
                    return Err(String::from("Bulk edit deste tipo ainda não é suportado."));
                }
            }
        }
    }

    Ok(warnings)
}

impl BulkEditOperation {
    pub fn property(&self) -> &str {
        match self {
            Self::SetProperty { property, .. } | Self::RemoveProperty { property } => property,
        }
    }
}

fn schema_type_accepts(expected: SchemaType, actual: SchemaType) -> bool {
    matches!(
        (expected, actual),
        (SchemaType::String, SchemaType::String)
            | (SchemaType::Integer, SchemaType::Integer)
            | (SchemaType::Float, SchemaType::Float)
            | (SchemaType::Boolean, SchemaType::Boolean)
            | (SchemaType::Relation, SchemaType::Relation)
            | (SchemaType::Null, SchemaType::Null)
            | (SchemaType::Float, SchemaType::Integer)
            | (SchemaType::Mixed | SchemaType::Unknown, _)
            | (_, SchemaType::Null)
    )
}

pub fn build_bulk_edit_plan(
    selection: BulkEditSelection,
    operation: BulkEditOperation,
    documents: &[Document],
    editor: &EditorState,
    schema_catalog: &SchemaCatalog,
) -> Result<BulkEditPlan, String> {
    if selection.document_paths.is_empty() {
        return Err(String::from("Nenhum documento selecionado."));
    }
    let warnings =
        validate_bulk_edit_operation(selection.collection_id.as_str(), &operation, schema_catalog)?;

    let selected = selection
        .document_paths
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let documents_by_path = documents
        .iter()
        .map(|document| (document.path.clone(), document))
        .collect::<BTreeMap<_, _>>();
    let mut changes = Vec::new();

    for path in selected {
        let Some(document) = documents_by_path.get(&path) else {
            changes.push(blocked_change(
                path.clone(),
                PathBuf::from(""),
                0,
                "Documento ausente.",
            ));
            continue;
        };
        if document.collection_id != selection.collection_id {
            changes.push(blocked_change(
                path.clone(),
                document.relative_path.clone(),
                document
                    .source_content
                    .as_deref()
                    .map(content_fingerprint)
                    .unwrap_or(0),
                "Documento não pertence à Collection atual.",
            ));
            continue;
        }
        let Some(source) = document.source_content.as_deref() else {
            changes.push(blocked_change(
                path.clone(),
                document.relative_path.clone(),
                0,
                "Não foi possível ler o conteúdo do arquivo.",
            ));
            continue;
        };
        let fingerprint = content_fingerprint(source);
        if let Some(tab) = editor.tab(&path) {
            if tab.dirty {
                changes.push(blocked_change(
                    path.clone(),
                    document.relative_path.clone(),
                    fingerprint,
                    "Arquivo possui alterações não salvas.",
                ));
                continue;
            }
            if tab.external_conflict.is_some() {
                changes.push(blocked_change(
                    path.clone(),
                    document.relative_path.clone(),
                    fingerprint,
                    "Arquivo possui conflito com alteração externa.",
                ));
                continue;
            }
        }

        changes.push(plan_file_change(document, source, fingerprint, &operation));
    }

    Ok(BulkEditPlan {
        collection_id: selection.collection_id,
        operation,
        changes,
        warnings,
    })
}

fn blocked_change(
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
        status: BulkEditChangeStatus::Blocked,
        reason: Some(reason.to_owned()),
        new_content: None,
    }
}

fn plan_file_change(
    document: &Document,
    source: &str,
    fingerprint: u64,
    operation: &BulkEditOperation,
) -> BulkEditFileChange {
    match patch_frontmatter(source, operation) {
        Ok(PatchOutcome::Changed {
            before,
            after,
            content,
        }) => BulkEditFileChange {
            path: document.path.clone(),
            relative_path: document.relative_path.clone(),
            original_fingerprint: fingerprint,
            before,
            after,
            status: BulkEditChangeStatus::Changed,
            reason: None,
            new_content: Some(content),
        },
        Ok(PatchOutcome::NoChange { before }) => BulkEditFileChange {
            path: document.path.clone(),
            relative_path: document.relative_path.clone(),
            original_fingerprint: fingerprint,
            before,
            after: None,
            status: BulkEditChangeStatus::NoChange,
            reason: Some(String::from("No change")),
            new_content: None,
        },
        Err(message) => BulkEditFileChange {
            path: document.path.clone(),
            relative_path: document.relative_path.clone(),
            original_fingerprint: fingerprint,
            before: None,
            after: None,
            status: BulkEditChangeStatus::Unsupported,
            reason: Some(message),
            new_content: None,
        },
    }
}

enum PatchOutcome {
    Changed {
        before: Option<String>,
        after: Option<String>,
        content: String,
    },
    NoChange {
        before: Option<String>,
    },
}

fn patch_frontmatter(source: &str, operation: &BulkEditOperation) -> Result<PatchOutcome, String> {
    let newline = detect_newline(source);
    let Some(bounds) = frontmatter_bounds(source) else {
        return match operation {
            BulkEditOperation::SetProperty { property, value } => {
                let mut content = String::new();
                content.push_str("---");
                content.push_str(newline);
                content.push_str(property);
                content.push_str(": ");
                content.push_str(value.yaml_scalar().as_str());
                content.push_str(newline);
                content.push_str("---");
                content.push_str(newline);
                content.push_str(source);
                Ok(PatchOutcome::Changed {
                    before: None,
                    after: Some(format!("{property}: {}", value.yaml_scalar())),
                    content,
                })
            }
            BulkEditOperation::RemoveProperty { .. } => Ok(PatchOutcome::NoChange { before: None }),
        };
    };

    let yaml = &source[bounds.content_start..bounds.content_end];
    let entries = top_level_entries(yaml, bounds.content_start)?;
    let property = operation.property();
    let entry = entries.iter().find(|entry| entry.key == property);

    match operation {
        BulkEditOperation::SetProperty { value, .. } => {
            let rendered = value.yaml_scalar();
            if let Some(entry) = entry {
                if !entry.scalar {
                    return Err(String::from("Bulk edit deste tipo ainda não é suportado."));
                }
                let before_line = source[entry.line_start..entry.line_end_no_newline].to_owned();
                let after_line = format!("{property}: {rendered}");
                if before_line.trim() == after_line {
                    return Ok(PatchOutcome::NoChange {
                        before: Some(before_line),
                    });
                }
                let mut content = String::new();
                content.push_str(&source[..entry.line_start]);
                content.push_str(&after_line);
                content.push_str(&source[entry.line_end_no_newline..]);
                Ok(PatchOutcome::Changed {
                    before: Some(before_line),
                    after: Some(after_line),
                    content,
                })
            } else {
                let insertion = format!("{property}: {rendered}{newline}");
                let mut content = String::new();
                content.push_str(&source[..bounds.closing_start]);
                content.push_str(&insertion);
                content.push_str(&source[bounds.closing_start..]);
                Ok(PatchOutcome::Changed {
                    before: None,
                    after: Some(insertion.trim_end().to_owned()),
                    content,
                })
            }
        }
        BulkEditOperation::RemoveProperty { .. } => {
            let Some(entry) = entry else {
                return Ok(PatchOutcome::NoChange { before: None });
            };
            if !entry.scalar {
                return Err(String::from("Bulk edit deste tipo ainda não é suportado."));
            }
            let before_line = source[entry.line_start..entry.line_end_no_newline].to_owned();
            let mut content = String::new();
            content.push_str(&source[..entry.line_start]);
            content.push_str(&source[entry.line_end..]);
            Ok(PatchOutcome::Changed {
                before: Some(before_line),
                after: None,
                content,
            })
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct FrontmatterBounds {
    content_start: usize,
    content_end: usize,
    closing_start: usize,
}

fn frontmatter_bounds(source: &str) -> Option<FrontmatterBounds> {
    if !source.starts_with("---\n") && !source.starts_with("---\r\n") {
        return None;
    }
    let first_newline = source.find('\n')? + 1;
    let mut cursor = first_newline;
    while cursor < source.len() {
        let next = source[cursor..]
            .find('\n')
            .map(|offset| cursor + offset + 1)
            .unwrap_or(source.len());
        let line_end_no_newline = if next > cursor && source.as_bytes()[next - 1] == b'\n' {
            let mut end = next - 1;
            if end > cursor && source.as_bytes()[end - 1] == b'\r' {
                end -= 1;
            }
            end
        } else {
            next
        };
        if source[cursor..line_end_no_newline].trim() == "---" {
            return Some(FrontmatterBounds {
                content_start: first_newline,
                content_end: cursor,
                closing_start: cursor,
            });
        }
        cursor = next;
    }
    None
}

#[derive(Debug)]
struct YamlEntry {
    key: String,
    line_start: usize,
    line_end: usize,
    line_end_no_newline: usize,
    scalar: bool,
}

fn top_level_entries(yaml: &str, source_offset: usize) -> Result<Vec<YamlEntry>, String> {
    let mut entries = Vec::new();
    let mut cursor = 0;
    while cursor < yaml.len() {
        let next = yaml[cursor..]
            .find('\n')
            .map(|offset| cursor + offset + 1)
            .unwrap_or(yaml.len());
        let line = &yaml[cursor..next];
        let line_no_newline = line.trim_end_matches(['\r', '\n']);
        if !line_no_newline.trim().is_empty()
            && !line_no_newline.starts_with(char::is_whitespace)
            && !line_no_newline.trim_start().starts_with('#')
        {
            let Some(colon) = line_no_newline.find(':') else {
                return Err(String::from("Frontmatter YAML complexo não suportado."));
            };
            let key = line_no_newline[..colon].trim();
            if key.is_empty() || key.contains(['{', '}', '[', ']', ',']) {
                return Err(String::from("Frontmatter YAML complexo não suportado."));
            }
            let value = line_no_newline[colon + 1..].trim();
            let scalar = !value.is_empty()
                && !value.starts_with(['[', '{'])
                && !value.starts_with('&')
                && !value.starts_with('*');
            entries.push(YamlEntry {
                key: unquote_key(key),
                line_start: source_offset + cursor,
                line_end: source_offset + next,
                line_end_no_newline: source_offset + cursor + line_no_newline.len(),
                scalar,
            });
        }
        cursor = next;
    }
    Ok(entries)
}

fn unquote_key(key: &str) -> String {
    key.trim_matches('"').trim_matches('\'').to_owned()
}

fn quote_yaml_string(value: &str) -> String {
    if value.is_empty()
        || value == "null"
        || matches!(value, "true" | "false")
        || value
            .chars()
            .any(|c| matches!(c, ':' | '#' | '[' | ']' | '{' | '}' | '\n' | '\r'))
        || value.starts_with(char::is_whitespace)
        || value.ends_with(char::is_whitespace)
    {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        value.to_owned()
    }
}

fn detect_newline(source: &str) -> &'static str {
    if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

pub fn content_fingerprint(content: &str) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET;
    for byte in content.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

pub fn apply_bulk_edit_plan(plan: &BulkEditPlan) -> Result<BulkEditResult, BulkEditApplyError> {
    if !plan.can_apply() {
        return Ok(BulkEditResult {
            changed_paths: Vec::new(),
        });
    }
    let changed = plan
        .changes
        .iter()
        .filter(|change| change.status == BulkEditChangeStatus::Changed)
        .collect::<Vec<_>>();

    for change in &changed {
        let current = read_current(change)?;
        if content_fingerprint(&current) != change.original_fingerprint {
            return Err(BulkEditApplyError::StalePreview {
                path: change.path.clone(),
            });
        }
        if change.new_content.is_none() {
            return Err(BulkEditApplyError::Preflight {
                path: change.path.clone(),
                message: String::from("Conteúdo final ausente."),
            });
        }
        let file = fs::OpenOptions::new()
            .write(true)
            .open(&change.path)
            .map_err(|error| BulkEditApplyError::Preflight {
                path: change.path.clone(),
                message: error.to_string(),
            })?;
        drop(file);
    }

    let mut staged = Vec::<(PathBuf, PathBuf, String)>::new();
    for change in &changed {
        let temp_path = temp_path_for(&change.path, "stage");
        if let Err(error) = fs::write(&temp_path, change.new_content.as_deref().unwrap()) {
            cleanup_paths(staged.iter().map(|(_, temp, _)| temp));
            return Err(BulkEditApplyError::Stage {
                path: change.path.clone(),
                message: error.to_string(),
            });
        }
        let original = fs::read_to_string(&change.path).map_err(|error| {
            cleanup_paths(staged.iter().map(|(_, temp, _)| temp));
            let _ = fs::remove_file(&temp_path);
            BulkEditApplyError::Preflight {
                path: change.path.clone(),
                message: error.to_string(),
            }
        })?;
        staged.push((change.path.clone(), temp_path, original));
    }

    let mut committed = Vec::<(PathBuf, String)>::new();
    for (path, temp_path, original) in staged {
        if let Err(error) = fs::rename(&temp_path, &path) {
            let rollback_failed = rollback(committed);
            let _ = fs::remove_file(&temp_path);
            return Err(BulkEditApplyError::Commit {
                path,
                message: error.to_string(),
                rollback_failed,
            });
        }
        committed.push((path, original));
    }

    Ok(BulkEditResult {
        changed_paths: changed
            .into_iter()
            .map(|change| change.path.clone())
            .collect(),
    })
}

fn read_current(change: &BulkEditFileChange) -> Result<String, BulkEditApplyError> {
    fs::read_to_string(&change.path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            BulkEditApplyError::StalePreview {
                path: change.path.clone(),
            }
        } else {
            BulkEditApplyError::Preflight {
                path: change.path.clone(),
                message: error.to_string(),
            }
        }
    })
}

fn temp_path_for(path: &Path, label: &str) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    parent.join(format!(
        ".{file_name}.flokinmd-bulk-{label}-{}.tmp",
        std::process::id()
    ))
}

fn cleanup_paths<'a>(paths: impl Iterator<Item = &'a PathBuf>) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

fn rollback(committed: Vec<(PathBuf, String)>) -> Vec<PathBuf> {
    let mut failed = Vec::new();
    for (path, original) in committed.into_iter().rev() {
        if fs::write(&path, original).is_err() {
            failed.push(path);
        }
    }
    failed
}

pub fn selectable_properties(
    schema: Option<&CollectionSchema>,
    documents: &[&Document],
) -> Vec<String> {
    let mut properties = BTreeSet::new();
    if let Some(schema) = schema {
        for field in &schema.fields {
            if !field.structural {
                properties.insert(field.name.clone());
            }
        }
    }
    for document in documents {
        properties.extend(document.properties.keys().cloned());
    }
    properties.into_iter().collect()
}

pub fn explicit_schema_loaded(schema_catalog: &SchemaCatalog) -> bool {
    matches!(
        schema_catalog.explicit_schema,
        ExplicitSchemaState::Loaded(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        scan_workspace, Collection, ExplicitCollectionSchema, ExplicitFieldSchema, ExplicitSchema,
        RelationIndex, SchemaCatalog,
    };
    use std::{
        env, fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn patches_supported_scalars_without_touching_body_or_unrelated_frontmatter() {
        let cases = [
            (
                "set existing String",
                "---\ntitle: CARF\nstatus: active\npriority: 10\n# importante\n---\n# Body\n",
                BulkEditOperation::SetProperty {
                    property: String::from("status"),
                    value: BulkEditValue::String(String::from("archived")),
                },
                "---\ntitle: CARF\nstatus: archived\npriority: 10\n# importante\n---\n# Body\n",
            ),
            (
                "set Integer",
                "---\npriority: 1\n---\n# Body\n",
                BulkEditOperation::SetProperty {
                    property: String::from("priority"),
                    value: BulkEditValue::Integer(String::from("10")),
                },
                "---\npriority: 10\n---\n# Body\n",
            ),
            (
                "set Float",
                "---\nscore: 1.0\n---\n# Body\n",
                BulkEditOperation::SetProperty {
                    property: String::from("score"),
                    value: BulkEditValue::Float(String::from("10.5")),
                },
                "---\nscore: 10.5\n---\n# Body\n",
            ),
            (
                "Boolean true",
                "---\npublished: false\n---\n# Body\n",
                BulkEditOperation::SetProperty {
                    property: String::from("published"),
                    value: BulkEditValue::Boolean(true),
                },
                "---\npublished: true\n---\n# Body\n",
            ),
            (
                "Boolean false",
                "---\npublished: true\n---\n# Body\n",
                BulkEditOperation::SetProperty {
                    property: String::from("published"),
                    value: BulkEditValue::Boolean(false),
                },
                "---\npublished: false\n---\n# Body\n",
            ),
            (
                "Null",
                "---\nowner: Sergio\n---\n# Body\n",
                BulkEditOperation::SetProperty {
                    property: String::from("owner"),
                    value: BulkEditValue::Null,
                },
                "---\nowner: null\n---\n# Body\n",
            ),
            (
                "Relation",
                "---\nowner: Sergio\n---\n# Body\n",
                BulkEditOperation::SetProperty {
                    property: String::from("owner"),
                    value: BulkEditValue::Relation(String::from("CARF")),
                },
                "---\nowner: \"[[CARF]]\"\n---\n# Body\n",
            ),
            (
                "add missing property",
                "---\ntitle: CARF\n---\n# Body\n",
                BulkEditOperation::SetProperty {
                    property: String::from("owner"),
                    value: BulkEditValue::String(String::from("Sergio")),
                },
                "---\ntitle: CARF\nowner: Sergio\n---\n# Body\n",
            ),
            (
                "remove property",
                "---\ntitle: CARF\nowner: Sergio\n---\n# Body\n",
                BulkEditOperation::RemoveProperty {
                    property: String::from("owner"),
                },
                "---\ntitle: CARF\n---\n# Body\n",
            ),
            (
                "no frontmatter add",
                "# Existing body\n",
                BulkEditOperation::SetProperty {
                    property: String::from("status"),
                    value: BulkEditValue::String(String::from("active")),
                },
                "---\nstatus: active\n---\n# Existing body\n",
            ),
            (
                "unicode",
                "---\ntitle: Ações\n---\n# Visão\n",
                BulkEditOperation::SetProperty {
                    property: String::from("cidade"),
                    value: BulkEditValue::String(String::from("São Paulo")),
                },
                "---\ntitle: Ações\ncidade: São Paulo\n---\n# Visão\n",
            ),
            (
                "crlf",
                "---\r\nstatus: active\r\n---\r\n# Body\r\n",
                BulkEditOperation::SetProperty {
                    property: String::from("status"),
                    value: BulkEditValue::String(String::from("archived")),
                },
                "---\r\nstatus: archived\r\n---\r\n# Body\r\n",
            ),
        ];

        for (name, source, operation, expected) in cases {
            let outcome = patch_frontmatter(source, &operation).unwrap();
            let PatchOutcome::Changed { content, .. } = outcome else {
                panic!("{name} should change");
            };
            assert_eq!(content, expected, "{name}");
            assert!(content.ends_with(
                source
                    .split_once("---\n#")
                    .map(|(_, body)| body)
                    .unwrap_or("")
            ));
        }
    }

    #[test]
    fn no_frontmatter_remove_and_same_value_are_noops() {
        assert!(matches!(
            patch_frontmatter(
                "# Body\n",
                &BulkEditOperation::RemoveProperty {
                    property: String::from("owner")
                }
            )
            .unwrap(),
            PatchOutcome::NoChange { .. }
        ));
        assert!(matches!(
            patch_frontmatter(
                "---\nstatus: archived\n---\n# Body\n",
                &BulkEditOperation::SetProperty {
                    property: String::from("status"),
                    value: BulkEditValue::String(String::from("archived"))
                }
            )
            .unwrap(),
            PatchOutcome::NoChange { .. }
        ));
    }

    #[test]
    fn complex_top_level_property_is_unsupported() {
        let operation = BulkEditOperation::SetProperty {
            property: String::from("tags"),
            value: BulkEditValue::String(String::from("a")),
        };
        assert!(patch_frontmatter("---\ntags:\n  - a\n---\n# Body\n", &operation).is_err());
    }

    #[test]
    fn plan_counts_are_deterministic_and_dirty_tabs_block() {
        let workspace = TestWorkspace::new("bulk-plan");
        workspace.write(
            "projects/b.md",
            "---\ntype: project\nstatus: archived\n---\n# B\n",
        );
        workspace.write(
            "projects/a.md",
            "---\ntype: project\nstatus: active\n---\n# A\n",
        );
        let scan = scan_workspace(workspace.path()).unwrap();
        let mut editor = EditorState::default();
        let dirty_path = workspace.path().join("projects/b.md");
        editor.tabs.push(crate::EditorTab {
            document_path: dirty_path.clone(),
            relative_path: PathBuf::from("projects/b.md"),
            title: String::from("b.md"),
            kind: crate::EditorTabKind::Markdown,
            buffer: String::from("dirty"),
            saved_content: String::from("saved"),
            dirty: true,
            view_mode: crate::EditorViewMode::Edit,
            split_ratio: 500,
            external_conflict: None,
            ignored_external_conflict: None,
            save_error: None,
        });
        let schema = SchemaCatalog::build(
            &scan.documents,
            &scan.collections,
            &RelationIndex::build(&scan.documents),
            ExplicitSchemaState::Absent,
        );
        let selection = BulkEditSelection::new(
            String::from("project"),
            vec![dirty_path, workspace.path().join("projects/a.md")],
        );
        let plan = build_bulk_edit_plan(
            selection,
            BulkEditOperation::SetProperty {
                property: String::from("status"),
                value: BulkEditValue::String(String::from("archived")),
            },
            &scan.documents,
            &editor,
            &schema,
        )
        .unwrap();

        let summary = plan.summary();
        assert_eq!(summary.selected, 2);
        assert_eq!(summary.changed, 1);
        assert_eq!(summary.blocked, 1);
        assert_eq!(
            plan.changes
                .iter()
                .map(|change| change.relative_path.clone())
                .collect::<Vec<_>>(),
            vec![
                PathBuf::from("projects/a.md"),
                PathBuf::from("projects/b.md")
            ]
        );
    }

    #[test]
    fn apply_aborts_when_file_changed_after_preview() {
        let workspace = TestWorkspace::new("bulk-stale");
        workspace.write("a.md", "---\nstatus: active\n---\n# A\n");
        let scan = scan_workspace(workspace.path()).unwrap();
        let schema = SchemaCatalog::build(
            &scan.documents,
            &scan.collections,
            &RelationIndex::build(&scan.documents),
            ExplicitSchemaState::Absent,
        );
        let plan = build_bulk_edit_plan(
            BulkEditSelection::new(
                scan.documents[0].collection_id.clone(),
                vec![workspace.path().join("a.md")],
            ),
            BulkEditOperation::SetProperty {
                property: String::from("status"),
                value: BulkEditValue::String(String::from("archived")),
            },
            &scan.documents,
            &EditorState::default(),
            &schema,
        )
        .unwrap();
        workspace.write("a.md", "---\nstatus: paused\n---\n# A\n");
        assert!(matches!(
            apply_bulk_edit_plan(&plan),
            Err(BulkEditApplyError::StalePreview { .. })
        ));
        assert_eq!(
            fs::read_to_string(workspace.path().join("a.md")).unwrap(),
            "---\nstatus: paused\n---\n# A\n"
        );
    }

    #[test]
    fn explicit_schema_blocks_type_mismatch_and_required_remove_but_warns_undeclared() {
        let collection = Collection {
            id: String::from("project"),
            display_name: String::from("Projects"),
            document_count: 1,
        };
        let mut explicit_collection = ExplicitCollectionSchema::default();
        explicit_collection.fields.insert(
            String::from("priority"),
            ExplicitFieldSchema {
                field_type: SchemaType::Integer,
                required: false,
                target: None,
            },
        );
        explicit_collection.fields.insert(
            String::from("status"),
            ExplicitFieldSchema {
                field_type: SchemaType::String,
                required: true,
                target: None,
            },
        );
        let mut explicit = ExplicitSchema::default();
        explicit
            .collections
            .insert(String::from("project"), explicit_collection);
        let catalog = SchemaCatalog::build(
            &[],
            &[collection],
            &RelationIndex::default(),
            ExplicitSchemaState::Loaded(explicit),
        );

        assert!(validate_bulk_edit_operation(
            "project",
            &BulkEditOperation::SetProperty {
                property: String::from("priority"),
                value: BulkEditValue::String(String::from("high"))
            },
            &catalog
        )
        .is_err());
        assert!(validate_bulk_edit_operation(
            "project",
            &BulkEditOperation::SetProperty {
                property: String::from("priority"),
                value: BulkEditValue::Integer(String::from("10"))
            },
            &catalog
        )
        .is_ok());
        assert!(validate_bulk_edit_operation(
            "project",
            &BulkEditOperation::RemoveProperty {
                property: String::from("status")
            },
            &catalog
        )
        .is_err());
        let warnings = validate_bulk_edit_operation(
            "project",
            &BulkEditOperation::SetProperty {
                property: String::from("owner"),
                value: BulkEditValue::Relation(String::from("CARF")),
            },
            &catalog,
        )
        .unwrap();
        assert_eq!(warnings, vec!["Campo não declarado no schema explícito."]);
    }

    struct TestWorkspace {
        root: PathBuf,
    }

    impl TestWorkspace {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = env::temp_dir().join(format!("flokinmd-{name}-{nonce}"));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        fn path(&self) -> &Path {
            &self.root
        }

        fn write(&self, relative_path: &str, content: &str) {
            let path = self.root.join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, content).unwrap();
        }
    }

    impl Drop for TestWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
