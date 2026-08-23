mod mock;
mod model;

pub use mock::mock_shell;
pub use model::{
    workspace_display, Activity, BottomTab, DocumentTab, ExplorerNode, ExplorerNodeId,
    ExplorerNodeKind, FilterCount, InspectorField, ShellModel, TagCount, WorkspaceDisplay,
    WorkspaceTab,
};
