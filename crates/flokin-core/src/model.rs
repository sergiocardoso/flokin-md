use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf, MAIN_SEPARATOR},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    build_bulk_edit_plan, build_health, load_explicit_schema, relation_display_property,
    search_documents, BulkEditOperation, BulkEditPlan, BulkEditSelection, BulkEditValue,
    Collection, ContextSection, DatabaseHealth, Document, ExplicitSchemaState, HealthIssue,
    HistoryState, MutationHistoryEntry, PropertyValue, Relation, RelationIndex, RelationStatus,
    ScanError, ScanResult, SchemaCatalog, SchemaType, SearchQuery, SearchState, SortDirection,
    SqlCatalog, SqlError, SqlQueryResult, SqlWritePlan, TableSort, WorkspaceUpdate,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    Explorer,
    Context,
    Relations,
    Links,
    Tags,
    Calendar,
    Favorites,
    History,
    Health,
    Terminal,
    Settings,
}

impl Activity {
    pub const ALL: [Self; 11] = [
        Self::Explorer,
        Self::Context,
        Self::Relations,
        Self::Links,
        Self::Tags,
        Self::Calendar,
        Self::Favorites,
        Self::History,
        Self::Health,
        Self::Terminal,
        Self::Settings,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Explorer => "Explorer",
            Self::Context => "Context",
            Self::Relations => "Relations",
            Self::Links => "Links",
            Self::Tags => "Tags",
            Self::Calendar => "Calendar",
            Self::Favorites => "Favorites",
            Self::History => "History",
            Self::Health => "Health",
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
    pub semantic_kind: Option<SemanticKind>,
    pub path: PathBuf,
    pub children: Vec<ExplorerNode>,
    pub expanded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerNodeKind {
    Folder,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticKind {
    Agent,
    AgentInstructions,
    Skill,
    Spec,
    Ice,
    Context,
    Prompt,
    Rules,
    Memory,
    Mcp,
}

impl ExplorerNode {
    pub fn folder(id: usize, name: impl Into<String>, path: PathBuf, children: Vec<Self>) -> Self {
        let name = name.into();
        let semantic_kind = classify_semantic_entry(&name, ExplorerNodeKind::Folder, &children);
        Self {
            id: ExplorerNodeId(id),
            name,
            kind: ExplorerNodeKind::Folder,
            semantic_kind,
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
        let name = name.into();
        Self {
            id: ExplorerNodeId(id),
            semantic_kind: classify_semantic_entry(&name, ExplorerNodeKind::File, &[]),
            name,
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

pub fn classify_semantic_entry(
    name: &str,
    kind: ExplorerNodeKind,
    children: &[ExplorerNode],
) -> Option<SemanticKind> {
    match kind {
        ExplorerNodeKind::Folder => classify_semantic_folder(name, children),
        ExplorerNodeKind::File => classify_semantic_file(name),
    }
}

fn classify_semantic_folder(name: &str, children: &[ExplorerNode]) -> Option<SemanticKind> {
    let normalized = normalized_entry_name(name);
    let by_name = match normalized.as_str() {
        "agent" | "agents" => Some(SemanticKind::Agent),
        "skill" | "skills" => Some(SemanticKind::Skill),
        "spec" | "specs" | "sdd" => Some(SemanticKind::Spec),
        "ice" => Some(SemanticKind::Ice),
        "context" | "contexts" => Some(SemanticKind::Context),
        "prompt" | "prompts" => Some(SemanticKind::Prompt),
        "rules" | "instructions" => Some(SemanticKind::Rules),
        "memory" | "memories" => Some(SemanticKind::Memory),
        ".mcp" | "mcp" => Some(SemanticKind::Mcp),
        _ => None,
    };

    by_name.or_else(|| {
        children
            .iter()
            .any(|child| {
                matches!(child.kind, ExplorerNodeKind::File)
                    && child.name.eq_ignore_ascii_case("SKILL.md")
            })
            .then_some(SemanticKind::Skill)
    })
}

fn classify_semantic_file(name: &str) -> Option<SemanticKind> {
    let normalized = normalized_entry_name(name);
    match normalized.as_str() {
        "skill.md" => Some(SemanticKind::Skill),
        "spec.md" | "sdd_template.md" => Some(SemanticKind::Spec),
        "ice.md" | "ice_template.md" => Some(SemanticKind::Ice),
        "context.md" => Some(SemanticKind::Context),
        "prompt.md" => Some(SemanticKind::Prompt),
        "rules.md" | "instructions.md" => Some(SemanticKind::Rules),
        "memory.md" => Some(SemanticKind::Memory),
        "mcp.json" => Some(SemanticKind::Mcp),
        "agents.md" => Some(SemanticKind::AgentInstructions),
        _ if normalized.ends_with(".spec.md") => Some(SemanticKind::Spec),
        _ if normalized.ends_with(".ice.md") => Some(SemanticKind::Ice),
        _ if normalized.starts_with("sdd-")
            && (normalized.ends_with(".md") || normalized.ends_with(".markdown")) =>
        {
            Some(SemanticKind::Spec)
        }
        _ => None,
    }
}

fn normalized_entry_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
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
    pub mode: SqlExplorerMode,
    pub catalog: Option<SqlCatalog>,
    pub result: Option<SqlQueryResult>,
    pub write_plan: Option<SqlWritePlan>,
    pub error: Option<String>,
    pub last_result: Option<String>,
    pub running: bool,
    pub stale: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SqlExplorerMode {
    #[default]
    Query,
    Update,
}

impl SqlExplorerState {
    pub fn closed() -> Self {
        Self {
            open: false,
            query: String::new(),
            mode: SqlExplorerMode::Query,
            catalog: None,
            result: None,
            write_plan: None,
            error: None,
            last_result: None,
            running: false,
            stale: false,
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
pub struct HealthIssueInspector {
    pub issue: HealthIssue,
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
    HealthIssue(HealthIssueInspector),
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
    pub kind: EditorTabKind,
    pub buffer: String,
    pub saved_content: String,
    pub dirty: bool,
    pub view_mode: EditorViewMode,
    pub split_ratio: u16,
    pub external_conflict: Option<EditorExternalConflict>,
    pub ignored_external_conflict: Option<EditorExternalConflict>,
    pub save_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTabKind {
    Markdown,
    Schema,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorViewMode {
    #[default]
    Edit,
    Split,
    Preview,
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
    pub schema_catalog: SchemaCatalog,
    pub health: DatabaseHealth,
    pub health_filter: HealthFilter,
    pub health_query: String,
    pub selected_health_issue_id: Option<String>,
    pub context_section: ContextSection,
    pub selected_context_artifact: Option<PathBuf>,
    pub workspace_errors: Vec<ScanError>,
    pub selected_schema_field: Option<(String, String)>,
    pub collection_panel: CollectionPanel,
    pub bulk_edit: BulkEditState,
    pub editor: EditorState,
    pub sql_explorer: SqlExplorerState,
    pub history: HistoryState,
    pub collapsed_sql_tables: BTreeSet<String>,
    pub filters: Vec<FilterCount>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HealthFilter {
    #[default]
    All,
    Errors,
    Warnings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CollectionPanel {
    #[default]
    Data,
    Schema,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkEditState {
    pub selected_paths: BTreeSet<PathBuf>,
    pub editor_open: bool,
    pub step: BulkEditStep,
    pub operation_kind: BulkEditOperationKind,
    pub property: String,
    pub new_property: String,
    pub value_type: BulkEditValueType,
    pub value: String,
    pub bool_value: bool,
    pub plan: Option<BulkEditPlan>,
    pub error: Option<String>,
    pub last_result: Option<String>,
    pub stale: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BulkEditStep {
    #[default]
    Configure,
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BulkEditOperationKind {
    #[default]
    Set,
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BulkEditValueType {
    #[default]
    String,
    Integer,
    Float,
    Boolean,
    Null,
    Relation,
}

impl Default for BulkEditState {
    fn default() -> Self {
        Self {
            selected_paths: BTreeSet::new(),
            editor_open: false,
            step: BulkEditStep::Configure,
            operation_kind: BulkEditOperationKind::Set,
            property: String::new(),
            new_property: String::new(),
            value_type: BulkEditValueType::String,
            value: String::new(),
            bool_value: true,
            plan: None,
            error: None,
            last_result: None,
            stale: false,
        }
    }
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
            self.clear_workspace_state();
            self.scan_state = ScanState::Scanning;
        }
    }

    pub fn close_workspace(&mut self) {
        self.current_workspace = None;
        self.clear_workspace_state();
        self.active_activity = Activity::Explorer;
        self.scan_state = ScanState::Idle;
    }

    fn clear_workspace_state(&mut self) {
        self.explorer.clear();
        self.documents.clear();
        self.collections.clear();
        self.selected_document_path = None;
        self.selected_collection = None;
        self.collection_table_sort = None;
        self.search = SearchState::closed();
        self.relation_index = RelationIndex::default();
        self.schema_catalog = SchemaCatalog::default();
        self.health = DatabaseHealth::default();
        self.health_filter = HealthFilter::All;
        self.health_query.clear();
        self.selected_health_issue_id = None;
        self.context_section = ContextSection::Overview;
        self.selected_context_artifact = None;
        self.workspace_errors.clear();
        self.selected_schema_field = None;
        self.collection_panel = CollectionPanel::Data;
        self.bulk_edit = BulkEditState::default();
        self.editor = EditorState::default();
        self.sql_explorer = SqlExplorerState::closed();
        self.history = HistoryState::default();
        self.collapsed_sql_tables.clear();
        self.filters.clear();
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

    pub fn select_context_section(&mut self, section: ContextSection) {
        self.context_section = section;
        self.selected_context_artifact = None;
    }

    pub fn select_context_artifact(&mut self, path: PathBuf) -> bool {
        if self.documents.iter().any(|document| document.path == path) {
            self.selected_context_artifact = Some(path);
            true
        } else {
            self.sync_context_selection_with_documents();
            false
        }
    }

    pub fn history_loaded(&mut self, result: Result<Vec<MutationHistoryEntry>, String>) {
        match result {
            Ok(entries) => {
                self.history.entries = entries;
                if let Some(selected) = self.history.selected_entry_id.as_ref() {
                    if !self
                        .history
                        .entries
                        .iter()
                        .any(|entry| &entry.id == selected)
                    {
                        self.history.selected_entry_id = None;
                    }
                }
                if self.history.selected_entry_id.is_none() {
                    self.history.selected_entry_id =
                        self.history.entries.first().map(|entry| entry.id.clone());
                }
                self.history.error = None;
                self.history.undo_plan = None;
                self.history.clear_confirm = false;
            }
            Err(error) => {
                self.history.entries.clear();
                self.history.selected_entry_id = None;
                self.history.error = Some(error);
                self.history.undo_plan = None;
            }
        }
    }

    pub fn select_history_entry(&mut self, id: String) -> bool {
        if self.history.entries.iter().any(|entry| entry.id == id) {
            self.history.selected_entry_id = Some(id);
            self.history.undo_plan = None;
            self.history.error = None;
            true
        } else {
            false
        }
    }

    pub fn selected_history_entry(&self) -> Option<&MutationHistoryEntry> {
        self.history.selected_entry()
    }

    pub fn undo_preview_completed(&mut self, result: Result<BulkEditPlan, String>) {
        match result {
            Ok(plan) => {
                self.history.undo_plan = Some(plan);
                self.history.error = None;
                self.history.last_result = None;
            }
            Err(error) => {
                self.history.undo_plan = None;
                self.history.error = Some(error);
            }
        }
    }

    pub fn cancel_undo_preview(&mut self) {
        self.history.undo_plan = None;
        self.history.error = None;
    }

    pub fn undo_apply_completed(&mut self, result: Result<usize, String>) {
        match result {
            Ok(count) => {
                self.history.undo_plan = None;
                self.history.error = None;
                self.history.last_result = Some(if count == 1 {
                    String::from("1 arquivo restaurado.")
                } else {
                    format!("{count} arquivos restaurados.")
                });
            }
            Err(error) => {
                self.history.error = Some(error);
            }
        }
    }

    pub fn request_clear_history(&mut self) {
        self.history.clear_confirm = true;
        self.history.error = None;
    }

    pub fn cancel_clear_history(&mut self) {
        self.history.clear_confirm = false;
    }

    pub fn clear_history_completed(&mut self, result: Result<(), String>) {
        self.history.clear_confirm = false;
        match result {
            Ok(()) => {
                self.history.entries.clear();
                self.history.selected_entry_id = None;
                self.history.undo_plan = None;
                self.history.error = None;
                self.history.last_result = Some(String::from("Histórico do workspace limpo."));
            }
            Err(error) => self.history.error = Some(error),
        }
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
            self.selected_schema_field = None;
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
            self.selected_schema_field = None;
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
            self.selected_schema_field = None;
            self.sql_explorer.open = false;
            true
        } else {
            false
        }
    }

    fn sync_context_selection_with_documents(&mut self) {
        if let Some(path) = self.selected_context_artifact.as_ref() {
            if !self.documents.iter().any(|document| &document.path == path) {
                self.selected_context_artifact = None;
            }
        }
    }

    pub fn select_collection(&mut self, collection_id: String) {
        if self
            .collections
            .iter()
            .any(|collection| collection.id == collection_id)
        {
            if self.selected_collection.as_deref() != Some(collection_id.as_str()) {
                self.bulk_edit = BulkEditState::default();
            }
            self.selected_collection = Some(collection_id);
            self.selected_document_path = None;
            self.editor.active_path = None;
            self.collection_table_sort = None;
            self.selected_schema_field = None;
        }
    }

    pub fn select_collection_panel(&mut self, panel: CollectionPanel) {
        self.collection_panel = panel;
        self.selected_schema_field = None;
        if panel != CollectionPanel::Data {
            self.close_bulk_edit();
        }
    }

    pub fn select_schema_field(&mut self, collection_id: String, field_name: String) -> bool {
        if self
            .schema_catalog
            .collection(&collection_id)
            .is_some_and(|schema| schema.fields.iter().any(|field| field.name == field_name))
        {
            self.selected_schema_field = Some((collection_id, field_name));
            true
        } else {
            false
        }
    }

    pub fn toggle_bulk_selection(&mut self, path: PathBuf) -> bool {
        let Some(collection_id) = self.selected_collection.as_deref() else {
            return false;
        };
        if !self
            .documents
            .iter()
            .any(|document| document.path == path && document.collection_id == collection_id)
        {
            return false;
        }
        if !self.bulk_edit.selected_paths.remove(&path) {
            self.bulk_edit.selected_paths.insert(path);
        }
        self.bulk_edit.plan = None;
        self.bulk_edit.error = None;
        self.bulk_edit.last_result = None;
        self.bulk_edit.stale = false;
        true
    }

    pub fn set_bulk_selection_for_current_collection(&mut self, select_all: bool) {
        self.bulk_edit.selected_paths.clear();
        if select_all {
            if let Some(collection_id) = self.selected_collection.as_deref() {
                self.bulk_edit.selected_paths.extend(
                    self.documents
                        .iter()
                        .filter(|document| document.collection_id == collection_id)
                        .map(|document| document.path.clone()),
                );
            }
        }
        self.bulk_edit.plan = None;
        self.bulk_edit.error = None;
        self.bulk_edit.last_result = None;
        self.bulk_edit.stale = false;
    }

    pub fn clear_bulk_selection(&mut self) {
        self.bulk_edit.selected_paths.clear();
        self.close_bulk_edit();
    }

    pub fn open_bulk_edit(&mut self) -> bool {
        if self.bulk_edit.selected_paths.is_empty() || self.selected_collection.is_none() {
            return false;
        }
        if self.bulk_edit.property.is_empty() {
            self.bulk_edit.property = self
                .bulk_property_options()
                .into_iter()
                .next()
                .unwrap_or_default();
        }
        self.bulk_edit.editor_open = true;
        self.bulk_edit.step = BulkEditStep::Configure;
        self.bulk_edit.plan = None;
        self.bulk_edit.error = None;
        self.bulk_edit.last_result = None;
        self.bulk_edit.stale = false;
        true
    }

    pub fn close_bulk_edit(&mut self) {
        self.bulk_edit.editor_open = false;
        self.bulk_edit.step = BulkEditStep::Configure;
        self.bulk_edit.plan = None;
        self.bulk_edit.error = None;
        self.bulk_edit.stale = false;
    }

    pub fn set_bulk_operation_kind(&mut self, kind: BulkEditOperationKind) {
        self.bulk_edit.operation_kind = kind;
        self.bulk_edit.plan = None;
        self.bulk_edit.error = None;
    }

    pub fn set_bulk_property(&mut self, property: String) {
        self.bulk_edit.property = property;
        self.bulk_edit.new_property.clear();
        self.bulk_edit.plan = None;
        self.bulk_edit.error = None;
        self.infer_bulk_value_type();
    }

    pub fn set_bulk_new_property(&mut self, property: String) {
        self.bulk_edit.new_property = property;
        self.bulk_edit.property.clear();
        self.bulk_edit.plan = None;
        self.bulk_edit.error = None;
        self.infer_bulk_value_type();
    }

    pub fn set_bulk_value_type(&mut self, value_type: BulkEditValueType) {
        self.bulk_edit.value_type = value_type;
        self.bulk_edit.plan = None;
        self.bulk_edit.error = None;
    }

    pub fn set_bulk_value(&mut self, value: String) {
        self.bulk_edit.value = value;
        self.bulk_edit.plan = None;
        self.bulk_edit.error = None;
    }

    pub fn set_bulk_bool_value(&mut self, value: bool) {
        self.bulk_edit.bool_value = value;
        self.bulk_edit.plan = None;
        self.bulk_edit.error = None;
    }

    pub fn build_bulk_preview(&mut self) -> bool {
        let Some(collection_id) = self.selected_collection.clone() else {
            self.bulk_edit.error = Some(String::from("Nenhuma Collection selecionada."));
            return false;
        };
        let Some(operation) = self.current_bulk_operation() else {
            return false;
        };
        let selection = BulkEditSelection::new(
            collection_id,
            self.bulk_edit.selected_paths.iter().cloned().collect(),
        );
        match build_bulk_edit_plan(
            selection,
            operation,
            &self.documents,
            &self.editor,
            &self.schema_catalog,
        ) {
            Ok(plan) => {
                self.bulk_edit.plan = Some(plan);
                self.bulk_edit.step = BulkEditStep::Review;
                self.bulk_edit.error = None;
                self.bulk_edit.stale = false;
                true
            }
            Err(error) => {
                self.bulk_edit.error = Some(error);
                self.bulk_edit.plan = None;
                false
            }
        }
    }

    pub fn return_to_bulk_configuration(&mut self) {
        self.bulk_edit.step = BulkEditStep::Configure;
        self.bulk_edit.plan = None;
        self.bulk_edit.error = None;
        self.bulk_edit.stale = false;
    }

    pub fn bulk_apply_completed(&mut self, result: Result<usize, String>) {
        match result {
            Ok(count) => {
                self.bulk_edit.last_result =
                    Some(format!("{count} arquivos atualizados com sucesso."));
                self.bulk_edit.selected_paths.clear();
                self.bulk_edit.editor_open = false;
                self.bulk_edit.plan = None;
                self.bulk_edit.error = None;
                self.bulk_edit.stale = false;
            }
            Err(error) => {
                self.bulk_edit.error = Some(error);
                self.bulk_edit.stale = true;
            }
        }
    }

    pub fn mark_bulk_preview_stale_for_paths(&mut self, changed_paths: &[PathBuf]) {
        let Some(plan) = self.bulk_edit.plan.as_ref() else {
            return;
        };
        if plan
            .changes
            .iter()
            .any(|change| changed_paths.iter().any(|path| path == &change.path))
        {
            self.bulk_edit.stale = true;
        }
    }

    pub fn mark_sql_preview_stale_for_paths(&mut self, changed_paths: &[PathBuf]) {
        if self.sql_explorer.write_plan.is_none() {
            return;
        };
        if !changed_paths.is_empty() {
            self.sql_explorer.stale = true;
        }
    }

    pub fn bulk_property_options(&self) -> Vec<String> {
        let Some(collection_id) = self.selected_collection.as_deref() else {
            return Vec::new();
        };
        crate::selectable_properties(
            self.schema_catalog.collection(collection_id),
            &self.collection_documents(collection_id),
        )
    }

    fn current_bulk_property(&self) -> Option<String> {
        let property = if self.bulk_edit.new_property.trim().is_empty() {
            self.bulk_edit.property.trim()
        } else {
            self.bulk_edit.new_property.trim()
        };
        if property.is_empty() {
            None
        } else {
            Some(property.to_owned())
        }
    }

    fn current_bulk_operation(&mut self) -> Option<BulkEditOperation> {
        let Some(property) = self.current_bulk_property() else {
            self.bulk_edit.error = Some(String::from("Informe uma propriedade."));
            return None;
        };
        match self.bulk_edit.operation_kind {
            BulkEditOperationKind::Remove => Some(BulkEditOperation::RemoveProperty { property }),
            BulkEditOperationKind::Set => {
                let value = self.current_bulk_value()?;
                Some(BulkEditOperation::SetProperty { property, value })
            }
        }
    }

    fn current_bulk_value(&mut self) -> Option<BulkEditValue> {
        let raw = self.bulk_edit.value.trim();
        match self.bulk_edit.value_type {
            BulkEditValueType::String => Some(BulkEditValue::String(self.bulk_edit.value.clone())),
            BulkEditValueType::Integer => {
                if raw.parse::<i64>().is_ok() {
                    Some(BulkEditValue::Integer(raw.to_owned()))
                } else {
                    self.bulk_edit.error = Some(String::from("Valor precisa ser Integer."));
                    None
                }
            }
            BulkEditValueType::Float => {
                if raw.parse::<f64>().is_ok() {
                    Some(BulkEditValue::Float(raw.to_owned()))
                } else {
                    self.bulk_edit.error = Some(String::from("Valor precisa ser Float."));
                    None
                }
            }
            BulkEditValueType::Boolean => Some(BulkEditValue::Boolean(self.bulk_edit.bool_value)),
            BulkEditValueType::Null => Some(BulkEditValue::Null),
            BulkEditValueType::Relation => {
                if raw.is_empty() {
                    self.bulk_edit.error = Some(String::from("Informe o alvo da relação."));
                    None
                } else {
                    Some(BulkEditValue::Relation(raw.to_owned()))
                }
            }
        }
    }

    fn infer_bulk_value_type(&mut self) {
        let property = if self.bulk_edit.new_property.trim().is_empty() {
            self.bulk_edit.property.trim()
        } else {
            self.bulk_edit.new_property.trim()
        };
        let Some(collection_id) = self.selected_collection.as_deref() else {
            return;
        };
        let Some(field) = self
            .schema_catalog
            .collection(collection_id)
            .and_then(|schema| schema.fields.iter().find(|field| field.name == property))
        else {
            return;
        };
        self.bulk_edit.value_type = match field.field_type {
            SchemaType::Integer => BulkEditValueType::Integer,
            SchemaType::Float => BulkEditValueType::Float,
            SchemaType::Boolean => BulkEditValueType::Boolean,
            SchemaType::Relation => BulkEditValueType::Relation,
            SchemaType::Null => BulkEditValueType::Null,
            _ => BulkEditValueType::String,
        };
    }

    pub fn select_health_filter(&mut self, filter: HealthFilter) {
        self.health_filter = filter;
    }

    pub fn update_health_query(&mut self, query: String) {
        self.health_query = query;
    }

    pub fn select_health_issue(&mut self, issue_id: String) -> bool {
        let Some(issue) = self
            .health
            .issues
            .iter()
            .find(|issue| issue.id == issue_id)
            .cloned()
        else {
            return false;
        };
        self.selected_health_issue_id = Some(issue.id);
        if let Some(path) = issue.document_path {
            self.select_document_without_opening(path);
        }
        true
    }

    pub fn selected_health_issue(&self) -> Option<&HealthIssue> {
        let id = self.selected_health_issue_id.as_ref()?;
        self.health.issues.iter().find(|issue| &issue.id == id)
    }

    pub fn filtered_health_issues(&self) -> Vec<&HealthIssue> {
        let query = self.health_query.trim().to_lowercase();
        self.health
            .issues
            .iter()
            .filter(|issue| match self.health_filter {
                HealthFilter::All => true,
                HealthFilter::Errors => issue.severity == crate::HealthSeverity::Error,
                HealthFilter::Warnings => issue.severity == crate::HealthSeverity::Warning,
            })
            .filter(|issue| {
                if query.is_empty() {
                    return true;
                }
                issue
                    .relative_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains(&query)
                    || issue
                        .property
                        .as_deref()
                        .unwrap_or_default()
                        .to_lowercase()
                        .contains(&query)
                    || issue.message.to_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn open_sql_explorer(&mut self) {
        self.sql_explorer.open = true;
        self.selected_document_path = None;
        self.editor.active_path = None;
        self.selected_collection = None;
        self.collection_table_sort = None;
        self.selected_schema_field = None;
        self.search.close();
    }

    pub fn update_sql_query(&mut self, query: String) {
        if self.sql_explorer.query != query {
            self.sql_explorer.query = query;
            self.sql_explorer.write_plan = None;
            self.sql_explorer.stale = false;
            self.sql_explorer.last_result = None;
        }
    }

    pub fn set_sql_mode(&mut self, mode: SqlExplorerMode) {
        if self.sql_explorer.mode != mode {
            self.sql_explorer.mode = mode;
            self.sql_explorer.result = None;
            self.sql_explorer.write_plan = None;
            self.sql_explorer.error = None;
            self.sql_explorer.last_result = None;
            self.sql_explorer.stale = false;
        }
    }

    pub fn toggle_sql_schema_table(&mut self, table_name: String) {
        if !self.collapsed_sql_tables.remove(&table_name) {
            self.collapsed_sql_tables.insert(table_name);
        }
    }

    pub fn sql_execution_started(&mut self) {
        self.sql_explorer.running = true;
        self.sql_explorer.error = None;
        self.sql_explorer.result = None;
        self.sql_explorer.write_plan = None;
        self.sql_explorer.last_result = None;
        self.sql_explorer.stale = false;
    }

    pub fn sql_execution_completed(&mut self, result: Result<SqlQueryResult, SqlError>) {
        self.sql_explorer.running = false;
        match result {
            Ok(result) => {
                self.sql_explorer.result = Some(result);
                self.sql_explorer.error = None;
                self.sql_explorer.write_plan = None;
            }
            Err(error) => {
                self.sql_explorer.error = Some(error.message);
                self.sql_explorer.result = None;
                self.sql_explorer.write_plan = None;
            }
        }
    }

    pub fn sql_update_preview_completed(&mut self, result: Result<SqlWritePlan, SqlError>) {
        self.sql_explorer.running = false;
        match result {
            Ok(plan) => {
                self.sql_explorer.write_plan = Some(plan);
                self.sql_explorer.result = None;
                self.sql_explorer.error = None;
                self.sql_explorer.last_result = None;
                self.sql_explorer.stale = false;
            }
            Err(error) => {
                self.sql_explorer.error = Some(error.message);
                self.sql_explorer.write_plan = None;
                self.sql_explorer.result = None;
            }
        }
    }

    pub fn sql_update_apply_completed(&mut self, result: Result<usize, String>) {
        self.sql_explorer.running = false;
        match result {
            Ok(count) => {
                self.sql_explorer.last_result = Some(if count == 1 {
                    String::from("1 documento atualizado.")
                } else {
                    format!("{count} documentos atualizados.")
                });
                self.sql_explorer.write_plan = None;
                self.sql_explorer.error = None;
                self.sql_explorer.stale = false;
            }
            Err(error) => {
                self.sql_explorer.error = Some(error);
                self.sql_explorer.stale = true;
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
        self.sql_explorer.write_plan = None;
        self.sql_explorer.stale = false;
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
        self.workspace_errors = result.errors.clone();
        self.relation_index = RelationIndex::build(&self.documents);
        self.rebuild_schema_catalog(None);
        self.rebuild_health();
        self.sync_editor_tabs_with_documents();
        self.sync_schema_editor_tab_with_file(&result.root);
        self.sql_explorer.write_plan = None;
        self.sql_explorer.stale = false;
        if let Some(selected_document_path) = self.selected_document_path.as_ref() {
            if !self
                .documents
                .iter()
                .any(|document| &document.path == selected_document_path)
            {
                self.selected_document_path = None;
            }
        }
        self.sync_context_selection_with_documents();
        if let Some(selected_collection) = self.selected_collection.as_ref() {
            if !self
                .collections
                .iter()
                .any(|collection| &collection.id == selected_collection)
            {
                self.selected_collection = None;
                self.collection_table_sort = None;
                self.selected_schema_field = None;
                self.bulk_edit = BulkEditState::default();
            }
        }
        self.retain_bulk_selection_in_current_collection();
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
        let schema_changed = update.schema_changed;
        let update_root = update.root.clone();

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
        self.workspace_errors = update.errors.clone();
        self.relation_index = RelationIndex::build(&self.documents);
        self.rebuild_schema_catalog(schema_changed.then(|| load_explicit_schema(&update_root)));
        self.rebuild_health();
        if schema_changed {
            self.sync_schema_editor_tab_with_file(&update_root);
        }

        if let Some(selected_collection) = self.selected_collection.as_ref() {
            if !self
                .collections
                .iter()
                .any(|collection| &collection.id == selected_collection)
            {
                self.selected_collection = None;
                self.collection_table_sort = None;
                self.selected_schema_field = None;
                self.bulk_edit = BulkEditState::default();
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
        self.sync_context_selection_with_documents();
        self.retain_bulk_selection_in_current_collection();

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
        self.workspace_errors.push(error.clone());
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
        self.rebuild_health();
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
        self.schema_catalog = SchemaCatalog::default();
        self.health = DatabaseHealth::default();
        self.health_filter = HealthFilter::All;
        self.health_query.clear();
        self.selected_health_issue_id = None;
        self.workspace_errors.clear();
        self.selected_schema_field = None;
        self.collection_panel = CollectionPanel::Data;
        self.bulk_edit = BulkEditState::default();
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
            kind: EditorTabKind::Markdown,
            buffer: content.clone(),
            saved_content: content,
            dirty: false,
            view_mode: EditorViewMode::Edit,
            split_ratio: 500,
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

    pub fn open_schema_tab(&mut self) -> Result<bool, String> {
        let Some(root) = self.current_workspace.as_ref() else {
            return Ok(false);
        };
        let path = crate::schema_path(root);
        if self.editor.tabs.iter().any(|tab| tab.document_path == path) {
            self.editor.active_path = Some(path);
            self.selected_document_path = None;
            self.selected_collection = None;
            self.collection_table_sort = None;
            return Ok(true);
        }

        let content = fs::read_to_string(&path).map_err(|error| {
            format!(
                "Não foi possível abrir {}: {error}",
                crate::SCHEMA_FILE_NAME
            )
        })?;
        self.editor.tabs.push(EditorTab {
            document_path: path.clone(),
            relative_path: PathBuf::from(crate::SCHEMA_FILE_NAME),
            title: String::from(crate::SCHEMA_FILE_NAME),
            kind: EditorTabKind::Schema,
            buffer: content.clone(),
            saved_content: content,
            dirty: false,
            view_mode: EditorViewMode::Edit,
            split_ratio: 500,
            external_conflict: None,
            ignored_external_conflict: None,
            save_error: None,
        });
        self.editor.active_path = Some(path);
        self.selected_document_path = None;
        self.selected_collection = None;
        self.collection_table_sort = None;
        Ok(true)
    }

    pub fn activate_editor_tab(&mut self, path: PathBuf) -> bool {
        if self.editor.tabs.iter().any(|tab| tab.document_path == path) {
            self.editor.active_path = Some(path.clone());
            self.selected_document_path = self
                .editor
                .active_tab()
                .filter(|tab| tab.kind == EditorTabKind::Markdown)
                .map(|tab| tab.document_path.clone());
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

    pub fn set_active_editor_view_mode(&mut self, mode: EditorViewMode) -> bool {
        let Some(tab) = self.editor.active_tab_mut() else {
            return false;
        };
        tab.view_mode = mode;
        true
    }

    pub fn set_active_editor_split_ratio(&mut self, ratio: f32) -> bool {
        let Some(tab) = self.editor.active_tab_mut() else {
            return false;
        };
        tab.split_ratio = (ratio.clamp(0.3, 0.7) * 1000.0).round() as u16;
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

    pub fn selected_collection_schema(&self) -> Option<&crate::CollectionSchema> {
        let id = self.selected_collection.as_ref()?;
        self.schema_catalog.collection(id)
    }

    pub fn selected_schema_field(&self) -> Option<&crate::SchemaField> {
        let (collection_id, field_name) = self.selected_schema_field.as_ref()?;
        self.schema_catalog
            .collection(collection_id)?
            .fields
            .iter()
            .find(|field| &field.name == field_name)
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
        if self.active_activity == Activity::Health {
            if let Some(issue) = self.selected_health_issue() {
                return InspectorModel::HealthIssue(HealthIssueInspector {
                    issue: issue.clone(),
                });
            }
        }

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

    fn rebuild_schema_catalog(&mut self, explicit_schema: Option<ExplicitSchemaState>) {
        let explicit_schema = explicit_schema.unwrap_or_else(|| {
            self.current_workspace
                .as_deref()
                .map(load_explicit_schema)
                .unwrap_or_default()
        });
        self.schema_catalog = SchemaCatalog::build(
            &self.documents,
            &self.collections,
            &self.relation_index,
            explicit_schema,
        );

        if let Some((collection_id, field_name)) = self.selected_schema_field.as_ref() {
            let still_exists = self
                .schema_catalog
                .collection(collection_id)
                .is_some_and(|schema| schema.fields.iter().any(|field| &field.name == field_name));
            if !still_exists {
                self.selected_schema_field = None;
            }
        }
    }

    fn rebuild_health(&mut self) {
        self.health = build_health(
            &self.documents,
            &self.workspace_errors,
            &self.schema_catalog,
            &self.relation_index,
        );
        if let Some(issue_id) = self.selected_health_issue_id.as_ref() {
            if !self.health.issues.iter().any(|issue| &issue.id == issue_id) {
                self.selected_health_issue_id = None;
            }
        }
    }

    fn retain_bulk_selection_in_current_collection(&mut self) {
        let Some(collection_id) = self.selected_collection.as_deref() else {
            self.bulk_edit = BulkEditState::default();
            return;
        };
        let allowed = self
            .documents
            .iter()
            .filter(|document| document.collection_id == collection_id)
            .map(|document| document.path.clone())
            .collect::<BTreeSet<_>>();
        self.bulk_edit
            .selected_paths
            .retain(|path| allowed.contains(path));
        if self.bulk_edit.selected_paths.is_empty() {
            self.close_bulk_edit();
        } else if self.bulk_edit.plan.is_some() {
            self.bulk_edit.stale = true;
        }
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
            self.selected_document_path = self
                .editor
                .active_tab()
                .filter(|tab| tab.kind == EditorTabKind::Markdown)
                .map(|tab| tab.document_path.clone());
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
                self.selected_document_path = self
                    .editor
                    .active_tab()
                    .filter(|tab| tab.kind == EditorTabKind::Markdown)
                    .map(|tab| tab.document_path.clone());
                return;
            }
        }
        self.editor.active_path = self
            .editor
            .tabs
            .first()
            .map(|tab| tab.document_path.clone());
        self.selected_document_path = self
            .editor
            .active_tab()
            .filter(|tab| tab.kind == EditorTabKind::Markdown)
            .map(|tab| tab.document_path.clone());
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

        self.editor.tabs.retain(|tab| match tab.kind {
            EditorTabKind::Markdown => documents.contains_key(&tab.document_path),
            EditorTabKind::Schema => self
                .current_workspace
                .as_ref()
                .map(|root| crate::schema_path(root))
                .is_some_and(|path| path == tab.document_path),
        });

        for tab in &mut self.editor.tabs {
            if tab.kind != EditorTabKind::Markdown {
                continue;
            }
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

    fn sync_schema_editor_tab_with_file(&mut self, root: &Path) {
        let path = crate::schema_path(root);
        let Some(tab) = self
            .editor
            .tabs
            .iter_mut()
            .find(|tab| tab.kind == EditorTabKind::Schema && tab.document_path == path)
        else {
            return;
        };

        match fs::read_to_string(&path) {
            Ok(content) if tab.dirty => {
                if content != tab.saved_content && tab.external_conflict.is_none() {
                    let conflict = EditorExternalConflict::Modified(content);
                    if tab.ignored_external_conflict.as_ref() != Some(&conflict) {
                        tab.external_conflict = Some(conflict);
                    }
                }
            }
            Ok(content) if content != tab.saved_content => {
                tab.buffer = content.clone();
                tab.saved_content = content;
                tab.external_conflict = None;
                tab.ignored_external_conflict = None;
                tab.save_error = None;
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if tab.dirty {
                    let conflict = EditorExternalConflict::Deleted;
                    if tab.ignored_external_conflict.as_ref() != Some(&conflict) {
                        tab.external_conflict = Some(conflict);
                    }
                } else {
                    self.close_editor_tab(&path);
                }
            }
            Err(error) => {
                tab.save_error = Some(format!(
                    "Não foi possível ler {}: {error}",
                    crate::SCHEMA_FILE_NAME
                ));
            }
        }
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
        generate_explicit_schema, mock_shell, save_markdown_file, scan_workspace,
        workspace_update_from_events, Collection, Document, DocumentMetadata, DocumentWarning,
        ExplicitSchemaState, HealthFilter, HealthIssueKind, PropertyValue, ScanResult, SchemaType,
        TableModel, WorkspaceEvent,
    };

    use std::{
        collections::BTreeMap,
        ffi::OsString,
        fs,
        path::{Path, PathBuf},
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use super::{
        classify_semantic_entry, workspace_display, Activity, BulkEditStep, EditorTabKind,
        EditorViewMode, ExplorerNode, ExplorerNodeId, ExplorerNodeKind, InspectorModel,
        InspectorValue, ScanState, SemanticKind,
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
    fn close_workspace_clears_workspace_state_without_deleting_history_storage() {
        let mut shell = mock_shell();
        let path = PathBuf::from("/home/sc/Documents/Knowledge");
        shell.workspace_selected(Some(path));
        shell.open_sql_explorer();
        shell.open_search();

        shell.close_workspace();

        assert_eq!(shell.current_workspace, None);
        assert_eq!(shell.scan_state, ScanState::Idle);
        assert!(shell.documents.is_empty());
        assert!(shell.explorer.is_empty());
        assert!(shell.editor.tabs.is_empty());
        assert!(!shell.sql_explorer.open);
        assert!(shell.history.entries.is_empty());
        assert_eq!(shell.active_activity, Activity::Explorer);
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
    fn classifies_explicit_ai_semantic_folders() {
        assert_eq!(
            classify_semantic_entry("skills", ExplorerNodeKind::Folder, &[]),
            Some(SemanticKind::Skill)
        );
        assert_eq!(
            classify_semantic_entry("specs", ExplorerNodeKind::Folder, &[]),
            Some(SemanticKind::Spec)
        );
        assert_eq!(
            classify_semantic_entry("ice", ExplorerNodeKind::Folder, &[]),
            Some(SemanticKind::Ice)
        );
        assert_eq!(
            classify_semantic_entry("context", ExplorerNodeKind::Folder, &[]),
            Some(SemanticKind::Context)
        );
        assert_eq!(
            classify_semantic_entry("prompts", ExplorerNodeKind::Folder, &[]),
            Some(SemanticKind::Prompt)
        );
        assert_eq!(
            classify_semantic_entry("agents", ExplorerNodeKind::Folder, &[]),
            Some(SemanticKind::Agent)
        );
        assert_eq!(
            classify_semantic_entry("rules", ExplorerNodeKind::Folder, &[]),
            Some(SemanticKind::Rules)
        );
        assert_eq!(
            classify_semantic_entry("memory", ExplorerNodeKind::Folder, &[]),
            Some(SemanticKind::Memory)
        );
        assert_eq!(
            classify_semantic_entry(".mcp", ExplorerNodeKind::Folder, &[]),
            Some(SemanticKind::Mcp)
        );
    }

    #[test]
    fn classifies_direct_skill_marker_folder_without_filesystem_io() {
        let marker = ExplorerNode::file(1, "SKILL.md", PathBuf::from("/workspace/foo/SKILL.md"));

        assert_eq!(
            classify_semantic_entry("deploy-production", ExplorerNodeKind::Folder, &[marker]),
            Some(SemanticKind::Skill)
        );
    }

    #[test]
    fn classifies_explicit_ai_semantic_files() {
        let cases = [
            ("SKILL.md", SemanticKind::Skill),
            ("SPEC.md", SemanticKind::Spec),
            ("foo.spec.md", SemanticKind::Spec),
            ("SDD_TEMPLATE.md", SemanticKind::Spec),
            ("SDD-0001-auth.md", SemanticKind::Spec),
            ("ICE.md", SemanticKind::Ice),
            ("ICE_TEMPLATE.md", SemanticKind::Ice),
            ("foo.ice.md", SemanticKind::Ice),
            ("CONTEXT.md", SemanticKind::Context),
            ("PROMPT.md", SemanticKind::Prompt),
            ("MEMORY.md", SemanticKind::Memory),
            ("RULES.md", SemanticKind::Rules),
            ("INSTRUCTIONS.md", SemanticKind::Rules),
            ("mcp.json", SemanticKind::Mcp),
            ("AGENTS.md", SemanticKind::AgentInstructions),
        ];

        for (name, expected) in cases {
            assert_eq!(
                classify_semantic_entry(name, ExplorerNodeKind::File, &[]),
                Some(expected)
            );
        }
    }

    #[test]
    fn leaves_normal_files_and_folders_without_semantic_kind() {
        assert_eq!(
            classify_semantic_entry("notes.md", ExplorerNodeKind::File, &[]),
            None
        );
        assert_eq!(
            classify_semantic_entry("docs", ExplorerNodeKind::Folder, &[]),
            None
        );
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
    fn schema_updates_when_markdown_properties_change_create_and_remove() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "projects/a.md",
            "---\ntype: project\nstatus: active\npriority: 10\n---\n# A\n",
        );
        workspace.write(
            "projects/b.md",
            "---\ntype: project\nstatus: active\npriority: 11\n---\n# B\n",
        );
        let mut shell = shell_from_workspace(&workspace);

        let project = shell.schema_catalog.collection("project").unwrap();
        assert_eq!(
            schema_field(project, "priority").field_type,
            SchemaType::Integer
        );
        assert!(schema_field(project, "status").required);

        workspace.write(
            "projects/a.md",
            "---\ntype: project\nstatus: active\npriority: high\n---\n# A\n",
        );
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("projects/a.md"),
            )],
        );

        let project = shell.schema_catalog.collection("project").unwrap();
        assert_eq!(
            schema_field(project, "priority").field_type,
            SchemaType::Mixed
        );
        assert_eq!(
            schema_field(project, "priority")
                .observed_types
                .iter()
                .map(|observed| observed.field_type)
                .collect::<Vec<_>>(),
            vec![SchemaType::Integer, SchemaType::String]
        );

        workspace.write(
            "projects/a.md",
            "---\ntype: project\nstatus: active\npriority: 20\n---\n# A\n",
        );
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("projects/a.md"),
            )],
        );

        let project = shell.schema_catalog.collection("project").unwrap();
        assert_eq!(
            schema_field(project, "priority").field_type,
            SchemaType::Integer
        );

        workspace.write(
            "projects/c.md",
            "---\ntype: project\npriority: 12\n---\n# C\n",
        );
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("projects/c.md"),
            )],
        );

        let project = shell.schema_catalog.collection("project").unwrap();
        assert!(!schema_field(project, "status").required);
        assert_eq!(schema_field(project, "status").observed_count, 2);
        assert_eq!(schema_field(project, "status").total_documents, 3);

        fs::remove_file(workspace.path().join("projects/c.md")).unwrap();
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Remove(
                workspace.path().join("projects/c.md"),
            )],
        );

        let project = shell.schema_catalog.collection("project").unwrap();
        assert!(schema_field(project, "status").required);
        assert_eq!(schema_field(project, "status").observed_count, 2);
        assert_eq!(schema_field(project, "status").total_documents, 2);
        assert_eq!(
            schema_field(project, "priority").field_type,
            SchemaType::Integer
        );
    }

    #[test]
    fn editor_view_mode_is_per_tab_and_switching_modes_keeps_buffer_dirty_state() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "# A\n");
        workspace.write("b.md", "# B\n");
        let mut shell = shell_from_workspace(&workspace);
        let a = workspace.path().join("a.md");
        let b = workspace.path().join("b.md");

        assert!(shell.open_editor_tab(a.clone()));
        assert!(shell.update_active_editor_buffer(String::from("# A local\n")));
        assert!(shell.set_active_editor_view_mode(EditorViewMode::Split));
        assert!(shell.set_active_editor_split_ratio(0.65));
        let a_tab = shell.editor.tab(&a).unwrap();
        assert_eq!(a_tab.view_mode, EditorViewMode::Split);
        assert_eq!(a_tab.buffer, "# A local\n");
        assert!(a_tab.dirty);
        assert_eq!(a_tab.split_ratio, 650);

        assert!(shell.open_editor_tab(b.clone()));
        assert_eq!(
            shell.editor.tab(&b).unwrap().view_mode,
            EditorViewMode::Edit
        );
        assert!(!shell.editor.tab(&b).unwrap().dirty);

        assert!(shell.activate_editor_tab(a.clone()));
        assert_eq!(
            shell.editor.active_tab().unwrap().view_mode,
            EditorViewMode::Split
        );
        assert_eq!(shell.editor.active_tab().unwrap().buffer, "# A local\n");
        assert!(shell.editor.active_tab().unwrap().dirty);

        assert!(shell.set_active_editor_view_mode(EditorViewMode::Preview));
        assert_eq!(shell.editor.active_tab().unwrap().buffer, "# A local\n");
        assert!(shell.editor.active_tab().unwrap().dirty);
    }

    #[test]
    fn clean_external_update_refreshes_editor_buffer_for_preview() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "# A\n");
        let mut shell = shell_from_workspace(&workspace);
        let path = workspace.path().join("a.md");
        assert!(shell.open_editor_tab(path.clone()));

        workspace.write("a.md", "# A externally changed\n");
        apply_events(&mut shell, &workspace, [WorkspaceEvent::Upsert(path)]);

        let tab = shell.editor.active_tab().unwrap();
        assert_eq!(tab.buffer, "# A externally changed\n");
        assert!(!tab.dirty);
    }

    #[test]
    fn dirty_external_conflict_keeps_local_buffer_for_preview() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "# A\n");
        let mut shell = shell_from_workspace(&workspace);
        let path = workspace.path().join("a.md");
        assert!(shell.open_editor_tab(path.clone()));
        assert!(shell.update_active_editor_buffer(String::from("# A local preview\n")));

        workspace.write("a.md", "# A external\n");
        apply_events(&mut shell, &workspace, [WorkspaceEvent::Upsert(path)]);

        let tab = shell.editor.active_tab().unwrap();
        assert_eq!(tab.buffer, "# A local preview\n");
        assert!(tab.dirty);
        assert!(tab.external_conflict.is_some());
    }

    #[test]
    fn schema_distinguishes_null_from_missing_through_watcher_updates() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "projects/a.md",
            "---\ntype: project\nstatus: active\n---\n# A\n",
        );
        workspace.write(
            "projects/b.md",
            "---\ntype: project\nstatus: null\n---\n# B\n",
        );
        let mut shell = shell_from_workspace(&workspace);

        let project = shell.schema_catalog.collection("project").unwrap();
        let status = schema_field(project, "status");
        assert!(status.required);
        assert!(status.nullable);
        assert_eq!(status.observed_count, 2);
        assert_eq!(status.null_count, 1);

        workspace.write("projects/b.md", "---\ntype: project\n---\n# B\n");
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("projects/b.md"),
            )],
        );

        let project = shell.schema_catalog.collection("project").unwrap();
        let status = schema_field(project, "status");
        assert!(!status.required);
        assert!(!status.nullable);
        assert_eq!(status.observed_count, 1);
        assert_eq!(status.total_documents, 2);
    }

    #[test]
    fn editor_save_pipeline_updates_schema_without_editor_coupling() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "projects/a.md",
            "---\ntype: project\npriority: 10\n---\n# A\n",
        );
        let mut shell = shell_from_workspace(&workspace);
        let path = workspace.path().join("projects/a.md");

        save_markdown_file(&path, "---\ntype: project\npriority: high\n---\n# A\n").unwrap();
        apply_events(&mut shell, &workspace, [WorkspaceEvent::Upsert(path)]);

        let project = shell.schema_catalog.collection("project").unwrap();
        assert_eq!(
            schema_field(project, "priority").field_type,
            SchemaType::String
        );
    }

    #[test]
    fn explicit_schema_reload_invalid_schema_and_absence_keep_inferred_schema() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "projects/a.md",
            "---\ntype: project\npriority: high\n---\n# A\n",
        );
        let mut shell = shell_from_workspace(&workspace);
        assert!(matches!(
            shell.schema_catalog.explicit_schema,
            ExplicitSchemaState::Absent
        ));

        workspace.write(
            "flokin.schema.yaml",
            "version: 1\ncollections:\n  projects:\n    fields:\n      priority:\n        type: integer\n        required: false\n",
        );
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("flokin.schema.yaml"),
            )],
        );
        let project = shell.schema_catalog.collection("project").unwrap();
        assert!(matches!(
            shell.schema_catalog.explicit_schema,
            ExplicitSchemaState::Loaded(_)
        ));
        assert_eq!(
            schema_field(project, "priority").field_type,
            SchemaType::Integer
        );
        assert!(schema_field(project, "priority").divergent);

        workspace.write("flokin.schema.yaml", "version: [broken\n");
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("flokin.schema.yaml"),
            )],
        );
        assert!(matches!(
            shell.schema_catalog.explicit_schema,
            ExplicitSchemaState::Invalid(_)
        ));
        assert!(!shell.schema_catalog.warnings.is_empty());
        let project = shell.schema_catalog.collection("project").unwrap();
        assert_eq!(
            schema_field(project, "priority").field_type,
            SchemaType::String
        );

        workspace.write(
            "flokin.schema.yaml",
            "version: 1\ncollections:\n  projects:\n    fields:\n      priority:\n        type: string\n        required: false\n",
        );
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("flokin.schema.yaml"),
            )],
        );
        assert!(matches!(
            shell.schema_catalog.explicit_schema,
            ExplicitSchemaState::Loaded(_)
        ));
        assert!(shell.schema_catalog.warnings.is_empty());
        let project = shell.schema_catalog.collection("project").unwrap();
        assert_eq!(
            schema_field(project, "priority").field_type,
            SchemaType::String
        );
        assert!(!schema_field(project, "priority").divergent);

        workspace.write("flokin.schema.yaml", "version: [broken\n");
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("flokin.schema.yaml"),
            )],
        );
        assert!(matches!(
            shell.schema_catalog.explicit_schema,
            ExplicitSchemaState::Invalid(_)
        ));
        assert!(!shell.schema_catalog.warnings.is_empty());

        workspace.write(
            "flokin.schema.yaml",
            "version: 1\ncollections:\n  projects:\n    fields:\n      priority:\n        type: string\n        required: false\n",
        );
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("flokin.schema.yaml"),
            )],
        );
        assert!(matches!(
            shell.schema_catalog.explicit_schema,
            ExplicitSchemaState::Loaded(_)
        ));
        assert!(shell.schema_catalog.warnings.is_empty());

        fs::remove_file(workspace.path().join("flokin.schema.yaml")).unwrap();
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Remove(
                workspace.path().join("flokin.schema.yaml"),
            )],
        );
        assert!(matches!(
            shell.schema_catalog.explicit_schema,
            ExplicitSchemaState::Absent
        ));
    }

    #[test]
    fn explicit_schema_tracks_divergence_and_extra_fields_without_health() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "projects/a.md",
            "---\ntype: project\nstatus: active\npriority: 10\nbudget: 100\n---\n# A\n",
        );
        workspace.write(
            "projects/b.md",
            "---\ntype: project\nstatus: paused\npriority: 20\n---\n# B\n",
        );
        workspace.write(
            "flokin.schema.yaml",
            "version: 1\ncollections:\n  projects:\n    fields:\n      title:\n        type: string\n        required: true\n      status:\n        type: string\n        required: true\n      priority:\n        type: integer\n        required: true\n",
        );
        let mut shell = shell_from_workspace(&workspace);

        let project = shell.schema_catalog.collection("project").unwrap();
        assert_eq!(
            schema_field(project, "priority").field_type,
            SchemaType::Integer
        );
        assert!(!schema_field(project, "priority").divergent);
        assert!(!schema_field(project, "budget").declared);

        workspace.write(
            "projects/b.md",
            "---\ntype: project\nstatus: paused\npriority: high\n---\n# B\n",
        );
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("projects/b.md"),
            )],
        );

        let project = shell.schema_catalog.collection("project").unwrap();
        let priority = schema_field(project, "priority");
        assert_eq!(priority.field_type, SchemaType::Integer);
        assert_eq!(
            priority
                .observed_types
                .iter()
                .map(|observed| observed.field_type)
                .collect::<Vec<_>>(),
            vec![SchemaType::Integer, SchemaType::String]
        );
        assert!(priority.divergent);
        assert!(!schema_field(project, "budget").declared);

        workspace.write(
            "projects/b.md",
            "---\ntype: project\nstatus: paused\npriority: 20\n---\n# B\n",
        );
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("projects/b.md"),
            )],
        );

        let project = shell.schema_catalog.collection("project").unwrap();
        assert!(!schema_field(project, "priority").divergent);
    }

    #[test]
    fn health_updates_when_schema_type_mismatch_is_corrected_by_save_pipeline() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "projects/a.md",
            "---\ntype: project\npriority: high\n---\n# A\n",
        );
        workspace.write(
            "projects/b.md",
            "---\ntype: project\npriority: 20\n---\n# B\n",
        );
        workspace.write(
            "flokin.schema.yaml",
            "version: 1\ncollections:\n  projects:\n    fields:\n      priority:\n        type: integer\n        required: true\n",
        );
        let mut shell = shell_from_workspace(&workspace);

        assert!(health_has_kind(&shell, HealthIssueKind::TypeMismatch));
        assert_eq!(shell.health.summary.errors, 1);

        let path = workspace.path().join("projects/a.md");
        save_markdown_file(&path, "---\ntype: project\npriority: 10\n---\n# A\n").unwrap();
        apply_events(&mut shell, &workspace, [WorkspaceEvent::Upsert(path)]);

        assert!(!health_has_kind(&shell, HealthIssueKind::TypeMismatch));
        assert_eq!(shell.health.summary.errors, 0);
        assert_eq!(shell.health.summary.healthy_documents, 2);
    }

    #[test]
    fn health_required_field_issue_tracks_create_and_delete() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "projects/a.md",
            "---\ntype: project\nstatus: active\n---\n# A\n",
        );
        workspace.write(
            "projects/b.md",
            "---\ntype: project\nstatus: paused\n---\n# B\n",
        );
        workspace.write(
            "flokin.schema.yaml",
            "version: 1\ncollections:\n  projects:\n    fields:\n      status:\n        type: string\n        required: true\n",
        );
        let mut shell = shell_from_workspace(&workspace);

        assert!(!health_has_kind(
            &shell,
            HealthIssueKind::RequiredFieldMissing
        ));
        assert_eq!(shell.health.summary.healthy_documents, 2);

        workspace.write("projects/c.md", "---\ntype: project\n---\n# C\n");
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("projects/c.md"),
            )],
        );

        assert!(health_has_kind(
            &shell,
            HealthIssueKind::RequiredFieldMissing
        ));
        assert_eq!(shell.health.summary.errors, 1);
        assert_eq!(shell.health.summary.healthy_documents, 2);

        let path = workspace.path().join("projects/c.md");
        fs::remove_file(&path).unwrap();
        apply_events(&mut shell, &workspace, [WorkspaceEvent::Remove(path)]);

        assert!(!health_has_kind(
            &shell,
            HealthIssueKind::RequiredFieldMissing
        ));
        assert_eq!(shell.health.summary.errors, 0);
        assert_eq!(shell.health.summary.healthy_documents, 2);
    }

    #[test]
    fn health_schema_file_recovery_clears_stale_explicit_schema_issue() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "projects/a.md",
            "---\ntype: project\npriority: 10\n---\n# A\n",
        );
        workspace.write("flokin.schema.yaml", "version: [broken\n");
        let mut shell = shell_from_workspace(&workspace);

        assert!(health_has_kind(
            &shell,
            HealthIssueKind::ExplicitSchemaInvalid
        ));

        workspace.write(
            "flokin.schema.yaml",
            "version: 1\ncollections:\n  projects:\n    fields:\n      priority:\n        type: integer\n",
        );
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("flokin.schema.yaml"),
            )],
        );

        assert!(!health_has_kind(
            &shell,
            HealthIssueKind::ExplicitSchemaInvalid
        ));

        workspace.write(
            "flokin.schema.yaml",
            "version: 999\ncollections:\n  projects:\n    fields:\n      priority:\n        type: integer\n",
        );
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("flokin.schema.yaml"),
            )],
        );
        assert!(health_has_kind(
            &shell,
            HealthIssueKind::ExplicitSchemaInvalid
        ));

        workspace.write(
            "flokin.schema.yaml",
            "version: 1\ncollections:\n  projects:\n    fields:\n      priority:\n        type: integer\n",
        );
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("flokin.schema.yaml"),
            )],
        );
        assert!(!health_has_kind(
            &shell,
            HealthIssueKind::ExplicitSchemaInvalid
        ));
    }

    #[test]
    fn absent_explicit_schema_is_not_a_health_issue() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "projects/a.md",
            "---\ntype: project\npriority: 10\n---\n# A\n",
        );
        let shell = shell_from_workspace(&workspace);

        assert!(matches!(
            shell.schema_catalog.explicit_schema,
            ExplicitSchemaState::Absent
        ));
        assert!(!health_has_kind(
            &shell,
            HealthIssueKind::ExplicitSchemaInvalid
        ));
        assert_eq!(shell.health.summary.errors, 0);
        assert_eq!(shell.health.summary.warnings, 0);
    }

    #[test]
    fn generated_schema_file_update_loads_explicit_schema() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "projects/a.md",
            "---\ntype: project\nstatus: active\npriority: 10\n---\n# A\n",
        );
        let mut shell = shell_from_workspace(&workspace);

        let generated = generate_explicit_schema(&shell.schema_catalog).unwrap();
        workspace.write("flokin.schema.yaml", &generated.yaml);
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("flokin.schema.yaml"),
            )],
        );

        assert!(matches!(
            shell.schema_catalog.explicit_schema,
            ExplicitSchemaState::Loaded(_)
        ));
        let project = shell.schema_catalog.collection("project").unwrap();
        assert_eq!(
            schema_field(project, "priority").field_type,
            SchemaType::Integer
        );
        assert!(schema_field(project, "priority").declared);
    }

    #[test]
    fn generated_structural_title_does_not_require_frontmatter_title() {
        let workspace = TempWorkspace::new();
        workspace.write("projects/a.md", "---\ntype: project\n---\n# A title\n");
        let mut shell = shell_from_workspace(&workspace);

        let generated = generate_explicit_schema(&shell.schema_catalog).unwrap();
        assert!(generated.yaml.contains("title:"));
        workspace.write("flokin.schema.yaml", &generated.yaml);
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("flokin.schema.yaml"),
            )],
        );

        assert!(!health_has_kind(
            &shell,
            HealthIssueKind::RequiredFieldMissing
        ));
        assert!(!health_has_kind(&shell, HealthIssueKind::TypeMismatch));
        assert_eq!(shell.health.summary.errors, 0);
    }

    #[test]
    fn schema_file_opens_as_special_editor_tab_without_document_selection() {
        let workspace = TempWorkspace::new();
        workspace.write("projects/a.md", "---\ntype: project\n---\n# A\n");
        workspace.write(
            "flokin.schema.yaml",
            "version: 1\ncollections:\n  projects:\n    fields: {}\n",
        );
        let mut shell = shell_from_workspace(&workspace);

        assert!(shell.open_schema_tab().unwrap());
        let tab = shell.active_editor_tab().unwrap();
        assert_eq!(tab.kind, EditorTabKind::Schema);
        assert_eq!(tab.title, crate::SCHEMA_FILE_NAME);
        assert_eq!(tab.relative_path, Path::new(crate::SCHEMA_FILE_NAME));
        assert_eq!(
            tab.buffer,
            "version: 1\ncollections:\n  projects:\n    fields: {}\n"
        );
        assert_eq!(shell.selected_document_path, None);
        assert!(matches!(
            shell.document_inspector(),
            InspectorModel::Empty { .. }
        ));
    }

    #[test]
    fn schema_file_editor_save_reloads_health_and_recovers() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "projects/a.md",
            "---\ntype: project\npriority: 10\n---\n# A\n",
        );
        workspace.write(
            "flokin.schema.yaml",
            "version: 1\ncollections:\n  projects:\n    fields:\n      priority:\n        type: integer\n        required: true\n",
        );
        let mut shell = shell_from_workspace(&workspace);
        let schema_path = workspace.path().join("flokin.schema.yaml");

        shell.open_schema_tab().unwrap();
        shell.update_active_editor_buffer(String::from("version: [broken\n"));
        save_markdown_file(&schema_path, shell.active_editor_buffer().unwrap()).unwrap();
        shell.editor_save_completed(&schema_path, "version: [broken\n", Ok(()));
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(schema_path.clone())],
        );

        assert!(health_has_kind(
            &shell,
            HealthIssueKind::ExplicitSchemaInvalid
        ));

        let valid = "version: 1\ncollections:\n  projects:\n    fields:\n      priority:\n        type: integer\n        required: true\n";
        shell.update_active_editor_buffer(valid.to_owned());
        save_markdown_file(&schema_path, shell.active_editor_buffer().unwrap()).unwrap();
        shell.editor_save_completed(&schema_path, valid, Ok(()));
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(schema_path)],
        );

        assert!(!health_has_kind(
            &shell,
            HealthIssueKind::ExplicitSchemaInvalid
        ));
        assert_eq!(shell.health.summary.errors, 0);
        assert!(!shell.active_editor_tab().unwrap().dirty);
    }

    #[test]
    fn health_relation_issues_follow_target_create_delete_and_ambiguity() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "meetings/carf.md",
            "---\ntype: meeting\nowner: \"[[Maria]]\"\n---\n# CARF\n",
        );
        let mut shell = shell_from_workspace(&workspace);

        assert!(health_has_kind(&shell, HealthIssueKind::RelationUnresolved));

        workspace.write("people/maria.md", "---\ntype: person\n---\n# Maria\n");
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("people/maria.md"),
            )],
        );
        assert!(!health_has_kind(
            &shell,
            HealthIssueKind::RelationUnresolved
        ));

        workspace.write("archive/maria.md", "---\ntype: person\n---\n# Maria\n");
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("archive/maria.md"),
            )],
        );
        assert!(health_has_kind(&shell, HealthIssueKind::RelationAmbiguous));

        let duplicate = workspace.path().join("archive/maria.md");
        fs::remove_file(&duplicate).unwrap();
        apply_events(&mut shell, &workspace, [WorkspaceEvent::Remove(duplicate)]);
        assert!(!health_has_kind(&shell, HealthIssueKind::RelationAmbiguous));

        let maria = workspace.path().join("people/maria.md");
        fs::remove_file(&maria).unwrap();
        apply_events(&mut shell, &workspace, [WorkspaceEvent::Remove(maria)]);
        assert!(health_has_kind(&shell, HealthIssueKind::RelationUnresolved));
    }

    #[test]
    fn health_workspace_change_discards_previous_projection() {
        let first = TempWorkspace::new();
        first.write("projects/a.md", "---\ntype: project\n---\n# A\n");
        first.write(
            "flokin.schema.yaml",
            "version: 1\ncollections:\n  projects:\n    fields:\n      status:\n        type: string\n        required: true\n",
        );
        let mut shell = shell_from_workspace(&first);
        assert!(health_has_kind(
            &shell,
            HealthIssueKind::RequiredFieldMissing
        ));

        let second = TempWorkspace::new();
        second.write(
            "people/sergio.md",
            "---\ntype: person\nactive: true\n---\n# Sergio\n",
        );
        shell.workspace_selected(Some(second.path().to_path_buf()));
        shell.scan_completed(scan_workspace(second.path()).unwrap());

        assert!(!health_has_kind(
            &shell,
            HealthIssueKind::RequiredFieldMissing
        ));
        assert_eq!(shell.health.summary.errors, 0);
        assert_eq!(shell.health.summary.total_documents, 1);
        assert!(shell.schema_catalog.collection("project").is_none());
    }

    #[test]
    fn health_filter_query_and_issue_selection_drive_inspector() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "projects/a.md",
            "---\ntype: project\npriority: high\nbudget: 100\n---\n# A\n",
        );
        workspace.write(
            "flokin.schema.yaml",
            "version: 1\ncollections:\n  projects:\n    fields:\n      priority:\n        type: integer\n        required: true\n",
        );
        let mut shell = shell_from_workspace(&workspace);
        shell.active_activity = Activity::Health;

        shell.select_health_filter(HealthFilter::Errors);
        assert_eq!(shell.filtered_health_issues().len(), 1);
        shell.select_health_filter(HealthFilter::Warnings);
        assert_eq!(shell.filtered_health_issues().len(), 1);
        shell.select_health_filter(HealthFilter::All);
        shell.update_health_query(String::from("budget"));
        assert_eq!(shell.filtered_health_issues().len(), 1);

        let issue_id = shell.filtered_health_issues()[0].id.clone();
        assert!(shell.select_health_issue(issue_id));
        assert!(matches!(
            shell.document_inspector(),
            InspectorModel::HealthIssue(_)
        ));
        assert_eq!(
            shell.selected_document_path.as_ref(),
            Some(&workspace.path().join("projects/a.md"))
        );
    }

    #[test]
    fn invalid_explicit_field_type_and_version_fall_back_to_inferred_schema() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "projects/a.md",
            "---\ntype: project\npriority: 10\n---\n# A\n",
        );
        workspace.write(
            "flokin.schema.yaml",
            "version: 1\ncollections:\n  projects:\n    fields:\n      priority:\n        type: banana\n",
        );
        let mut shell = shell_from_workspace(&workspace);

        assert!(matches!(
            shell.schema_catalog.explicit_schema,
            ExplicitSchemaState::Invalid(_)
        ));
        assert!(!shell.schema_catalog.warnings.is_empty());
        let project = shell.schema_catalog.collection("project").unwrap();
        assert_eq!(
            schema_field(project, "priority").field_type,
            SchemaType::Integer
        );

        workspace.write(
            "flokin.schema.yaml",
            "version: 999\ncollections:\n  projects:\n    fields:\n      priority:\n        type: string\n",
        );
        apply_events(
            &mut shell,
            &workspace,
            [WorkspaceEvent::Upsert(
                workspace.path().join("flokin.schema.yaml"),
            )],
        );

        assert!(matches!(
            shell.schema_catalog.explicit_schema,
            ExplicitSchemaState::Invalid(_)
        ));
        assert!(shell
            .schema_catalog
            .warnings
            .first()
            .is_some_and(|warning| warning.message.contains("versão 999 incompatível")));
        let project = shell.schema_catalog.collection("project").unwrap();
        assert_eq!(
            schema_field(project, "priority").field_type,
            SchemaType::Integer
        );
    }

    #[test]
    fn workspace_change_discards_previous_schema_catalog() {
        let first = TempWorkspace::new();
        first.write(
            "projects/a.md",
            "---\ntype: project\npriority: 10\n---\n# A\n",
        );
        first.write(
            "flokin.schema.yaml",
            "version: 1\ncollections:\n  projects:\n    fields:\n      priority:\n        type: integer\n",
        );
        let mut shell = shell_from_workspace(&first);
        assert!(shell.schema_catalog.collection("project").is_some());
        assert!(matches!(
            shell.schema_catalog.explicit_schema,
            ExplicitSchemaState::Loaded(_)
        ));

        let second = TempWorkspace::new();
        second.write(
            "people/sergio.md",
            "---\ntype: person\nactive: true\n---\n# Sergio\n",
        );
        shell.workspace_selected(Some(second.path().to_path_buf()));
        shell.scan_completed(scan_workspace(second.path()).unwrap());

        assert!(shell.schema_catalog.collection("project").is_none());
        assert!(shell.schema_catalog.collection("person").is_some());
        assert!(matches!(
            shell.schema_catalog.explicit_schema,
            ExplicitSchemaState::Absent
        ));
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

    fn schema_field<'a>(schema: &'a crate::CollectionSchema, name: &str) -> &'a crate::SchemaField {
        schema
            .fields
            .iter()
            .find(|field| field.name == name)
            .unwrap()
    }

    fn property_value(shell: &super::ShellModel, label: &str) -> InspectorValue {
        property(shell, label).unwrap().value
    }

    fn health_has_kind(shell: &super::ShellModel, kind: HealthIssueKind) -> bool {
        shell.health.issues.iter().any(|issue| issue.kind == kind)
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

    #[test]
    fn bulk_edit_review_back_returns_to_configuration_without_writing() {
        let mut shell = mock_shell();
        shell.bulk_edit.editor_open = true;
        shell.bulk_edit.step = BulkEditStep::Review;
        shell.bulk_edit.value = String::from("archived");

        shell.return_to_bulk_configuration();

        assert_eq!(shell.bulk_edit.step, BulkEditStep::Configure);
        assert!(shell.bulk_edit.plan.is_none());
        assert_eq!(shell.bulk_edit.value, "archived");
        assert!(shell.bulk_edit.editor_open);
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
