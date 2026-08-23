use std::path::PathBuf;

use flokin_core::{Activity, BottomTab, ExplorerNodeId, ScanResult, WorkspaceTab, WorkspaceUpdate};

use crate::services::file_watcher::WatcherMessage;

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
    ReindexWorkspace,
    WorkspaceWatcher(WatcherMessage),
    WorkspaceUpdateCompleted(PathBuf, Result<WorkspaceUpdate, String>),
    CollectionSelected(String),
    TableHeaderSelected(String),
    MarkdownSelected(PathBuf),
    ThemeToggled,
    MockAction,
}
