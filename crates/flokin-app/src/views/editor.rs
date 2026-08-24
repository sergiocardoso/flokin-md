use flokin_core::{
    BottomTab, Collection, ShellModel, SortDirection, SqlColumnType, SqlQueryResult, SqlValue,
    TableCell, TableColumn, TableModel, TableValueType, WorkspaceTab,
};
use iced::widget::{
    button, column, container, row, scrollable,
    scrollable::{Direction, Scrollbar},
    text, text_editor,
};
use iced::{keyboard, keyboard::Key, Alignment, Element, Length};

use crate::{
    message::{Message, SplitterKind},
    theme,
    views::data_grid,
    widgets,
};

pub fn tabs(model: &ShellModel) -> Element<'_, Message> {
    if model.sql_explorer.open {
        return container(row![widgets::tab_button(
            "SQL Explorer",
            true,
            Message::SqlExplorerOpened,
        )])
        .height(38)
        .padding([0.0, theme::spacing::SM])
        .style(theme::surface)
        .into();
    }

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

    if model.current_workspace.is_some() && model.documents.is_empty() {
        return container(row![])
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

pub fn view<'a>(
    model: &'a ShellModel,
    sql_editor: &'a text_editor::Content,
    sql_editor_height: f32,
) -> Element<'a, Message> {
    if model.sql_explorer.open {
        return sql_explorer_view(model, sql_editor, sql_editor_height);
    }

    if let Some(collection) = model.selected_collection() {
        return collection_view(model, collection.id.as_str());
    }

    if let Some(document) = model.selected_document() {
        return document_selection_view(document);
    }

    if model.current_workspace.is_some() && model.documents.is_empty() {
        return empty_workspace_view(model);
    }

    column![breadcrumb(), editor_area(model), bottom_panel(model)]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn empty_workspace_view(model: &ShellModel) -> Element<'_, Message> {
    let workspace = model.workspace_display();
    container(
        column![
            text("Nenhum arquivo Markdown encontrado.")
                .size(theme::typography::TITLE)
                .style(theme::text_normal),
            text(format!("Pasta escaneada: {}", workspace.path))
                .font(theme::mono())
                .size(theme::typography::BODY)
                .style(theme::text_muted),
            text("Abra uma pasta que contenha arquivos .md ou .markdown.")
                .size(theme::typography::BODY)
                .style(theme::text_muted),
        ]
        .spacing(theme::spacing::SM),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(theme::spacing::XXL)
    .style(theme::editor)
    .into()
}

fn sql_explorer_view<'a>(
    model: &'a ShellModel,
    sql_editor: &'a text_editor::Content,
    sql_editor_height: f32,
) -> Element<'a, Message> {
    let header = row![
        text("Query 1")
            .size(theme::typography::TITLE)
            .style(theme::text_normal)
            .width(Length::Fill),
        button(
            row![
                widgets::icon(theme::Icon::Terminal, theme::icons::TOOLBAR, false),
                text(if model.sql_explorer.running {
                    "Executando..."
                } else {
                    "Executar"
                })
                .size(theme::typography::BODY),
            ]
            .spacing(theme::spacing::SM)
            .align_y(Alignment::Center),
        )
        .padding([5.0, 10.0])
        .style(theme::button_toolbar)
        .on_press(Message::SqlExecute),
        text("Ctrl+Enter")
            .font(theme::mono())
            .size(theme::typography::LABEL)
            .style(theme::text_muted),
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    let editor = container(
        text_editor(sql_editor)
            .placeholder("SELECT *\nFROM projects\nLIMIT 100;")
            .on_action(Message::SqlEditorAction)
            .key_binding(sql_editor_key_binding)
            .font(theme::mono())
            .size(theme::typography::EDITOR)
            .height(Length::Fill)
            .padding(theme::spacing::MD)
            .wrapping(iced::widget::text::Wrapping::None)
            .style(theme::text_editor),
    )
    .height(Length::Fixed(sql_editor_height))
    .width(Length::Fill);

    let results = sql_results(model);
    let editor_splitter = iced::widget::mouse_area(
        container("")
            .height(7)
            .width(Length::Fill)
            .style(theme::splitter),
    )
    .on_press(Message::SplitterPressed(SplitterKind::SqlEditor, 0.0))
    .interaction(iced::mouse::Interaction::ResizingVertically);
    let body = column![
        editor,
        editor_splitter,
        container(results).height(Length::Fill)
    ]
    .spacing(theme::spacing::SM)
    .height(Length::Fill)
    .width(Length::Fill);

    container(column![header, body].spacing(theme::spacing::SM))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(theme::spacing::LG)
        .style(theme::editor)
        .into()
}

fn sql_editor_key_binding(press: text_editor::KeyPress) -> Option<text_editor::Binding<Message>> {
    if press.modifiers.control() && matches!(press.key, Key::Named(keyboard::key::Named::Enter)) {
        Some(text_editor::Binding::Custom(Message::SqlExecute))
    } else {
        text_editor::Binding::from_key_press(press)
    }
}

fn sql_results(model: &ShellModel) -> Element<'_, Message> {
    let metadata = if model.sql_explorer.running {
        String::from("Executando...")
    } else if let Some(result) = model.sql_explorer.result.as_ref() {
        let mut text = format!(
            "{} rows • {} ms",
            result.rows.len(),
            result.elapsed.as_millis()
        );
        if result.truncated {
            text.push_str(" • Resultados limitados a 1.000 linhas.");
        }
        text
    } else {
        String::from("Sem resultados")
    };

    let header = row![
        text("RESULTADOS")
            .size(theme::typography::LABEL)
            .style(theme::text_muted),
        text(metadata)
            .size(theme::typography::LABEL)
            .style(theme::text_muted),
    ]
    .spacing(theme::spacing::LG)
    .align_y(Alignment::Center);

    let body: Element<'_, Message> = if let Some(error) = model.sql_explorer.error.as_ref() {
        container(
            column![
                text("Erro:")
                    .size(theme::typography::BODY)
                    .style(theme::text_warning),
                text(error.as_str())
                    .font(theme::mono())
                    .size(theme::typography::BODY)
                    .style(theme::text_normal),
            ]
            .spacing(theme::spacing::SM),
        )
        .padding(theme::spacing::MD)
        .width(Length::Fill)
        .style(theme::elevated)
        .into()
    } else if let Some(result) = model.sql_explorer.result.as_ref() {
        result_grid(result)
    } else {
        container(
            text("Execute uma consulta SELECT para ver o grid.")
                .size(theme::typography::BODY)
                .style(theme::text_muted),
        )
        .padding(theme::spacing::MD)
        .width(Length::Fill)
        .style(theme::elevated)
        .into()
    };

    column![header, body]
        .spacing(theme::spacing::SM)
        .height(Length::Fill)
        .into()
}

fn result_grid(result: &SqlQueryResult) -> Element<'_, Message> {
    if result.columns.is_empty() {
        return container(
            text("Consulta executada sem colunas de resultado.")
                .size(theme::typography::BODY)
                .style(theme::text_muted),
        )
        .padding(theme::spacing::MD)
        .style(theme::elevated)
        .into();
    }

    let widths = result
        .columns
        .iter()
        .map(|column| result_column_width(column.name.as_str()))
        .collect::<Vec<_>>();
    let width = data_grid::grid_width(true, widths.iter().copied());
    let mut rows = column![result_header(result, widths.clone())].spacing(0);

    for (row_index, row_values) in result.rows.iter().enumerate() {
        let mut cells = row![data_grid::row_gutter(row_index, false)]
            .spacing(0)
            .align_y(Alignment::Center);
        for (index, value) in row_values.iter().enumerate() {
            let value_type = result
                .columns
                .get(index)
                .and_then(|column| column.value_type);
            cells = cells.push(result_cell(value, value_type, widths[index]));
        }
        rows = rows.push(
            button(container(cells).width(width))
                .width(width)
                .height(data_grid::ROW_HEIGHT)
                .padding(0)
                .style(move |theme, status| theme::data_row_button(theme, row_index, false, status))
                .on_press(Message::MockAction),
        );
    }

    scrollable(rows)
        .direction(Direction::Both {
            vertical: Scrollbar::default(),
            horizontal: Scrollbar::default(),
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn result_header(result: &SqlQueryResult, widths: Vec<f32>) -> Element<'_, Message> {
    let mut header = row![data_grid::header_gutter()]
        .spacing(0)
        .align_y(Alignment::Center);
    for (column, width) in result.columns.iter().zip(widths.iter()) {
        header = header.push(data_grid::header_cell(
            text(column.name.as_str())
                .font(theme::mono())
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
            *width,
        ));
    }

    container(header)
        .width(data_grid::grid_width(true, widths.iter().copied()))
        .height(data_grid::HEADER_HEIGHT)
        .style(theme::data_header)
        .into()
}

fn result_cell<'a>(
    value: &'a SqlValue,
    value_type: Option<SqlColumnType>,
    width: f32,
) -> Element<'a, Message> {
    let muted = matches!(value, SqlValue::Null);
    let display = value.display_value(value_type);

    data_grid::cell(
        text(display)
            .font(
                if matches!(
                    value,
                    SqlValue::Integer(_) | SqlValue::Real(_) | SqlValue::Null
                ) {
                    theme::mono()
                } else {
                    theme::typography::UI
                },
            )
            .size(theme::typography::BODY)
            .style(if muted {
                theme::text_muted
            } else {
                theme::text_normal
            }),
        width,
        sql_alignment(value_type),
    )
}

fn sql_alignment(value_type: Option<SqlColumnType>) -> iced::alignment::Horizontal {
    match value_type {
        Some(SqlColumnType::Integer | SqlColumnType::Real) => iced::alignment::Horizontal::Right,
        Some(SqlColumnType::Boolean) => iced::alignment::Horizontal::Center,
        _ => iced::alignment::Horizontal::Left,
    }
}

fn result_column_width(name: &str) -> f32 {
    match name {
        "title" => 240.0,
        "_path" => 320.0,
        "_file_name" => 180.0,
        _ => 160.0,
    }
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
    let width = data_grid::grid_width(true, table.columns.iter().map(|column| column.width as f32));
    let mut rows = column![table_header(&table.columns, model, width)].spacing(0);

    for (row_index, row_model) in table.rows.into_iter().enumerate() {
        let selected = model.selected_document_path.as_ref() == Some(&row_model.document_path);
        let mut cells = row![data_grid::row_gutter(row_index, selected)]
            .spacing(0)
            .align_y(Alignment::Center);

        for (column, cell) in table.columns.iter().zip(row_model.cells) {
            cells = cells.push(table_cell(column, cell, selected));
        }

        rows = rows.push(
            button(container(cells).width(width))
                .width(width)
                .height(data_grid::ROW_HEIGHT)
                .padding(0)
                .style(move |theme, status| {
                    theme::data_row_button(theme, row_index, selected, status)
                })
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
    let mut header = row![data_grid::header_gutter()]
        .spacing(0)
        .align_y(Alignment::Center);

    for column in columns {
        header = header.push(header_cell(column.clone(), model));
    }

    container(header)
        .width(width)
        .height(data_grid::HEADER_HEIGHT)
        .style(theme::data_header)
        .into()
}

fn header_cell<'a>(column: TableColumn, model: &'a ShellModel) -> Element<'a, Message> {
    let width = column.width as f32;
    let column_id = column.id.clone();

    data_grid::header_cell(
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
        .height(data_grid::HEADER_HEIGHT)
        .padding([0.0, theme::spacing::SM])
        .style(theme::button_table_header)
        .on_press(Message::TableHeaderSelected(column_id)),
        width,
    )
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

fn table_cell<'a>(column: &TableColumn, cell: TableCell, selected: bool) -> Element<'a, Message> {
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

    data_grid::cell(
        text(value)
            .size(theme::typography::BODY)
            .font(if is_mono {
                theme::mono()
            } else {
                theme::typography::UI
            })
            .style(style),
        column.width as f32,
        collection_alignment(column.inferred_type),
    )
}

fn collection_alignment(value_type: TableValueType) -> iced::alignment::Horizontal {
    match value_type {
        TableValueType::Number => iced::alignment::Horizontal::Right,
        TableValueType::Boolean => iced::alignment::Horizontal::Center,
        TableValueType::Null => iced::alignment::Horizontal::Center,
        _ => iced::alignment::Horizontal::Left,
    }
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

#[cfg(test)]
mod tests {
    use iced::{keyboard, keyboard::Key, widget::text_editor};

    use super::sql_editor_key_binding;
    use crate::message::Message;

    fn key_press(modifiers: keyboard::Modifiers) -> text_editor::KeyPress {
        text_editor::KeyPress {
            key: Key::Named(keyboard::key::Named::Enter),
            modified_key: Key::Named(keyboard::key::Named::Enter),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::Enter),
            modifiers,
            text: None,
            status: text_editor::Status::Focused { is_hovered: true },
        }
    }

    #[test]
    fn ctrl_enter_in_sql_editor_publishes_execute_message() {
        assert_eq!(
            sql_editor_key_binding(key_press(keyboard::Modifiers::CTRL)),
            Some(text_editor::Binding::Custom(Message::SqlExecute))
        );
    }

    #[test]
    fn plain_enter_keeps_text_editor_newline_behavior() {
        assert_eq!(
            sql_editor_key_binding(key_press(keyboard::Modifiers::NONE)),
            Some(text_editor::Binding::Enter)
        );
    }
}
