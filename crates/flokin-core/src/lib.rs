mod mock;
mod model;
mod scanner;
mod search;
mod sql;
mod table;

pub use mock::mock_shell;
pub use model::{
    workspace_display, Activity, BottomTab, DocumentInspector, DocumentTab, ExplorerNode,
    ExplorerNodeId, ExplorerNodeKind, FilterCount, InspectorField, InspectorModel, InspectorValue,
    ScanState, ShellModel, SqlExplorerState, WorkspaceDisplay, WorkspaceTab,
};
pub use scanner::{
    is_markdown_path, is_workspace_markdown_path, scan_workspace, should_ignore_workspace_path,
    workspace_update_from_events, Collection, Document, DocumentMetadata, DocumentWarning,
    PropertyValue, ScanError, ScanResult, WorkspaceEvent, WorkspaceUpdate,
};
pub use search::{
    search_documents, SearchMatchedField, SearchOutcome, SearchQuery, SearchResult, SearchState,
    DEFAULT_SEARCH_LIMIT,
};
pub use sql::{
    default_query, normalize_identifier, SqlCatalog, SqlColumn, SqlColumnType, SqlError,
    SqlProjection, SqlQueryResult, SqlResultColumn, SqlTable, SqlValue, DEFAULT_RESULT_LIMIT,
};
pub use table::{
    SortDirection, TableCell, TableColumn, TableModel, TableRow, TableSort, TableValueType,
};
