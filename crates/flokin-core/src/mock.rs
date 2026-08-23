use crate::model::{
    Activity, BottomTab, DocumentTab, ExplorerNode, FilterCount, InspectorField, ShellModel,
    TagCount, WorkspaceTab,
};

pub fn mock_shell() -> ShellModel {
    ShellModel {
        active_activity: Activity::Explorer,
        current_workspace: None,
        explorer: vec![ExplorerNode::folder(
            1,
            "Knowledge",
            vec![
                ExplorerNode::folder(
                    2,
                    "Projects",
                    vec![
                        ExplorerNode::file(3, "carf.md"),
                        ExplorerNode::file(4, "cvm.md"),
                        ExplorerNode::file(5, "healthy-chew.md"),
                        ExplorerNode::file(6, "ideas.md"),
                    ],
                ),
                ExplorerNode::collapsed_folder(7, "People", Vec::new()),
                ExplorerNode::collapsed_folder(8, "Meetings", Vec::new()),
                ExplorerNode::collapsed_folder(9, "Areas", Vec::new()),
                ExplorerNode::collapsed_folder(10, "Resources", Vec::new()),
                ExplorerNode::collapsed_folder(11, "Archive", Vec::new()),
            ],
        )],
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
        inspector: vec![
            InspectorField {
                label: "Status",
                value: "Ativo",
            },
            InspectorField {
                label: "Tipo",
                value: "Documento",
            },
            InspectorField {
                label: "Owner",
                value: "Sergio",
            },
            InspectorField {
                label: "Projeto",
                value: "Projects",
            },
            InspectorField {
                label: "Tags",
                value: "fiscal, governo",
            },
            InspectorField {
                label: "Criado em",
                value: "20/05/2024 10:15",
            },
            InspectorField {
                label: "Atualizado em",
                value: "31/05/2024 14:22",
            },
            InspectorField {
                label: "Fonte",
                value: "Manual",
            },
            InspectorField {
                label: "Anexos",
                value: "3 arquivos",
            },
            InspectorField {
                label: "Palavras",
                value: "512",
            },
            InspectorField {
                label: "Caracteres",
                value: "3.842",
            },
        ],
        tags: vec![
            TagCount {
                label: "fiscal",
                count: 24,
            },
            TagCount {
                label: "governo",
                count: 18,
            },
            TagCount {
                label: "tributario",
                count: 12,
            },
            TagCount {
                label: "brasil",
                count: 9,
            },
            TagCount {
                label: "receita-federal",
                count: 7,
            },
        ],
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
