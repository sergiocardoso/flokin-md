use flokin_core::{
    Collection, ExplorerNode, ExplorerNodeKind, ScanState, ShellModel, SqlCatalog, SqlTable,
};
use iced::widget::{
    button, column, container, row, scrollable,
    scrollable::{Direction, Scrollbar},
    text,
};
use iced::{alignment, Alignment, Element, Length};

use crate::{
    file_icons,
    message::Message,
    theme::{self, AppTheme},
    widgets,
};

pub fn view(model: &ShellModel, app_theme: AppTheme, width: f32) -> Element<'_, Message> {
    let workspace = model.workspace_display();
    let header = column![
        widgets::section_title("EXPLORER"),
        row![
            widgets::icon(theme::Icon::Database, theme::icons::META, true),
            text(workspace.name).size(theme::typography::TITLE)
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center),
        text(workspace.path)
            .size(theme::typography::LABEL)
            .font(theme::mono())
            .style(theme::text_muted),
    ]
    .spacing(theme::spacing::SM);

    let tree = tree(model, app_theme);

    let filters = filters();

    container(
        column![header, scrollable(tree).height(Length::Fill), filters].spacing(theme::spacing::XL),
    )
    .width(width)
    .height(Length::Fill)
    .padding(theme::spacing::LG)
    .style(theme::panel)
    .into()
}

pub fn sql_schema_view(model: &ShellModel, width: f32) -> Element<'_, Message> {
    let body: Element<'_, Message> = match model.sql_explorer.catalog.as_ref() {
        Some(catalog) if !catalog.tables.is_empty() => sql_schema(catalog, model),
        Some(_) => column![text("Nenhuma Collection disponível.")
            .size(theme::typography::BODY)
            .style(theme::text_muted),]
        .into(),
        None => column![text(if model.sql_explorer.running {
            "Construindo schema..."
        } else {
            "Abra uma pasta para gerar o schema."
        })
        .size(theme::typography::BODY)
        .style(theme::text_muted),]
        .into(),
    };

    container(
        column![
            row![
                widgets::icon(theme::Icon::Database, theme::icons::META, true),
                text("DATABASE")
                    .size(theme::typography::TITLE)
                    .style(theme::text_normal),
            ]
            .spacing(theme::spacing::SM)
            .align_y(Alignment::Center),
            text("SQLite :memory:")
                .font(theme::mono())
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
            scrollable(body)
                .direction(Direction::Vertical(Scrollbar::default().spacing(8)))
                .height(Length::Fill),
        ]
        .spacing(theme::spacing::MD),
    )
    .width(width)
    .height(Length::Fill)
    .padding(theme::spacing::LG)
    .style(theme::panel)
    .into()
}

pub fn data_view(model: &ShellModel, width: f32) -> Element<'_, Message> {
    let mut collections =
        column![widgets::section_title("COLLECTIONS")].spacing(theme::spacing::XS);
    for collection in &model.collections {
        collections = collections.push(collection_row(
            collection,
            model.selected_collection.as_deref(),
        ));
    }

    container(
        column![
            row![
                widgets::icon(theme::Icon::Database, theme::icons::META, true),
                text("DADOS").size(theme::typography::TITLE),
            ]
            .spacing(theme::spacing::SM)
            .align_y(Alignment::Center),
            scrollable(collections).height(Length::Fill),
        ]
        .spacing(theme::spacing::LG),
    )
    .width(width)
    .height(Length::Fill)
    .padding(theme::spacing::LG)
    .style(theme::panel)
    .into()
}

fn sql_schema<'a>(catalog: &'a SqlCatalog, model: &'a ShellModel) -> Element<'a, Message> {
    let mut tables = column![].spacing(theme::spacing::SM);
    for table in &catalog.tables {
        tables = tables.push(sql_schema_table(
            table,
            !model.collapsed_sql_tables.contains(&table.name),
        ));
    }
    tables.into()
}

fn sql_schema_table(table: &SqlTable, expanded: bool) -> Element<'_, Message> {
    let chevron = if expanded {
        theme::Icon::ChevronDown
    } else {
        theme::Icon::ChevronRight
    };
    let table_name = table.name.clone();
    let header = button(
        column![
            row![
                widgets::icon(chevron, theme::icons::TREE, false),
                text(table.display_name.as_str())
                    .size(theme::typography::BODY)
                    .style(theme::text_accent),
            ]
            .spacing(theme::spacing::XS)
            .align_y(Alignment::Center),
            row![
                container("").width(theme::icons::TREE),
                text(format!("SQL: {}", table.name))
                    .font(theme::mono())
                    .size(theme::typography::LABEL)
                    .style(theme::text_muted),
            ]
            .spacing(theme::spacing::XS)
            .align_y(Alignment::Center),
        ]
        .spacing(theme::spacing::XXS),
    )
    .width(Length::Fill)
    .padding([4.0, 0.0])
    .style(theme::button_tree)
    .on_press(Message::SqlSchemaTableToggled(table_name));

    let mut content = column![header].spacing(theme::spacing::XXS);
    if expanded {
        for column in &table.columns {
            content = content.push(
                row![
                    container("").width(theme::spacing::LG),
                    text(column.name.as_str())
                        .font(theme::mono())
                        .size(theme::typography::LABEL)
                        .width(Length::Fill)
                        .style(theme::text_normal),
                    container(
                        text(column.value_type.label())
                            .font(theme::mono())
                            .size(theme::typography::LABEL)
                            .style(theme::text_muted),
                    )
                    .width(64)
                    .align_x(alignment::Horizontal::Right),
                ]
                .spacing(theme::spacing::XS)
                .align_y(Alignment::Center),
            );
        }
    }
    content.into()
}

fn tree(model: &ShellModel, app_theme: AppTheme) -> iced::widget::Column<'_, Message> {
    if model.current_workspace.is_none() {
        return no_workspace();
    }

    match &model.scan_state {
        ScanState::Idle => column![],
        ScanState::Scanning => scan_message("Analisando documentos..."),
        ScanState::Failed(_) => scan_message("Falha ao analisar workspace"),
        ScanState::Completed {
            documents,
            errors,
            warnings,
            ..
        }
        | ScanState::Updating {
            documents,
            errors,
            warnings,
            ..
        } => {
            if *documents == 0 {
                return scan_message("Nenhum arquivo Markdown encontrado.");
            }

            let mut tree = column![
                scan_summary(*documents, *errors, *warnings),
                widgets::section_title("FILES")
            ]
            .spacing(theme::spacing::XXS);
            for node in &model.explorer {
                tree = tree.push(tree_node(
                    node,
                    0,
                    model.selected_document_path.as_ref(),
                    app_theme,
                ));
            }
            tree = tree.push(container("").height(theme::spacing::MD));
            tree = tree.push(widgets::section_title("DATA"));
            tree = tree.push(sql_explorer_row(model.sql_explorer.open));
            tree = tree.push(container("").height(theme::spacing::MD));
            tree = tree.push(widgets::section_title("COLLECTIONS"));
            for collection in &model.collections {
                tree = tree.push(collection_row(
                    collection,
                    model.selected_collection.as_deref(),
                ));
            }
            tree
        }
    }
}

fn sql_explorer_row<'a>(selected: bool) -> Element<'a, Message> {
    button(
        row![
            widgets::icon(theme::Icon::Terminal, theme::icons::TREE, false),
            text("SQL Explorer")
                .size(theme::typography::BODY)
                .width(Length::Fill)
                .style(if selected {
                    theme::text_accent
                } else {
                    theme::text_normal
                }),
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([4.0, 4.0])
    .style(if selected {
        theme::button_tree_selected
    } else {
        theme::button_tree
    })
    .on_press(Message::SqlExplorerOpened)
    .into()
}

fn scan_message<'a>(message: &'a str) -> iced::widget::Column<'a, Message> {
    column![container(
        row![
            widgets::icon(theme::Icon::Folder, theme::icons::TREE, false),
            text(message)
                .size(theme::typography::BODY)
                .style(theme::text_muted)
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center),
    )
    .padding([5.0, 8.0])]
    .spacing(theme::spacing::XXS)
}

fn scan_summary(documents: usize, errors: usize, warnings: usize) -> Element<'static, Message> {
    let mut message = format!("{documents} documentos encontrados");
    if errors > 0 {
        message.push_str(&format!(", {errors} itens não puderam ser acessados"));
    }
    if warnings > 0 {
        message.push_str(&format!(", {warnings} warnings"));
    }

    container(
        text(message)
            .size(theme::typography::LABEL)
            .style(theme::text_muted),
    )
    .padding([0.0, 8.0])
    .into()
}

fn tree_node<'a>(
    node: &'a ExplorerNode,
    depth: usize,
    selected_document_path: Option<&std::path::PathBuf>,
    app_theme: AppTheme,
) -> Element<'a, Message> {
    let selected = selected_document_path
        .map(|selected| selected == &node.path)
        .unwrap_or(false);
    let style = if selected {
        theme::button_tree_selected
    } else {
        theme::button_tree
    };

    let chevron = if matches!(node.kind, ExplorerNodeKind::Folder) {
        if node.expanded {
            widgets::icon(theme::Icon::ChevronDown, theme::icons::TREE, false)
        } else {
            widgets::icon(theme::Icon::ChevronRight, theme::icons::TREE, false)
        }
    } else {
        container("").width(theme::icons::TREE).into()
    };

    let icon = match node.kind {
        ExplorerNodeKind::Folder => widgets::icon(theme::Icon::Folder, theme::icons::TREE, false),
        ExplorerNodeKind::File => file_icon(&node.path, app_theme),
    };

    let item = button(
        row![
            container("").width((depth as f32) * 14.0),
            chevron,
            icon,
            text(node.name.as_str())
                .size(theme::typography::BODY)
                .style(if selected {
                    theme::text_accent
                } else {
                    theme::text_normal
                })
        ]
        .spacing(theme::spacing::XS)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([4.0, 4.0])
    .style(style)
    .on_press(Message::ExplorerNodeToggled(node.id));

    let mut children = column![item].spacing(theme::spacing::XXS);
    if node.expanded {
        for child in &node.children {
            children = children.push(tree_node(
                child,
                depth + 1,
                selected_document_path,
                app_theme,
            ));
        }
    }

    children.into()
}

fn file_icon(path: &std::path::Path, app_theme: AppTheme) -> Element<'static, Message> {
    let icon = file_icons::icon_for_path(path, app_theme);

    if let Some(font) = icon.font {
        return text(icon.glyph.to_string())
            .font(font)
            .size(theme::typography::BODY)
            .style(move |_| iced::widget::text::Style {
                color: Some(icon.color),
            })
            .width(theme::icons::TREE)
            .into();
    }

    container(
        text(icon.fallback_label)
            .font(theme::mono())
            .size(theme::typography::LABEL)
            .style(move |_| iced::widget::text::Style {
                color: Some(icon.color),
            }),
    )
    .width(22)
    .into()
}

fn collection_row<'a>(
    collection: &'a Collection,
    selected_collection: Option<&str>,
) -> Element<'a, Message> {
    let selected = selected_collection == Some(collection.id.as_str());
    let style = if selected {
        theme::button_tree_selected
    } else {
        theme::button_tree
    };

    button(
        row![
            widgets::icon(theme::Icon::Database, theme::icons::TREE, false),
            text(collection.display_name.as_str())
                .size(theme::typography::BODY)
                .width(Length::Fill)
                .style(if selected {
                    theme::text_accent
                } else {
                    theme::text_normal
                }),
            text(collection.document_count.to_string())
                .font(theme::mono())
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([4.0, 4.0])
    .style(style)
    .on_press(Message::CollectionSelected(collection.id.clone()))
    .into()
}

fn no_workspace<'a>() -> iced::widget::Column<'a, Message> {
    column![
        text("Nenhuma pasta aberta")
            .size(theme::typography::BODY)
            .style(theme::text_muted),
        button(
            row![
                widgets::icon(theme::Icon::Folder, theme::icons::TOOLBAR, false),
                text("Abrir pasta").size(theme::typography::BODY)
            ]
            .spacing(theme::spacing::SM)
            .align_y(Alignment::Center)
        )
        .padding([6.0, 9.0])
        .style(theme::button_toolbar)
        .on_press(Message::OpenFolder)
    ]
    .spacing(theme::spacing::MD)
}

fn filters<'a>() -> Element<'a, Message> {
    let list = column![
        widgets::section_title("FILTROS"),
        text("Disponíveis após indexação")
            .size(theme::typography::BODY)
            .style(theme::text_muted)
    ]
    .spacing(theme::spacing::MD);

    container(list)
        .padding([theme::spacing::LG, theme::spacing::XS])
        .into()
}
