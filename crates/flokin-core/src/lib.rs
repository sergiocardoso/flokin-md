mod mock;
mod model;

pub use mock::mock_shell;
pub use model::{
    Activity, BottomTab, DocumentTab, ExplorerNode, ExplorerNodeId, ExplorerNodeKind, FilterCount,
    InspectorField, ShellModel, TagCount, WorkspaceTab,
};
