mod bulk_edit;
mod graph;
mod health;
mod history;
mod mock;
mod model;
mod relation;
mod scanner;
mod schema;
mod search;
mod sql;
mod sql_completion;
mod table;

pub use bulk_edit::{
    apply_bulk_edit_plan, build_bulk_edit_plan, content_fingerprint, explicit_schema_loaded,
    patch_frontmatter_properties, selectable_properties, validate_bulk_edit_operation,
    BulkEditApplyError, BulkEditChangeStatus, BulkEditFileChange, BulkEditOperation, BulkEditPlan,
    BulkEditResult, BulkEditSelection, BulkEditSummary, BulkEditValue, FrontmatterPatchOutcome,
    FrontmatterPropertyChange,
};
pub use graph::{
    clamp_graph_zoom, document_node_id, fit_graph_viewport, graph_bounds, graph_collections_map,
    initial_graph_layout, GraphBounds, GraphEdge, GraphEdgeStatus, GraphNode, GraphNodeId,
    GraphNodeKind, GraphPoint, GraphProjection, GraphViewport, GRAPH_MAX_ZOOM, GRAPH_MIN_ZOOM,
};
pub use health::{
    build_health, CollectionHealthSummary, DatabaseHealth, HealthCategory, HealthIssue,
    HealthIssueKind, HealthSeverity, HealthSummary,
};
pub use history::{
    build_undo_plan, bulk_history_entry, new_history_id, now_unix_seconds, sql_history_entry,
    undo_history_entry, workspace_identity, HistoryFileChange, HistoryState, MutationHistoryEntry,
    MutationHistoryStore, MutationSource, UndoBuildError, HISTORY_RETENTION_LIMIT,
    HISTORY_STORAGE_VERSION,
};
pub use mock::mock_shell;
pub use model::{
    classify_semantic_entry, save_markdown_file, workspace_display, Activity,
    BulkEditOperationKind, BulkEditState, BulkEditStep, BulkEditValueType, CollectionPanel,
    DocumentInspector, DocumentSourceView, EditorDialog, EditorExternalConflict, EditorState,
    EditorTab, EditorTabKind, EditorViewMode, ExplorerNode, ExplorerNodeId, ExplorerNodeKind,
    FilterCount, HealthFilter, HealthIssueInspector, InspectorField, InspectorModel,
    InspectorRelation, InspectorRelationStatus, InspectorValue, RelationDocumentSummary, ScanState,
    SemanticKind, ShellModel, SqlExplorerMode, SqlExplorerState, WorkspaceDisplay,
};
pub use relation::{
    display_relation_value, parse_wikilink, relation_display_property, Relation, RelationDocument,
    RelationIndex, RelationStatus, RelationTarget,
};
pub use scanner::{
    is_markdown_path, is_workspace_markdown_path, markdown_body_without_frontmatter,
    scan_workspace, should_ignore_workspace_path, workspace_update_from_events, Collection,
    Document, DocumentMetadata, DocumentWarning, PropertyValue, ScanError, ScanResult,
    WorkspaceEvent, WorkspaceUpdate,
};
pub use schema::{
    generate_explicit_schema, is_workspace_schema_path, load_explicit_schema, schema_path,
    schema_type_for_property_value, CollectionSchema, ExplicitCollectionSchema,
    ExplicitFieldSchema, ExplicitSchema, ExplicitSchemaState, GeneratedExplicitSchema,
    GeneratedSchemaOmittedField, ObservedSchemaType, SchemaCatalog, SchemaField,
    SchemaGenerationError, SchemaSource, SchemaType, SchemaWarning, SCHEMA_FILE_NAME,
};
pub use search::{
    search_documents, SearchMatchedField, SearchOutcome, SearchQuery, SearchResult, SearchState,
    DEFAULT_SEARCH_LIMIT,
};
pub use sql::{
    default_query, normalize_identifier, SqlCatalog, SqlColumn, SqlColumnType, SqlError,
    SqlProjection, SqlQueryResult, SqlResultColumn, SqlTable, SqlValue, SqlWritePlan,
    DEFAULT_RESULT_LIMIT,
};
pub use sql_completion::{
    complete_sql, completion_context, quote_identifier_if_needed, replace_sql_completion,
    SqlCompletionContext, SqlCompletionItem, SqlCompletionKind, DEFAULT_SQL_COMPLETION_LIMIT,
};
pub use table::{
    SortDirection, TableCell, TableColumn, TableModel, TableRow, TableSort, TableValueType,
};
