use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf, MAIN_SEPARATOR},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    search_documents, Collection, Document, PropertyValue, ScanError, ScanResult, SearchQuery,
    SearchState, SortDirection, TableSort, WorkspaceUpdate,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceTab {
    Carf,
    Cvm,
    HealthyChew,
}

impl WorkspaceTab {
    pub const ALL: [Self; 3] = [Self::Carf, Self::Cvm, Self::HealthyChew];

    pub const fn title(self) -> &'static str {
        match self {
            Self::Carf => "carf.md",
            Self::Cvm => "cvm.md",
            Self::HealthyChew => "healthy-chew.md",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomTab {
    View,
    Graph,
    Backlinks,
    Attachments,
    History,
}

impl BottomTab {
    pub const ALL: [Self; 5] = [
        Self::View,
        Self::Graph,
        Self::Backlinks,
        Self::Attachments,
        Self::History,
    ];

    pub const fn title(self) -> &'static str {
        match self {
            Self::View => "VER",
            Self::Graph => "GRAFO",
            Self::Backlinks => "BACKLINKS",
            Self::Attachments => "ANEXOS",
            Self::History => "HISTÓRICO",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentTab {
    pub selected: WorkspaceTab,
    pub content: &'static str,
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
    pub metadata: Vec<InspectorField>,
    pub tags: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectorModel {
    Empty { title: String, description: String },
    Document(DocumentInspector),
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub filters: Vec<FilterCount>,
    pub selected_tab: WorkspaceTab,
    pub bottom_tab: BottomTab,
    pub document: DocumentTab,
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

    pub fn select_workspace_tab(&mut self, tab: WorkspaceTab) {
        self.selected_tab = tab;
    }

    pub fn select_bottom_tab(&mut self, tab: BottomTab) {
        self.bottom_tab = tab;
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
            self.selected_document_path = Some(path);
            self.selected_collection = None;
            self.collection_table_sort = None;
            self.search.close();
            true
        } else {
            false
        }
    }

    pub fn select_markdown_path(&mut self, path: PathBuf) {
        if self.documents.iter().any(|document| document.path == path) {
            self.selected_document_path = Some(path);
            self.selected_collection = None;
            self.collection_table_sort = None;
        }
    }

    pub fn select_search_result_path(&mut self, path: PathBuf) -> bool {
        if let Some(document) = self.documents.iter().find(|document| document.path == path) {
            self.selected_document_path = Some(document.path.clone());
            self.selected_collection = Some(document.collection_id.clone());
            self.collection_table_sort = None;
            self.search.close();
            true
        } else {
            self.refresh_search_results();
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
            self.collection_table_sort = None;
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
        let expanded_paths = expanded_folder_paths(&self.explorer);
        self.explorer = explorer_from_scan_result(&result);
        restore_expanded_folder_paths(&mut self.explorer, &expanded_paths);
        self.documents = result.documents;
        self.collections = result.collections;
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
            if self.selected_document_path.as_ref() == Some(&path) {
                self.selected_document_path = None;
            }
        }

        for document in update.upserts {
            if let Some(existing) = self
                .documents
                .iter_mut()
                .find(|existing| existing.path == document.path)
            {
                *existing = document;
            } else {
                self.documents.push(document);
            }
        }

        self.documents
            .sort_by(|left, right| compare_paths(&left.relative_path, &right.relative_path));
        self.collections = collections_from_documents(&self.documents);

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
            properties: inspector_properties(document),
            metadata: inspector_metadata(document, self.collection_display_name(document)),
            tags: inspector_tags(document),
            warnings: document
                .warnings
                .iter()
                .map(|warning| user_warning_message(warning.message.as_str()))
                .collect(),
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

fn inspector_properties(document: &Document) -> Vec<InspectorField> {
    let mut properties = vec![InspectorField {
        label: String::from("Title"),
        value: InspectorValue::Text(document.title.clone()),
    }];

    properties.extend(document.properties.iter().filter_map(|(key, value)| {
        if is_special_property(key) {
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
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;

    if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn format_system_time(time: SystemTime) -> String {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{}s desde UNIX epoch", duration.as_secs()),
        Err(_) => String::from("—"),
    }
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
        mock_shell, scan_workspace, workspace_update_from_events, Collection, Document,
        DocumentMetadata, DocumentWarning, PropertyValue, ScanResult, TableModel, WorkspaceEvent,
    };

    use std::{
        collections::BTreeMap,
        ffi::OsString,
        fs,
        path::{Path, PathBuf},
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use super::{
        workspace_display, BottomTab, ExplorerNode, ExplorerNodeId, InspectorModel, InspectorValue,
        ScanState, WorkspaceTab,
    };

    #[test]
    fn shell_starts_with_expected_mock_state() {
        let shell = mock_shell();

        assert_eq!(shell.current_workspace, None);
        assert_eq!(shell.scan_state, ScanState::Idle);
        assert_eq!(shell.selected_tab, WorkspaceTab::Carf);
        assert_eq!(shell.bottom_tab, BottomTab::View);
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
