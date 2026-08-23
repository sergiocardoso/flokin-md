use std::path::{Path, PathBuf, MAIN_SEPARATOR};

use crate::{Collection, Document, ScanResult, SortDirection, TableSort};

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
    pub label: &'static str,
    pub value: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagCount {
    pub label: &'static str,
    pub count: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellModel {
    pub active_activity: Activity,
    pub current_workspace: Option<PathBuf>,
    pub explorer: Vec<ExplorerNode>,
    pub documents: Vec<Document>,
    pub collections: Vec<Collection>,
    pub scan_state: ScanState,
    pub selected_markdown: Option<PathBuf>,
    pub selected_collection: Option<String>,
    pub collection_table_sort: Option<TableSort>,
    pub filters: Vec<FilterCount>,
    pub selected_tab: WorkspaceTab,
    pub bottom_tab: BottomTab,
    pub document: DocumentTab,
    pub inspector: Vec<InspectorField>,
    pub tags: Vec<TagCount>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanState {
    Idle,
    Scanning,
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
            self.selected_markdown = None;
            self.selected_collection = None;
            self.collection_table_sort = None;
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
            self.selected_markdown = Some(path);
            self.selected_collection = None;
            self.collection_table_sort = None;
            true
        } else {
            false
        }
    }

    pub fn select_markdown_path(&mut self, path: PathBuf) {
        if self.documents.iter().any(|document| document.path == path) {
            self.selected_markdown = Some(path);
        }
    }

    pub fn select_collection(&mut self, collection_id: String) {
        if self
            .collections
            .iter()
            .any(|collection| collection.id == collection_id)
        {
            self.selected_collection = Some(collection_id);
            self.selected_markdown = None;
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
        self.explorer = explorer_from_scan_result(&result);
        self.documents = result.documents;
        self.collections = result.collections;
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
    }

    pub fn scan_failed(&mut self, message: String) {
        self.explorer.clear();
        self.documents.clear();
        self.collections.clear();
        self.selected_markdown = None;
        self.selected_collection = None;
        self.collection_table_sort = None;
        self.scan_state = ScanState::Failed(message);
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
        let path = self.selected_markdown.as_ref()?;
        self.documents
            .iter()
            .find(|document| &document.path == path)
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
    use crate::mock_shell;

    use std::path::PathBuf;

    use super::{
        workspace_display, BottomTab, ExplorerNode, ExplorerNodeId, ScanState, WorkspaceTab,
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
}
