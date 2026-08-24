use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf, MAIN_SEPARATOR},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    relation_display_property, search_documents, Collection, Document, PropertyValue, Relation,
    RelationIndex, RelationStatus, ScanError, ScanResult, SearchQuery, SearchState, SortDirection,
    SqlCatalog, SqlError, SqlQueryResult, TableSort, WorkspaceUpdate,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    Explorer,
    Relations,
    Links,
    Tags,
    Calendar,
    Favorites,
    History,
    Terminal,
    Settings,
}

impl Activity {
    pub const ALL: [Self; 9] = [
        Self::Explorer,
        Self::Relations,
        Self::Links,
        Self::Tags,
        Self::Calendar,
        Self::Favorites,
        Self::History,
        Self::Terminal,
        Self::Settings,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Explorer => "Explorer",
            Self::Relations => "Relations",
            Self::Links => "Links",
            Self::Tags => "Tags",
            Self::Calendar => "Calendar",
            Self::Favorites => "Favorites",
            Self::History => "History",
            Self::Terminal => "Terminal",
            Self::Settings => "Settings",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExplorerNodeId(pub usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplorerNode {
    pub id: ExplorerNodeId,
    pub name: String,
    pub kind: ExplorerNodeKind,
    pub path: PathBuf,
    pub children: Vec<ExplorerNode>,
    pub expanded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerNodeKind {
    Folder,
    File,
}

impl ExplorerNode {
    pub fn folder(id: usize, name: impl Into<String>, path: PathBuf, children: Vec<Self>) -> Self {
        Self {
            id: ExplorerNodeId(id),
            name: name.into(),
            kind: ExplorerNodeKind::Folder,
            path,
            children,
            expanded: true,
        }
    }

    pub fn collapsed_folder(
        id: usize,
        name: impl Into<String>,
        path: PathBuf,
        children: Vec<Self>,
    ) -> Self {
        Self {
            expanded: false,
            ..Self::folder(id, name, path, children)
        }
    }

    pub fn file(id: usize, name: impl Into<String>, path: PathBuf) -> Self {
        Self {
            id: ExplorerNodeId(id),
            name: name.into(),
            kind: ExplorerNodeKind::File,
            path,
            children: Vec::new(),
            expanded: false,
        }
    }

    pub const fn is_folder(&self) -> bool {
        matches!(self.kind, ExplorerNodeKind::Folder)
    }

    pub fn toggle(&mut self, id: ExplorerNodeId) -> bool {
        if self.id == id && self.is_folder() {
            self.expanded = !self.expanded;
            return true;
        }

        self.children.iter_mut().any(|child| child.toggle(id))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterCount {
    pub label: &'static str,
    pub count: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SqlExplorerState {
    pub open: bool,
    pub query: String,
    pub catalog: Option<SqlCatalog>,
    pub result: Option<SqlQueryResult>,
    pub error: Option<String>,
    pub running: bool,
}

impl SqlExplorerState {
    pub fn closed() -> Self {
        Self {
            open: false,
            query: String::new(),
            catalog: None,
            result: None,
            error: None,
            running: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectorField {
    pub label: String,
    pub value: InspectorValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectorValue {
    Text(String),
    Number(String),
    Bool(bool),
    Empty,
    Array(Vec<String>),
    Object,
}

impl InspectorValue {
    pub fn display_value(&self) -> String {
        match self {
            Self::Text(value) | Self::Number(value) => value.clone(),
            Self::Bool(true) => String::from("✓"),
            Self::Bool(false) => String::from("✕"),
            Self::Empty => String::from("—"),
            Self::Array(values) if values.is_empty() => String::from("—"),
            Self::Array(values) => values.join(", "),
            Self::Object => String::from("{...}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentInspector {
    pub properties: Vec<InspectorField>,
    pub outgoing_relations: Vec<InspectorRelation>,
    pub incoming_relations: Vec<InspectorRelation>,
    pub metadata: Vec<InspectorField>,
    pub tags: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectorRelation {
    pub property: String,
    pub label: String,
    pub target_path: Option<PathBuf>,
    pub status: InspectorRelationStatus,
    pub candidates: Vec<RelationDocumentSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectorRelationStatus {
    Resolved,
    Unresolved,
    Ambiguous(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationDocumentSummary {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectorModel {
    Empty { title: String, description: String },
    Document(DocumentInspector),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSourceView {
    pub title: String,
    pub relative_path: PathBuf,
    pub content: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorTab {
    pub document_path: PathBuf,
    pub relative_path: PathBuf,
    pub title: String,
    pub buffer: String,
    pub saved_content: String,
    pub dirty: bool,
    pub external_conflict: Option<EditorExternalConflict>,
    pub ignored_external_conflict: Option<EditorExternalConflict>,
    pub save_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorExternalConflict {
    Modified(String),
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorDialog {
    CloseDirty { document_path: PathBuf },
    CloseWorkspaceDirty { dirty_count: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditorState {
    pub tabs: Vec<EditorTab>,
    pub active_path: Option<PathBuf>,
    pub dialog: Option<EditorDialog>,
}

impl EditorState {
    pub fn active_tab(&self) -> Option<&EditorTab> {
        let path = self.active_path.as_ref()?;
        self.tabs.iter().find(|tab| &tab.document_path == path)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut EditorTab> {
        let path = self.active_path.clone()?;
        self.tabs.iter_mut().find(|tab| tab.document_path == path)
    }

    pub fn tab(&self, path: &Path) -> Option<&EditorTab> {
        self.tabs.iter().find(|tab| tab.document_path == path)
    }

    pub fn has_dirty_tabs(&self) -> bool {
        self.tabs.iter().any(|tab| tab.dirty)
    }

    pub fn dirty_count(&self) -> usize {
        self.tabs.iter().filter(|tab| tab.dirty).count()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShellModel {
    pub active_activity: Activity,
    pub current_workspace: Option<PathBuf>,
    pub explorer: Vec<ExplorerNode>,
    pub documents: Vec<Document>,
    pub collections: Vec<Collection>,
    pub scan_state: ScanState,
    pub selected_document_path: Option<PathBuf>,
    pub selected_collection: Option<String>,
    pub collection_table_sort: Option<TableSort>,
    pub search: SearchState,
    pub relation_index: RelationIndex,
    pub editor: EditorState,
    pub sql_explorer: SqlExplorerState,
    pub collapsed_sql_tables: BTreeSet<String>,
    pub filters: Vec<FilterCount>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanState {
    Idle,
    Scanning,
    Updating {
        documents: usize,
        directories: usize,
        collections: usize,
        errors: usize,
        warnings: usize,
    },
    Completed {
        documents: usize,
        directories: usize,
        collections: usize,
        errors: usize,
        warnings: usize,
    },
    Failed(String),
}

impl ShellModel {
    pub fn workspace_selected(&mut self, path: Option<PathBuf>) {
        if let Some(path) = path {
            self.current_workspace = Some(path);
            self.explorer.clear();
            self.documents.clear();
            self.collections.clear();
            self.selected_document_path = None;
            self.selected_collection = None;
            self.collection_table_sort = None;
            self.search = SearchState::closed();
            self.relation_index = RelationIndex::default();
            self.editor = EditorState::default();
            self.sql_explorer.open = false;
            self.sql_explorer.catalog = None;
            self.sql_explorer.result = None;
            self.sql_explorer.error = None;
            self.sql_explorer.running = false;
            self.collapsed_sql_tables.clear();
            self.scan_state = ScanState::Scanning;
        }
    }

    pub fn workspace_display(&self) -> WorkspaceDisplay {
        self.current_workspace
            .as_deref()
            .map(workspace_display)
            .unwrap_or_else(WorkspaceDisplay::none)
    }

    pub fn select_activity(&mut self, activity: Activity) {
        self.active_activity = activity;
    }

    pub fn toggle_explorer_node(&mut self, id: ExplorerNodeId) -> bool {
        self.explorer.iter_mut().any(|node| node.toggle(id))
    }

    pub fn select_explorer_node(&mut self, id: ExplorerNodeId) -> bool {
        let selected = self
            .explorer
            .iter()
            .find_map(|node| node.file_path(id))
            .cloned();

        if let Some(path) = selected {
            self.select_markdown_path(path);
            self.search.close();
            true
        } else {
            false
        }
    }

    pub fn select_markdown_path(&mut self, path: PathBuf) -> bool {
        if self.open_editor_tab(path.clone()) {
            self.selected_document_path = Some(path);
            self.selected_collection = None;
            self.collection_table_sort = None;
            true
        } else {
            false
        }
    }

    pub fn select_search_result_path(&mut self, path: PathBuf) -> bool {
        if let Some(document) = self.documents.iter().find(|document| document.path == path) {
            let path = document.path.clone();
            self.open_editor_tab(path.clone());
            self.selected_document_path = Some(path);
            self.selected_collection = None;
            self.collection_table_sort = None;
            self.search.close();
            true
        } else {
            self.refresh_search_results();
            false
        }
    }

    pub fn select_document_without_opening(&mut self, path: PathBuf) -> bool {
        if self.documents.iter().any(|document| document.path == path) {
            self.selected_document_path = Some(path);
            self.selected_collection = None;
            self.collection_table_sort = None;
            self.sql_explorer.open = false;
            true
        } else {
            false
        }
    }

    pub fn select_collection(&mut self, collection_id: String) {
        if self
            .collections
            .iter()
            .any(|collection| collection.id == collection_id)
        {
            self.selected_collection = Some(collection_id);
            self.selected_document_path = None;
            self.editor.active_path = None;
            self.collection_table_sort = None;
        }
    }

    pub fn open_sql_explorer(&mut self) {
        self.sql_explorer.open = true;
        self.selected_document_path = None;
        self.editor.active_path = None;
        self.selected_collection = None;
        self.collection_table_sort = None;
        self.search.close();
    }

    pub fn update_sql_query(&mut self, query: String) {
        self.sql_explorer.query = query;
    }

    pub fn toggle_sql_schema_table(&mut self, table_name: String) {
        if !self.collapsed_sql_tables.remove(&table_name) {
            self.collapsed_sql_tables.insert(table_name);
        }
    }

    pub fn sql_execution_started(&mut self) {
        self.sql_explorer.running = true;
        self.sql_explorer.error = None;
    }

    pub fn sql_execution_completed(&mut self, result: Result<SqlQueryResult, SqlError>) {
        self.sql_explorer.running = false;
        match result {
            Ok(result) => {
                self.sql_explorer.result = Some(result);
                self.sql_explorer.error = None;
            }
            Err(error) => {
                self.sql_explorer.error = Some(error.message);
                self.sql_explorer.result = None;
            }
        }
    }

    pub fn sql_projection_completed(&mut self, catalog: Result<SqlCatalog, SqlError>) {
        self.sql_explorer.result = None;
        self.sql_explorer.running = false;
        match catalog {
            Ok(catalog) => {
                self.sql_explorer.catalog = Some(catalog);
                self.sql_explorer.error = None;
            }
            Err(error) => {
                self.sql_explorer.catalog = None;
                self.sql_explorer.error = Some(error.message);
            }
        }
    }

    pub fn toggle_collection_sort(&mut self, column_id: String) {
        self.collection_table_sort = Some(match self.collection_table_sort.take() {
            Some(sort) if sort.column_id == column_id => TableSort {
                column_id,
                direction: match sort.direction {
                    SortDirection::Ascending => SortDirection::Descending,
                    SortDirection::Descending => SortDirection::Ascending,
                },
            },
            _ => TableSort {
                column_id,
                direction: SortDirection::Ascending,
            },
        });
    }

    pub fn scan_completed(&mut self, result: ScanResult) {
        let expanded_paths =
            (!self.explorer.is_empty()).then(|| expanded_folder_paths(&self.explorer));
        self.explorer = explorer_from_scan_result(&result);
        if let Some(expanded_paths) = expanded_paths.as_ref() {
            restore_expanded_folder_paths(&mut self.explorer, expanded_paths);
        }
        self.documents = result.documents;
        self.collections = result.collections;
        self.relation_index = RelationIndex::build(&self.documents);
        self.sync_editor_tabs_with_documents();
        if let Some(selected_document_path) = self.selected_document_path.as_ref() {
            if !self
                .documents
                .iter()
                .any(|document| &document.path == selected_document_path)
            {
                self.selected_document_path = None;
            }
        }
        if let Some(selected_collection) = self.selected_collection.as_ref() {
            if !self
                .collections
                .iter()
                .any(|collection| &collection.id == selected_collection)
            {
                self.selected_collection = None;
                self.collection_table_sort = None;
            }
        }
        let warnings = self
            .documents
            .iter()
            .map(|document| document.warnings.len())
            .sum();
        self.scan_state = ScanState::Completed {
            documents: self.documents.len(),
            directories: result.directories.len(),
            collections: self.collections.len(),
            errors: result.errors.len(),
            warnings,
        };
        self.refresh_search_results();
    }

    pub fn workspace_update_started(&mut self) {
        if let ScanState::Completed {
            documents,
            directories,
            collections,
            errors,
            warnings,
        }
        | ScanState::Updating {
            documents,
            directories,
            collections,
            errors,
            warnings,
        } = self.scan_state
        {
            self.scan_state = ScanState::Updating {
                documents,
                directories,
                collections,
                errors,
                warnings,
            };
        }
    }

    pub fn workspace_update_completed(&mut self, update: WorkspaceUpdate) {
        if update.needs_rescan {
            self.scan_state = ScanState::Scanning;
            return;
        }

        for path in update.removals {
            self.documents.retain(|document| document.path != path);
            self.sync_editor_tab_removed(&path);
            if self.selected_document_path.as_ref() == Some(&path)
                && self.editor.tab(&path).is_none()
            {
                self.selected_document_path = None;
            }
        }

        for document in update.upserts {
            if self.is_stale_document_update(&document) {
                continue;
            }
            let path = document.path.clone();
            if let Some(existing) = self
                .documents
                .iter_mut()
                .find(|existing| existing.path == document.path)
            {
                *existing = document;
            } else {
                self.documents.push(document);
            }
            self.sync_editor_tab_upsert(&path);
        }

        self.documents
            .sort_by(|left, right| compare_paths(&left.relative_path, &right.relative_path));
        self.collections = collections_from_documents(&self.documents);
        self.relation_index = RelationIndex::build(&self.documents);

        if let Some(selected_collection) = self.selected_collection.as_ref() {
            if !self
                .collections
                .iter()
                .any(|collection| &collection.id == selected_collection)
            {
                self.selected_collection = None;
                self.collection_table_sort = None;
            }
        }

        if let Some(selected_document_path) = self.selected_document_path.as_ref() {
            if !self
                .documents
                .iter()
                .any(|document| &document.path == selected_document_path)
            {
                self.selected_document_path = None;
            }
        }

        let expanded_paths = expanded_folder_paths(&self.explorer);
        let result = ScanResult {
            root: update.root,
            directories: directories_from_documents(&self.documents),
            documents: self.documents.clone(),
            collections: self.collections.clone(),
            errors: update.errors,
            duration: update.duration,
        };
        self.explorer = explorer_from_scan_result(&result);
        restore_expanded_folder_paths(&mut self.explorer, &expanded_paths);
        self.set_completed_state(result.errors.len());
        self.refresh_search_results();
    }

    pub fn workspace_update_failed(&mut self, error: ScanError) {
        let errors = match self.scan_state {
            ScanState::Completed { errors, .. } | ScanState::Updating { errors, .. } => errors + 1,
            _ => 1,
        };
        if let Some(document) = self.selected_document_path.as_ref().and_then(|path| {
            self.documents
                .iter_mut()
                .find(|document| &document.path == path)
        }) {
            document.warnings.push(crate::DocumentWarning {
                path: error.path,
                message: error.message,
            });
        }
        self.set_completed_state(errors);
    }

    pub fn scan_failed(&mut self, message: String) {
        self.explorer.clear();
        self.documents.clear();
        self.collections.clear();
        self.selected_document_path = None;
        self.selected_collection = None;
        self.collection_table_sort = None;
        self.search = SearchState::closed();
        self.relation_index = RelationIndex::default();
        self.editor = EditorState::default();
        self.scan_state = ScanState::Failed(message);
    }

    pub fn open_search(&mut self) {
        self.search.open();
        self.refresh_search_results();
    }

    pub fn close_search(&mut self) {
        self.search.close();
    }

    pub fn update_search_query(&mut self, query: String) {
        self.search.set_query(query);
    }

    pub fn refresh_search_results(&mut self) {
        let outcome =
            search_documents(SearchQuery::new(self.search.query.clone()), &self.documents);
        self.search.apply_outcome(outcome);
    }

    pub fn select_next_search_result(&mut self) {
        self.search.select_next();
    }

    pub fn select_previous_search_result(&mut self) {
        self.search.select_previous();
    }

    pub fn activate_selected_search_result(&mut self) -> bool {
        let Some(path) = self
            .search
            .selected_result()
            .map(|result| result.document_path.clone())
        else {
            return false;
        };

        self.select_search_result_path(path)
    }

    pub fn open_editor_tab(&mut self, path: PathBuf) -> bool {
        if self.editor.tabs.iter().any(|tab| tab.document_path == path) {
            self.editor.active_path = Some(path);
            return true;
        }

        let Some(document) = self.documents.iter().find(|document| document.path == path) else {
            return false;
        };
        let content = document.source_content.clone().unwrap_or_default();
        self.editor.tabs.push(EditorTab {
            document_path: document.path.clone(),
            relative_path: document.relative_path.clone(),
            title: document.file_name.to_string_lossy().into_owned(),
            buffer: content.clone(),
            saved_content: content,
            dirty: false,
            external_conflict: None,
            ignored_external_conflict: None,
            save_error: document
                .source_content
                .is_none()
                .then(|| String::from("Não foi possível exibir o conteúdo deste arquivo.")),
        });
        self.editor.active_path = Some(path);
        true
    }

    pub fn activate_editor_tab(&mut self, path: PathBuf) -> bool {
        if self.editor.tabs.iter().any(|tab| tab.document_path == path) {
            self.editor.active_path = Some(path.clone());
            self.selected_document_path = Some(path);
            self.selected_collection = None;
            self.collection_table_sort = None;
            true
        } else {
            false
        }
    }

    pub fn active_editor_tab(&self) -> Option<&EditorTab> {
        self.editor.active_tab()
    }

    pub fn active_editor_buffer(&self) -> Option<&str> {
        self.editor.active_tab().map(|tab| tab.buffer.as_str())
    }

    pub fn update_active_editor_buffer(&mut self, buffer: String) -> bool {
        let Some(tab) = self.editor.active_tab_mut() else {
            return false;
        };
        tab.buffer = buffer;
        tab.dirty = tab.buffer != tab.saved_content;
        tab.save_error = None;
        true
    }

    pub fn request_close_editor_tab(&mut self, path: PathBuf) -> bool {
        let Some(tab) = self.editor.tab(&path) else {
            return false;
        };
        if tab.dirty {
            self.editor.dialog = Some(EditorDialog::CloseDirty {
                document_path: path,
            });
        } else {
            self.close_editor_tab(&path);
        }
        true
    }

    pub fn request_close_active_editor_tab(&mut self) -> bool {
        let Some(path) = self.editor.active_path.clone() else {
            return false;
        };
        self.request_close_editor_tab(path)
    }

    pub fn request_close_workspace(&mut self) -> bool {
        let dirty_count = self.editor.dirty_count();
        if dirty_count == 0 {
            false
        } else if dirty_count == 1 {
            let Some(tab) = self.editor.tabs.iter().find(|tab| tab.dirty) else {
                return false;
            };
            self.editor.dialog = Some(EditorDialog::CloseDirty {
                document_path: tab.document_path.clone(),
            });
            true
        } else {
            self.editor.dialog = Some(EditorDialog::CloseWorkspaceDirty { dirty_count });
            true
        }
    }

    pub fn cancel_editor_dialog(&mut self) {
        self.editor.dialog = None;
    }

    pub fn discard_dialog_changes(&mut self) -> Vec<PathBuf> {
        let Some(dialog) = self.editor.dialog.take() else {
            return Vec::new();
        };
        match dialog {
            EditorDialog::CloseDirty { document_path } => {
                self.close_editor_tab(&document_path);
                vec![document_path]
            }
            EditorDialog::CloseWorkspaceDirty { .. } => {
                let dirty_paths = self
                    .editor
                    .tabs
                    .iter()
                    .filter(|tab| tab.dirty)
                    .map(|tab| tab.document_path.clone())
                    .collect::<Vec<_>>();
                self.editor.tabs.retain(|tab| !tab.dirty);
                self.ensure_active_tab_exists();
                dirty_paths
            }
        }
    }

    pub fn pending_save_paths(&self) -> Vec<PathBuf> {
        match self.editor.dialog.as_ref() {
            Some(EditorDialog::CloseDirty { document_path }) => vec![document_path.clone()],
            Some(EditorDialog::CloseWorkspaceDirty { .. }) => self
                .editor
                .tabs
                .iter()
                .filter(|tab| tab.dirty)
                .map(|tab| tab.document_path.clone())
                .collect(),
            None => self
                .editor
                .active_tab()
                .filter(|tab| tab.dirty)
                .map(|tab| vec![tab.document_path.clone()])
                .unwrap_or_default(),
        }
    }

    pub fn editor_save_completed(
        &mut self,
        path: &Path,
        saved_content: &str,
        result: Result<(), String>,
    ) -> bool {
        let Some(tab) = self
            .editor
            .tabs
            .iter_mut()
            .find(|tab| tab.document_path == path)
        else {
            return false;
        };

        match result {
            Ok(()) => {
                tab.saved_content = saved_content.to_owned();
                tab.dirty = tab.buffer != tab.saved_content;
                tab.external_conflict = None;
                tab.ignored_external_conflict = None;
                tab.save_error = None;
                true
            }
            Err(error) => {
                tab.save_error = Some(error);
                false
            }
        }
    }

    pub fn close_saved_dialog_tab(&mut self, path: &Path) {
        if matches!(
            self.editor.dialog.as_ref(),
            Some(EditorDialog::CloseDirty { document_path }) if document_path == path
        ) {
            self.editor.dialog = None;
            self.close_editor_tab(path);
        }
    }

    pub fn close_saved_workspace_tabs(&mut self, saved_paths: &[PathBuf]) -> bool {
        if !matches!(
            self.editor.dialog,
            Some(EditorDialog::CloseWorkspaceDirty { .. })
        ) {
            return false;
        }
        for path in saved_paths {
            if let Some(tab) = self
                .editor
                .tabs
                .iter()
                .find(|tab| &tab.document_path == path)
            {
                if tab.dirty {
                    return false;
                }
            }
        }
        self.editor.dialog = None;
        self.editor
            .tabs
            .retain(|tab| !saved_paths.contains(&tab.document_path));
        self.ensure_active_tab_exists();
        true
    }

    pub fn reload_external_editor_change(&mut self) -> bool {
        let Some(tab) = self.editor.active_tab_mut() else {
            return false;
        };
        let Some(conflict) = tab.external_conflict.take() else {
            return false;
        };
        match conflict {
            EditorExternalConflict::Modified(content) => {
                tab.buffer = content.clone();
                tab.saved_content = content;
                tab.dirty = false;
            }
            EditorExternalConflict::Deleted => {
                tab.dirty = true;
            }
        }
        tab.ignored_external_conflict = None;
        tab.save_error = None;
        true
    }

    pub fn keep_local_editor_changes(&mut self) -> bool {
        let Some(tab) = self.editor.active_tab_mut() else {
            return false;
        };
        if let Some(conflict) = tab.external_conflict.take() {
            tab.ignored_external_conflict = Some(conflict);
            true
        } else {
            false
        }
    }

    pub fn selected_collection(&self) -> Option<&Collection> {
        let id = self.selected_collection.as_ref()?;
        self.collections
            .iter()
            .find(|collection| &collection.id == id)
    }

    pub fn collection_documents(&self, collection_id: &str) -> Vec<&Document> {
        self.documents
            .iter()
            .filter(|document| document.collection_id == collection_id)
            .collect()
    }

    pub fn selected_document(&self) -> Option<&Document> {
        let path = self.selected_document_path.as_ref()?;
        self.documents
            .iter()
            .find(|document| &document.path == path)
    }

    pub fn document_inspector(&self) -> InspectorModel {
        let Some(document) = self.selected_document() else {
            return InspectorModel::Empty {
                title: String::from("Nenhum documento selecionado."),
                description: String::from(
                    "Selecione um documento ou registro para ver suas propriedades.",
                ),
            };
        };

        InspectorModel::Document(DocumentInspector {
            properties: inspector_properties(document, &self.relation_index),
            outgoing_relations: self
                .relation_index
                .outgoing(&document.path)
                .into_iter()
                .map(inspector_outgoing_relation)
                .collect(),
            incoming_relations: self
                .relation_index
                .incoming(&document.path)
                .into_iter()
                .map(inspector_incoming_relation)
                .collect(),
            metadata: inspector_metadata(document, self.collection_display_name(document)),
            tags: inspector_tags(document),
            warnings: document
                .warnings
                .iter()
                .map(|warning| user_warning_message(warning.message.as_str()))
                .collect(),
        })
    }

    pub fn selected_document_source(&self) -> Option<DocumentSourceView> {
        let document = self.selected_document()?;
        Some(DocumentSourceView {
            title: document.title.clone(),
            relative_path: document.relative_path.clone(),
            content: document.source_content.clone(),
        })
    }

    fn collection_display_name(&self, document: &Document) -> String {
        self.collections
            .iter()
            .find(|collection| collection.id == document.collection_id)
            .map(|collection| collection.display_name.clone())
            .unwrap_or_else(|| document.collection_id.clone())
    }

    fn set_completed_state(&mut self, errors: usize) {
        let warnings = self
            .documents
            .iter()
            .map(|document| document.warnings.len())
            .sum();
        self.scan_state = ScanState::Completed {
            documents: self.documents.len(),
            directories: directories_from_documents(&self.documents).len(),
            collections: self.collections.len(),
            errors,
            warnings,
        };
    }

    fn close_editor_tab(&mut self, path: &Path) {
        let Some(index) = self
            .editor
            .tabs
            .iter()
            .position(|tab| tab.document_path == path)
        else {
            return;
        };
        self.editor.tabs.remove(index);
        if self.editor.active_path.as_deref() == Some(path) {
            self.editor.active_path = self
                .editor
                .tabs
                .get(index.saturating_sub(1))
                .or_else(|| self.editor.tabs.first())
                .map(|tab| tab.document_path.clone());
            self.selected_document_path = self.editor.active_path.clone();
        }
    }

    fn ensure_active_tab_exists(&mut self) {
        if let Some(path) = self.editor.active_path.as_ref() {
            if self
                .editor
                .tabs
                .iter()
                .any(|tab| &tab.document_path == path)
            {
                self.selected_document_path = Some(path.clone());
                return;
            }
        }
        self.editor.active_path = self
            .editor
            .tabs
            .first()
            .map(|tab| tab.document_path.clone());
        self.selected_document_path = self.editor.active_path.clone();
    }

    fn sync_editor_tabs_with_documents(&mut self) {
        let documents = self
            .documents
            .iter()
            .map(|document| {
                (
                    document.path.clone(),
                    (
                        document.relative_path.clone(),
                        document.file_name.to_string_lossy().into_owned(),
                        document.source_content.clone(),
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();

        self.editor
            .tabs
            .retain(|tab| documents.contains_key(&tab.document_path));

        for tab in &mut self.editor.tabs {
            let Some((relative_path, title, source_content)) = documents.get(&tab.document_path)
            else {
                continue;
            };
            tab.relative_path = relative_path.clone();
            tab.title = title.clone();
            let Some(source_content) = source_content.clone() else {
                tab.save_error = Some(String::from(
                    "Não foi possível exibir o conteúdo deste arquivo.",
                ));
                continue;
            };
            if tab.dirty {
                if source_content != tab.saved_content && tab.external_conflict.is_none() {
                    let conflict = EditorExternalConflict::Modified(source_content);
                    if tab.ignored_external_conflict.as_ref() != Some(&conflict) {
                        tab.external_conflict = Some(conflict);
                    }
                }
            } else if source_content != tab.saved_content {
                tab.buffer = source_content.clone();
                tab.saved_content = source_content;
                tab.external_conflict = None;
                tab.ignored_external_conflict = None;
                tab.save_error = None;
            }
        }

        self.ensure_active_tab_exists();
    }

    fn is_stale_document_update(&self, document: &Document) -> bool {
        let Some(existing) = self
            .documents
            .iter()
            .find(|existing| existing.path == document.path)
        else {
            return false;
        };
        match (existing.metadata.modified, document.metadata.modified) {
            (Some(existing_modified), Some(incoming_modified)) => {
                incoming_modified < existing_modified
            }
            _ => false,
        }
    }

    fn sync_editor_tab_upsert(&mut self, path: &Path) {
        let Some(document) = self.documents.iter().find(|document| document.path == path) else {
            return;
        };
        let Some(tab) = self
            .editor
            .tabs
            .iter_mut()
            .find(|tab| tab.document_path == path)
        else {
            return;
        };

        tab.relative_path = document.relative_path.clone();
        tab.title = document.file_name.to_string_lossy().into_owned();
        let Some(source_content) = document.source_content.clone() else {
            tab.save_error = Some(String::from(
                "Não foi possível exibir o conteúdo deste arquivo.",
            ));
            return;
        };

        if tab.dirty {
            if source_content == tab.saved_content {
                return;
            }
            let conflict = EditorExternalConflict::Modified(source_content);
            if tab.ignored_external_conflict.as_ref() != Some(&conflict) {
                tab.external_conflict = Some(conflict);
            }
        } else if source_content != tab.saved_content {
            tab.buffer = source_content.clone();
            tab.saved_content = source_content;
            tab.external_conflict = None;
            tab.ignored_external_conflict = None;
            tab.save_error = None;
        }
    }

    fn sync_editor_tab_removed(&mut self, path: &Path) {
        let Some(tab) = self
            .editor
            .tabs
            .iter_mut()
            .find(|tab| tab.document_path == path)
        else {
            return;
        };

        if tab.dirty {
            let conflict = EditorExternalConflict::Deleted;
            if tab.ignored_external_conflict.as_ref() != Some(&conflict) {
                tab.external_conflict = Some(conflict);
            }
        } else {
            self.close_editor_tab(path);
        }
    }
}

impl ExplorerNode {
    fn file_path(&self, id: ExplorerNodeId) -> Option<&PathBuf> {
        if self.id == id && matches!(self.kind, ExplorerNodeKind::File) {
            return Some(&self.path);
        }

        self.children.iter().find_map(|child| child.file_path(id))
    }
}

fn explorer_from_scan_result(result: &ScanResult) -> Vec<ExplorerNode> {
    let mut next_id = 1;
    let root_name = result
        .root
        .file_name()
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| result.root.display().to_string());

    let children = build_children(&result.root, Path::new(""), result, &mut next_id);
    if children.is_empty() {
        Vec::new()
    } else {
        vec![ExplorerNode::folder(
            take_id(&mut next_id),
            root_name,
            result.root.clone(),
            children,
        )]
    }
}

fn expanded_folder_paths(nodes: &[ExplorerNode]) -> BTreeSet<PathBuf> {
    let mut paths = BTreeSet::new();
    collect_expanded_folder_paths(nodes, &mut paths);
    paths
}

fn collect_expanded_folder_paths(nodes: &[ExplorerNode], paths: &mut BTreeSet<PathBuf>) {
    for node in nodes {
        if node.is_folder() && node.expanded {
            paths.insert(node.path.clone());
        }
        collect_expanded_folder_paths(&node.children, paths);
    }
}

fn restore_expanded_folder_paths(nodes: &mut [ExplorerNode], expanded_paths: &BTreeSet<PathBuf>) {
    for node in nodes {
        if node.is_folder() {
            node.expanded = expanded_paths.contains(&node.path);
        }
        restore_expanded_folder_paths(&mut node.children, expanded_paths);
    }
}

fn directories_from_documents(documents: &[Document]) -> Vec<PathBuf> {
    let mut directories = BTreeSet::new();
    for document in documents {
        if let Some(parent) = document.relative_path.parent() {
            if !parent.as_os_str().is_empty() {
                directories.insert(parent.to_path_buf());
                for ancestor in parent.ancestors().skip(1) {
                    if !ancestor.as_os_str().is_empty() {
                        directories.insert(ancestor.to_path_buf());
                    }
                }
            }
        }
    }
    directories.into_iter().collect()
}

fn collections_from_documents(documents: &[Document]) -> Vec<Collection> {
    let mut counts = BTreeMap::<String, usize>::new();
    for document in documents {
        *counts.entry(document.collection_id.clone()).or_default() += 1;
    }

    let mut collections = counts
        .into_iter()
        .map(|(id, document_count)| Collection {
            display_name: collection_display_name(&id),
            id,
            document_count,
        })
        .collect::<Vec<_>>();
    collections.sort_by(|left, right| {
        left.display_name
            .to_lowercase()
            .cmp(&right.display_name.to_lowercase())
    });
    collections
}

fn collection_display_name(id: &str) -> String {
    match id {
        "project" => String::from("Projects"),
        "person" => String::from("People"),
        "meeting" => String::from("Meetings"),
        "document" | "documents" => String::from("Documents"),
        value => {
            let mut chars = value.chars();
            match chars.next() {
                Some(first) => format!(
                    "{}{}s",
                    first.to_uppercase().collect::<String>(),
                    chars.collect::<String>()
                ),
                None => String::from("Documents"),
            }
        }
    }
}

fn build_children(
    absolute_dir: &Path,
    relative_dir: &Path,
    result: &ScanResult,
    next_id: &mut usize,
) -> Vec<ExplorerNode> {
    let mut directories = result
        .directories
        .iter()
        .filter(|directory| directory.parent() == Some(relative_dir))
        .collect::<Vec<_>>();
    directories.sort_by(|left, right| compare_paths(left, right));

    let mut files = result
        .documents
        .iter()
        .filter(|document| document.relative_path.parent() == Some(relative_dir))
        .collect::<Vec<_>>();
    files.sort_by(|left, right| compare_paths(&left.relative_path, &right.relative_path));

    let mut children = Vec::with_capacity(directories.len() + files.len());

    for directory in directories {
        let path = absolute_dir.join(directory.file_name().unwrap_or_default());
        let directory_children = build_children(&path, directory, result, next_id);
        let name = directory
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| directory.display().to_string());
        children.push(ExplorerNode::folder(
            take_id(next_id),
            name,
            path,
            directory_children,
        ));
    }

    for document in files {
        children.push(ExplorerNode::file(
            take_id(next_id),
            document.file_name.to_string_lossy().into_owned(),
            document.path.clone(),
        ));
    }

    children
}

fn take_id(next_id: &mut usize) -> usize {
    let id = *next_id;
    *next_id += 1;
    id
}

fn compare_paths(left: &Path, right: &Path) -> std::cmp::Ordering {
    left.to_string_lossy()
        .to_lowercase()
        .cmp(&right.to_string_lossy().to_lowercase())
}

fn inspector_properties(
    document: &Document,
    relation_index: &RelationIndex,
) -> Vec<InspectorField> {
    let relation_properties = relation_index
        .outgoing(&document.path)
        .into_iter()
        .map(|relation| relation.property.as_str())
        .collect::<BTreeSet<_>>();
    let mut properties = vec![InspectorField {
        label: String::from("Title"),
        value: InspectorValue::Text(document.title.clone()),
    }];

    properties.extend(document.properties.iter().filter_map(|(key, value)| {
        if is_special_property(key) || relation_properties.contains(key.as_str()) {
            None
        } else {
            Some(InspectorField {
                label: key.clone(),
                value: inspector_value(value),
            })
        }
    }));

    properties
}

fn inspector_outgoing_relation(relation: &Relation) -> InspectorRelation {
    let (target_path, status, candidates) = relation_status_summary(&relation.status);
    InspectorRelation {
        property: relation_display_property(&relation.property),
        label: relation.target.display.clone(),
        target_path,
        status,
        candidates,
    }
}

fn inspector_incoming_relation(relation: &Relation) -> InspectorRelation {
    InspectorRelation {
        property: relation_display_property(&relation.property),
        label: relation.source_title.clone(),
        target_path: Some(relation.source_document.clone()),
        status: InspectorRelationStatus::Resolved,
        candidates: Vec::new(),
    }
}

fn relation_status_summary(
    status: &RelationStatus,
) -> (
    Option<PathBuf>,
    InspectorRelationStatus,
    Vec<RelationDocumentSummary>,
) {
    match status {
        RelationStatus::Resolved(target) => (
            Some(target.path.clone()),
            InspectorRelationStatus::Resolved,
            Vec::new(),
        ),
        RelationStatus::Unresolved => (None, InspectorRelationStatus::Unresolved, Vec::new()),
        RelationStatus::Ambiguous(candidates) => (
            None,
            InspectorRelationStatus::Ambiguous(candidates.len()),
            candidates
                .iter()
                .map(|candidate| RelationDocumentSummary {
                    path: candidate.path.clone(),
                    relative_path: candidate.relative_path.clone(),
                    title: candidate.title.clone(),
                })
                .collect(),
        ),
    }
}

fn inspector_metadata(document: &Document, collection_display_name: String) -> Vec<InspectorField> {
    vec![
        InspectorField {
            label: String::from("Arquivo"),
            value: InspectorValue::Text(document.file_name.to_string_lossy().into_owned()),
        },
        InspectorField {
            label: String::from("Caminho"),
            value: InspectorValue::Text(document.relative_path.display().to_string()),
        },
        InspectorField {
            label: String::from("Tipo"),
            value: document
                .document_type
                .as_ref()
                .map(|value| InspectorValue::Text(value.clone()))
                .unwrap_or(InspectorValue::Empty),
        },
        InspectorField {
            label: String::from("Collection"),
            value: InspectorValue::Text(collection_display_name),
        },
        InspectorField {
            label: String::from("Tamanho"),
            value: document
                .metadata
                .file_size
                .map(format_file_size)
                .map(InspectorValue::Text)
                .unwrap_or(InspectorValue::Empty),
        },
        InspectorField {
            label: String::from("Modificado"),
            value: document
                .metadata
                .modified
                .map(format_system_time)
                .map(InspectorValue::Text)
                .unwrap_or(InspectorValue::Empty),
        },
    ]
}

fn inspector_tags(document: &Document) -> Vec<String> {
    match document.properties.get("tags") {
        Some(PropertyValue::Array(values)) => values
            .iter()
            .map(compact_property_value)
            .filter(|value| !value.trim().is_empty() && value != "—")
            .collect(),
        Some(value) => {
            let value = compact_property_value(value);
            if value.trim().is_empty() || value == "—" {
                Vec::new()
            } else {
                vec![value]
            }
        }
        None => Vec::new(),
    }
}

fn inspector_value(value: &PropertyValue) -> InspectorValue {
    match value {
        PropertyValue::Null => InspectorValue::Empty,
        PropertyValue::Bool(value) => InspectorValue::Bool(*value),
        PropertyValue::Number(value) => InspectorValue::Number(value.clone()),
        PropertyValue::String(value) => InspectorValue::Text(value.clone()),
        PropertyValue::Array(values) => {
            InspectorValue::Array(values.iter().map(compact_property_value).collect())
        }
        PropertyValue::Object(_) => InspectorValue::Object,
    }
}

fn compact_property_value(value: &PropertyValue) -> String {
    match value {
        PropertyValue::Null => String::from("—"),
        PropertyValue::Bool(true) => String::from("✓"),
        PropertyValue::Bool(false) => String::from("✕"),
        PropertyValue::Number(value) | PropertyValue::String(value) => value.clone(),
        PropertyValue::Array(values) => {
            let values = values
                .iter()
                .map(compact_property_value)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{values}]")
        }
        PropertyValue::Object(_) => String::from("{...}"),
    }
}

fn is_special_property(key: &str) -> bool {
    matches!(key, "title" | "type" | "tags")
}

fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn format_system_time(time: SystemTime) -> String {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| format_unix_timestamp(duration.as_secs()))
        .unwrap_or_else(|| String::from("—"))
}

fn format_unix_timestamp(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let seconds_of_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    format!("{day:02}/{month:02}/{year:04} {hour:02}:{minute:02}")
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month as u32, day as u32)
}

fn user_warning_message(message: &str) -> String {
    if message.to_lowercase().contains("yaml") {
        String::from("Frontmatter inválido.")
    } else {
        message.to_owned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDisplay {
    pub name: String,
    pub path: String,
    pub is_open: bool,
}

impl WorkspaceDisplay {
    fn none() -> Self {
        Self {
            name: String::from("Nenhuma pasta aberta"),
            path: String::from("Selecione uma pasta para usar como workspace"),
            is_open: false,
        }
    }
}

pub fn workspace_display(path: &Path) -> WorkspaceDisplay {
    let name = path
        .file_name()
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());

    WorkspaceDisplay {
        name,
        path: abbreviate_home(path),
        is_open: true,
    }
}

pub fn save_markdown_file(path: &Path, content: &str) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "arquivo sem diretório"))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "arquivo sem nome"))?;
    let temp_name = format!(
        ".{}.flokinmd-{}.tmp",
        file_name.to_string_lossy(),
        std::process::id()
    );
    let temp_path = parent.join(temp_name);

    match fs::write(&temp_path, content).and_then(|()| fs::rename(&temp_path, path)) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(&temp_path);
            Err(error)
        }
    }
}

fn abbreviate_home(path: &Path) -> String {
    home_dir()
        .and_then(|home| {
            path.strip_prefix(&home).ok().map(|relative| {
                if relative.as_os_str().is_empty() {
                    String::from("~")
                } else {
                    format!("~{}{}", MAIN_SEPARATOR, relative.display())
                }
            })
        })
        .unwrap_or_else(|| path.display().to_string())
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use crate::{
        mock_shell, save_markdown_file, scan_workspace, workspace_update_from_events, Collection,
        Document, DocumentMetadata, DocumentWarning, PropertyValue, ScanResult, TableModel,
        WorkspaceEvent,
    };

    use std::{
        collections::BTreeMap,
        ffi::OsString,
        fs,
        path::{Path, PathBuf},
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use super::{
        workspace_display, ExplorerNode, ExplorerNodeId, InspectorModel, InspectorValue, ScanState,
    };

    #[test]
    fn shell_starts_with_expected_mock_state() {
        let shell = mock_shell();

        assert_eq!(shell.current_workspace, None);
        assert_eq!(shell.scan_state, ScanState::Idle);
    }

    #[test]
    fn folder_selected_sets_workspace() {
        let mut shell = mock_shell();
        let path = PathBuf::from("/home/sc/Documents/Knowledge");

        shell.workspace_selected(Some(path.clone()));

        assert_eq!(shell.current_workspace, Some(path));
        assert_eq!(shell.scan_state, ScanState::Scanning);
    }

    #[test]
    fn selecting_a_folder_returns_to_file_explorer() {
        let mut shell = mock_shell();
        shell.open_sql_explorer();

        shell.workspace_selected(Some(PathBuf::from("/tmp/flokinmd-mdb004-test")));

        assert!(!shell.sql_explorer.open);
        assert!(shell.explorer.is_empty());
        assert_eq!(shell.scan_state, ScanState::Scanning);
    }

    #[test]
    fn first_scan_keeps_workspace_root_expanded() {
        let workspace = TempWorkspace::new();
        workspace.write("projects/carf.md", "# CARF");

        let shell = shell_from_workspace(&workspace);

        assert_eq!(shell.documents.len(), 1);
        assert_eq!(shell.explorer.len(), 1);
        assert!(shell.explorer[0].expanded);
        assert_eq!(shell.explorer[0].children[0].name, "projects");
    }

    #[test]
    fn folder_selection_cancel_keeps_existing_workspace() {
        let mut shell = mock_shell();
        let path = PathBuf::from("/home/sc/Documents/Knowledge");
        shell.workspace_selected(Some(path.clone()));

        shell.workspace_selected(None);

        assert_eq!(shell.current_workspace, Some(path));
    }

    #[test]
    fn selecting_another_folder_replaces_workspace() {
        let mut shell = mock_shell();
        let first = PathBuf::from("/home/sc/Documents/Knowledge");
        let second = PathBuf::from("/home/sc/Jobs/Flokin/repos/flokin-md");

        shell.workspace_selected(Some(first));
        shell.workspace_selected(Some(second.clone()));

        assert_eq!(shell.current_workspace, Some(second));
    }

    #[test]
    fn workspace_display_uses_folder_name_and_path() {
        let display = workspace_display(PathBuf::from("/tmp/flokin-md").as_path());

        assert_eq!(display.name, "flokin-md");
        assert_eq!(display.path, "/tmp/flokin-md");
        assert!(display.is_open);
    }

    #[test]
    fn workspace_display_handles_unicode_paths() {
        let display = workspace_display(PathBuf::from("/tmp/Conhecimento/ação").as_path());

        assert_eq!(display.name, "ação");
        assert!(display.path.ends_with("Conhecimento/ação"));
    }

    #[test]
    fn toggles_expanded_tree_nodes() {
        let mut shell = mock_shell();
        shell.explorer = vec![ExplorerNode::folder(
            1,
            "Knowledge",
            PathBuf::from("/tmp/Knowledge"),
            vec![ExplorerNode::folder(
                2,
                "Projects",
                PathBuf::from("/tmp/Knowledge/Projects"),
                Vec::new(),
            )],
        )];

        assert!(shell.toggle_explorer_node(ExplorerNodeId(2)));
        assert!(!shell.explorer[0].children[0].expanded);
        assert!(shell.toggle_explorer_node(ExplorerNodeId(2)));
        assert!(shell.explorer[0].children[0].expanded);
    }

    #[test]
    fn ignores_toggle_for_unknown_tree_nodes() {
        let mut shell = mock_shell();

        assert!(!shell.toggle_explorer_node(ExplorerNodeId(999)));
    }

    #[test]
    fn table_row_selection_selects_document_for_inspector() {
        let mut shell = shell_with_documents(vec![
            document(
                "projects/task-42.md",
                "Task 42",
                "project",
                [("priority", number("42"))],
            ),
            document(
                "projects/task-43.md",
                "Task 43",
                "project",
                [("priority", number("43"))],
            ),
        ]);
        shell.select_collection(String::from("project"));
        let table = TableModel::collection("project", &shell.documents, None);

        shell.select_markdown_path(table.rows[0].document_path.clone());

        assert_eq!(shell.selected_document().unwrap().title, "Task 42");
        assert_eq!(property_value(&shell, "Title").display_value(), "Task 42");
    }

    #[test]
    fn path_selection_selects_document_for_inspector() {
        let mut shell = shell_with_documents(vec![document(
            "people/sergio.md",
            "Sérgio",
            "person",
            [("active", bool_value(true))],
        )]);
        let path = shell.documents[0].path.clone();

        shell.select_markdown_path(path);

        assert_eq!(property_value(&shell, "Title").display_value(), "Sérgio");
    }

    #[test]
    fn search_selection_points_to_real_document_for_inspector() {
        let mut shell = shell_with_documents(vec![
            document("projects/carf.md", "CARF", "project", []),
            document("meetings/reforma-carf.md", "Reforma do CARF", "meeting", []),
        ]);

        shell.open_search();
        shell.update_search_query(String::from("reforma"));
        shell.refresh_search_results();
        assert!(shell.activate_selected_search_result());

        assert_eq!(shell.selected_document().unwrap().title, "Reforma do CARF");
        assert_eq!(
            property_value(&shell, "Title").display_value(),
            "Reforma do CARF"
        );
    }

    #[test]
    fn changing_selection_updates_inspector_without_stale_data() {
        let mut shell = shell_with_documents(vec![
            document(
                "projects/task-42.md",
                "Task 42",
                "project",
                [("priority", number("42"))],
            ),
            document(
                "projects/task-43.md",
                "Task 43",
                "project",
                [("priority", number("43"))],
            ),
        ]);

        shell.select_markdown_path(shell.documents[0].path.clone());
        assert_eq!(property_value(&shell, "priority").display_value(), "42");

        shell.select_markdown_path(shell.documents[1].path.clone());
        assert_eq!(property_value(&shell, "priority").display_value(), "43");
    }

    #[test]
    fn workspace_change_clears_document_selection() {
        let mut shell = shell_with_documents(vec![document("task.md", "Task", "project", [])]);
        shell.select_markdown_path(shell.documents[0].path.clone());

        shell.workspace_selected(Some(PathBuf::from("/tmp/another-workspace")));

        assert!(shell.selected_document().is_none());
        assert!(matches!(
            shell.document_inspector(),
            InspectorModel::Empty { .. }
        ));
    }

    #[test]
    fn collection_change_clears_document_selection() {
        let mut shell = shell_with_documents(vec![
            document("projects/task.md", "Task", "project", []),
            document("people/sergio.md", "Sérgio", "person", []),
        ]);
        shell.select_markdown_path(shell.documents[0].path.clone());

        shell.select_collection(String::from("person"));

        assert!(shell.selected_document().is_none());
        assert!(matches!(
            shell.document_inspector(),
            InspectorModel::Empty { .. }
        ));
    }

    #[test]
    fn frontmatter_properties_reach_inspector_model() {
        let mut shell = shell_with_documents(vec![document(
            "projects/carf.md",
            "CARF via Frontmatter",
            "project",
            [("owner", string("Sergio"))],
        )]);

        shell.select_markdown_path(shell.documents[0].path.clone());

        assert_eq!(
            property_value(&shell, "Title").display_value(),
            "CARF via Frontmatter"
        );
        assert_eq!(property_value(&shell, "owner").display_value(), "Sergio");
        assert!(property(&shell, "title").is_none());
    }

    #[test]
    fn inspector_preserves_string_number_bool_array_and_null_values() {
        let mut shell = shell_with_documents(vec![document(
            "projects/types.md",
            "Types",
            "project",
            [
                ("name", string("Sergio")),
                ("score", number("42")),
                ("active", bool_value(true)),
                ("done", bool_value(false)),
                (
                    "skills",
                    PropertyValue::Array(vec![string("rust"), string("jota")]),
                ),
                ("empty", PropertyValue::Null),
            ],
        )]);

        shell.select_markdown_path(shell.documents[0].path.clone());

        assert_eq!(
            property_value(&shell, "name"),
            InspectorValue::Text(String::from("Sergio"))
        );
        assert_eq!(
            property_value(&shell, "score"),
            InspectorValue::Number(String::from("42"))
        );
        assert_eq!(property_value(&shell, "active"), InspectorValue::Bool(true));
        assert_eq!(property_value(&shell, "done"), InspectorValue::Bool(false));
        assert_eq!(
            property_value(&shell, "skills"),
            InspectorValue::Array(vec![String::from("rust"), String::from("jota")])
        );
        assert_eq!(property_value(&shell, "empty"), InspectorValue::Empty);
    }

    #[test]
    fn inspector_renders_real_tags_without_global_mock_counts() {
        let mut shell = shell_with_documents(vec![document(
            "projects/tags.md",
            "Tags",
            "project",
            [(
                "tags",
                PropertyValue::Array(vec![string("rust"), string("jota")]),
            )],
        )]);

        shell.select_markdown_path(shell.documents[0].path.clone());

        let InspectorModel::Document(inspector) = shell.document_inspector() else {
            panic!("expected selected document inspector");
        };
        assert_eq!(inspector.tags, vec!["rust", "jota"]);
        assert!(inspector
            .properties
            .iter()
            .all(|field| field.label != "tags"));
    }

    #[test]
    fn inspector_metadata_includes_filename_relative_path_and_file_size() {
        let mut shell = shell_with_documents(vec![document_with_metadata(
            "tasks/task-42.md",
            "Task 42",
            "project",
            [],
            Some(42),
        )]);

        shell.select_markdown_path(shell.documents[0].path.clone());

        assert_eq!(
            metadata_value(&shell, "Arquivo").display_value(),
            "task-42.md"
        );
        assert_eq!(
            metadata_value(&shell, "Caminho").display_value(),
            "tasks/task-42.md"
        );
        assert_eq!(metadata_value(&shell, "Tamanho").display_value(), "42 B");
    }

    #[test]
    fn inspector_warns_about_invalid_yaml_without_debug_text() {
        let mut document = document("broken.md", "Broken Frontmatter", "documents", []);
        document.warnings.push(DocumentWarning {
            path: document.path.clone(),
            message: String::from("YAML frontmatter inválido: parser details"),
        });
        let mut shell = shell_with_documents(vec![document]);

        shell.select_markdown_path(shell.documents[0].path.clone());

        let InspectorModel::Document(inspector) = shell.document_inspector() else {
            panic!("expected selected document inspector");
        };
        assert_eq!(inspector.warnings, vec!["Frontmatter inválido."]);
    }

    #[test]
    fn document_without_frontmatter_still_has_title_and_metadata() {
        let mut shell = shell_with_documents(vec![document("empty.md", "empty", "documents", [])]);

        shell.select_markdown_path(shell.documents[0].path.clone());

        assert_eq!(property_value(&shell, "Title").display_value(), "empty");
        assert_eq!(metadata_value(&shell, "Tipo"), InspectorValue::Empty);
    }

    #[test]
    fn inspector_supports_unicode_values() {
        let mut shell = shell_with_documents(vec![document(
            "ações/visão.md",
            "Visão Geral",
            "documents",
            [("responsável", string("Sérgio"))],
        )]);

        shell.select_markdown_path(shell.documents[0].path.clone());

        assert_eq!(
            property_value(&shell, "Title").display_value(),
            "Visão Geral"
        );
        assert_eq!(
            property_value(&shell, "responsável").display_value(),
            "Sérgio"
        );
        assert_eq!(
            metadata_value(&shell, "Caminho").display_value(),
            "ações/visão.md"
        );
    }

    #[test]
    fn empty_selection_returns_empty_inspector_state() {
        let shell = shell_with_documents(vec![document("task.md", "Task", "project", [])]);

        assert_eq!(
            shell.document_inspector(),
            InspectorModel::Empty {
                title: String::from("Nenhum documento selecionado."),
                description: String::from(
                    "Selecione um documento ou registro para ver suas propriedades."
                ),
            }
        );
    }

    #[test]
    fn workspace_modify_updates_document_properties() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "projects/task.md",
            "---\ntype: project\npriority: 42\n---\n# Task\n",
        );
        let mut shell = shell_from_workspace(&workspace);

        workspace.write(
            "projects/task.md",
            "---\ntype: project\npriority: 999\n---\n# Task\n",
        );
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("projects/task.md"),
            )],
        );

        let table = TableModel::collection("project", &shell.documents, None);
        assert_eq!(table.rows[0].cells[1].display_value(), "999");
    }

    #[test]
    fn workspace_create_adds_document_and_updates_collection() {
        let workspace = TempWorkspace::new();
        workspace.write("projects/carf.md", "---\ntype: project\n---\n# CARF\n");
        let mut shell = shell_from_workspace(&workspace);

        workspace.write(
            "projects/new-project.md",
            "---\ntype: project\n---\n# New\n",
        );
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("projects/new-project.md"),
            )],
        );

        assert_eq!(shell.documents.len(), 2);
        assert_eq!(
            shell
                .collections
                .iter()
                .find(|collection| collection.id == "project")
                .unwrap()
                .document_count,
            2
        );
    }

    #[test]
    fn workspace_remove_removes_document_and_selected_document() {
        let workspace = TempWorkspace::new();
        workspace.write("projects/carf.md", "---\ntype: project\n---\n# CARF\n");
        let mut shell = shell_from_workspace(&workspace);
        let path = workspace.path().join("projects/carf.md");
        shell.select_markdown_path(path.clone());

        fs::remove_file(&path).unwrap();
        apply_events(&mut shell, &workspace, [WorkspaceEvent::Remove(path)]);

        assert!(shell.documents.is_empty());
        assert!(shell.selected_document().is_none());
        assert!(matches!(
            shell.document_inspector(),
            InspectorModel::Empty { .. }
        ));
    }

    #[test]
    fn workspace_rename_does_not_duplicate_document() {
        let workspace = TempWorkspace::new();
        workspace.write("projects/carf.md", "---\ntype: project\n---\n# CARF\n");
        let mut shell = shell_from_workspace(&workspace);
        let from = workspace.path().join("projects/carf.md");
        let to = workspace.path().join("projects/carf-2026.md");

        fs::rename(&from, &to).unwrap();
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Rename {
                from: from.clone(),
                to: to.clone(),
            }],
        );

        assert_eq!(shell.documents.len(), 1);
        assert_eq!(shell.documents[0].path, to);
    }

    #[test]
    fn workspace_move_updates_path() {
        let workspace = TempWorkspace::new();
        workspace.write("projects/carf.md", "---\ntype: project\n---\n# CARF\n");
        let mut shell = shell_from_workspace(&workspace);
        let from = workspace.path().join("projects/carf.md");
        let to = workspace.path().join("archive/carf.md");
        fs::create_dir_all(workspace.path().join("archive")).unwrap();

        fs::rename(&from, &to).unwrap();
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Rename {
                from: from.clone(),
                to: to.clone(),
            }],
        );

        assert_eq!(
            shell.documents[0].relative_path,
            PathBuf::from("archive/carf.md")
        );
        assert!(shell
            .explorer
            .iter()
            .any(|node| node.children.iter().any(|child| child.name == "archive")));
    }

    #[test]
    fn workspace_type_change_moves_between_collections() {
        let workspace = TempWorkspace::new();
        workspace.write("projects/carf.md", "---\ntype: project\n---\n# CARF\n");
        let mut shell = shell_from_workspace(&workspace);

        workspace.write("projects/carf.md", "---\ntype: person\n---\n# CARF\n");
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("projects/carf.md"),
            )],
        );

        assert!(shell.collection_documents("project").is_empty());
        assert_eq!(shell.collection_documents("person").len(), 1);
    }

    #[test]
    fn workspace_new_property_updates_table_columns() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "projects/carf.md",
            "---\ntype: project\nstatus: active\n---\n# CARF\n",
        );
        let mut shell = shell_from_workspace(&workspace);

        workspace.write(
            "projects/carf.md",
            "---\ntype: project\nstatus: active\nbudget: 1000\n---\n# CARF\n",
        );
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("projects/carf.md"),
            )],
        );

        let table = TableModel::collection("project", &shell.documents, None);
        assert!(table.columns.iter().any(|column| column.label == "Budget"));
    }

    #[test]
    fn selected_document_source_uses_real_loaded_content() {
        let workspace = TempWorkspace::new();
        let content = "---\ntitle: CARF Daily\ntype: meeting\n---\n\n# CARF Daily\n";
        workspace.write("meetings/carf.md", content);
        let mut shell = shell_from_workspace(&workspace);

        shell.select_markdown_path(workspace.path().join("meetings/carf.md"));

        let source = shell.selected_document_source().unwrap();
        assert_eq!(source.title, "CARF Daily");
        assert_eq!(source.relative_path, PathBuf::from("meetings/carf.md"));
        assert_eq!(source.content.as_deref(), Some(content));
    }

    #[test]
    fn different_documents_do_not_share_source_content() {
        let workspace = TempWorkspace::new();
        workspace.write("projects/carf.md", "# CARF\n");
        workspace.write("people/sergio.md", "# Sergio\n");
        let mut shell = shell_from_workspace(&workspace);

        shell.select_markdown_path(workspace.path().join("projects/carf.md"));
        assert_eq!(
            shell.selected_document_source().unwrap().content.as_deref(),
            Some("# CARF\n")
        );

        shell.select_markdown_path(workspace.path().join("people/sergio.md"));
        assert_eq!(
            shell.selected_document_source().unwrap().content.as_deref(),
            Some("# Sergio\n")
        );
    }

    #[test]
    fn selected_document_source_handles_empty_and_unicode_files() {
        let workspace = TempWorkspace::new();
        workspace.write("empty.md", "");
        workspace.write("ações/visão.md", "# Visão\nConteúdo real.\n");
        let mut shell = shell_from_workspace(&workspace);

        shell.select_markdown_path(workspace.path().join("empty.md"));
        assert_eq!(
            shell.selected_document_source().unwrap().content.as_deref(),
            Some("")
        );

        shell.select_markdown_path(workspace.path().join("ações/visão.md"));
        assert_eq!(
            shell.selected_document_source().unwrap().content.as_deref(),
            Some("# Visão\nConteúdo real.\n")
        );
    }

    #[test]
    fn watcher_updates_selected_document_source_content() {
        let workspace = TempWorkspace::new();
        workspace.write("projects/carf.md", "# CARF\ntexto original\n");
        let mut shell = shell_from_workspace(&workspace);
        shell.select_markdown_path(workspace.path().join("projects/carf.md"));

        workspace.write("projects/carf.md", "# CARF\ntexto alterado pelo watcher\n");
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("projects/carf.md"),
            )],
        );

        assert_eq!(
            shell.selected_document_source().unwrap().content.as_deref(),
            Some("# CARF\ntexto alterado pelo watcher\n")
        );
    }

    #[test]
    fn relation_navigation_changes_selected_document_source() {
        let mut shell = shell_with_documents(vec![
            document_with_source("projects/carf.md", "CARF", "project", [], "# CARF\n"),
            document_with_source(
                "meetings/carf.md",
                "CARF Daily",
                "meeting",
                [("project", string("[[CARF]]"))],
                "# CARF Daily\n",
            ),
        ]);
        shell.select_markdown_path(path(&shell, "meetings/carf.md"));
        let InspectorModel::Document(inspector) = shell.document_inspector() else {
            panic!("expected document inspector");
        };
        let target = inspector.outgoing_relations[0].target_path.clone().unwrap();

        shell.select_markdown_path(target);

        assert_eq!(
            shell.selected_document_source().unwrap().content.as_deref(),
            Some("# CARF\n")
        );
    }

    #[test]
    fn incoming_relation_navigation_changes_selected_document_source() {
        let mut shell = shell_with_documents(vec![
            document_with_source("projects/carf.md", "CARF", "project", [], "# CARF\n"),
            document_with_source(
                "meetings/carf.md",
                "CARF Daily",
                "meeting",
                [("project", string("[[CARF]]"))],
                "# CARF Daily\n",
            ),
        ]);
        shell.select_markdown_path(path(&shell, "projects/carf.md"));
        let InspectorModel::Document(inspector) = shell.document_inspector() else {
            panic!("expected document inspector");
        };
        let source = inspector.incoming_relations[0].target_path.clone().unwrap();

        shell.select_markdown_path(source);

        assert_eq!(
            shell.selected_document_source().unwrap().content.as_deref(),
            Some("# CARF Daily\n")
        );
    }

    #[test]
    fn human_timestamp_does_not_expose_raw_epoch() {
        let value = super::format_system_time(UNIX_EPOCH);

        assert_eq!(value, "01/01/1970 00:00");
        assert!(!value.contains("UNIX"));
        assert!(!value.contains("epoch"));
    }

    #[test]
    fn workspace_change_clears_previous_document_source() {
        let mut shell = shell_with_documents(vec![document_with_source(
            "projects/carf.md",
            "CARF",
            "project",
            [],
            "# CARF\n",
        )]);
        shell.select_markdown_path(path(&shell, "projects/carf.md"));
        assert!(shell.selected_document_source().is_some());

        shell.workspace_selected(Some(PathBuf::from("/tmp/new-workspace")));

        assert!(shell.selected_document_source().is_none());
    }

    #[test]
    fn inspector_shows_outgoing_and_incoming_relations() {
        let mut shell = shell_with_documents(vec![
            document("projects/carf.md", "CARF", "project", []),
            document("people/sergio.md", "Sergio", "person", []),
            document(
                "meetings/carf.md",
                "CARF Daily",
                "meeting",
                [
                    ("project", string("[[CARF]]")),
                    ("owner", string("[[Sergio]]")),
                ],
            ),
        ]);
        let meeting = path(&shell, "meetings/carf.md");

        shell.select_markdown_path(meeting);
        let InspectorModel::Document(inspector) = shell.document_inspector() else {
            panic!("expected document inspector");
        };
        assert_eq!(inspector.outgoing_relations.len(), 2);
        assert!(inspector
            .properties
            .iter()
            .all(|field| field.label != "project" && field.label != "owner"));
        assert!(inspector
            .outgoing_relations
            .iter()
            .any(|relation| relation.property == "Project" && relation.label == "CARF"));

        shell.select_markdown_path(path(&shell, "projects/carf.md"));
        let InspectorModel::Document(inspector) = shell.document_inspector() else {
            panic!("expected document inspector");
        };
        assert_eq!(inspector.incoming_relations.len(), 1);
        assert_eq!(inspector.incoming_relations[0].label, "CARF Daily");
        assert_eq!(inspector.incoming_relations[0].property, "Project");
    }

    #[test]
    fn watcher_change_updates_relation_index() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "people/sergio.md",
            "---\ntitle: Sergio\ntype: person\n---\n",
        );
        workspace.write("people/maria.md", "---\ntitle: Maria\ntype: person\n---\n");
        workspace.write(
            "meetings/carf.md",
            "---\ntitle: CARF Daily\ntype: meeting\nowner: \"[[Sergio]]\"\n---\n",
        );
        let mut shell = shell_from_workspace(&workspace);

        workspace.write(
            "meetings/carf.md",
            "---\ntitle: CARF Daily\ntype: meeting\nowner: \"[[Maria]]\"\n---\n",
        );
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("meetings/carf.md"),
            )],
        );

        assert_eq!(
            shell
                .relation_index
                .incoming(&workspace.path().join("people/sergio.md"))
                .len(),
            0
        );
        assert_eq!(
            shell
                .relation_index
                .incoming(&workspace.path().join("people/maria.md"))
                .len(),
            1
        );
    }

    #[test]
    fn creating_and_removing_target_changes_resolution() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "meetings/carf.md",
            "---\ntitle: CARF Daily\ntype: meeting\nowner: \"[[Maria]]\"\n---\n",
        );
        let mut shell = shell_from_workspace(&workspace);
        assert!(matches!(
            shell.relation_index.all()[0].status,
            crate::RelationStatus::Unresolved
        ));

        workspace.write("people/maria.md", "---\ntitle: Maria\ntype: person\n---\n");
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("people/maria.md"),
            )],
        );
        assert!(matches!(
            shell.relation_index.all()[0].status,
            crate::RelationStatus::Resolved(_)
        ));

        let maria = workspace.path().join("people/maria.md");
        fs::remove_file(&maria).unwrap();
        apply_events(&mut shell, &workspace, [WorkspaceEvent::Remove(maria)]);
        assert!(matches!(
            shell.relation_index.all()[0].status,
            crate::RelationStatus::Unresolved
        ));
    }

    #[test]
    fn title_rename_breaks_title_relation_but_not_path_relation() {
        let workspace = TempWorkspace::new();
        workspace.write("projects/carf.md", "---\ntitle: CARF\ntype: project\n---\n");
        workspace.write(
            "meetings/by-title.md",
            "---\ntitle: By Title\ntype: meeting\nproject: \"[[CARF]]\"\n---\n",
        );
        workspace.write(
            "meetings/by-path.md",
            "---\ntitle: By Path\ntype: meeting\nproject: \"[[projects/carf.md]]\"\n---\n",
        );
        let mut shell = shell_from_workspace(&workspace);

        workspace.write(
            "projects/carf.md",
            "---\ntitle: Conselho CARF\ntype: project\n---\n",
        );
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("projects/carf.md"),
            )],
        );

        let by_title = shell
            .relation_index
            .all()
            .iter()
            .find(|relation| relation.source_title == "By Title")
            .unwrap();
        assert!(matches!(by_title.status, crate::RelationStatus::Unresolved));

        let by_path = shell
            .relation_index
            .all()
            .iter()
            .find(|relation| relation.source_title == "By Path")
            .unwrap();
        assert!(matches!(by_path.status, crate::RelationStatus::Resolved(_)));
    }

    #[test]
    fn workspace_change_rebuilds_relation_index_without_leaking_old_workspace() {
        let mut shell = shell_with_documents(vec![
            document("projects/carf.md", "CARF", "project", []),
            document(
                "meetings/carf.md",
                "CARF Daily",
                "meeting",
                [("project", string("[[CARF]]"))],
            ),
        ]);
        assert_eq!(shell.relation_index.all().len(), 1);

        shell.workspace_selected(Some(PathBuf::from("/tmp/another-workspace")));
        assert!(shell.relation_index.all().is_empty());

        shell.scan_completed(ScanResult {
            root: PathBuf::from("/tmp/another-workspace"),
            documents: vec![document("people/maria.md", "Maria", "person", [])],
            collections: vec![Collection {
                id: String::from("person"),
                display_name: String::from("People"),
                document_count: 1,
            }],
            directories: Vec::new(),
            errors: Vec::new(),
            duration: Duration::ZERO,
        });
        assert!(shell.relation_index.all().is_empty());
    }

    #[test]
    fn opening_markdown_file_creates_real_tab_once_and_activates_it() {
        let workspace = TempWorkspace::new();
        workspace.write("projects/carf.md", "---\ntitle: CARF\n---\n# CARF\n");
        let mut shell = shell_from_workspace(&workspace);
        let carf = path(&shell, "projects/carf.md");

        assert!(shell.select_markdown_path(carf.clone()));
        assert!(shell.select_markdown_path(carf.clone()));

        assert_eq!(shell.editor.tabs.len(), 1);
        assert_eq!(shell.editor.active_path, Some(carf.clone()));
        assert_eq!(shell.selected_document_path, Some(carf));
        assert_eq!(
            shell.active_editor_buffer(),
            Some("---\ntitle: CARF\n---\n# CARF\n")
        );
    }

    #[test]
    fn multiple_markdown_files_create_multiple_tabs_and_switch_active_document() {
        let workspace = TempWorkspace::new();
        workspace.write("projects/carf.md", "# CARF\n");
        workspace.write("people/sergio.md", "# Sergio\n");
        let mut shell = shell_from_workspace(&workspace);
        let carf = path(&shell, "projects/carf.md");
        let sergio = path(&shell, "people/sergio.md");

        shell.select_markdown_path(carf.clone());
        shell.select_markdown_path(sergio);
        shell.activate_editor_tab(carf.clone());

        assert_eq!(shell.editor.tabs.len(), 2);
        assert_eq!(shell.editor.active_path, Some(carf.clone()));
        assert_eq!(shell.selected_document_path, Some(carf));
        assert_eq!(shell.active_editor_buffer(), Some("# CARF\n"));
    }

    #[test]
    fn editing_marks_dirty_and_returning_to_saved_marks_clean() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "# Saved\n");
        let mut shell = shell_from_workspace(&workspace);
        shell.select_markdown_path(path(&shell, "doc.md"));

        shell.update_active_editor_buffer(String::from("# Changed\n"));
        assert!(shell.active_editor_tab().unwrap().dirty);

        shell.update_active_editor_buffer(String::from("# Saved\n"));
        assert!(!shell.active_editor_tab().unwrap().dirty);
    }

    #[test]
    fn save_writes_file_and_save_completion_clears_dirty() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "# Saved\n");
        let mut shell = shell_from_workspace(&workspace);
        let doc = path(&shell, "doc.md");
        shell.select_markdown_path(doc.clone());
        shell.update_active_editor_buffer(String::from("# Changed\n"));

        save_markdown_file(&doc, shell.active_editor_buffer().unwrap()).unwrap();
        shell.editor_save_completed(&doc, "# Changed\n", Ok(()));

        assert_eq!(fs::read_to_string(&doc).unwrap(), "# Changed\n");
        assert!(!shell.active_editor_tab().unwrap().dirty);
    }

    #[test]
    fn close_clean_tab_closes_and_close_dirty_requests_dialog() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "# Saved\n");
        let mut shell = shell_from_workspace(&workspace);
        let doc = path(&shell, "doc.md");
        shell.select_markdown_path(doc.clone());

        shell.request_close_editor_tab(doc.clone());
        assert!(shell.editor.tabs.is_empty());

        shell.select_markdown_path(doc.clone());
        shell.update_active_editor_buffer(String::from("# Dirty\n"));
        shell.request_close_editor_tab(doc.clone());

        assert_eq!(shell.editor.tabs.len(), 1);
        assert_eq!(
            shell.editor.dialog,
            Some(super::EditorDialog::CloseDirty { document_path: doc })
        );
    }

    #[test]
    fn cancel_preserves_dirty_tab_and_discard_preserves_file() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "# Saved\n");
        let mut shell = shell_from_workspace(&workspace);
        let doc = path(&shell, "doc.md");
        shell.select_markdown_path(doc.clone());
        shell.update_active_editor_buffer(String::from("# Dirty\n"));
        shell.request_close_editor_tab(doc.clone());

        shell.cancel_editor_dialog();
        assert_eq!(shell.editor.tabs.len(), 1);
        assert_eq!(shell.active_editor_buffer(), Some("# Dirty\n"));

        shell.request_close_editor_tab(doc.clone());
        shell.discard_dialog_changes();
        assert!(shell.editor.tabs.is_empty());
        assert_eq!(fs::read_to_string(doc).unwrap(), "# Saved\n");
    }

    #[test]
    fn save_failure_keeps_dirty_tab_and_error() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "# Saved\n");
        let mut shell = shell_from_workspace(&workspace);
        let doc = path(&shell, "doc.md");
        shell.select_markdown_path(doc.clone());
        shell.update_active_editor_buffer(String::from("# Dirty\n"));

        let saved = shell.editor_save_completed(&doc, "# Dirty\n", Err(String::from("falhou")));

        assert!(!saved);
        let tab = shell.active_editor_tab().unwrap();
        assert!(tab.dirty);
        assert_eq!(tab.save_error.as_deref(), Some("falhou"));
    }

    #[test]
    fn clean_open_tab_tracks_external_watcher_update() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "# Original\n");
        let mut shell = shell_from_workspace(&workspace);
        let doc = path(&shell, "doc.md");
        shell.select_markdown_path(doc.clone());

        workspace.write("doc.md", "# External\n");
        apply_events(&mut shell, &workspace, [WorkspaceEvent::Upsert(doc)]);

        let tab = shell.active_editor_tab().unwrap();
        assert_eq!(tab.buffer, "# External\n");
        assert_eq!(tab.saved_content, "# External\n");
        assert!(!tab.dirty);
    }

    #[test]
    fn dirty_open_tab_keeps_local_buffer_and_records_external_conflict() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "# Original\n");
        let mut shell = shell_from_workspace(&workspace);
        let doc = path(&shell, "doc.md");
        shell.select_markdown_path(doc.clone());
        shell.update_active_editor_buffer(String::from("# Local\n"));

        workspace.write("doc.md", "# External\n");
        apply_events(&mut shell, &workspace, [WorkspaceEvent::Upsert(doc)]);

        let tab = shell.active_editor_tab().unwrap();
        assert_eq!(tab.buffer, "# Local\n");
        assert!(matches!(
            tab.external_conflict.as_ref(),
            Some(super::EditorExternalConflict::Modified(content)) if content == "# External\n"
        ));
    }

    #[test]
    fn reload_external_discards_local_buffer_and_keep_preserves_dirty_buffer() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "# Original\n");
        let mut shell = shell_from_workspace(&workspace);
        let doc = path(&shell, "doc.md");
        shell.select_markdown_path(doc.clone());
        shell.update_active_editor_buffer(String::from("# Local\n"));
        workspace.write("doc.md", "# External\n");
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(doc.clone())],
        );

        shell.keep_local_editor_changes();
        assert_eq!(shell.active_editor_buffer(), Some("# Local\n"));
        assert!(shell.active_editor_tab().unwrap().dirty);
        assert!(shell
            .active_editor_tab()
            .unwrap()
            .external_conflict
            .is_none());

        workspace.write("doc.md", "# External 2\n");
        apply_events(&mut shell, &workspace, [WorkspaceEvent::Upsert(doc)]);
        shell.reload_external_editor_change();
        assert_eq!(shell.active_editor_buffer(), Some("# External 2\n"));
        assert!(!shell.active_editor_tab().unwrap().dirty);
    }

    #[test]
    fn own_save_watcher_event_does_not_make_tab_dirty_again() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "# Original\n");
        let mut shell = shell_from_workspace(&workspace);
        let doc = path(&shell, "doc.md");
        shell.select_markdown_path(doc.clone());
        shell.update_active_editor_buffer(String::from("# Saved by app\n"));
        save_markdown_file(&doc, shell.active_editor_buffer().unwrap()).unwrap();
        shell.editor_save_completed(&doc, "# Saved by app\n", Ok(()));

        apply_events(&mut shell, &workspace, [WorkspaceEvent::Upsert(doc)]);

        assert!(!shell.active_editor_tab().unwrap().dirty);
        assert!(shell
            .active_editor_tab()
            .unwrap()
            .external_conflict
            .is_none());
    }

    #[test]
    fn empty_unicode_and_frontmatter_content_stay_in_editor_buffer() {
        let workspace = TempWorkspace::new();
        workspace.write("empty.md", "");
        workspace.write(
            "unicode.md",
            "---\ntitle: Reunião São Paulo\n---\n# Café\nDescrição\n",
        );
        let mut shell = shell_from_workspace(&workspace);

        shell.select_markdown_path(path(&shell, "empty.md"));
        assert_eq!(shell.active_editor_buffer(), Some(""));

        shell.select_markdown_path(path(&shell, "unicode.md"));
        assert_eq!(
            shell.active_editor_buffer(),
            Some("---\ntitle: Reunião São Paulo\n---\n# Café\nDescrição\n")
        );
    }

    #[test]
    fn duplicate_filenames_are_distinct_tabs_by_path() {
        let workspace = TempWorkspace::new();
        workspace.write("projects/index.md", "# Project\n");
        workspace.write("people/index.md", "# Person\n");
        let mut shell = shell_from_workspace(&workspace);
        shell.select_markdown_path(path(&shell, "projects/index.md"));
        shell.select_markdown_path(path(&shell, "people/index.md"));

        assert_eq!(shell.editor.tabs.len(), 2);
        assert_ne!(
            shell.editor.tabs[0].document_path,
            shell.editor.tabs[1].document_path
        );
        assert_eq!(shell.editor.tabs[0].title, "index.md");
        assert_eq!(shell.editor.tabs[1].title, "index.md");
    }

    #[test]
    fn frontmatter_save_updates_document_collection_through_pipeline() {
        let workspace = TempWorkspace::new();
        workspace.write("item.md", "---\ntitle: Item\ntype: project\n---\n");
        let mut shell = shell_from_workspace(&workspace);
        let item = path(&shell, "item.md");
        shell.select_markdown_path(item.clone());
        shell.update_active_editor_buffer(String::from("---\ntitle: Item\ntype: person\n---\n"));
        save_markdown_file(&item, shell.active_editor_buffer().unwrap()).unwrap();
        shell.editor_save_completed(&item, "---\ntitle: Item\ntype: person\n---\n", Ok(()));
        apply_events(&mut shell, &workspace, [WorkspaceEvent::Upsert(item)]);

        assert_eq!(shell.selected_document().unwrap().collection_id, "person");
        assert!(shell
            .collections
            .iter()
            .any(|collection| collection.id == "person"));
    }

    #[test]
    fn relation_edit_save_updates_relation_index_through_pipeline() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "people/sergio.md",
            "---\ntitle: Sergio\ntype: person\n---\n",
        );
        workspace.write("people/maria.md", "---\ntitle: Maria\ntype: person\n---\n");
        workspace.write(
            "meeting.md",
            "---\ntitle: Meeting\nowner: \"[[Sergio]]\"\n---\n",
        );
        let mut shell = shell_from_workspace(&workspace);
        let meeting = path(&shell, "meeting.md");
        shell.select_markdown_path(meeting.clone());
        shell.update_active_editor_buffer(String::from(
            "---\ntitle: Meeting\nowner: \"[[Maria]]\"\n---\n",
        ));
        save_markdown_file(&meeting, shell.active_editor_buffer().unwrap()).unwrap();
        shell.editor_save_completed(
            &meeting,
            "---\ntitle: Meeting\nowner: \"[[Maria]]\"\n---\n",
            Ok(()),
        );
        apply_events(&mut shell, &workspace, [WorkspaceEvent::Upsert(meeting)]);

        assert_eq!(
            shell
                .relation_index
                .incoming(&workspace.path().join("people/sergio.md"))
                .len(),
            0
        );
        assert_eq!(
            shell
                .relation_index
                .incoming(&workspace.path().join("people/maria.md"))
                .len(),
            1
        );
    }

    #[test]
    fn workspace_change_with_dirty_tabs_requests_safe_dialog() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "# Original\n");
        let mut shell = shell_from_workspace(&workspace);
        shell.select_markdown_path(path(&shell, "doc.md"));
        shell.update_active_editor_buffer(String::from("# Dirty\n"));

        assert!(shell.request_close_workspace());
        assert!(matches!(
            shell.editor.dialog,
            Some(super::EditorDialog::CloseDirty { .. })
        ));
    }

    #[test]
    fn dirty_tab_survives_transient_filesystem_replace() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "# Original\n");
        let mut shell = shell_from_workspace(&workspace);
        let doc = path(&shell, "doc.md");
        shell.select_markdown_path(doc.clone());
        shell.update_active_editor_buffer(String::from("# Local\n"));
        workspace.write("doc.md", "# External final\n");

        apply_events(
            &mut shell,
            &workspace,
            [
                WorkspaceEvent::Remove(doc.clone()),
                WorkspaceEvent::Upsert(doc.clone()),
            ],
        );

        let tab = shell.editor.tab(&doc).unwrap();
        assert_eq!(tab.buffer, "# Local\n");
        assert!(tab.dirty);
        assert!(tab.external_conflict.is_some());
    }

    #[test]
    fn event_storm_keeps_clean_tab_open_once_with_final_content() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "# Original\n");
        let mut shell = shell_from_workspace(&workspace);
        let doc = path(&shell, "doc.md");
        shell.select_markdown_path(doc.clone());
        workspace.write("doc.md", "# Final\n");

        apply_events(
            &mut shell,
            &workspace,
            [
                WorkspaceEvent::Upsert(doc.clone()),
                WorkspaceEvent::Upsert(doc.clone()),
                WorkspaceEvent::Remove(doc.clone()),
                WorkspaceEvent::Upsert(doc.clone()),
            ],
        );

        assert_eq!(
            shell
                .documents
                .iter()
                .filter(|document| document.path == doc)
                .count(),
            1
        );
        assert_eq!(shell.editor.tabs.len(), 1);
        assert_eq!(shell.editor.tab(&doc).unwrap().buffer, "# Final\n");
        assert_eq!(
            shell.selected_document().unwrap().source_content.as_deref(),
            Some("# Final\n")
        );
    }

    #[test]
    fn dirty_tab_survives_real_external_delete_with_buffer_intact() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "# Original\n");
        let mut shell = shell_from_workspace(&workspace);
        let doc = path(&shell, "doc.md");
        shell.select_markdown_path(doc.clone());
        shell.update_active_editor_buffer(String::from("# Local\n"));
        fs::remove_file(&doc).unwrap();

        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Remove(doc.clone())],
        );

        let tab = shell.editor.tab(&doc).unwrap();
        assert_eq!(tab.buffer, "# Local\n");
        assert!(tab.dirty);
        assert!(tab.external_conflict.is_some());
    }

    #[test]
    fn save_then_immediate_edit_is_not_overwritten_by_save_watcher_update() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "# Original\n");
        let mut shell = shell_from_workspace(&workspace);
        let doc = path(&shell, "doc.md");
        shell.select_markdown_path(doc.clone());
        shell.update_active_editor_buffer(String::from("# A\n"));
        save_markdown_file(&doc, "# A\n").unwrap();
        shell.editor_save_completed(&doc, "# A\n", Ok(()));
        shell.update_active_editor_buffer(String::from("# B\n"));

        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(doc.clone())],
        );

        let tab = shell.editor.tab(&doc).unwrap();
        assert_eq!(tab.buffer, "# B\n");
        assert!(tab.dirty);
        assert!(tab.external_conflict.is_none());
    }

    #[test]
    fn a_dirty_b_a_preserves_a_buffer() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "A\n");
        workspace.write("b.md", "B\n");
        let mut shell = shell_from_workspace(&workspace);
        let a = path(&shell, "a.md");
        let b = path(&shell, "b.md");

        shell.select_markdown_path(a.clone());
        shell.update_active_editor_buffer(String::from("A local\n"));
        shell.select_markdown_path(b);
        shell.activate_editor_tab(a.clone());

        assert_eq!(shell.editor.tab(&a).unwrap().buffer, "A local\n");
        assert!(shell.editor.tab(&a).unwrap().dirty);
    }

    #[test]
    fn dirty_tabs_keep_independent_buffers_and_saving_one_does_not_change_other() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "A\n");
        workspace.write("b.md", "B\n");
        let mut shell = shell_from_workspace(&workspace);
        let a = path(&shell, "a.md");
        let b = path(&shell, "b.md");

        shell.select_markdown_path(a.clone());
        shell.update_active_editor_buffer(String::from("A local\n"));
        shell.select_markdown_path(b.clone());
        shell.update_active_editor_buffer(String::from("B local\n"));
        shell.activate_editor_tab(a.clone());
        save_markdown_file(&a, "A local\n").unwrap();
        shell.editor_save_completed(&a, "A local\n", Ok(()));

        assert!(!shell.editor.tab(&a).unwrap().dirty);
        assert_eq!(shell.editor.tab(&b).unwrap().buffer, "B local\n");
        assert!(shell.editor.tab(&b).unwrap().dirty);
    }

    #[test]
    fn clean_external_update_updates_only_target_tab() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "A\n");
        workspace.write("b.md", "B\n");
        let mut shell = shell_from_workspace(&workspace);
        let a = path(&shell, "a.md");
        let b = path(&shell, "b.md");
        shell.select_markdown_path(a.clone());
        shell.select_markdown_path(b.clone());
        workspace.write("b.md", "B external\n");

        apply_events(&mut shell, &workspace, [WorkspaceEvent::Upsert(b.clone())]);

        assert_eq!(shell.editor.tab(&a).unwrap().buffer, "A\n");
        assert_eq!(shell.editor.tab(&b).unwrap().buffer, "B external\n");
        assert!(!shell.editor.tab(&b).unwrap().dirty);
    }

    #[test]
    fn dirty_external_update_conflicts_only_target_without_overwrite() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "A\n");
        workspace.write("b.md", "B\n");
        let mut shell = shell_from_workspace(&workspace);
        let a = path(&shell, "a.md");
        let b = path(&shell, "b.md");
        shell.select_markdown_path(a.clone());
        shell.update_active_editor_buffer(String::from("A local\n"));
        shell.select_markdown_path(b.clone());
        workspace.write("a.md", "A external\n");

        apply_events(&mut shell, &workspace, [WorkspaceEvent::Upsert(a.clone())]);

        assert_eq!(shell.editor.tab(&a).unwrap().buffer, "A local\n");
        assert!(shell.editor.tab(&a).unwrap().external_conflict.is_some());
        assert_eq!(shell.editor.tab(&b).unwrap().buffer, "B\n");
        assert!(shell.editor.tab(&b).unwrap().external_conflict.is_none());
    }

    #[test]
    fn keep_local_conflict_does_not_reappear_on_unrelated_event() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "A\n");
        workspace.write("b.md", "B\n");
        let mut shell = shell_from_workspace(&workspace);
        let a = path(&shell, "a.md");
        let b = path(&shell, "b.md");
        shell.select_markdown_path(a.clone());
        shell.update_active_editor_buffer(String::from("A local\n"));
        shell.select_markdown_path(b.clone());
        workspace.write("a.md", "A external\n");
        apply_events(&mut shell, &workspace, [WorkspaceEvent::Upsert(a.clone())]);

        shell.activate_editor_tab(a.clone());
        shell.keep_local_editor_changes();
        workspace.write("b.md", "B external\n");
        apply_events(&mut shell, &workspace, [WorkspaceEvent::Upsert(b)]);

        assert!(shell.editor.tab(&a).unwrap().external_conflict.is_none());
        assert_eq!(shell.editor.tab(&a).unwrap().buffer, "A local\n");
    }

    #[test]
    fn late_workspace_update_cannot_restore_older_content_after_newer_update() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "A\n");
        let mut shell = shell_from_workspace(&workspace);
        let doc = path(&shell, "doc.md");
        shell.select_markdown_path(doc.clone());
        shell.documents[0].metadata.modified = Some(UNIX_EPOCH);

        let mut older = workspace_update_with_content(&workspace, "doc.md", "A1\n");
        older.upserts[0].metadata.modified = Some(UNIX_EPOCH + Duration::from_secs(1));
        let mut newer = workspace_update_with_content(&workspace, "doc.md", "A2\n");
        newer.upserts[0].metadata.modified = Some(UNIX_EPOCH + Duration::from_secs(2));

        shell.workspace_update_completed(newer);
        shell.workspace_update_completed(older);

        assert_eq!(
            shell.selected_document().unwrap().source_content.as_deref(),
            Some("A2\n")
        );
        assert_eq!(shell.editor.tab(&doc).unwrap().buffer, "A2\n");
    }

    #[test]
    fn workspace_invalid_yaml_warning_is_recoverable() {
        let workspace = TempWorkspace::new();
        workspace.write("broken.md", "---\ntype: project\n---\n# Broken\n");
        let mut shell = shell_from_workspace(&workspace);

        workspace.write("broken.md", "---\ntype: [broken\n---\n# Broken\n");
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(workspace.path().join("broken.md"))],
        );
        assert_eq!(shell.documents[0].warnings.len(), 1);

        workspace.write(
            "broken.md",
            "---\ntype: project\nstatus: ok\n---\n# Broken\n",
        );
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(workspace.path().join("broken.md"))],
        );
        assert!(shell.documents[0].warnings.is_empty());
    }

    #[test]
    fn workspace_ignores_non_markdown_and_technical_directories() {
        let workspace = TempWorkspace::new();
        workspace.write("kept.md", "# Kept\n");
        let mut shell = shell_from_workspace(&workspace);
        workspace.write("notes.txt", "ignored");
        workspace.write(".git/ignored.md", "# Ignored\n");
        workspace.write("target/ignored.md", "# Ignored\n");
        workspace.write("node_modules/ignored.md", "# Ignored\n");

        apply_events(
            &mut shell,
            &workspace,
            [
                WorkspaceEvent::Upsert(workspace.path().join("notes.txt")),
                WorkspaceEvent::Upsert(workspace.path().join(".git/ignored.md")),
                WorkspaceEvent::Upsert(workspace.path().join("target/ignored.md")),
                WorkspaceEvent::Upsert(workspace.path().join("node_modules/ignored.md")),
            ],
        );

        assert_eq!(shell.documents.len(), 1);
        assert_eq!(shell.documents[0].relative_path, PathBuf::from("kept.md"));
    }

    #[test]
    fn workspace_unicode_path_and_quick_saves_converge() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "ações/visão.md",
            "---\ntype: project\nstatus: one\n---\n# Visão\n",
        );
        let mut shell = shell_from_workspace(&workspace);

        workspace.write(
            "ações/visão.md",
            "---\ntype: project\nstatus: final\n---\n# Visão\n",
        );
        let path = workspace.path().join("ações/visão.md");
        apply_events(
            &mut shell,
            &workspace,
            [
                WorkspaceEvent::Upsert(path.clone()),
                WorkspaceEvent::Upsert(path.clone()),
                WorkspaceEvent::Upsert(path),
            ],
        );

        assert_eq!(
            shell.documents[0].properties.get("status"),
            Some(&PropertyValue::String(String::from("final")))
        );
        assert_eq!(
            shell.documents[0].relative_path,
            PathBuf::from("ações/visão.md")
        );
    }

    #[test]
    fn workspace_deleted_during_processing_does_not_panic() {
        let workspace = TempWorkspace::new();
        workspace.write("gone.md", "# Gone\n");
        let mut shell = shell_from_workspace(&workspace);
        let path = workspace.path().join("gone.md");
        fs::remove_file(&path).unwrap();

        apply_events(&mut shell, &workspace, [WorkspaceEvent::Upsert(path)]);

        assert!(shell.documents.is_empty());
    }

    fn shell_with_documents(documents: Vec<Document>) -> super::ShellModel {
        let mut shell = mock_shell();
        let mut counts = BTreeMap::<String, usize>::new();
        for document in &documents {
            *counts.entry(document.collection_id.clone()).or_default() += 1;
        }

        let collections = counts
            .into_iter()
            .map(|(id, document_count)| Collection {
                display_name: match id.as_str() {
                    "project" => String::from("Projects"),
                    "person" => String::from("People"),
                    _ => String::from("Documents"),
                },
                id,
                document_count,
            })
            .collect::<Vec<_>>();

        shell.scan_completed(ScanResult {
            root: PathBuf::from("/workspace"),
            documents,
            collections,
            directories: Vec::new(),
            errors: Vec::new(),
            duration: Duration::from_secs(0),
        });
        shell
    }

    fn document<const N: usize>(
        relative_path: &str,
        title: &str,
        collection_id: &str,
        properties: [(&str, PropertyValue); N],
    ) -> Document {
        document_with_metadata(relative_path, title, collection_id, properties, None)
    }

    fn document_with_source<const N: usize>(
        relative_path: &str,
        title: &str,
        collection_id: &str,
        properties: [(&str, PropertyValue); N],
        source_content: &str,
    ) -> Document {
        let mut document = document(relative_path, title, collection_id, properties);
        document.source_content = Some(source_content.to_owned());
        document.markdown_content = source_content.to_owned();
        document
    }

    fn document_with_metadata<const N: usize>(
        relative_path: &str,
        title: &str,
        collection_id: &str,
        properties: [(&str, PropertyValue); N],
        file_size: Option<u64>,
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
                file_size,
                modified: None,
            },
            title: title.to_owned(),
            source_content: Some(String::new()),
            markdown_content: String::new(),
            properties: properties
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
            document_type: match collection_id {
                "documents" => None,
                value => Some(value.to_owned()),
            },
            collection_id: collection_id.to_owned(),
            warnings: Vec::new(),
        }
    }

    fn property(shell: &super::ShellModel, label: &str) -> Option<super::InspectorField> {
        let InspectorModel::Document(inspector) = shell.document_inspector() else {
            return None;
        };
        inspector
            .properties
            .into_iter()
            .find(|field| field.label == label)
    }

    fn property_value(shell: &super::ShellModel, label: &str) -> InspectorValue {
        property(shell, label).unwrap().value
    }

    fn path(shell: &super::ShellModel, relative_path: &str) -> PathBuf {
        shell
            .documents
            .iter()
            .find(|document| document.relative_path == Path::new(relative_path))
            .map(|document| document.path.clone())
            .unwrap_or_else(|| panic!("missing document {relative_path}"))
    }

    fn metadata_value(shell: &super::ShellModel, label: &str) -> InspectorValue {
        let InspectorModel::Document(inspector) = shell.document_inspector() else {
            panic!("expected selected document inspector");
        };
        inspector
            .metadata
            .into_iter()
            .find(|field| field.label == label)
            .unwrap()
            .value
    }

    fn string(value: &str) -> PropertyValue {
        PropertyValue::String(value.to_owned())
    }

    fn number(value: &str) -> PropertyValue {
        PropertyValue::Number(value.to_owned())
    }

    fn bool_value(value: bool) -> PropertyValue {
        PropertyValue::Bool(value)
    }

    fn shell_from_workspace(workspace: &TempWorkspace) -> super::ShellModel {
        let mut shell = mock_shell();
        shell.workspace_selected(Some(workspace.path().to_path_buf()));
        shell.scan_completed(scan_workspace(workspace.path()).unwrap());
        shell
    }

    fn apply_events<const N: usize>(
        shell: &mut super::ShellModel,
        workspace: &TempWorkspace,
        events: [WorkspaceEvent; N],
    ) {
        let update = workspace_update_from_events(workspace.path(), &events).unwrap();
        shell.workspace_update_completed(update);
    }

    fn workspace_update_with_content(
        workspace: &TempWorkspace,
        relative_path: &str,
        content: &str,
    ) -> crate::WorkspaceUpdate {
        workspace.write(relative_path, content);
        workspace_update_from_events(
            workspace.path(),
            &[WorkspaceEvent::Upsert(workspace.path().join(relative_path))],
        )
        .unwrap()
    }

    struct TempWorkspace {
        path: PathBuf,
    }

    impl TempWorkspace {
        fn new() -> Self {
            let mut path = std::env::temp_dir();
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            path.push(format!("flokin-md-model-{}-{unique}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn write(&self, relative_path: &str, content: &str) {
            let path = self.path.join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, content).unwrap();
        }
    }

    impl Drop for TempWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
