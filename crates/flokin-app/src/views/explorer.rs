use flokin_core::{Collection, ExplorerNode, ExplorerNodeKind, ScanState, ShellModel};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::{
    file_icons,
    message::Message,
    theme::{self, AppTheme},
    widgets,
};

pub fn view(model: &ShellModel, app_theme: AppTheme) -> Element<'_, Message> {
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
    .width(272)
    .height(Length::Fill)
    .padding(theme::spacing::LG)
    .style(theme::panel)
    .into()
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
