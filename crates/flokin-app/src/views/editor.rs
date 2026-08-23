use flokin_core::{
    BottomTab, Collection, ShellModel, SortDirection, TableCell, TableColumn, TableModel,
    WorkspaceTab,
};
use iced::widget::{
    button, column, container, row, scrollable,
    scrollable::{Direction, Scrollbar},
    text,
};
use iced::{Alignment, Element, Length};

use crate::{message::Message, theme, widgets};

pub fn tabs(model: &ShellModel) -> Element<'_, Message> {
    if let Some(collection) = model.selected_collection() {
        return container(row![widgets::tab_button(
            collection.display_name.as_str(),
            true,
            Message::MockAction,
        )])
        .height(38)
        .padding([0.0, theme::spacing::SM])
        .style(theme::surface)
        .into();
    }

    if let Some(document) = model.selected_document() {
        return container(row![widgets::tab_button(
            document.title.as_str(),
            true,
            Message::MockAction,
        )])
        .height(38)
        .padding([0.0, theme::spacing::SM])
        .style(theme::surface)
        .into();
    }

    let mut tabs = row![]
        .spacing(theme::spacing::XXS)
        .align_y(Alignment::Center);

    for tab in WorkspaceTab::ALL {
        tabs = tabs.push(widgets::tab_button(
            tab.title(),
            tab == model.selected_tab,
            Message::WorkspaceTabSelected(tab),
        ));
    }

    tabs = tabs.push(widgets::tab_icon_button(
        theme::Icon::Plus,
        Message::MockAction,
    ));

    container(tabs)
        .height(38)
        .padding([0.0, theme::spacing::SM])
        .style(theme::surface)
        .into()
}

pub fn view(model: &ShellModel) -> Element<'_, Message> {
    if let Some(collection) = model.selected_collection() {
        return collection_view(model, collection.id.as_str());
    }

    if let Some(document) = model.selected_document() {
        return document_selection_view(document);
    }

    column![breadcrumb(), editor_area(model), bottom_panel(model)]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn collection_view<'a>(model: &'a ShellModel, collection_id: &'a str) -> Element<'a, Message> {
    let Some(collection) = model.selected_collection() else {
        return container("").into();
    };
    let table = TableModel::collection(
        collection_id,
        &model.documents,
        model.collection_table_sort.as_ref(),
    );

    container(
        column![
            collection_header(collection, table.columns.len().saturating_sub(1)),
            if table.rows.is_empty() {
                empty_collection_view()
            } else {
                table_view(model, table)
            }
        ]
        .spacing(theme::spacing::LG),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(theme::spacing::XXL)
    .style(theme::editor)
    .into()
}

fn collection_header<'a>(
    collection: &'a Collection,
    property_count: usize,
) -> Element<'a, Message> {
    container(
        row![
            column![
                text(collection.display_name.as_str())
                    .size(22)
                    .style(theme::text_accent),
                text(format!("{} documentos", collection.document_count))
                    .size(theme::typography::BODY)
                    .style(theme::text_muted),
            ]
            .spacing(theme::spacing::XS)
            .width(Length::Fill),
            text(format!("{property_count} propriedades"))
                .size(theme::typography::BODY)
                .style(theme::text_muted),
        ]
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .into()
}

fn empty_collection_view<'a>() -> Element<'a, Message> {
    container(
        text("Nenhum documento nesta Collection.")
            .size(theme::typography::BODY)
            .style(theme::text_muted),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn table_view<'a>(model: &'a ShellModel, table: TableModel) -> Element<'a, Message> {
    let width = table_width(&table.columns);
    let mut rows = column![table_header(&table.columns, model, width)].spacing(0);

    for row_model in table.rows {
        let selected = model.selected_markdown.as_ref() == Some(&row_model.document_path);
        let mut cells = row![].spacing(0).align_y(Alignment::Center);

        for (column, cell) in table.columns.iter().zip(row_model.cells) {
            cells = cells.push(table_cell(column.clone(), cell, selected));
        }

        let style = if selected {
            theme::button_tree_selected
        } else {
            theme::button_tree
        };

        rows = rows.push(
            button(container(cells).width(width).style(if selected {
                theme::table_row_selected
            } else {
                theme::table_row
            }))
            .width(width)
            .height(30)
            .padding(0)
            .style(style)
            .on_press(Message::MarkdownSelected(row_model.document_path)),
        );
    }

    container(
        scrollable(rows)
            .direction(Direction::Both {
                vertical: Scrollbar::default(),
                horizontal: Scrollbar::default(),
            })
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn table_header<'a>(
    columns: &[TableColumn],
    model: &'a ShellModel,
    width: f32,
) -> Element<'a, Message> {
    let mut header = row![].spacing(0).align_y(Alignment::Center);

    for column in columns {
        header = header.push(header_cell(column.clone(), model));
    }

    container(header)
        .width(width)
        .height(32)
        .style(theme::table_header)
        .into()
}

fn header_cell<'a>(column: TableColumn, model: &'a ShellModel) -> Element<'a, Message> {
    let width = column.width as f32;
    let column_id = column.id.clone();

    button(
        row![
            text(column.label)
                .size(theme::typography::LABEL)
                .style(theme::text_muted)
                .width(Length::Fill),
            sort_indicator(column_id.as_str(), model)
        ]
        .spacing(theme::spacing::XS)
        .align_y(Alignment::Center),
    )
    .width(width)
    .height(32)
    .padding([0.0, theme::spacing::SM])
    .style(theme::button_table_header)
    .on_press(Message::TableHeaderSelected(column_id))
    .into()
}

fn sort_indicator<'a>(column_id: &str, model: &'a ShellModel) -> Element<'a, Message> {
    let indicator = model
        .collection_table_sort
        .as_ref()
        .filter(|sort| sort.column_id == column_id)
        .map(|sort| match sort.direction {
            SortDirection::Ascending => "↑",
            SortDirection::Descending => "↓",
        })
        .unwrap_or("");

    text(indicator)
        .size(theme::typography::LABEL)
        .width(12)
        .style(theme::text_accent)
        .into()
}

fn table_cell<'a>(column: TableColumn, cell: TableCell, selected: bool) -> Element<'a, Message> {
    let muted = matches!(&cell, TableCell::Missing | TableCell::Null);
    let is_mono = matches!(
        &cell,
        TableCell::Number(_) | TableCell::Bool(_) | TableCell::Missing | TableCell::Null
    );
    let value = cell.display_value();
    let style = if muted {
        theme::text_muted
    } else if selected {
        theme::text_accent
    } else {
        theme::text_normal
    };

    container(
        text(value)
            .size(theme::typography::BODY)
            .font(if is_mono {
                theme::mono()
            } else {
                theme::typography::UI
            })
            .style(style),
    )
    .width(column.width as f32)
    .height(30)
    .padding([6.0, theme::spacing::SM])
    .into()
}

fn table_width(columns: &[TableColumn]) -> f32 {
    columns
        .iter()
        .map(|column| column.width as f32)
        .sum::<f32>()
}

fn document_selection_view(document: &flokin_core::Document) -> Element<'_, Message> {
    container(
        column![
            text(document.title.as_str())
                .size(22)
                .style(theme::text_accent),
            text(document.relative_path.display().to_string())
                .font(theme::mono())
                .size(theme::typography::BODY)
                .style(theme::text_muted),
            text("Conteúdo real será aberto em milestone futura.")
                .size(theme::typography::BODY)
                .style(theme::text_muted),
        ]
        .spacing(theme::spacing::MD),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(theme::spacing::XXL)
    .style(theme::editor)
    .into()
}

fn breadcrumb<'a>() -> Element<'a, Message> {
    container(
        row![
            text("Projects")
                .size(theme::typography::BODY)
                .style(theme::text_muted),
            text("›")
                .size(theme::typography::BODY)
                .style(theme::text_muted),
            text("carf.md").size(theme::typography::BODY),
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center),
    )
    .height(32)
    .padding([0.0, theme::spacing::MD])
    .style(theme::elevated)
    .into()
}

fn editor_area(model: &ShellModel) -> Element<'_, Message> {
    let mut lines = column![].spacing(0);

    for (index, line) in model.document.content.lines().enumerate() {
        lines = lines.push(editor_line(index + 1, line));
    }

    container(scrollable(lines).height(Length::Fill))
        .height(Length::FillPortion(3))
        .style(theme::editor)
        .into()
}

fn editor_line(line_number: usize, line: &str) -> Element<'_, Message> {
    let is_heading = line.starts_with('#');
    let is_active = line_number == 1;
    let line_text = if line.is_empty() { " " } else { line };
    let code = text(line_text)
        .font(theme::mono())
        .size(theme::typography::EDITOR)
        .style(if is_heading {
            theme::text_accent
        } else {
            theme::text_normal
        });

    let line_row = row![
        container(
            text(format!("{line_number:>3}"))
                .font(theme::mono())
                .size(theme::typography::LABEL)
                .style(theme::text_muted)
        )
        .width(62)
        .padding([3.0, theme::spacing::LG])
        .style(theme::gutter),
        container(code)
            .width(Length::Fill)
            .padding([3.0, theme::spacing::LG])
    ]
    .height(24);

    if is_active {
        container(line_row).style(theme::active_line).into()
    } else {
        line_row.into()
    }
}

fn bottom_panel(model: &ShellModel) -> Element<'_, Message> {
    let mut tabs = row![]
        .spacing(theme::spacing::XXS)
        .align_y(Alignment::Center);

    for tab in BottomTab::ALL {
        tabs = tabs.push(widgets::tab_button(
            tab.title(),
            tab == model.bottom_tab,
            Message::BottomTabSelected(tab),
        ));
    }

    let preview = column![
        row![
            widgets::tab_button("Prévia", true, Message::MockAction),
            widgets::tab_button("Código-fonte", false, Message::MockAction),
        ]
        .spacing(theme::spacing::SM),
        container(
            column![
                text("CARF").size(18).style(theme::text_accent),
                text("Conselho Administrativo de Recursos Fiscais.").size(theme::typography::BODY),
                text("Visão Geral").size(theme::typography::TITLE),
                text("• Instância administrativa").size(theme::typography::BODY),
                text("• Julgamento de recursos fiscais").size(theme::typography::BODY),
            ]
            .spacing(theme::spacing::SM)
        )
        .padding(theme::spacing::MD)
        .width(Length::Fill)
        .style(theme::elevated)
    ]
    .spacing(theme::spacing::SM);

    container(column![tabs, preview].spacing(theme::spacing::SM))
        .height(Length::FillPortion(1))
        .padding(theme::spacing::MD)
        .style(theme::panel)
        .into()
}
