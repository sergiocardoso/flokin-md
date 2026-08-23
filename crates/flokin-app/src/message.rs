use flokin_core::{Activity, BottomTab, ExplorerNodeId, WorkspaceTab};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    ActivitySelected(Activity),
    ExplorerNodeToggled(ExplorerNodeId),
    WorkspaceTabSelected(WorkspaceTab),
    BottomTabSelected(BottomTab),
    ThemeToggled,
    MockAction,
}
