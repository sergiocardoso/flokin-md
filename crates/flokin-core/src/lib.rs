mod graph;
mod mock;
mod model;
mod relation;
mod scanner;
mod search;
mod sql;
mod sql_completion;
mod table;

pub use graph::{
    clamp_graph_zoom, document_node_id, fit_graph_viewport, graph_bounds, graph_collections_map,
    initial_graph_layout, GraphBounds, GraphEdge, GraphEdgeStatus, GraphNode, GraphNodeId,
    GraphNodeKind, GraphPoint, GraphProjection, GraphViewport, GRAPH_MAX_ZOOM, GRAPH_MIN_ZOOM,
};
pub use mock::mock_shell;
pub use model::{
    save_markdown_file, workspace_display, Activity, DocumentInspector, DocumentSourceView,
    EditorDialog, EditorExternalConflict, EditorState, EditorTab, ExplorerNode, ExplorerNodeId,
    ExplorerNodeKind, FilterCount, InspectorField, InspectorModel, InspectorRelation,
    InspectorRelationStatus, InspectorValue, RelationDocumentSummary, ScanState, ShellModel,
    SqlExplorerState, WorkspaceDisplay,
};
pub use relation::{
    display_relation_value, parse_wikilink, relation_display_property, Relation, RelationDocument,
    RelationIndex, RelationStatus, RelationTarget,
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
pub use sql_completion::{
    complete_sql, completion_context, quote_identifier_if_needed, replace_sql_completion,
    SqlCompletionContext, SqlCompletionItem, SqlCompletionKind, DEFAULT_SQL_COMPLETION_LIMIT,
};
pub use table::{
    SortDirection, TableCell, TableColumn, TableModel, TableRow, TableSort, TableValueType,
};
