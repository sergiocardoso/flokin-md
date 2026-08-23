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
    pub name: &'static str,
    pub kind: ExplorerNodeKind,
    pub children: Vec<ExplorerNode>,
    pub expanded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerNodeKind {
    Folder,
    File,
}

impl ExplorerNode {
    pub fn folder(id: usize, name: &'static str, children: Vec<Self>) -> Self {
        Self {
            id: ExplorerNodeId(id),
            name,
            kind: ExplorerNodeKind::Folder,
            children,
            expanded: true,
        }
    }

    pub fn collapsed_folder(id: usize, name: &'static str, children: Vec<Self>) -> Self {
        Self {
            expanded: false,
            ..Self::folder(id, name, children)
        }
    }

    pub fn file(id: usize, name: &'static str) -> Self {
        Self {
            id: ExplorerNodeId(id),
            name,
            kind: ExplorerNodeKind::File,
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
    pub root_name: &'static str,
    pub root_path: &'static str,
    pub explorer: Vec<ExplorerNode>,
    pub filters: Vec<FilterCount>,
    pub selected_tab: WorkspaceTab,
    pub bottom_tab: BottomTab,
    pub document: DocumentTab,
    pub inspector: Vec<InspectorField>,
    pub tags: Vec<TagCount>,
}

impl ShellModel {
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
}

#[cfg(test)]
mod tests {
    use crate::mock_shell;

    use super::{BottomTab, ExplorerNodeId, WorkspaceTab};

    #[test]
    fn shell_starts_with_expected_mock_state() {
        let shell = mock_shell();

        assert_eq!(shell.root_name, "Knowledge");
        assert_eq!(shell.root_path, "~/Documents/Knowledge");
        assert_eq!(shell.selected_tab, WorkspaceTab::Carf);
        assert_eq!(shell.bottom_tab, BottomTab::View);
    }

    #[test]
    fn toggles_expanded_tree_nodes() {
        let mut shell = mock_shell();

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
