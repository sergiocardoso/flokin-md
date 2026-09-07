use crate::model::{Activity, CollectionPanel, FilterCount, HealthFilter, ScanState, ShellModel};
use crate::{
    ContextSection, DatabaseHealth, HistoryState, RelationIndex, SchemaCatalog, SearchState,
    SqlExplorerState,
};
use std::collections::BTreeSet;

pub fn mock_shell() -> ShellModel {
    ShellModel {
        active_activity: Activity::Explorer,
        current_workspace: None,
        explorer: Vec::new(),
        documents: Vec::new(),
        collections: Vec::new(),
        scan_state: ScanState::Idle,
        selected_document_path: None,
        selected_explorer_folder_path: None,
        selected_collection: None,
        collection_table_sort: None,
        search: SearchState::closed(),
        relation_index: RelationIndex::default(),
        schema_catalog: SchemaCatalog::default(),
        health: DatabaseHealth::default(),
        health_filter: HealthFilter::All,
        health_query: String::new(),
        selected_health_issue_id: None,
        context_section: ContextSection::Overview,
        selected_context_artifact: None,
        workspace_errors: Vec::new(),
        selected_schema_field: None,
        collection_panel: CollectionPanel::Data,
        bulk_edit: crate::model::BulkEditState::default(),
        editor: crate::model::EditorState::default(),
        sql_explorer: SqlExplorerState::closed(),
        history: HistoryState::default(),
        collapsed_sql_tables: BTreeSet::new(),
        filters: vec![
            FilterCount {
                label: "Todos os documentos",
                count: 127,
            },
            FilterCount {
                label: "Favoritos",
                count: 12,
            },
            FilterCount {
                label: "Modificados hoje",
                count: 8,
            },
            FilterCount {
                label: "Sem tags",
                count: 15,
            },
            FilterCount {
                label: "Com anexos",
                count: 34,
            },
        ],
    }
}
