use std::path::PathBuf;

use flokin_core::{Activity, BottomTab, ExplorerNodeId, ScanResult, WorkspaceTab};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    ActivitySelected(Activity),
    #[allow(dead_code)]
    ExplorerNodeToggled(ExplorerNodeId),
    WorkspaceTabSelected(WorkspaceTab),
    BottomTabSelected(BottomTab),
    OpenFolder,
    FolderSelected(Option<PathBuf>),
    ScanCompleted(PathBuf, Result<ScanResult, String>),
    CollectionSelected(String),
    MarkdownSelected(PathBuf),
    ThemeToggled,
    MockAction,
}
