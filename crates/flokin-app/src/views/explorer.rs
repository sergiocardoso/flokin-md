use flokin_core::{ExplorerNode, ExplorerNodeKind, ShellModel};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::{message::Message, theme, widgets};

pub fn view(model: &ShellModel) -> Element<'_, Message> {
    let header = column![
        widgets::section_title("EXPLORER"),
        row![
            widgets::icon(theme::Icon::Database, theme::icons::META, true),
            text(model.root_name).size(theme::typography::TITLE)
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center),
        text(model.root_path)
            .size(theme::typography::LABEL)
            .font(theme::mono())
            .style(theme::text_muted),
    ]
    .spacing(theme::spacing::SM);

    let mut tree = column![].spacing(theme::spacing::XXS);
    for node in &model.explorer {
        tree = push_node(tree, node, 0);
    }

    let filters = filters(model);

    container(
        column![header, scrollable(tree).height(Length::Fill), filters].spacing(theme::spacing::XL),
    )
    .width(272)
    .height(Length::Fill)
    .padding(theme::spacing::LG)
    .style(theme::panel)
    .into()
}

fn push_node<'a>(
    mut tree: iced::widget::Column<'a, Message>,
    node: &'a ExplorerNode,
    depth: u16,
) -> iced::widget::Column<'a, Message> {
    tree = tree.push(node_row(node, depth));

    if node.expanded {
        for child in &node.children {
            tree = push_node(tree, child, depth + 1);
        }
    }

    tree
}

fn node_row(node: &ExplorerNode, depth: u16) -> Element<'_, Message> {
    let disclosure = match (node.kind, node.expanded) {
        (ExplorerNodeKind::Folder, true) => Some(theme::Icon::ChevronDown),
        (ExplorerNodeKind::Folder, false) => Some(theme::Icon::ChevronRight),
        (ExplorerNodeKind::File, _) => None,
    };
    let icon = match node.kind {
        ExplorerNodeKind::Folder => theme::Icon::Folder,
        ExplorerNodeKind::File => theme::Icon::FileText,
    };
    let is_selected = node.name == "carf.md";
    let style = if is_selected {
        theme::button_tree_selected
    } else {
        theme::button_tree
    };

    button(
        row![
            disclosure.map_or_else(
                || container("").width(theme::icons::TREE).into(),
                |icon| widgets::icon(icon, theme::icons::TREE, false)
            ),
            widgets::icon(icon, theme::icons::TREE, is_selected),
            text(node.name).size(theme::typography::BODY)
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center),
    )
    .padding([5.0, 8.0 + f32::from(depth) * 16.0])
    .width(Length::Fill)
    .style(style)
    .on_press(Message::ExplorerNodeToggled(node.id))
    .into()
}

fn filters(model: &ShellModel) -> Element<'_, Message> {
    let mut list = column![widgets::section_title("FILTROS")].spacing(theme::spacing::MD);

    for filter in &model.filters {
        list = list.push(
            container(
                row![
                    text(filter.label)
                        .size(theme::typography::BODY)
                        .style(theme::text_muted)
                        .width(Length::Fill),
                    text(filter.count.to_string())
                        .size(theme::typography::BODY)
                        .font(theme::mono())
                        .style(theme::text_normal),
                ]
                .align_y(Alignment::Center),
            )
            .padding([4.0, 0.0]),
        );
    }

    container(list)
        .padding([theme::spacing::LG, theme::spacing::XS])
        .into()
}
