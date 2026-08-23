use std::path::PathBuf;

use flokin_core::{Activity, BottomTab, ExplorerNodeId, WorkspaceTab};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    ActivitySelected(Activity),
    #[allow(dead_code)]
    ExplorerNodeToggled(ExplorerNodeId),
    WorkspaceTabSelected(WorkspaceTab),
    BottomTabSelected(BottomTab),
    OpenFolder,
    FolderSelected(Option<PathBuf>),
    ThemeToggled,
    MockAction,
}
