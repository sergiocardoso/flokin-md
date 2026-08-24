use flokin_core::{
    Collection, CollectionPanel, CollectionSchema, EditorExternalConflict, EditorTab,
    EditorTabKind, EditorViewMode, ExplicitSchemaState, SchemaField, SchemaSource, SchemaType,
    ShellModel, SortDirection, SqlColumnType, SqlCompletionItem, SqlCompletionKind, SqlQueryResult,
    SqlValue, TableCell, TableColumn, TableModel, TableValueType,
};
use iced::widget::{
    button, column, container, markdown, mouse_area, row, scrollable,
    scrollable::{Direction, Scrollbar},
    stack, text,
    text::{LineHeight, Wrapping},
    text_editor,
};
use iced::{keyboard, keyboard::Key, Alignment, Element, Length};

use crate::{
    message::{Message, SplitterKind},
    theme::{self, AppTheme},
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
        .height(theme::sizes::TAB_HEIGHT)
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
        .height(theme::sizes::TAB_HEIGHT)
        .padding([0.0, theme::spacing::SM])
        .style(theme::surface)
        .into();
    }

    if !model.editor.tabs.is_empty() {
        let mut tabs = row![].spacing(theme::spacing::XS);
        for tab in &model.editor.tabs {
            tabs = tabs.push(editor_tab(model, tab));
        }
        return container(scrollable(tabs).direction(Direction::Horizontal(Scrollbar::default())))
            .height(theme::sizes::TAB_HEIGHT)
            .padding([0.0, theme::spacing::SM])
            .style(theme::surface)
            .into();
    }

    container(row![])
        .height(theme::sizes::TAB_HEIGHT)
        .padding([0.0, theme::spacing::SM])
        .style(theme::surface)
        .into()
}

fn editor_tab<'a>(model: &'a ShellModel, tab: &'a EditorTab) -> Element<'a, Message> {
    let active = model.editor.active_path.as_ref() == Some(&tab.document_path);
    let label = editor_tab_label(model, tab);
    let dirty = if tab.dirty { " ●" } else { "" };
    let style = if active {
        theme::button_selected
    } else {
        theme::button_toolbar
    };

    row![
        button(
            row![text(format!("{label}{dirty}"))
                .size(theme::typography::BODY)
                .style(if active {
                    theme::text_accent
                } else {
                    theme::text_normal
                }),]
            .align_y(Alignment::Center)
        )
        .height(theme::sizes::TAB_BUTTON_HEIGHT)
        .padding([4.0, 9.0])
        .style(style)
        .on_press(Message::EditorTabSelected(tab.document_path.clone())),
        button(widgets::icon(
            theme::Icon::X,
            theme::sizes::TAB_ICON_SIZE,
            false,
        ))
        .width(theme::sizes::TAB_CLOSE_WIDTH)
        .height(theme::sizes::TAB_BUTTON_HEIGHT)
        .padding(0)
        .style(theme::button_toolbar)
        .on_press(Message::EditorTabCloseRequested(tab.document_path.clone()))
    ]
    .spacing(0)
    .align_y(Alignment::Center)
    .into()
}

fn editor_tab_label(model: &ShellModel, tab: &EditorTab) -> String {
    let duplicates = model
        .editor
        .tabs
        .iter()
        .filter(|other| other.title == tab.title)
        .count();
    if duplicates <= 1 {
        return tab.title.clone();
    }

    let parent = tab
        .relative_path
        .parent()
        .and_then(|parent| parent.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| String::from("."));
    format!("{} — {}", tab.title, parent)
}

#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    model: &'a ShellModel,
    app_theme: AppTheme,
    sql_editor: &'a text_editor::Content,
    markdown_editor: &'a text_editor::Content,
    markdown_preview: &'a [markdown::Item],
    sql_completion_items: &'a [SqlCompletionItem],
    sql_completion_selected: usize,
    sql_completion_open: bool,
    sql_editor_height: f32,
) -> Element<'a, Message> {
    if model.sql_explorer.open {
        return sql_explorer_view(
            model,
            sql_editor,
            sql_completion_items,
            sql_completion_selected,
            sql_completion_open,
            sql_editor_height,
        );
    }

    if let Some(collection) = model.selected_collection() {
        return collection_view(model, collection.id.as_str());
    }

    if let Some(tab) = model.active_editor_tab() {
        return markdown_editor_view(tab, markdown_editor, markdown_preview, app_theme, model);
    }

    if model.current_workspace.is_some() && model.documents.is_empty() {
        return empty_workspace_view(model);
    }

    empty_document_area()
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
    sql_completion_items: &'a [SqlCompletionItem],
    sql_completion_selected: usize,
    sql_completion_open: bool,
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

    let editor_widget = container(
        text_editor(sql_editor)
            .placeholder("SELECT *\nFROM projects\nLIMIT 100;")
            .on_action(Message::SqlEditorAction)
            .key_binding(move |press| sql_editor_key_binding(press, sql_completion_open))
            .font(theme::mono())
            .size(theme::typography::EDITOR)
            .line_height(LineHeight::Relative(theme::sizes::EDITOR_LINE_HEIGHT_RATIO))
            .height(Length::Fill)
            .padding(theme::spacing::MD)
            .wrapping(iced::widget::text::Wrapping::None)
            .style(theme::text_editor),
    )
    .height(Length::Fixed(sql_editor_height))
    .width(Length::Fill);
    let editor: Element<'a, Message> = if sql_completion_open && !sql_completion_items.is_empty() {
        stack![
            editor_widget,
            sql_completion_popup(sql_completion_items, sql_completion_selected)
        ]
        .width(Length::Fill)
        .height(Length::Fixed(sql_editor_height))
        .into()
    } else {
        editor_widget.into()
    };

    let results = sql_results(model);
    let editor_splitter = iced::widget::mouse_area(
        container("")
            .height(theme::sizes::SPLITTER_HIT_AREA)
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

fn sql_editor_key_binding(
    press: text_editor::KeyPress,
    completion_open: bool,
) -> Option<text_editor::Binding<Message>> {
    if press.modifiers.control() && matches!(press.key, Key::Named(keyboard::key::Named::Enter)) {
        Some(text_editor::Binding::Custom(Message::SqlExecute))
    } else if press.modifiers.control()
        && matches!(press.key, Key::Named(keyboard::key::Named::Space))
    {
        Some(text_editor::Binding::Custom(
            Message::SqlCompletionRequested,
        ))
    } else if completion_open {
        match press.key.as_ref() {
            Key::Named(keyboard::key::Named::ArrowDown) => {
                Some(text_editor::Binding::Custom(Message::SqlCompletionNext))
            }
            Key::Named(keyboard::key::Named::ArrowUp) => {
                Some(text_editor::Binding::Custom(Message::SqlCompletionPrevious))
            }
            Key::Named(keyboard::key::Named::Enter) | Key::Named(keyboard::key::Named::Tab) => {
                Some(text_editor::Binding::Custom(Message::SqlCompletionAccepted))
            }
            Key::Named(keyboard::key::Named::Escape) => {
                Some(text_editor::Binding::Custom(Message::SqlCompletionClosed))
            }
            _ => text_editor::Binding::from_key_press(press),
        }
    } else {
        text_editor::Binding::from_key_press(press)
    }
}

fn sql_completion_popup<'a>(
    items: &'a [SqlCompletionItem],
    selected_index: usize,
) -> Element<'a, Message> {
    let mut rows = column![].spacing(0);
    for (index, item) in items.iter().take(50).enumerate() {
        rows = rows.push(sql_completion_row(item, index == selected_index, index));
    }

    container(
        container(scrollable(rows).height(Length::Shrink))
            .width(360)
            .max_height(336)
            .padding(theme::spacing::XS)
            .style(theme::sql_completion_popup),
    )
    .padding([42.0, 24.0])
    .width(Length::Shrink)
    .height(Length::Shrink)
    .into()
}

fn sql_completion_row<'a>(
    item: &'a SqlCompletionItem,
    selected: bool,
    index: usize,
) -> Element<'a, Message> {
    let kind = match item.kind {
        SqlCompletionKind::Keyword => "K",
        SqlCompletionKind::Table => "T",
        SqlCompletionKind::Column => "C",
        SqlCompletionKind::Alias => "A",
        SqlCompletionKind::Function => "F",
    };
    mouse_area(
        button(
            row![
                container(
                    text(kind)
                        .font(theme::mono())
                        .size(theme::typography::LABEL)
                        .style(theme::text_muted)
                )
                .width(22),
                text(item.label.as_str())
                    .font(theme::mono())
                    .size(theme::typography::BODY)
                    .style(theme::text_normal)
                    .width(Length::Fill),
                text(item.detail.as_str())
                    .font(theme::mono())
                    .size(theme::typography::LABEL)
                    .style(theme::text_muted),
            ]
            .spacing(theme::spacing::SM)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .height(26)
        .padding([3.0, theme::spacing::SM])
        .style(move |theme, status| theme::sql_completion_button(theme, selected, status))
        .on_press(Message::SqlCompletionSelected(index)),
    )
    .on_press(Message::SqlCompletionSelected(index))
    .into()
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
    let table = TableModel::collection_with_relations(
        collection_id,
        &model.documents,
        model.collection_table_sort.as_ref(),
        Some(&model.relation_index),
    );
    let property_count = table.columns.len().saturating_sub(1);
    let schema = model.selected_collection_schema();
    let content = match model.collection_panel {
        CollectionPanel::Data => {
            if table.rows.is_empty() {
                empty_collection_view()
            } else {
                table_view(model, table)
            }
        }
        CollectionPanel::Schema => schema
            .map(|schema| schema_view(model, schema))
            .unwrap_or_else(empty_schema_view),
    };

    container(
        column![
            collection_header(collection, property_count, model),
            content
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
    model: &'a ShellModel,
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
            collection_panel_switch(model),
            text(format!("{property_count} propriedades"))
                .size(theme::typography::BODY)
                .style(theme::text_muted),
        ]
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .into()
}

fn collection_panel_switch(model: &ShellModel) -> Element<'_, Message> {
    row![
        collection_panel_button("Dados", CollectionPanel::Data, model),
        collection_panel_button("Schema", CollectionPanel::Schema, model),
    ]
    .spacing(theme::spacing::XS)
    .align_y(Alignment::Center)
    .into()
}

fn collection_panel_button(
    label: &'static str,
    panel: CollectionPanel,
    model: &ShellModel,
) -> Element<'static, Message> {
    button(text(label).size(theme::typography::LABEL))
        .height(28)
        .padding([0.0, theme::spacing::MD])
        .style(if model.collection_panel == panel {
            theme::button_selected
        } else {
            theme::button_toolbar
        })
        .on_press(Message::CollectionPanelSelected(panel))
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

fn empty_schema_view<'a>() -> Element<'a, Message> {
    container(
        text("Nenhum schema disponível para esta Collection.")
            .size(theme::typography::BODY)
            .style(theme::text_muted),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn schema_view<'a>(model: &'a ShellModel, schema: &'a CollectionSchema) -> Element<'a, Message> {
    let source = match schema.source {
        SchemaSource::Inferred => "Schema inferido",
        SchemaSource::Explicit => "Schema explícito + observações inferidas",
    };
    let warning = model
        .schema_catalog
        .warnings
        .first()
        .map(|warning| warning.message.as_str());

    let mut content = column![row![
        text(format!("{} documentos", schema.document_count))
            .size(theme::typography::BODY)
            .style(theme::text_muted),
        text(source)
            .size(theme::typography::BODY)
            .style(theme::text_muted),
    ]
    .spacing(theme::spacing::MD)
    .align_y(Alignment::Center),]
    .spacing(theme::spacing::SM)
    .height(Length::Fill);

    if let Some(warning) = warning {
        content = content.push(
            container(
                text(warning)
                    .size(theme::typography::BODY)
                    .wrapping(Wrapping::Word)
                    .style(theme::text_warning),
            )
            .padding([6.0, theme::spacing::MD])
            .width(Length::Fill)
            .style(theme::elevated),
        );
    }

    content = content.push(schema_onboarding_panel(model));
    content = content.push(schema_grid(model, schema));

    if let Some(field) = model.selected_schema_field() {
        content = content.push(schema_field_details(field));
    }

    content.into()
}

fn schema_onboarding_panel<'a>(model: &'a ShellModel) -> Element<'a, Message> {
    match &model.schema_catalog.explicit_schema {
        ExplicitSchemaState::Absent => {
            let has_collections = model
                .schema_catalog
                .collections
                .iter()
                .any(|collection| collection.document_count > 0);
            let mut action = button(
                text(if has_collections {
                    "Criar schema explícito"
                } else {
                    "Nenhuma Collection disponível para gerar schema"
                })
                .size(theme::typography::BODY),
            )
            .padding([5.0, 10.0])
            .style(if has_collections {
                theme::button_toolbar
            } else {
                theme::button_ghost
            });
            if has_collections {
                action = action.on_press(Message::SchemaCreateRequested);
            }
            container(
                row![
                    column![
                        text("Schema inferido")
                            .size(theme::typography::BODY)
                            .style(theme::text_normal),
                        text("O FlokinMD detectou esta estrutura automaticamente a partir dos seus documentos. Crie um schema explícito para definir tipos e campos obrigatórios.")
                            .size(theme::typography::BODY)
                            .wrapping(Wrapping::Word)
                            .style(theme::text_muted),
                    ]
                    .spacing(theme::spacing::XXS)
                    .width(Length::Fill),
                    action,
                ]
                .spacing(theme::spacing::MD)
                .align_y(Alignment::Center),
            )
            .padding([8.0, theme::spacing::MD])
            .width(Length::Fill)
            .style(theme::elevated)
            .into()
        }
        ExplicitSchemaState::Loaded(_) => container(
            row![
                column![
                    text("Schema explícito")
                        .size(theme::typography::BODY)
                        .style(theme::text_normal),
                    text(flokin_core::SCHEMA_FILE_NAME)
                        .font(theme::mono())
                        .size(theme::typography::LABEL)
                        .style(theme::text_muted),
                ]
                .spacing(theme::spacing::XXS)
                .width(Length::Fill),
                button(text("Abrir schema").size(theme::typography::BODY))
                    .padding([5.0, 10.0])
                    .style(theme::button_toolbar)
                    .on_press(Message::SchemaOpenRequested),
            ]
            .spacing(theme::spacing::MD)
            .align_y(Alignment::Center),
        )
        .padding([8.0, theme::spacing::MD])
        .width(Length::Fill)
        .style(theme::elevated)
        .into(),
        ExplicitSchemaState::Invalid(_) => container(
            row![
                column![
                    text("Schema explícito inválido")
                        .size(theme::typography::BODY)
                        .style(theme::text_warning),
                    text(flokin_core::SCHEMA_FILE_NAME)
                        .font(theme::mono())
                        .size(theme::typography::LABEL)
                        .style(theme::text_muted),
                ]
                .spacing(theme::spacing::XXS)
                .width(Length::Fill),
                button(text("Abrir schema").size(theme::typography::BODY))
                    .padding([5.0, 10.0])
                    .style(theme::button_toolbar)
                    .on_press(Message::SchemaOpenRequested),
            ]
            .spacing(theme::spacing::MD)
            .align_y(Alignment::Center),
        )
        .padding([8.0, theme::spacing::MD])
        .width(Length::Fill)
        .style(theme::elevated)
        .into(),
    }
}

fn schema_grid<'a>(model: &'a ShellModel, schema: &'a CollectionSchema) -> Element<'a, Message> {
    let widths = [260.0, 150.0, 110.0, 130.0];
    let width = data_grid::grid_width(true, widths.into_iter());
    let mut rows = column![schema_header(widths, width)].spacing(0);

    for (row_index, field) in schema.fields.iter().enumerate() {
        let selected = model.selected_schema_field.as_ref()
            == Some(&(schema.collection_id.clone(), field.name.clone()));
        let mut cells = row![data_grid::row_gutter(row_index, selected)]
            .spacing(0)
            .align_y(Alignment::Center);
        cells = cells.push(schema_cell(
            schema_field_label(field),
            widths[0],
            iced::alignment::Horizontal::Left,
            selected,
            false,
        ));
        cells = cells.push(schema_cell(
            schema_type_label(field),
            widths[1],
            iced::alignment::Horizontal::Left,
            selected,
            field.divergent || field.field_type == SchemaType::Mixed,
        ));
        cells = cells.push(schema_cell(
            if field.required {
                "✓".to_owned()
            } else {
                "✕".to_owned()
            },
            widths[2],
            iced::alignment::Horizontal::Center,
            selected,
            false,
        ));
        cells = cells.push(schema_cell(
            format!("{} / {}", field.observed_count, field.total_documents),
            widths[3],
            iced::alignment::Horizontal::Center,
            selected,
            false,
        ));

        rows = rows.push(
            button(container(cells).width(width))
                .width(width)
                .height(data_grid::ROW_HEIGHT)
                .padding(0)
                .style(move |theme, status| {
                    theme::data_row_button(theme, row_index, selected, status)
                })
                .on_press(Message::SchemaFieldSelected {
                    collection_id: schema.collection_id.clone(),
                    field_name: field.name.clone(),
                }),
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

fn schema_header<'a>(widths: [f32; 4], width: f32) -> Element<'a, Message> {
    let mut header = row![data_grid::header_gutter()]
        .spacing(0)
        .align_y(Alignment::Center);
    for (label, width) in [
        ("FIELD", widths[0]),
        ("TYPE", widths[1]),
        ("REQUIRED", widths[2]),
        ("PRESENT", widths[3]),
    ] {
        header = header.push(data_grid::header_cell(
            text(label)
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
            width,
        ));
    }

    container(header)
        .width(width)
        .height(data_grid::HEADER_HEIGHT)
        .style(theme::data_header)
        .into()
}

fn schema_cell<'a>(
    value: String,
    width: f32,
    alignment: iced::alignment::Horizontal,
    selected: bool,
    warning: bool,
) -> Element<'a, Message> {
    let style = if warning {
        theme::text_warning
    } else if selected {
        theme::text_accent
    } else {
        theme::text_normal
    };
    data_grid::cell(
        text(value)
            .size(theme::typography::BODY)
            .font(theme::mono())
            .style(style),
        width,
        alignment,
    )
}

fn schema_field_label(field: &SchemaField) -> String {
    if field.structural {
        format!("{} · derivado", field.name)
    } else {
        field.name.clone()
    }
}

fn schema_type_label(field: &SchemaField) -> String {
    let mut label = field.field_type.label().to_owned();
    if field.divergent || field.field_type == SchemaType::Mixed {
        label.push_str("  ⚠");
    }
    label
}

fn schema_field_details<'a>(field: &'a SchemaField) -> Element<'a, Message> {
    let observed = if field.observed_types.is_empty() {
        String::from("Unknown")
    } else {
        field
            .observed_types
            .iter()
            .map(|observed| format!("{} {}", observed.field_type.label(), observed.count))
            .collect::<Vec<_>>()
            .join(" · ")
    };
    let declared = field
        .declared_type
        .map(|field_type| field_type.label())
        .unwrap_or("Não declarado");
    let structural = if field.structural {
        " · campo estrutural/derivado"
    } else {
        ""
    };

    container(
        column![
            text("FIELD")
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
            text(format!("{}{}", field.name, structural))
                .size(theme::typography::TITLE)
                .style(theme::text_accent),
            row![
                schema_detail_item("Type", field.field_type.label().to_owned()),
                schema_detail_item(
                    "Required",
                    if field.required { "Sim" } else { "Não" }.to_owned(),
                ),
                schema_detail_item(
                    "Present in",
                    format!(
                        "{} / {} documents",
                        field.observed_count, field.total_documents
                    ),
                ),
                schema_detail_item("Null values", field.null_count.to_string()),
                schema_detail_item("Declared", declared.to_owned()),
            ]
            .spacing(theme::spacing::XL)
            .align_y(Alignment::Center),
            text(format!("Observed types: {observed}"))
                .font(theme::mono())
                .size(theme::typography::BODY)
                .style(if field.divergent {
                    theme::text_warning
                } else {
                    theme::text_muted
                }),
        ]
        .spacing(theme::spacing::XS),
    )
    .padding(theme::spacing::MD)
    .width(Length::Fill)
    .style(theme::elevated)
    .into()
}

fn schema_detail_item<'a>(label: &'static str, value: String) -> Element<'a, Message> {
    column![
        text(label)
            .size(theme::typography::LABEL)
            .style(theme::text_muted),
        text(value)
            .font(theme::mono())
            .size(theme::typography::BODY)
            .style(theme::text_normal),
    ]
    .spacing(theme::spacing::XXS)
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

fn markdown_editor_view<'a>(
    tab: &'a EditorTab,
    markdown_editor: &'a text_editor::Content,
    markdown_preview: &'a [markdown::Item],
    app_theme: AppTheme,
    _model: &'a ShellModel,
) -> Element<'a, Message> {
    let header = row![
        column![
            text(tab.title.as_str())
                .size(theme::typography::TITLE)
                .style(theme::text_accent),
            text(tab.relative_path.display().to_string())
                .font(theme::mono())
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
        ]
        .spacing(theme::spacing::XXS)
        .width(Length::Fill),
        if tab.kind == EditorTabKind::Markdown {
            editor_view_mode_controls(tab.view_mode)
        } else {
            row![].into()
        },
        save_button(tab),
    ]
    .spacing(theme::spacing::MD)
    .align_y(Alignment::Center);

    let mut content = column![header]
        .spacing(theme::spacing::SM)
        .height(Length::Fill);

    if let Some(error) = tab.save_error.as_ref() {
        content = content.push(
            container(
                text(error.as_str())
                    .size(theme::typography::BODY)
                    .style(theme::text_warning),
            )
            .padding([6.0, theme::spacing::MD])
            .width(Length::Fill)
            .style(theme::elevated),
        );
    }

    if let Some(conflict) = tab.external_conflict.as_ref() {
        content = content.push(external_conflict_banner(conflict));
    }

    content = content.push(markdown_document_body(
        tab,
        markdown_editor,
        markdown_preview,
        app_theme,
    ));

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(theme::spacing::LG)
        .style(theme::editor)
        .into()
}

fn editor_view_mode_controls<'a>(active: EditorViewMode) -> Element<'a, Message> {
    row![
        editor_view_mode_button("Editar", EditorViewMode::Edit, active),
        editor_view_mode_button("Dividido", EditorViewMode::Split, active),
        editor_view_mode_button("Prévia", EditorViewMode::Preview, active),
    ]
    .spacing(theme::spacing::XS)
    .align_y(Alignment::Center)
    .into()
}

fn editor_view_mode_button<'a>(
    label: &'static str,
    mode: EditorViewMode,
    active: EditorViewMode,
) -> Element<'a, Message> {
    button(text(label).size(theme::typography::LABEL))
        .height(28)
        .padding([0.0, theme::spacing::MD])
        .style(if mode == active {
            theme::button_selected
        } else {
            theme::button_toolbar
        })
        .on_press(Message::EditorViewModeSelected(mode))
        .into()
}

fn save_button<'a>(tab: &'a EditorTab) -> Element<'a, Message> {
    let control = button(
        row![
            widgets::icon(theme::Icon::Save, theme::icons::TOOLBAR, !tab.dirty),
            text("Salvar").size(theme::typography::BODY)
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center),
    )
    .padding([5.0, 10.0])
    .style(if tab.dirty {
        theme::button_toolbar
    } else {
        theme::button_ghost
    });

    let control = if tab.dirty {
        control.on_press(Message::EditorSaveRequested)
    } else {
        control
    };

    iced::widget::tooltip(
        control,
        text("Salvar (Ctrl+S)"),
        iced::widget::tooltip::Position::Bottom,
    )
    .into()
}

fn external_conflict_banner<'a>(conflict: &'a EditorExternalConflict) -> Element<'a, Message> {
    let message = match conflict {
        EditorExternalConflict::Modified(_) => "O arquivo foi alterado externamente.",
        EditorExternalConflict::Deleted => "O arquivo foi removido externamente.",
    };

    container(
        row![
            text(message)
                .size(theme::typography::BODY)
                .style(theme::text_warning)
                .width(Length::Fill),
            button(text("Recarregar do disco"))
                .padding([5.0, 10.0])
                .style(theme::button_toolbar)
                .on_press(Message::EditorExternalReload),
            button(text("Manter minhas alterações"))
                .padding([5.0, 10.0])
                .style(theme::button_toolbar)
                .on_press(Message::EditorExternalKeep),
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center),
    )
    .padding([6.0, theme::spacing::MD])
    .width(Length::Fill)
    .style(theme::elevated)
    .into()
}

fn markdown_document_body<'a>(
    tab: &'a EditorTab,
    markdown_editor: &'a text_editor::Content,
    markdown_preview: &'a [markdown::Item],
    app_theme: AppTheme,
) -> Element<'a, Message> {
    match tab.view_mode {
        _ if tab.kind == EditorTabKind::Schema => markdown_editor_body(tab, markdown_editor),
        EditorViewMode::Edit => markdown_editor_body(tab, markdown_editor),
        EditorViewMode::Preview => markdown_preview_body(markdown_preview, app_theme),
        EditorViewMode::Split => {
            markdown_split_body(tab, markdown_editor, markdown_preview, app_theme)
        }
    }
}

fn markdown_split_body<'a>(
    tab: &'a EditorTab,
    markdown_editor: &'a text_editor::Content,
    markdown_preview: &'a [markdown::Item],
    app_theme: AppTheme,
) -> Element<'a, Message> {
    iced::widget::responsive(move |size| {
        let total_width = size.width.max(1.0);
        let splitter_width = theme::sizes::SPLITTER_HIT_AREA;
        let available = (total_width - splitter_width).max(1.0);
        let minimum = 280.0;
        let left_width = if available < minimum * 2.0 {
            available * 0.5
        } else {
            (available * (f32::from(tab.split_ratio) / 1000.0)).clamp(minimum, available - minimum)
        };
        let right_width = (available - left_width).max(1.0);

        row![
            container(markdown_editor_body(tab, markdown_editor))
                .width(left_width)
                .height(Length::Fill),
            markdown_splitter(),
            container(markdown_preview_body(markdown_preview, app_theme))
                .width(right_width)
                .height(Length::Fill),
        ]
        .spacing(0)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    })
    .height(Length::Fill)
    .width(Length::Fill)
    .into()
}

fn markdown_splitter<'a>() -> Element<'a, Message> {
    mouse_area(
        container("")
            .width(theme::sizes::SPLITTER_HIT_AREA)
            .height(Length::Fill)
            .style(theme::splitter),
    )
    .on_press(Message::SplitterPressed(SplitterKind::MarkdownPreview, 0.0))
    .interaction(iced::mouse::Interaction::ResizingHorizontally)
    .into()
}

fn markdown_preview_body<'a>(
    markdown_preview: &'a [markdown::Item],
    app_theme: AppTheme,
) -> Element<'a, Message> {
    let body: Element<'a, Message> = if markdown_preview.is_empty() {
        container(
            text("Prévia vazia.")
                .size(theme::typography::BODY)
                .style(theme::text_muted),
        )
        .width(Length::Fill)
        .padding(theme::spacing::LG)
        .into()
    } else {
        markdown::view(
            markdown_preview,
            theme::markdown_preview_settings(app_theme),
        )
        .map(Message::MarkdownLinkClicked)
    };

    container(
        scrollable(
            container(body)
                .width(Length::Fill)
                .padding([theme::spacing::XL, theme::spacing::XXL]),
        )
        .direction(Direction::Vertical(Scrollbar::default()))
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::markdown_preview)
    .into()
}

fn markdown_editor_body<'a>(
    _tab: &'a EditorTab,
    markdown_editor: &'a text_editor::Content,
) -> Element<'a, Message> {
    let editor = stack![
        editor_zebra_background(),
        text_editor(markdown_editor)
            .placeholder("Arquivo vazio.")
            .on_action(Message::MarkdownEditorAction)
            .key_binding(markdown_editor_key_binding)
            .font(theme::mono())
            .size(theme::typography::EDITOR)
            .line_height(LineHeight::Relative(theme::sizes::EDITOR_LINE_HEIGHT_RATIO,))
            .height(Length::Fill)
            .padding(theme::spacing::MD)
            .wrapping(Wrapping::None)
            .style(theme::markdown_text_editor)
    ]
    .height(Length::Fill)
    .width(Length::Fill);

    row![line_number_gutter(markdown_editor.line_count()), editor]
        .spacing(0)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

fn editor_zebra_background<'a>() -> Element<'a, Message> {
    iced::widget::responsive(|size| {
        let line_height = editor_line_height_px();
        let usable_height = (size.height - theme::spacing::MD * 2.0).max(0.0);
        let visible_rows = (usable_height / line_height).ceil() as usize + 1;
        let mut rows = column![].spacing(0);

        for index in 0..visible_rows.max(1) {
            rows = rows.push(
                container("")
                    .width(Length::Fill)
                    .height(line_height)
                    .style(move |theme| theme::editor_row(theme, index)),
            );
        }

        container(rows)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([theme::spacing::MD, 0.0])
            .into()
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn editor_line_height_px() -> f32 {
    theme::typography::EDITOR as f32 * theme::sizes::EDITOR_LINE_HEIGHT_RATIO
}

fn line_number_gutter<'a>(line_count: usize) -> Element<'a, Message> {
    let mut lines = column![].spacing(0);
    for index in 1..=line_count.max(1) {
        lines = lines.push(
            container(
                text(format!("{index:>4}"))
                    .font(theme::mono())
                    .size(theme::typography::EDITOR_LINE_NUMBER)
                    .line_height(LineHeight::Relative(theme::sizes::EDITOR_LINE_HEIGHT_RATIO))
                    .style(theme::text_muted),
            )
            .width(Length::Fill)
            .style(move |theme| theme::editor_row(theme, index - 1)),
        );
    }

    container(scrollable(lines).height(Length::Fill))
        .width(theme::sizes::EDITOR_GUTTER_WIDTH)
        .height(Length::Fill)
        .padding([theme::spacing::MD, theme::spacing::SM])
        .style(theme::gutter)
        .into()
}

fn markdown_editor_key_binding(
    press: text_editor::KeyPress,
) -> Option<text_editor::Binding<Message>> {
    if press.modifiers.control()
        && press
            .key
            .to_latin(press.physical_key)
            .or_else(|| press.modified_key.to_latin(press.physical_key))
            == Some('s')
    {
        Some(text_editor::Binding::Custom(Message::EditorSaveRequested))
    } else if press.modifiers.control()
        && press
            .key
            .to_latin(press.physical_key)
            .or_else(|| press.modified_key.to_latin(press.physical_key))
            == Some('w')
    {
        Some(text_editor::Binding::Custom(
            Message::EditorCloseActiveRequested,
        ))
    } else {
        text_editor::Binding::from_key_press(press)
    }
}

fn empty_document_area<'a>() -> Element<'a, Message> {
    container(
        text("Selecione um documento Markdown para ver o conteúdo.")
            .size(theme::typography::BODY)
            .style(theme::text_muted),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(theme::spacing::XXL)
    .style(theme::editor)
    .into()
}

#[cfg(test)]
mod tests {
    use iced::{keyboard, keyboard::Key, widget::text_editor};

    use super::{markdown_editor_key_binding, sql_editor_key_binding};
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

    fn latin_key_press(character: char, modifiers: keyboard::Modifiers) -> text_editor::KeyPress {
        text_editor::KeyPress {
            key: Key::Character(character.to_string().into()),
            modified_key: Key::Character(character.to_string().into()),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyS),
            modifiers,
            text: Some(character.to_string().into()),
            status: text_editor::Status::Focused { is_hovered: true },
        }
    }

    #[test]
    fn ctrl_enter_in_sql_editor_publishes_execute_message() {
        assert_eq!(
            sql_editor_key_binding(key_press(keyboard::Modifiers::CTRL), false),
            Some(text_editor::Binding::Custom(Message::SqlExecute))
        );
        assert_eq!(
            sql_editor_key_binding(key_press(keyboard::Modifiers::CTRL), true),
            Some(text_editor::Binding::Custom(Message::SqlExecute))
        );
    }

    #[test]
    fn ctrl_s_in_markdown_editor_publishes_save_message() {
        assert_eq!(
            markdown_editor_key_binding(latin_key_press('s', keyboard::Modifiers::CTRL)),
            Some(text_editor::Binding::Custom(Message::EditorSaveRequested))
        );
    }

    #[test]
    fn ctrl_w_in_markdown_editor_publishes_close_message() {
        let mut press = latin_key_press('w', keyboard::Modifiers::CTRL);
        press.physical_key = keyboard::key::Physical::Code(keyboard::key::Code::KeyW);
        assert_eq!(
            markdown_editor_key_binding(press),
            Some(text_editor::Binding::Custom(
                Message::EditorCloseActiveRequested
            ))
        );
    }

    #[test]
    fn plain_enter_keeps_text_editor_newline_behavior() {
        assert_eq!(
            sql_editor_key_binding(key_press(keyboard::Modifiers::NONE), false),
            Some(text_editor::Binding::Enter)
        );
    }

    #[test]
    fn plain_enter_accepts_completion_when_popup_is_open() {
        assert_eq!(
            sql_editor_key_binding(key_press(keyboard::Modifiers::NONE), true),
            Some(text_editor::Binding::Custom(Message::SqlCompletionAccepted))
        );
    }

    #[test]
    fn tab_accepts_completion_when_popup_is_open() {
        let mut press = key_press(keyboard::Modifiers::NONE);
        press.key = Key::Named(keyboard::key::Named::Tab);
        press.modified_key = Key::Named(keyboard::key::Named::Tab);
        assert_eq!(
            sql_editor_key_binding(press, true),
            Some(text_editor::Binding::Custom(Message::SqlCompletionAccepted))
        );
    }

    #[test]
    fn arrows_and_escape_control_open_completion_popup() {
        let mut down = key_press(keyboard::Modifiers::NONE);
        down.key = Key::Named(keyboard::key::Named::ArrowDown);
        down.modified_key = Key::Named(keyboard::key::Named::ArrowDown);
        assert_eq!(
            sql_editor_key_binding(down, true),
            Some(text_editor::Binding::Custom(Message::SqlCompletionNext))
        );

        let mut up = key_press(keyboard::Modifiers::NONE);
        up.key = Key::Named(keyboard::key::Named::ArrowUp);
        up.modified_key = Key::Named(keyboard::key::Named::ArrowUp);
        assert_eq!(
            sql_editor_key_binding(up, true),
            Some(text_editor::Binding::Custom(Message::SqlCompletionPrevious))
        );

        let mut escape = key_press(keyboard::Modifiers::NONE);
        escape.key = Key::Named(keyboard::key::Named::Escape);
        escape.modified_key = Key::Named(keyboard::key::Named::Escape);
        assert_eq!(
            sql_editor_key_binding(escape, true),
            Some(text_editor::Binding::Custom(Message::SqlCompletionClosed))
        );
    }
}
