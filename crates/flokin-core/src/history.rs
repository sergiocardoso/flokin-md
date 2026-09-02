use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    content_fingerprint, BulkEditChangeStatus, BulkEditFileChange, BulkEditOperation, BulkEditPlan,
    EditorState, FrontmatterPropertyChange, SqlWritePlan,
};

pub const HISTORY_STORAGE_VERSION: i64 = 1;
pub const HISTORY_RETENTION_LIMIT: usize = 100;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationHistoryEntry {
    pub id: String,
    pub workspace_id: String,
    pub created_at_unix: i64,
    pub source: MutationSource,
    pub summary: String,
    pub sql: Option<String>,
    pub original_entry_id: Option<String>,
    pub files: Vec<HistoryFileChange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationSource {
    BulkEdit,
    SqlUpdate,
    Undo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryFileChange {
    pub relative_path: PathBuf,
    pub before_fingerprint: u64,
    pub after_fingerprint: u64,
    pub before_content: String,
    pub after_content: String,
    pub property_changes: Vec<FrontmatterPropertyChange>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HistoryState {
    pub entries: Vec<MutationHistoryEntry>,
    pub selected_entry_id: Option<String>,
    pub undo_plan: Option<BulkEditPlan>,
    pub clear_confirm: bool,
    pub error: Option<String>,
    pub last_result: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UndoBuildError {
    WrongWorkspace,
    UnsupportedSource,
    MissingFile {
        relative_path: PathBuf,
    },
    DirtyFile {
        relative_path: PathBuf,
    },
    ExternalConflict {
        relative_path: PathBuf,
    },
    StaleFile {
        relative_path: PathBuf,
    },
    ReadFile {
        relative_path: PathBuf,
        message: String,
    },
}

#[derive(Debug)]
pub struct MutationHistoryStore {
    path: PathBuf,
    connection: Connection,
}

impl HistoryState {
    pub fn selected_entry(&self) -> Option<&MutationHistoryEntry> {
        let id = self.selected_entry_id.as_ref()?;
        self.entries.iter().find(|entry| &entry.id == id)
    }

    pub fn is_entry_undone(&self, original: &MutationHistoryEntry) -> bool {
        self.entries.iter().any(|entry| {
            entry.source == MutationSource::Undo
                && entry.workspace_id == original.workspace_id
                && entry.original_entry_id.as_deref() == Some(original.id.as_str())
        })
    }

    pub fn is_entry_undoable(&self, entry: &MutationHistoryEntry) -> bool {
        entry.is_intrinsically_undoable() && !self.is_entry_undone(entry)
    }

    pub fn entry_status_label(&self, entry: &MutationHistoryEntry) -> &'static str {
        if entry.source == MutationSource::Undo {
            "undo indisponível"
        } else if self.is_entry_undone(entry) {
            "desfeita"
        } else if entry.is_intrinsically_undoable() {
            "undo disponível"
        } else {
            "undo indisponível"
        }
    }
}

impl MutationSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BulkEdit => "bulk_edit",
            Self::SqlUpdate => "sql_update",
            Self::Undo => "undo",
        }
    }

    pub const fn label_pt(self) -> &'static str {
        match self {
            Self::BulkEdit => "Bulk Edit",
            Self::SqlUpdate => "SQL Update",
            Self::Undo => "Undo",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "bulk_edit" => Some(Self::BulkEdit),
            "sql_update" => Some(Self::SqlUpdate),
            "undo" => Some(Self::Undo),
            _ => None,
        }
    }
}

impl MutationHistoryEntry {
    pub fn is_intrinsically_undoable(&self) -> bool {
        !matches!(self.source, MutationSource::Undo) && !self.files.is_empty()
    }

    pub fn is_undoable(&self) -> bool {
        self.is_intrinsically_undoable()
    }

    pub fn file_count_label(&self) -> String {
        match self.files.len() {
            1 => String::from("1 documento"),
            count => format!("{count} documentos"),
        }
    }
}

impl MutationHistoryStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, String> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "Não foi possível criar o diretório de histórico {}: {error}",
                    parent.display()
                )
            })?;
        }
        let connection = Connection::open(&path)
            .map_err(|error| format!("Não foi possível abrir histórico: {error}"))?;
        let store = Self { path, connection };
        store.migrate()?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn migrate(&self) -> Result<(), String> {
        self.connection
            .execute_batch(
                "
                PRAGMA foreign_keys = ON;
                CREATE TABLE IF NOT EXISTS history_meta (
                    key TEXT PRIMARY KEY,
                    value INTEGER NOT NULL
                );
                CREATE TABLE IF NOT EXISTS mutation_entries (
                    id TEXT PRIMARY KEY,
                    workspace_id TEXT NOT NULL,
                    created_at_unix INTEGER NOT NULL,
                    source TEXT NOT NULL,
                    summary TEXT NOT NULL,
                    sql TEXT,
                    original_entry_id TEXT
                );
                CREATE TABLE IF NOT EXISTS mutation_files (
                    entry_id TEXT NOT NULL,
                    file_index INTEGER NOT NULL,
                    relative_path TEXT NOT NULL,
                    before_fingerprint INTEGER NOT NULL,
                    after_fingerprint INTEGER NOT NULL,
                    before_content TEXT NOT NULL,
                    after_content TEXT NOT NULL,
                    property_changes_json TEXT NOT NULL,
                    PRIMARY KEY (entry_id, file_index),
                    FOREIGN KEY (entry_id) REFERENCES mutation_entries(id) ON DELETE CASCADE
                );
                ",
            )
            .map_err(|error| format!("Não foi possível migrar histórico: {error}"))?;
        let version: Option<i64> = self
            .connection
            .query_row(
                "SELECT value FROM history_meta WHERE key = 'version'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| format!("Não foi possível ler versão do histórico: {error}"))?;
        match version {
            Some(HISTORY_STORAGE_VERSION) => Ok(()),
            Some(other) => Err(format!(
                "Versão de histórico não suportada: {other}. Esperado {HISTORY_STORAGE_VERSION}."
            )),
            None => self
                .connection
                .execute(
                    "INSERT INTO history_meta (key, value) VALUES ('version', ?1)",
                    [HISTORY_STORAGE_VERSION],
                )
                .map(|_| ())
                .map_err(|error| format!("Não foi possível gravar versão do histórico: {error}")),
        }
    }

    pub fn load_workspace(&self, workspace_id: &str) -> Result<Vec<MutationHistoryEntry>, String> {
        let mut statement = self
            .connection
            .prepare(
                "
                SELECT id, workspace_id, created_at_unix, source, summary, sql, original_entry_id
                FROM mutation_entries
                WHERE workspace_id = ?1
                ORDER BY created_at_unix DESC, id DESC
                ",
            )
            .map_err(|error| format!("Não foi possível ler histórico: {error}"))?;
        let rows = statement
            .query_map([workspace_id], |row| {
                let source: String = row.get(3)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    source,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                ))
            })
            .map_err(|error| format!("Não foi possível ler histórico: {error}"))?;

        let mut entries = Vec::new();
        for row in rows {
            let (id, workspace_id, created_at_unix, source, summary, sql, original_entry_id) =
                row.map_err(|error| format!("Não foi possível ler histórico: {error}"))?;
            let source = MutationSource::from_str(&source)
                .ok_or_else(|| format!("Histórico possui source inválido: {source}"))?;
            let files = self.load_files(&id)?;
            entries.push(MutationHistoryEntry {
                id,
                workspace_id,
                created_at_unix,
                source,
                summary,
                sql,
                original_entry_id,
                files,
            });
        }
        Ok(entries)
    }

    fn load_files(&self, entry_id: &str) -> Result<Vec<HistoryFileChange>, String> {
        let mut statement = self
            .connection
            .prepare(
                "
                SELECT relative_path, before_fingerprint, after_fingerprint,
                       before_content, after_content, property_changes_json
                FROM mutation_files
                WHERE entry_id = ?1
                ORDER BY file_index ASC
                ",
            )
            .map_err(|error| format!("Não foi possível ler arquivos do histórico: {error}"))?;
        let rows = statement
            .query_map([entry_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .map_err(|error| format!("Não foi possível ler arquivos do histórico: {error}"))?;
        let mut files = Vec::new();
        for row in rows {
            let (
                relative_path,
                before_fingerprint,
                after_fingerprint,
                before_content,
                after_content,
                property_changes_json,
            ) = row
                .map_err(|error| format!("Não foi possível ler arquivos do histórico: {error}"))?;
            let property_changes = serde_json::from_str(&property_changes_json)
                .map_err(|error| format!("Histórico possui diff inválido: {error}"))?;
            files.push(HistoryFileChange {
                relative_path: PathBuf::from(relative_path),
                before_fingerprint: before_fingerprint as u64,
                after_fingerprint: after_fingerprint as u64,
                before_content,
                after_content,
                property_changes,
            });
        }
        Ok(files)
    }

    pub fn save_entry(&mut self, entry: &MutationHistoryEntry) -> Result<(), String> {
        let transaction = self
            .connection
            .transaction()
            .map_err(|error| format!("Não foi possível iniciar gravação do histórico: {error}"))?;
        transaction
            .execute(
                "
                INSERT INTO mutation_entries
                    (id, workspace_id, created_at_unix, source, summary, sql, original_entry_id)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ",
                params![
                    entry.id,
                    entry.workspace_id,
                    entry.created_at_unix,
                    entry.source.as_str(),
                    entry.summary,
                    entry.sql,
                    entry.original_entry_id
                ],
            )
            .map_err(|error| format!("Não foi possível gravar entrada de histórico: {error}"))?;

        for (index, file) in entry.files.iter().enumerate() {
            let property_changes_json = serde_json::to_string(&file.property_changes)
                .map_err(|error| format!("Não foi possível serializar diff histórico: {error}"))?;
            transaction
                .execute(
                    "
                    INSERT INTO mutation_files
                        (entry_id, file_index, relative_path, before_fingerprint,
                         after_fingerprint, before_content, after_content, property_changes_json)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                    ",
                    params![
                        entry.id,
                        index as i64,
                        file.relative_path.display().to_string(),
                        file.before_fingerprint as i64,
                        file.after_fingerprint as i64,
                        file.before_content,
                        file.after_content,
                        property_changes_json
                    ],
                )
                .map_err(|error| {
                    format!("Não foi possível gravar arquivo no histórico: {error}")
                })?;
        }

        retain_newest_entries(&transaction, &entry.workspace_id, HISTORY_RETENTION_LIMIT)?;
        transaction
            .commit()
            .map_err(|error| format!("Não foi possível concluir gravação do histórico: {error}"))
    }

    pub fn clear_workspace(&mut self, workspace_id: &str) -> Result<(), String> {
        self.connection
            .execute(
                "DELETE FROM mutation_entries WHERE workspace_id = ?1",
                [workspace_id],
            )
            .map(|_| ())
            .map_err(|error| format!("Não foi possível limpar histórico: {error}"))
    }
}

fn retain_newest_entries(
    connection: &Connection,
    workspace_id: &str,
    limit: usize,
) -> Result<(), String> {
    connection
        .execute(
            "
            DELETE FROM mutation_entries
            WHERE workspace_id = ?1
              AND id NOT IN (
                SELECT id FROM mutation_entries
                WHERE workspace_id = ?1
                ORDER BY created_at_unix DESC, id DESC
                LIMIT ?2
              )
            ",
            params![workspace_id, limit as i64],
        )
        .map(|_| ())
        .map_err(|error| format!("Não foi possível aplicar retenção do histórico: {error}"))
}

pub fn workspace_identity(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

pub fn new_history_id() -> String {
    let now = now_unix_seconds();
    let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    format!("hist-{now}-{}-{sequence}", std::process::id())
}

pub fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

pub fn bulk_history_entry(
    workspace_id: String,
    plan: &BulkEditPlan,
    changed_paths: &[PathBuf],
) -> Result<MutationHistoryEntry, String> {
    let files = history_files_from_plan(plan, changed_paths)?;
    let summary = bulk_summary(&plan.operation, files.len());
    Ok(MutationHistoryEntry {
        id: new_history_id(),
        workspace_id,
        created_at_unix: now_unix_seconds(),
        source: MutationSource::BulkEdit,
        summary,
        sql: None,
        original_entry_id: None,
        files,
    })
}

pub fn sql_history_entry(
    workspace_id: String,
    plan: &SqlWritePlan,
    changed_paths: &[PathBuf],
) -> Result<MutationHistoryEntry, String> {
    let files = history_files_from_plan(&plan.mutation_plan, changed_paths)?;
    Ok(MutationHistoryEntry {
        id: new_history_id(),
        workspace_id,
        created_at_unix: now_unix_seconds(),
        source: MutationSource::SqlUpdate,
        summary: format!(
            "SQL Update em {} — {}",
            plan.collection_display_name,
            documents_label(files.len())
        ),
        sql: Some(plan.sql.clone()),
        original_entry_id: None,
        files,
    })
}

pub fn undo_history_entry(
    workspace_id: String,
    original: &MutationHistoryEntry,
    changed_paths: &[PathBuf],
) -> Result<MutationHistoryEntry, String> {
    let changed = changed_paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<std::collections::BTreeSet<_>>();
    let mut files = Vec::new();
    for file in &original.files {
        if !changed.contains(&file.relative_path.display().to_string()) {
            continue;
        }
        files.push(HistoryFileChange {
            relative_path: file.relative_path.clone(),
            before_fingerprint: file.after_fingerprint,
            after_fingerprint: file.before_fingerprint,
            before_content: file.after_content.clone(),
            after_content: file.before_content.clone(),
            property_changes: reverse_property_changes(&file.property_changes),
        });
    }
    Ok(MutationHistoryEntry {
        id: new_history_id(),
        workspace_id,
        created_at_unix: now_unix_seconds(),
        source: MutationSource::Undo,
        summary: format!("Desfez operação {}", original.id),
        sql: None,
        original_entry_id: Some(original.id.clone()),
        files,
    })
}

fn history_files_from_plan(
    plan: &BulkEditPlan,
    changed_paths: &[PathBuf],
) -> Result<Vec<HistoryFileChange>, String> {
    let changed = changed_paths
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    plan.changes
        .iter()
        .filter(|change| {
            change.status == BulkEditChangeStatus::Changed && changed.contains(&change.path)
        })
        .map(history_file_from_change)
        .collect()
}

fn history_file_from_change(change: &BulkEditFileChange) -> Result<HistoryFileChange, String> {
    let before_content = change
        .original_content
        .clone()
        .ok_or_else(|| format!("Conteúdo original ausente para {}", change.path.display()))?;
    let after_content = fs::read_to_string(&change.path).map_err(|error| {
        format!(
            "Não foi possível ler {} após commit: {error}",
            change.path.display()
        )
    })?;
    Ok(HistoryFileChange {
        relative_path: change.relative_path.clone(),
        before_fingerprint: change.original_fingerprint,
        after_fingerprint: content_fingerprint(&after_content),
        before_content,
        after_content,
        property_changes: change.property_changes.clone(),
    })
}

pub fn build_undo_plan(
    workspace: &Path,
    entry: &MutationHistoryEntry,
    editor: &EditorState,
) -> Result<BulkEditPlan, UndoBuildError> {
    if entry.workspace_id != workspace_identity(workspace) {
        return Err(UndoBuildError::WrongWorkspace);
    }
    if !entry.is_undoable() {
        return Err(UndoBuildError::UnsupportedSource);
    }
    let mut changes = Vec::new();
    for file in &entry.files {
        let path = workspace.join(&file.relative_path);
        let relative_path = file.relative_path.clone();
        if !path.exists() {
            return Err(UndoBuildError::MissingFile { relative_path });
        }
        if let Some(tab) = editor.tab(&path) {
            if tab.dirty {
                return Err(UndoBuildError::DirtyFile { relative_path });
            }
            if tab.external_conflict.is_some() {
                return Err(UndoBuildError::ExternalConflict { relative_path });
            }
        }
        let current = fs::read_to_string(&path).map_err(|error| UndoBuildError::ReadFile {
            relative_path: relative_path.clone(),
            message: error.to_string(),
        })?;
        if content_fingerprint(&current) != file.after_fingerprint {
            return Err(UndoBuildError::StaleFile { relative_path });
        }
        changes.push(BulkEditFileChange {
            path,
            relative_path: file.relative_path.clone(),
            original_fingerprint: file.after_fingerprint,
            original_content: Some(file.after_content.clone()),
            before: None,
            after: None,
            property_changes: reverse_property_changes(&file.property_changes),
            status: BulkEditChangeStatus::Changed,
            reason: None,
            new_content: Some(file.before_content.clone()),
        });
    }

    Ok(BulkEditPlan {
        collection_id: String::from("history"),
        operation: BulkEditOperation::SetProperty {
            property: String::from("__undo__"),
            value: crate::BulkEditValue::String(String::new()),
        },
        changes,
        warnings: Vec::new(),
    })
}

fn reverse_property_changes(
    changes: &[FrontmatterPropertyChange],
) -> Vec<FrontmatterPropertyChange> {
    changes
        .iter()
        .map(|change| FrontmatterPropertyChange {
            property: change.property.clone(),
            before: change.after.clone(),
            after: change.before.clone(),
        })
        .collect()
}

fn bulk_summary(operation: &BulkEditOperation, count: usize) -> String {
    match operation {
        BulkEditOperation::SetProperty { property, value } => {
            format!(
                "Definiu {property} = {} em {}",
                bulk_value_display(value),
                documents_label(count)
            )
        }
        BulkEditOperation::RemoveProperty { property } => {
            format!("Removeu {property} de {}", documents_label(count))
        }
    }
}

fn bulk_value_display(value: &crate::BulkEditValue) -> String {
    match value {
        crate::BulkEditValue::String(value)
        | crate::BulkEditValue::Integer(value)
        | crate::BulkEditValue::Float(value)
        | crate::BulkEditValue::Relation(value) => value.clone(),
        crate::BulkEditValue::Boolean(value) => value.to_string(),
        crate::BulkEditValue::Null => String::from("null"),
    }
}

fn documents_label(count: usize) -> String {
    match count {
        1 => String::from("1 documento"),
        count => format!("{count} documentos"),
    }
}

impl std::fmt::Display for UndoBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongWorkspace => f.write_str("Esta operação pertence a outro workspace."),
            Self::UnsupportedSource => f.write_str("Undo desta operação não é suportado."),
            Self::MissingFile { .. } => f.write_str("O arquivo não existe mais."),
            Self::DirtyFile { .. } => {
                f.write_str("Um ou mais arquivos possuem alterações não salvas.")
            }
            Self::ExternalConflict { .. } => {
                f.write_str("Um ou mais arquivos possuem conflito com alteração externa.")
            }
            Self::StaleFile { .. } => {
                f.write_str("Um ou mais arquivos foram alterados após esta operação.")
            }
            Self::ReadFile { message, .. } => write!(f, "Não foi possível ler arquivo: {message}"),
        }
    }
}

impl std::error::Error for UndoBuildError {}

impl From<io::Error> for UndoBuildError {
    fn from(error: io::Error) -> Self {
        Self::ReadFile {
            relative_path: PathBuf::new(),
            message: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        apply_bulk_edit_plan, scan_workspace, BulkEditSelection, BulkEditValue,
        EditorExternalConflict, EditorState, EditorTab, EditorTabKind, EditorViewMode,
    };
    use std::{env, time::SystemTime};

    #[test]
    fn storage_saves_loads_isolates_and_clears_workspace_entries() {
        let temp = TempDir::new();
        let db = temp.path().join("history.sqlite3");
        let mut store = MutationHistoryStore::open(&db).unwrap();
        let first = sample_entry("one");
        let second = sample_entry("two");

        store.save_entry(&first).unwrap();
        store.save_entry(&second).unwrap();

        assert_eq!(store.load_workspace("one").unwrap().len(), 1);
        assert_eq!(store.load_workspace("two").unwrap().len(), 1);

        store.clear_workspace("one").unwrap();
        assert!(store.load_workspace("one").unwrap().is_empty());
        assert_eq!(store.load_workspace("two").unwrap().len(), 1);
    }

    #[test]
    fn storage_persists_across_store_instances() {
        let temp = TempDir::new();
        let db = temp.path().join("history.sqlite3");
        MutationHistoryStore::open(&db)
            .unwrap()
            .save_entry(&sample_entry("workspace"))
            .unwrap();

        let store = MutationHistoryStore::open(&db).unwrap();

        assert_eq!(store.load_workspace("workspace").unwrap().len(), 1);
    }

    #[test]
    fn storage_applies_retention_limit_per_workspace() {
        let temp = TempDir::new();
        let db = temp.path().join("history.sqlite3");
        let mut store = MutationHistoryStore::open(&db).unwrap();
        for index in 0..(HISTORY_RETENTION_LIMIT + 5) {
            let mut entry = sample_entry("workspace");
            entry.id = format!("entry-{index:03}");
            entry.created_at_unix = index as i64;
            store.save_entry(&entry).unwrap();
        }

        let entries = store.load_workspace("workspace").unwrap();

        assert_eq!(entries.len(), HISTORY_RETENTION_LIMIT);
        assert_eq!(entries.first().unwrap().id, "entry-104");
        assert_eq!(entries.last().unwrap().id, "entry-005");
    }

    #[test]
    fn storage_rejects_unsupported_version() {
        let temp = TempDir::new();
        let db = temp.path().join("history.sqlite3");
        {
            let connection = Connection::open(&db).unwrap();
            connection
                .execute_batch(
                    "
                    CREATE TABLE history_meta (key TEXT PRIMARY KEY, value INTEGER NOT NULL);
                    INSERT INTO history_meta (key, value) VALUES ('version', 999);
                    ",
                )
                .unwrap();
        }

        let error = MutationHistoryStore::open(&db).unwrap_err();

        assert!(error.contains("Versão de histórico não suportada"));
    }

    #[test]
    fn storage_reports_corrupt_file_payload_without_panicking() {
        let temp = TempDir::new();
        let db = temp.path().join("history.sqlite3");
        {
            let mut store = MutationHistoryStore::open(&db).unwrap();
            store.save_entry(&sample_entry("workspace")).unwrap();
        }
        let connection = Connection::open(&db).unwrap();
        connection
            .execute(
                "UPDATE mutation_files SET property_changes_json = 'not-json'",
                [],
            )
            .unwrap();

        let store = MutationHistoryStore::open(&db).unwrap();
        let error = store.load_workspace("workspace").unwrap_err();

        assert!(error.contains("Histórico possui diff inválido"));
    }

    #[test]
    fn history_state_derives_undoable_undone_and_workspace_isolation() {
        let bulk = sample_entry("workspace");
        let mut sql = sample_entry("workspace");
        sql.id = String::from("sql-entry");
        sql.source = MutationSource::SqlUpdate;
        let mut undo_bulk = sample_entry("workspace");
        undo_bulk.id = String::from("undo-bulk");
        undo_bulk.source = MutationSource::Undo;
        undo_bulk.original_entry_id = Some(bulk.id.clone());
        let mut undo_other_workspace = sample_entry("other");
        undo_other_workspace.id = String::from("undo-other");
        undo_other_workspace.source = MutationSource::Undo;
        undo_other_workspace.original_entry_id = Some(sql.id.clone());

        let state = HistoryState {
            entries: vec![bulk.clone(), sql.clone(), undo_bulk, undo_other_workspace],
            ..HistoryState::default()
        };

        assert!(!state.is_entry_undoable(&bulk));
        assert_eq!(state.entry_status_label(&bulk), "desfeita");
        assert!(state.is_entry_undoable(&sql));
        assert_eq!(state.entry_status_label(&sql), "undo disponível");
        assert_eq!(
            state.entry_status_label(
                state
                    .entries
                    .iter()
                    .find(|entry| entry.source == MutationSource::Undo)
                    .unwrap()
            ),
            "undo indisponível"
        );
    }

    #[test]
    fn history_state_marks_sql_entry_undone_after_undo() {
        let mut sql = sample_entry("workspace");
        sql.id = String::from("sql-entry");
        sql.source = MutationSource::SqlUpdate;
        let mut undo = sample_entry("workspace");
        undo.source = MutationSource::Undo;
        undo.original_entry_id = Some(sql.id.clone());
        let state = HistoryState {
            entries: vec![undo, sql.clone()],
            ..HistoryState::default()
        };

        assert!(!state.is_entry_undoable(&sql));
        assert_eq!(state.entry_status_label(&sql), "desfeita");
    }

    #[test]
    fn undone_status_survives_restart_from_persisted_entries() {
        let temp = TempDir::new();
        let db = temp.path().join("history.sqlite3");
        let original = sample_entry("workspace");
        let undo = undo_history_entry(
            String::from("workspace"),
            &original,
            &[PathBuf::from("a.md")],
        )
        .unwrap();
        {
            let mut store = MutationHistoryStore::open(&db).unwrap();
            store.save_entry(&original).unwrap();
            store.save_entry(&undo).unwrap();
        }

        let entries = MutationHistoryStore::open(&db)
            .unwrap()
            .load_workspace("workspace")
            .unwrap();
        let state = HistoryState {
            entries,
            ..HistoryState::default()
        };

        assert_eq!(state.entry_status_label(&original), "desfeita");
        assert!(!state.is_entry_undoable(&original));
    }

    #[test]
    fn build_undo_plan_restores_exact_before_content() {
        let temp = TempDir::new();
        temp.write("projects/a.md", "---\nstatus: archived\n---\n# A\n");
        let entry = MutationHistoryEntry {
            id: String::from("entry"),
            workspace_id: workspace_identity(temp.path()),
            created_at_unix: 1,
            source: MutationSource::BulkEdit,
            summary: String::from("summary"),
            sql: None,
            original_entry_id: None,
            files: vec![HistoryFileChange {
                relative_path: PathBuf::from("projects/a.md"),
                before_fingerprint: content_fingerprint("---\nstatus: active\n---\n# A\n"),
                after_fingerprint: content_fingerprint("---\nstatus: archived\n---\n# A\n"),
                before_content: String::from("---\nstatus: active\n---\n# A\n"),
                after_content: String::from("---\nstatus: archived\n---\n# A\n"),
                property_changes: vec![FrontmatterPropertyChange {
                    property: String::from("status"),
                    before: Some(String::from("status: active")),
                    after: Some(String::from("status: archived")),
                }],
            }],
        };

        let plan = build_undo_plan(temp.path(), &entry, &EditorState::default()).unwrap();

        assert!(plan.can_apply());
        assert_eq!(
            plan.changes[0].new_content.as_deref(),
            Some("---\nstatus: active\n---\n# A\n")
        );
        assert_eq!(
            plan.changes[0].property_changes[0].before.as_deref(),
            Some("status: archived")
        );
    }

    #[test]
    fn build_undo_plan_rejects_wrong_workspace_and_undo_entries() {
        let temp = TempDir::new();
        temp.write("a.md", "after");
        let mut entry = sample_entry("other");
        entry.files[0].after_content = String::from("after");
        entry.files[0].after_fingerprint = content_fingerprint("after");

        assert!(matches!(
            build_undo_plan(temp.path(), &entry, &EditorState::default()),
            Err(UndoBuildError::WrongWorkspace)
        ));

        entry.workspace_id = workspace_identity(temp.path());
        entry.source = MutationSource::Undo;
        assert!(matches!(
            build_undo_plan(temp.path(), &entry, &EditorState::default()),
            Err(UndoBuildError::UnsupportedSource)
        ));
    }

    #[test]
    fn build_undo_plan_blocks_dirty_conflict_and_missing_files() {
        let temp = TempDir::new();
        temp.write("a.md", "after");
        let mut entry = sample_entry(&workspace_identity(temp.path()));
        entry.files[0].after_content = String::from("after");
        entry.files[0].after_fingerprint = content_fingerprint("after");

        let dirty = editor_with_tab(temp.path().join("a.md"), true, None);
        assert!(matches!(
            build_undo_plan(temp.path(), &entry, &dirty),
            Err(UndoBuildError::DirtyFile { .. })
        ));

        let conflict = editor_with_tab(
            temp.path().join("a.md"),
            false,
            Some(EditorExternalConflict::Modified(String::from("disk"))),
        );
        assert!(matches!(
            build_undo_plan(temp.path(), &entry, &conflict),
            Err(UndoBuildError::ExternalConflict { .. })
        ));

        fs::remove_file(temp.path().join("a.md")).unwrap();
        assert!(matches!(
            build_undo_plan(temp.path(), &entry, &EditorState::default()),
            Err(UndoBuildError::MissingFile { .. })
        ));
    }

    #[test]
    fn build_undo_plan_blocks_current_fingerprint_mismatch_for_entire_batch() {
        let temp = TempDir::new();
        temp.write("a.md", "after\n");
        temp.write("b.md", "manual\n");
        let mut entry = sample_entry(&workspace_identity(temp.path()));
        entry.files = vec![
            HistoryFileChange {
                relative_path: PathBuf::from("a.md"),
                before_fingerprint: content_fingerprint("before\n"),
                after_fingerprint: content_fingerprint("after\n"),
                before_content: String::from("before\n"),
                after_content: String::from("after\n"),
                property_changes: Vec::new(),
            },
            HistoryFileChange {
                relative_path: PathBuf::from("b.md"),
                before_fingerprint: content_fingerprint("before\n"),
                after_fingerprint: content_fingerprint("after\n"),
                before_content: String::from("before\n"),
                after_content: String::from("after\n"),
                property_changes: Vec::new(),
            },
        ];

        let error = build_undo_plan(temp.path(), &entry, &EditorState::default()).unwrap_err();

        assert!(matches!(error, UndoBuildError::StaleFile { .. }));
    }

    #[test]
    fn undo_plan_restores_single_and_multi_file_content_via_safe_writer() {
        let temp = TempDir::new();
        temp.write("a.md", "---\nstatus: archived\n---\n# A\n");
        temp.write("b.md", "---\npriority: 20\n---\n# B\n");
        let entry = MutationHistoryEntry {
            id: String::from("entry"),
            workspace_id: workspace_identity(temp.path()),
            created_at_unix: 1,
            source: MutationSource::BulkEdit,
            summary: String::from("summary"),
            sql: None,
            original_entry_id: None,
            files: vec![
                HistoryFileChange {
                    relative_path: PathBuf::from("a.md"),
                    before_fingerprint: content_fingerprint("---\nstatus: active\n---\n# A\n"),
                    after_fingerprint: content_fingerprint("---\nstatus: archived\n---\n# A\n"),
                    before_content: String::from("---\nstatus: active\n---\n# A\n"),
                    after_content: String::from("---\nstatus: archived\n---\n# A\n"),
                    property_changes: vec![FrontmatterPropertyChange {
                        property: String::from("status"),
                        before: Some(String::from("status: active")),
                        after: Some(String::from("status: archived")),
                    }],
                },
                HistoryFileChange {
                    relative_path: PathBuf::from("b.md"),
                    before_fingerprint: content_fingerprint("---\npriority: 10\n---\n# B\n"),
                    after_fingerprint: content_fingerprint("---\npriority: 20\n---\n# B\n"),
                    before_content: String::from("---\npriority: 10\n---\n# B\n"),
                    after_content: String::from("---\npriority: 20\n---\n# B\n"),
                    property_changes: vec![FrontmatterPropertyChange {
                        property: String::from("priority"),
                        before: Some(String::from("priority: 10")),
                        after: Some(String::from("priority: 20")),
                    }],
                },
            ],
        };

        let plan = build_undo_plan(temp.path(), &entry, &EditorState::default()).unwrap();
        let result = apply_bulk_edit_plan(&plan).unwrap();

        assert_eq!(result.changed_paths.len(), 2);
        assert_eq!(
            fs::read_to_string(temp.path().join("a.md")).unwrap(),
            "---\nstatus: active\n---\n# A\n"
        );
        assert_eq!(
            fs::read_to_string(temp.path().join("b.md")).unwrap(),
            "---\npriority: 10\n---\n# B\n"
        );
    }

    #[test]
    fn successful_undo_creates_auditable_history_entry() {
        let original = sample_entry("workspace");
        let entry = undo_history_entry(
            String::from("workspace"),
            &original,
            &[PathBuf::from("a.md")],
        )
        .unwrap();

        assert_eq!(entry.source, MutationSource::Undo);
        assert_eq!(
            entry.original_entry_id.as_deref(),
            Some(original.id.as_str())
        );
        assert_eq!(entry.files.len(), 1);
        assert_eq!(
            entry.files[0].before_content,
            original.files[0].after_content
        );
        assert_eq!(
            entry.files[0].after_content,
            original.files[0].before_content
        );
    }

    #[test]
    fn successful_bulk_plan_can_be_recorded_after_commit() {
        let temp = TempDir::new();
        temp.write(
            "projects/a.md",
            "---\ntype: project\nstatus: active\n---\n# A\n",
        );
        let scan = scan_workspace(temp.path()).unwrap();
        let selection = BulkEditSelection::new(
            String::from("project"),
            vec![temp.path().join("projects/a.md")],
        );
        let plan = crate::build_bulk_edit_plan(
            selection,
            BulkEditOperation::SetProperty {
                property: String::from("status"),
                value: BulkEditValue::String(String::from("archived")),
            },
            &scan.documents,
            &EditorState::default(),
            &crate::SchemaCatalog::build(
                &scan.documents,
                &scan.collections,
                &crate::RelationIndex::build(&scan.documents),
                crate::ExplicitSchemaState::Absent,
            ),
        )
        .unwrap();
        let result = crate::apply_bulk_edit_plan(&plan).unwrap();

        let entry =
            bulk_history_entry(String::from("workspace"), &plan, &result.changed_paths).unwrap();

        assert_eq!(entry.source, MutationSource::BulkEdit);
        assert_eq!(entry.files.len(), 1);
        assert!(entry.files[0].before_content.contains("status: active"));
        assert!(entry.files[0].after_content.contains("status: archived"));
    }

    #[test]
    fn successful_sql_plan_records_statement_metadata() {
        let temp = TempDir::new();
        temp.write(
            "projects/a.md",
            "---\ntype: project\npriority: 10\n---\n# A\n",
        );
        let scan = scan_workspace(temp.path()).unwrap();
        let plan = crate::SqlProjection::preview_update(
            "UPDATE projects SET priority = 20 WHERE priority = 10;",
            &scan.documents,
            &scan.collections,
            &EditorState::default(),
            &crate::SchemaCatalog::build(
                &scan.documents,
                &scan.collections,
                &crate::RelationIndex::build(&scan.documents),
                crate::ExplicitSchemaState::Absent,
            ),
        )
        .unwrap();
        let result = apply_bulk_edit_plan(&plan.mutation_plan).unwrap();

        let entry =
            sql_history_entry(String::from("workspace"), &plan, &result.changed_paths).unwrap();

        assert_eq!(entry.source, MutationSource::SqlUpdate);
        assert_eq!(
            entry.sql.as_deref(),
            Some("UPDATE projects SET priority = 20 WHERE priority = 10;")
        );
        assert!(entry.summary.contains("SQL Update em Projects"));
        assert_eq!(entry.files.len(), 1);
    }

    #[test]
    fn restart_style_load_then_undo_succeeds() {
        let temp = TempDir::new();
        temp.write(
            "projects/a.md",
            "---\ntype: project\nstatus: active\n---\n# A\n",
        );
        let scan = scan_workspace(temp.path()).unwrap();
        let plan = crate::build_bulk_edit_plan(
            BulkEditSelection::new(
                String::from("project"),
                vec![temp.path().join("projects/a.md")],
            ),
            BulkEditOperation::SetProperty {
                property: String::from("status"),
                value: BulkEditValue::String(String::from("archived")),
            },
            &scan.documents,
            &EditorState::default(),
            &crate::SchemaCatalog::build(
                &scan.documents,
                &scan.collections,
                &crate::RelationIndex::build(&scan.documents),
                crate::ExplicitSchemaState::Absent,
            ),
        )
        .unwrap();
        let result = apply_bulk_edit_plan(&plan).unwrap();
        let history_entry = bulk_history_entry(
            workspace_identity(temp.path()),
            &plan,
            &result.changed_paths,
        )
        .unwrap();
        let db = temp.path().join("app-data").join("history.sqlite3");
        MutationHistoryStore::open(&db)
            .unwrap()
            .save_entry(&history_entry)
            .unwrap();

        let entries = MutationHistoryStore::open(&db)
            .unwrap()
            .load_workspace(&workspace_identity(temp.path()))
            .unwrap();
        let undo_plan = build_undo_plan(temp.path(), &entries[0], &EditorState::default()).unwrap();
        apply_bulk_edit_plan(&undo_plan).unwrap();

        assert_eq!(
            fs::read_to_string(temp.path().join("projects/a.md")).unwrap(),
            "---\ntype: project\nstatus: active\n---\n# A\n"
        );
    }

    fn editor_with_tab(
        path: PathBuf,
        dirty: bool,
        external_conflict: Option<EditorExternalConflict>,
    ) -> EditorState {
        EditorState {
            tabs: vec![EditorTab {
                document_path: path,
                relative_path: PathBuf::from("a.md"),
                title: String::from("a"),
                kind: EditorTabKind::Markdown,
                buffer: if dirty {
                    String::from("dirty")
                } else {
                    String::from("after")
                },
                saved_content: String::from("after"),
                dirty,
                view_mode: EditorViewMode::Edit,
                split_ratio: 500,
                external_conflict,
                ignored_external_conflict: None,
                save_error: None,
            }],
            active_path: None,
            dialog: None,
        }
    }

    fn sample_entry(workspace_id: &str) -> MutationHistoryEntry {
        MutationHistoryEntry {
            id: new_history_id(),
            workspace_id: workspace_id.to_owned(),
            created_at_unix: now_unix_seconds(),
            source: MutationSource::BulkEdit,
            summary: String::from("Definiu status = archived em 1 documento"),
            sql: None,
            original_entry_id: None,
            files: vec![HistoryFileChange {
                relative_path: PathBuf::from("a.md"),
                before_fingerprint: content_fingerprint("before"),
                after_fingerprint: content_fingerprint("after"),
                before_content: String::from("before"),
                after_content: String::from("after"),
                property_changes: Vec::new(),
            }],
        }
    }

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = env::temp_dir().join(format!("flokin-history-test-{nonce}"));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn write(&self, relative: &str, content: &str) {
            let path = self.path.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
