use crate::model::{
    Activity, BottomTab, DocumentTab, FilterCount, ScanState, ShellModel, WorkspaceTab,
};
use crate::{SearchState, SqlExplorerState};
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
        selected_tab: WorkspaceTab::Carf,
        bottom_tab: BottomTab::View,
        document: DocumentTab {
            selected: WorkspaceTab::Carf,
            content: CARF_MARKDOWN,
        },
    }
}

const CARF_MARKDOWN: &str = "# CARF

Conselho Administrativo de Recursos Fiscais.

Órgão colegiado responsável pelo julgamento de recursos administrativos de decisões fiscais no âmbito da Receita Federal do Brasil.

## Visão Geral

* Instância administrativa
* Julgamento de recursos fiscais
* Vinculado ao Ministério da Fazenda
* Composição paritária entre Fazenda e contribuintes

## Estrutura

| Área | Função |
| --- | --- |
| Turmas | Julgamento colegiado |
| Câmara Superior | Uniformização de decisões |

## Links

* Site Oficial
* Regimento Interno
";
