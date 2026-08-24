use crate::model::{Activity, FilterCount, ScanState, ShellModel};
use crate::{RelationIndex, SearchState, SqlExplorerState};
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
        selected_collection: None,
        collection_table_sort: None,
        search: SearchState::closed(),
        relation_index: RelationIndex::default(),
        editor: crate::model::EditorState::default(),
        sql_explorer: SqlExplorerState::closed(),
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
