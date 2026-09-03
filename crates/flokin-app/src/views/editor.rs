use flokin_core::{
    BulkEditChangeStatus, BulkEditOperationKind, BulkEditStep, BulkEditValueType, CollectionPanel,
    CollectionSchema, EditorExternalConflict, EditorTab, EditorTabKind, EditorViewMode,
    ExplicitSchemaState, SchemaField, SchemaSource, SchemaType, ShellModel, SortDirection,
    SqlColumnType, SqlExplorerMode, SqlQueryResult, SqlValue, SqlWritePlan, TableCell, TableColumn,
    TableModel, TableValueType,
};
use iced::widget::{
    button, column, container, markdown, mouse_area, pick_list, row, scrollable,
    scrollable::{Direction, Scrollbar},
    stack, text,
    text::{LineHeight, Wrapping},
    text_editor, text_input, Space,
};
use iced::{alignment, keyboard, keyboard::Key, Alignment, Element, Length, Padding};

use crate::{
    i18n::I18nCatalog,
    message::{Message, SplitterKind},
    theme::{self, AppTheme},
    views::{data_grid, health as health_view},
    widgets,
};

pub const ACTIVE_MARKDOWN_EDITOR_ID: &str = "active-markdown-editor";

pub fn tabs(model: &ShellModel) -> Element<'_, Message> {
    if model.sql_explorer.open {
        return container(row![widgets::tab_button(
            "SQL Explorer",
            true,
            Message::SqlExplorerOpened,
        )])
        .height(theme::sizes::TAB_HEIGHT)
        .padding([theme::spacing::XS, theme::spacing::MD])
        .style(theme::document_header)
        .into();
    }

    if let Some(collection) = model.selected_collection() {
        return container(row![widgets::tab_button(
            collection.display_name.as_str(),
            true,
            Message::MockAction,
        )])
        .height(theme::sizes::TAB_HEIGHT)
        .padding([theme::spacing::XS, theme::spacing::MD])
        .style(theme::document_header)
        .into();
    }

    if !model.editor.tabs.is_empty() {
        let mut tabs = row![].spacing(theme::spacing::XS);
        for tab in &model.editor.tabs {
            tabs = tabs.push(editor_tab(model, tab));
        }
        return container(scrollable(tabs).direction(Direction::Horizontal(Scrollbar::default())))
            .height(theme::sizes::TAB_HEIGHT)
            .padding([theme::spacing::XS, theme::spacing::MD])
            .style(theme::document_header)
            .into();
    }

    container(row![])
        .height(theme::sizes::TAB_HEIGHT)
        .padding([theme::spacing::XS, theme::spacing::MD])
        .style(theme::document_header)
        .into()
}

fn editor_tab<'a>(model: &'a ShellModel, tab: &'a EditorTab) -> Element<'a, Message> {
    let active = model.editor.active_path.as_ref() == Some(&tab.document_path);
    let label = editor_tab_label(model, tab);
    let dirty = if tab.dirty { " ●" } else { "" };
    let style = if active {
        theme::button_tab_selected
    } else {
        theme::button_tab
    };

    row![
        button(
            container(
                text(format!("{label}{dirty}"))
                    .size(theme::typography::BODY)
                    .line_height(LineHeight::Relative(1.0))
                    .style(if active {
                        theme::text_accent
                    } else {
                        theme::text_normal
                    }),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(alignment::Vertical::Center),
        )
        .height(theme::sizes::TAB_BUTTON_HEIGHT)
        .padding([0.0, theme::spacing::LG])
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
        .style(theme::button_tab)
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
    markdown_editor_scroll_y: f32,
    markdown_preview: &'a [markdown::Item],
    sql_editor_height: f32,
    collection_page: usize,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    if model.sql_explorer.open {
        return sql_explorer_view(model, sql_editor, sql_editor_height, i18n);
    }

    if let Some(collection) = model.selected_collection() {
        return collection_view(model, collection.id.as_str(), collection_page, i18n);
    }

    if let Some(tab) = model.active_editor_tab() {
        return markdown_editor_view(
            tab,
            markdown_editor,
            markdown_editor_scroll_y,
            markdown_preview,
            app_theme,
            model,
            i18n,
        );
    }

    if model.current_workspace.is_some() && model.documents.is_empty() {
        return empty_workspace_view(model, i18n);
    }

    empty_document_area(i18n)
}

fn empty_workspace_view<'a>(model: &'a ShellModel, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    let workspace = model.workspace_display();
    container(
        column![
            text(i18n.tr("explorer-no-markdown"))
                .size(theme::typography::TITLE)
                .style(theme::text_normal),
            text(i18n.tr_with(
                "editor-scanned-folder",
                &[("path", workspace.path.as_str().into())]
            ))
                .font(theme::mono())
                .size(theme::typography::BODY)
                .style(theme::text_muted),
            text(i18n.tr("editor-empty-workspace-hint"))
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
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let update_mode = model.sql_explorer.mode == SqlExplorerMode::Update;
    let action_label = if model.sql_explorer.running {
        if update_mode {
            i18n.tr("sql-reviewing")
        } else {
            i18n.tr("sql-running")
        }
    } else if update_mode {
        i18n.tr("sql-review-update")
    } else {
        i18n.tr("sql-run")
    };
    let header = row![
        text(i18n.tr("sql-query-tab"))
            .size(theme::typography::TITLE)
            .style(theme::text_normal),
        sql_mode_button(
            i18n.tr("sql-mode-query"),
            SqlExplorerMode::Query,
            model.sql_explorer.mode
        ),
        sql_mode_button(
            i18n.tr("sql-mode-update"),
            SqlExplorerMode::Update,
            model.sql_explorer.mode
        ),
        iced::widget::Space::new().width(Length::Fill),
        button(
            container(widgets::icon_text(
                theme::Icon::Terminal,
                action_label,
                theme::icons::TOOLBAR,
                false,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center),
        )
        .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT)
        .padding([0.0, 10.0])
        .style(theme::button_primary)
        .on_press(Message::SqlExecute),
        container(
            text("Ctrl+Enter")
                .font(theme::mono())
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
        )
        .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT)
        .align_y(alignment::Vertical::Center),
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);
    let context_text = if update_mode {
        i18n.tr("sql-update-context")
    } else {
        i18n.tr("sql-query-context")
    };
    let placeholder = if update_mode {
        "UPDATE projects\nSET status = 'archived'\nWHERE status = 'active';"
    } else {
        "SELECT *\nFROM projects\nLIMIT 100;"
    };

    let editor_widget = container(
        text_editor(sql_editor)
            .placeholder(placeholder)
            .on_action(Message::SqlEditorAction)
            .key_binding(sql_editor_key_binding)
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

    let results = sql_results(model, i18n);
    let editor_splitter = iced::widget::mouse_area(
        container("")
            .height(theme::sizes::SPLITTER_HIT_AREA)
            .width(Length::Fill)
            .style(theme::splitter),
    )
    .on_press(Message::SplitterPressed(SplitterKind::SqlEditor, 0.0))
    .interaction(iced::mouse::Interaction::ResizingVertically);
    let body = column![
        text(context_text)
            .size(theme::typography::LABEL)
            .style(theme::text_muted),
        editor_widget,
        editor_splitter,
        container(results).height(Length::Fill)
    ]
    .spacing(theme::spacing::SM)
    .height(Length::Fill)
    .width(Length::Fill);

    container(column![header, body].spacing(theme::spacing::MD))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(theme::spacing::MD)
        .style(theme::document_surface)
        .into()
}

fn sql_mode_button(
    label: String,
    mode: SqlExplorerMode,
    current: SqlExplorerMode,
) -> Element<'static, Message> {
    button(
        container(text(label).size(theme::typography::LABEL))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center),
    )
    .height(28)
    .padding([0.0, theme::spacing::MD])
    .style(if current == mode {
        theme::button_selected
    } else {
        theme::button_toolbar
    })
    .on_press(Message::SqlModeSelected(mode))
    .into()
}

fn sql_editor_key_binding(press: text_editor::KeyPress) -> Option<text_editor::Binding<Message>> {
    if press.modifiers.control() && matches!(press.key, Key::Named(keyboard::key::Named::Enter)) {
        Some(text_editor::Binding::Custom(Message::SqlExecute))
    } else {
        text_editor::Binding::from_key_press(press)
    }
}

fn sql_results<'a>(model: &'a ShellModel, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    let metadata = if model.sql_explorer.running {
        if model.sql_explorer.mode == SqlExplorerMode::Update {
            i18n.tr("sql-reviewing")
        } else {
            i18n.tr("sql-running")
        }
    } else if let Some(plan) = model.sql_explorer.write_plan.as_ref() {
        i18n.tr_with(
            "sql-preview-status",
            &[
                ("matched", plan.matched_rows.into()),
                ("changed", plan.affected_rows.into()),
            ],
        )
    } else if let Some(result) = model.sql_explorer.result.as_ref() {
        let mut text = i18n.tr_with(
            "sql-result-status",
            &[
                ("rows", result.rows.len().into()),
                ("ms", (result.elapsed.as_millis() as i64).into()),
            ],
        );
        if result.truncated {
            text.push_str(" • ");
            text.push_str(&i18n.tr("sql-results-limited"));
        }
        text
    } else {
        i18n.tr("sql-no-results")
    };

    let header = row![
        text(i18n.tr("sql-results"))
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
                text(i18n.tr("sql-error"))
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
    } else if let Some(plan) = model.sql_explorer.write_plan.as_ref() {
        sql_update_preview(model, plan, i18n)
    } else if let Some(result) = model.sql_explorer.result.as_ref() {
        result_grid(result, i18n)
    } else if let Some(result) = model.sql_explorer.last_result.as_ref() {
        container(
            text(result.as_str())
                .size(theme::typography::BODY)
                .style(theme::text_accent),
        )
        .padding(theme::spacing::MD)
        .width(Length::Fill)
        .style(theme::elevated)
        .into()
    } else {
        container(
            text(if model.sql_explorer.mode == SqlExplorerMode::Update {
                i18n.tr("sql-empty-update-preview")
            } else {
                i18n.tr("sql-empty-query-results")
            })
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

fn sql_update_preview<'a>(
    model: &'a ShellModel,
    plan: &'a SqlWritePlan,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let summary = plan.mutation_plan.summary();
    let mut list = column![].spacing(theme::spacing::SM);
    if plan.matched_rows == 0 {
        list = list.push(
            text(i18n.tr("sql-update-no-matches"))
                .size(theme::typography::BODY)
                .style(theme::text_muted),
        );
    } else if plan.affected_rows == 0 {
        list = list.push(
            text(i18n.tr_with(
                "sql-update-no-changes",
                &[("count", plan.matched_rows.into())],
            ))
            .size(theme::typography::BODY)
            .style(theme::text_muted),
        );
    }
    for warning in &plan.warnings {
        list = list.push(
            text(warning.as_str())
                .size(theme::typography::BODY)
                .style(theme::text_warning),
        );
    }
    for change in &plan.mutation_plan.changes {
        let status = match change.status {
            BulkEditChangeStatus::Changed => i18n.tr("change-status-changed"),
            BulkEditChangeStatus::NoChange => i18n.tr("change-status-no-change"),
            BulkEditChangeStatus::Blocked => i18n.tr("change-status-blocked"),
            BulkEditChangeStatus::Unsupported => i18n.tr("change-status-unsupported"),
        };
        let mut item = column![row![
            text(change.relative_path.display().to_string())
                .font(theme::mono())
                .size(theme::typography::LABEL)
                .style(theme::text_normal)
                .width(Length::Fill),
            text(status)
                .size(theme::typography::LABEL)
                .style(match change.status {
                    BulkEditChangeStatus::Changed => theme::text_accent,
                    BulkEditChangeStatus::NoChange => theme::text_muted,
                    BulkEditChangeStatus::Blocked | BulkEditChangeStatus::Unsupported => {
                        theme::text_warning
                    }
                }),
        ]
        .align_y(Alignment::Center)]
        .spacing(theme::spacing::XXS);
        for property_change in &change.property_changes {
            if let Some(before) = property_change.before.as_ref() {
                item = item.push(
                    text(format!("- {before}"))
                        .font(theme::mono())
                        .size(theme::typography::LABEL)
                        .style(theme::text_warning),
                );
            }
            if let Some(after) = property_change.after.as_ref() {
                item = item.push(
                    text(format!("+ {after}"))
                        .font(theme::mono())
                        .size(theme::typography::LABEL)
                        .style(theme::text_accent),
                );
            }
        }
        if let Some(reason) = change.reason.as_ref() {
            item = item.push(
                text(reason.as_str())
                    .size(theme::typography::LABEL)
                    .style(theme::text_muted),
            );
        }
        list = list.push(
            container(item)
                .padding(theme::spacing::SM)
                .style(theme::surface),
        );
    }
    let count = summary.changed;
    let label = i18n.tr_with("apply-changes", &[("count", count.into())]);
    let apply = button(text(label).size(theme::typography::LABEL))
        .height(34)
        .padding([0.0, theme::spacing::MD])
        .style(
            if plan.mutation_plan.can_apply() && !model.sql_explorer.stale {
                theme::button_selected
            } else {
                theme::button_toolbar
            },
        );
    let apply = if plan.mutation_plan.can_apply() && !model.sql_explorer.stale {
        apply.on_press(Message::SqlUpdateApplyRequested)
    } else {
        apply
    };
    let stale_message: Element<'_, Message> = if model.sql_explorer.stale {
        text(i18n.tr("error-stale-preview"))
            .size(theme::typography::BODY)
            .style(theme::text_warning)
            .into()
    } else {
        container("").height(0).into()
    };
    let footer = row![
        button(text(i18n.tr("action-back")).size(theme::typography::LABEL))
            .height(34)
            .padding([0.0, theme::spacing::MD])
            .style(theme::button_toolbar)
            .on_press(Message::SqlUpdateBackToEditor),
        button(text(i18n.tr("action-cancel")).size(theme::typography::LABEL))
            .height(34)
            .padding([0.0, theme::spacing::MD])
            .style(theme::button_toolbar)
            .on_press(Message::SqlUpdatePreviewCanceled),
        container("").width(Length::Fill),
        apply,
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    column![
        text(i18n.tr("sql-review-update"))
            .size(theme::typography::TITLE)
            .style(theme::text_accent),
        text(plan.sql.as_str())
            .font(theme::mono())
            .size(theme::typography::LABEL)
            .style(theme::text_muted),
        row![
            text(i18n.tr_with(
                "sql-documents-match",
                &[("count", plan.matched_rows.into())],
            ))
                .size(theme::typography::LABEL),
            text(i18n.tr_with("changes-will-change", &[("count", summary.changed.into())]))
                .size(theme::typography::LABEL),
            text(i18n.tr_with("changes-no-change", &[("count", summary.no_change.into())]))
                .size(theme::typography::LABEL),
            text(i18n.tr_with(
                "changes-blocked",
                &[("count", (summary.blocked + summary.unsupported).into())],
            ))
            .size(theme::typography::LABEL),
        ]
        .spacing(theme::spacing::MD),
        stale_message,
        scrollable(list).height(Length::Fill),
        footer,
    ]
    .spacing(theme::spacing::MD)
    .height(Length::Fill)
    .into()
}

fn result_grid<'a>(result: &'a SqlQueryResult, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    if result.columns.is_empty() {
        return container(
            text(i18n.tr("sql-no-result-columns"))
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

fn collection_view<'a>(
    model: &'a ShellModel,
    collection_id: &'a str,
    collection_page: usize,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
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
                empty_collection_view(i18n)
            } else {
                table_view(model, table, collection_page, i18n)
            }
        }
        CollectionPanel::Schema => schema
            .map(|schema| schema_view(model, schema, i18n))
            .unwrap_or_else(|| empty_schema_view(i18n)),
    };

    let page = container(
        column![
            collection_header(collection.document_count, property_count, model, i18n),
            bulk_selection_toolbar(model, i18n),
            content
        ]
        .spacing(theme::spacing::LG),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(theme::spacing::XXL)
    .style(theme::editor);

    if model.bulk_edit.editor_open {
        stack![page, bulk_edit_overlay(model, i18n)].into()
    } else {
        page.into()
    }
}

fn collection_header<'a>(
    document_count: usize,
    property_count: usize,
    model: &'a ShellModel,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    container(
        row![
            column![text(i18n.tr_with(
                "status-documents",
                &[("count", document_count.into())],
            ))
                .size(theme::typography::BODY)
                .style(theme::text_muted),]
            .spacing(theme::spacing::XS)
            .width(Length::Fill),
            collection_panel_switch(model, i18n),
            text(i18n.tr_with(
                "data-properties",
                &[("count", property_count.into())],
            ))
                .size(theme::typography::BODY)
                .style(theme::text_muted),
        ]
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .into()
}

fn collection_panel_switch<'a>(
    model: &'a ShellModel,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    row![
        collection_panel_button(i18n.tr("data-panel-data"), CollectionPanel::Data, model),
        collection_panel_button(i18n.tr("data-panel-schema"), CollectionPanel::Schema, model),
    ]
    .spacing(theme::spacing::XS)
    .align_y(Alignment::Center)
    .into()
}

fn collection_panel_button(
    label: String,
    panel: CollectionPanel,
    model: &ShellModel,
) -> Element<'_, Message> {
    button(
        container(text(label).size(theme::typography::LABEL))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center),
    )
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

fn empty_collection_view<'a>(i18n: &'a I18nCatalog) -> Element<'a, Message> {
    container(
        text(i18n.tr("data-empty-collection"))
            .size(theme::typography::BODY)
            .style(theme::text_muted),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn empty_schema_view<'a>(i18n: &'a I18nCatalog) -> Element<'a, Message> {
    container(
        text(i18n.tr("data-empty-schema"))
            .size(theme::typography::BODY)
            .style(theme::text_muted),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn bulk_selection_toolbar<'a>(model: &'a ShellModel, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    let count = model.bulk_edit.selected_paths.len();
    if count == 0 || model.collection_panel != CollectionPanel::Data {
        return container("").height(0).into();
    }

    container(
        row![
            text(i18n.tr_with("bulk-selected-count", &[("count", count.into())]))
                .size(theme::typography::BODY)
                .style(theme::text_muted)
                .width(Length::Fill),
            button(text(i18n.tr("bulk-edit-title")).size(theme::typography::LABEL))
                .height(28)
                .padding([0.0, theme::spacing::MD])
                .style(theme::button_selected)
                .on_press(Message::BulkEditOpened),
            button(text(i18n.tr("bulk-clear-selection")).size(theme::typography::LABEL))
                .height(28)
                .padding([0.0, theme::spacing::MD])
                .style(theme::button_toolbar)
                .on_press(Message::BulkSelectionCleared),
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center),
    )
    .padding([theme::spacing::XS, theme::spacing::SM])
    .style(theme::surface)
    .into()
}

fn schema_view<'a>(
    model: &'a ShellModel,
    schema: &'a CollectionSchema,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let source = match schema.source {
        SchemaSource::Inferred => i18n.tr("schema-source-inferred"),
        SchemaSource::Explicit => i18n.tr("schema-source-explicit"),
    };
    let warning = model
        .schema_catalog
        .warnings
        .first()
        .map(|warning| warning.message.as_str());

    let mut content = column![row![
        text(i18n.tr_with(
            "status-documents",
            &[("count", schema.document_count.into())],
        ))
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

    content = content.push(schema_onboarding_panel(model, i18n));
    content = content.push(schema_grid(model, schema, i18n));

    if let Some(field) = model.selected_schema_field() {
        content = content.push(schema_field_details(field, i18n));
    }

    content.into()
}

fn schema_onboarding_panel<'a>(
    model: &'a ShellModel,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    match &model.schema_catalog.explicit_schema {
        ExplicitSchemaState::Absent => {
            let has_collections = model
                .schema_catalog
                .collections
                .iter()
                .any(|collection| collection.document_count > 0);
            let mut action = button(
                text(if has_collections {
                    i18n.tr("schema-create-title")
                } else {
                    i18n.tr("schema-none-available")
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
                        text(i18n.tr("schema-inferred-title"))
                            .size(theme::typography::BODY)
                            .style(theme::text_normal),
                        text(i18n.tr("schema-inferred-description"))
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
                    text(i18n.tr("schema-explicit-title"))
                        .size(theme::typography::BODY)
                        .style(theme::text_normal),
                    text(flokin_core::SCHEMA_FILE_NAME)
                        .font(theme::mono())
                        .size(theme::typography::LABEL)
                        .style(theme::text_muted),
                ]
                .spacing(theme::spacing::XXS)
                .width(Length::Fill),
                button(text(i18n.tr("schema-open")).size(theme::typography::BODY))
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
                    text(i18n.tr("schema-explicit-invalid"))
                        .size(theme::typography::BODY)
                        .style(theme::text_warning),
                    text(flokin_core::SCHEMA_FILE_NAME)
                        .font(theme::mono())
                        .size(theme::typography::LABEL)
                        .style(theme::text_muted),
                ]
                .spacing(theme::spacing::XXS)
                .width(Length::Fill),
                button(text(i18n.tr("schema-open")).size(theme::typography::BODY))
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

fn schema_grid<'a>(
    model: &'a ShellModel,
    schema: &'a CollectionSchema,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let widths = [260.0, 150.0, 110.0, 130.0];
    let width = data_grid::grid_width(true, widths.into_iter());
    let mut rows = column![schema_header(widths, width, i18n)].spacing(0);

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
            schema_field_type_label(field, i18n),
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

fn schema_header<'a>(
    widths: [f32; 4],
    width: f32,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let mut header = row![data_grid::header_gutter()]
        .spacing(0)
        .align_y(Alignment::Center);
    for (label, width) in [
        (i18n.tr("schema-field"), widths[0]),
        (i18n.tr("schema-type"), widths[1]),
        (i18n.tr("schema-required"), widths[2]),
        (i18n.tr("schema-present"), widths[3]),
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
    field.name.clone()
}

fn schema_field_type_label(field: &SchemaField, i18n: &I18nCatalog) -> String {
    let mut label = health_view::schema_type_label(field.field_type, i18n);
    if field.divergent || field.field_type == SchemaType::Mixed {
        label.push_str("  ⚠");
    }
    label
}

fn schema_field_details<'a>(field: &'a SchemaField, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    let observed = if field.observed_types.is_empty() {
        i18n.tr("schema-unknown")
    } else {
        field
            .observed_types
            .iter()
            .map(|observed| {
                format!(
                    "{} {}",
                    health_view::schema_type_label(observed.field_type, i18n),
                    observed.count
                )
            })
            .collect::<Vec<_>>()
            .join(" · ")
    };
    let declared = field
        .declared_type
        .map(|field_type| health_view::schema_type_label(field_type, i18n))
        .unwrap_or_else(|| i18n.tr("schema-not-declared"));
    let structural = if field.structural {
        i18n.tr("schema-structural-suffix")
    } else {
        String::new()
    };

    container(
        column![
            text(i18n.tr("schema-field"))
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
            text(format!("{}{}", field.name, structural))
                .size(theme::typography::TITLE)
                .style(theme::text_accent),
            row![
                schema_detail_item(
                    i18n.tr("schema-type"),
                    health_view::schema_type_label(field.field_type, i18n)
                ),
                schema_detail_item(
                    i18n.tr("schema-required"),
                    if field.required {
                        i18n.tr("action-yes")
                    } else {
                        i18n.tr("action-no")
                    },
                ),
                schema_detail_item(
                    i18n.tr("schema-present-in"),
                    i18n.tr_with(
                        "schema-present-ratio",
                        &[
                            ("observed", field.observed_count.into()),
                            ("total", field.total_documents.into()),
                        ],
                    ),
                ),
                schema_detail_item(i18n.tr("schema-null-values"), field.null_count.to_string()),
                schema_detail_item(i18n.tr("schema-declared"), declared),
            ]
            .spacing(theme::spacing::XL)
            .align_y(Alignment::Center),
            text(i18n.tr_with("schema-observed-types", &[("types", observed.into())]))
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

fn schema_detail_item<'a>(label: String, value: String) -> Element<'a, Message> {
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

const COLLECTION_PAGE_SIZE: usize = 50;

fn table_view<'a>(
    model: &'a ShellModel,
    table: TableModel,
    requested_page: usize,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let select_width = 34.0;
    let width = data_grid::grid_width(true, table.columns.iter().map(|column| column.width as f32))
        + select_width;
    let mut rows = column![table_header(&table.columns, model, width)].spacing(0);
    let total_pages = table.rows.len().div_ceil(COLLECTION_PAGE_SIZE).max(1);
    let page = requested_page.min(total_pages - 1);
    let start = page * COLLECTION_PAGE_SIZE;
    let total_rows = table.rows.len();

    for (row_index, row_model) in table
        .rows
        .into_iter()
        .enumerate()
        .skip(start)
        .take(COLLECTION_PAGE_SIZE)
    {
        let selected = model.selected_document_path.as_ref() == Some(&row_model.document_path);
        let bulk_selected = model
            .bulk_edit
            .selected_paths
            .contains(&row_model.document_path);
        let checkbox_path = row_model.document_path.clone();
        let mut cells = row![
            data_grid::selection_cell(
                checkbox_label(bulk_selected),
                select_width,
                Message::BulkSelectionToggled(checkbox_path),
                selected,
                row_index,
            ),
            data_grid::row_gutter(row_index, selected)
        ]
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

    container(column![
        scrollable(rows)
            .direction(Direction::Both {
                vertical: Scrollbar::default(),
                horizontal: Scrollbar::default(),
            })
            .width(Length::Fill)
            .height(Length::Fill),
        collection_pagination(page, total_pages, start, total_rows, i18n),
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn collection_pagination(
    page: usize,
    total_pages: usize,
    start: usize,
    total_rows: usize,
    i18n: &I18nCatalog,
) -> Element<'_, Message> {
    let end = (start + COLLECTION_PAGE_SIZE).min(total_rows);
    let previous = button(text(i18n.tr("action-previous")).size(theme::typography::LABEL))
        .height(28)
        .padding([0.0, theme::spacing::SM]);
    let previous = if page > 0 {
        previous
            .style(theme::button_toolbar)
            .on_press(Message::CollectionPagePrevious)
    } else {
        previous.style(theme::button_ghost)
    };
    let next = button(text(i18n.tr("action-next")).size(theme::typography::LABEL))
        .height(28)
        .padding([0.0, theme::spacing::SM]);
    let next = if page + 1 < total_pages {
        next.style(theme::button_toolbar)
            .on_press(Message::CollectionPageNext)
    } else {
        next.style(theme::button_ghost)
    };

    container(
        row![
            previous,
            text(i18n.tr_with(
                "pagination-status",
                &[
                    ("start", (start + 1).into()),
                    ("end", end.into()),
                    ("total", total_rows.into()),
                    ("page", (page + 1).into()),
                    ("pages", total_pages.into()),
                ],
            ))
            .font(theme::mono())
            .size(theme::typography::LABEL)
            .style(theme::text_muted),
            next,
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([theme::spacing::XS, 0.0])
    .into()
}

fn table_header<'a>(
    columns: &[TableColumn],
    model: &'a ShellModel,
    width: f32,
) -> Element<'a, Message> {
    let all_selected = if let Some(collection_id) = model.selected_collection.as_deref() {
        let docs = model.collection_documents(collection_id);
        !docs.is_empty()
            && docs
                .iter()
                .all(|document| model.bulk_edit.selected_paths.contains(&document.path))
    } else {
        false
    };
    let mut header = row![
        data_grid::selection_header(
            checkbox_label(all_selected),
            34.0,
            Message::BulkSelectAllVisible(!all_selected),
        ),
        data_grid::header_gutter()
    ]
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

fn checkbox_label(selected: bool) -> &'static str {
    if selected {
        "[x]"
    } else {
        "[ ]"
    }
}

fn bulk_edit_overlay<'a>(model: &'a ShellModel, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    let review = model.bulk_edit.step == BulkEditStep::Review;
    let header = row![
        column![
            text(i18n.tr("bulk-edit-title"))
                .size(theme::typography::TITLE)
                .style(theme::text_accent),
            text(i18n.tr_with(
                "bulk-selected-documents",
                &[("count", model.bulk_edit.selected_paths.len().into())],
            ))
            .size(theme::typography::BODY)
            .style(theme::text_muted),
        ]
        .spacing(theme::spacing::XXS)
        .width(Length::Fill),
        button(widgets::icon(theme::Icon::X, theme::icons::TOOLBAR, false))
            .width(30)
            .height(30)
            .padding(0)
            .style(theme::button_toolbar)
            .on_press(Message::BulkEditCanceled),
    ]
    .align_y(Alignment::Center);

    let steps = row![
        step_label(i18n.tr("bulk-step-configure"), !review),
        text("→")
            .size(theme::typography::LABEL)
            .style(theme::text_muted),
        step_label(i18n.tr("bulk-step-review"), review),
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    let content: Element<'_, Message> = if review {
        let plan = model.bulk_edit.plan.as_ref();
        if let Some(plan) = plan {
            bulk_preview(model, plan, i18n)
        } else {
            text(i18n.tr("bulk-preview-unavailable"))
                .style(theme::text_warning)
                .into()
        }
    } else {
        bulk_configure_content(model, i18n)
    };

    let footer = if review {
        bulk_review_footer(model, i18n)
    } else {
        row![
            button(text(i18n.tr("action-cancel")).size(theme::typography::LABEL))
                .height(34)
                .padding([0.0, theme::spacing::MD])
                .style(theme::button_toolbar)
                .on_press(Message::BulkEditCanceled),
            button(text(i18n.tr("bulk-review-changes")).size(theme::typography::LABEL))
                .height(34)
                .padding([0.0, theme::spacing::MD])
                .style(theme::button_selected)
                .on_press(Message::BulkPreviewRequested),
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center)
        .into()
    };

    let panel = column![header, steps, content, footer]
        .spacing(theme::spacing::MD)
        .align_x(iced::Alignment::Start);
    let panel = if review {
        container(panel)
            .width(820)
            .height(620)
            .padding(theme::spacing::XL)
            .style(theme::overlay_panel)
    } else {
        container(panel)
            .width(700)
            .padding(theme::spacing::XL)
            .style(theme::overlay_panel)
    };

    stack![
        mouse_area(
            container("")
                .width(Length::Fill)
                .height(Length::Fill)
                .style(theme::overlay_backdrop)
        )
        .on_press(Message::BulkEditCanceled),
        container(panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn step_label(label: String, active: bool) -> Element<'static, Message> {
    container(text(label).size(theme::typography::LABEL))
        .padding([theme::spacing::XS, theme::spacing::SM])
        .style(if active {
            theme::table_row_selected
        } else {
            theme::surface
        })
        .into()
}

fn bulk_configure_content<'a>(model: &'a ShellModel, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    let mut content = column![
        text(i18n.tr("bulk-operation"))
            .size(theme::typography::LABEL)
            .style(theme::text_muted),
        bulk_operation_controls(model, i18n),
        bulk_property_controls(model, i18n),
    ]
    .spacing(theme::spacing::SM);
    if model.bulk_edit.operation_kind == BulkEditOperationKind::Set {
        content = content.push(bulk_value_controls(model, i18n));
    }
    if let Some(error) = model.bulk_edit.error.as_deref() {
        content = content.push(
            text(error)
                .size(theme::typography::BODY)
                .style(theme::text_error),
        );
    }
    content.into()
}

fn bulk_review_footer<'a>(model: &'a ShellModel, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    let Some(plan) = model.bulk_edit.plan.as_ref() else {
        return row![].into();
    };
    let count = plan.summary().changed;
    let label = i18n.tr_with("apply-changes", &[("count", count.into())]);
    let apply = button(text(label).size(theme::typography::LABEL))
        .height(34)
        .padding([0.0, theme::spacing::MD])
        .style(if plan.can_apply() && !model.bulk_edit.stale {
            theme::button_selected
        } else {
            theme::button_toolbar
        });
    let apply = if plan.can_apply() && !model.bulk_edit.stale {
        apply.on_press(Message::BulkApplyRequested)
    } else {
        apply
    };
    row![
        button(text(i18n.tr("action-back")).size(theme::typography::LABEL))
            .height(34)
            .padding([0.0, theme::spacing::MD])
            .style(theme::button_toolbar)
            .on_press(Message::BulkEditBackToConfigure),
        button(text(i18n.tr("action-cancel")).size(theme::typography::LABEL))
            .height(34)
            .padding([0.0, theme::spacing::MD])
            .style(theme::button_toolbar)
            .on_press(Message::BulkEditCanceled),
        container("").width(Length::Fill),
        apply,
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center)
    .into()
}

fn bulk_operation_controls<'a>(
    model: &'a ShellModel,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    row![
        bulk_operation_button(
            i18n.tr("bulk-operation-set"),
            BulkEditOperationKind::Set,
            model
        ),
        bulk_operation_button(
            i18n.tr("bulk-operation-remove"),
            BulkEditOperationKind::Remove,
            model
        ),
    ]
    .spacing(theme::spacing::XS)
    .into()
}

fn bulk_operation_button(
    label: String,
    kind: BulkEditOperationKind,
    model: &ShellModel,
) -> Element<'_, Message> {
    button(text(label).size(theme::typography::LABEL))
        .height(28)
        .padding([0.0, theme::spacing::MD])
        .style(if model.bulk_edit.operation_kind == kind {
            theme::button_selected
        } else {
            theme::button_toolbar
        })
        .on_press(Message::BulkOperationSelected(kind))
        .into()
}

fn bulk_property_controls<'a>(model: &'a ShellModel, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    let mut options = model.bulk_property_options();
    let new_property_option = i18n.tr("bulk-new-property-option");
    options.push(new_property_option.clone());
    let selected =
        if model.bulk_edit.new_property.is_empty() && !model.bulk_edit.property.is_empty() {
            Some(model.bulk_edit.property.clone())
        } else {
            None
        };
    let picker = pick_list(options, selected, move |choice: String| {
        if choice == new_property_option {
            Message::BulkNewPropertyRequested
        } else {
            Message::BulkPropertySelected(choice)
        }
    })
    .placeholder(i18n.tr("bulk-property-placeholder"))
    .width(Length::Fill);

    if !model.bulk_edit.new_property.is_empty() || model.bulk_edit.property.is_empty() {
        column![
            text(i18n.tr("bulk-property"))
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
            picker,
            text(i18n.tr("bulk-property-name"))
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
            text_input(i18n.tr_static("bulk-property-name-placeholder"), &model.bulk_edit.new_property)
                .padding(theme::spacing::SM)
                .size(theme::typography::BODY)
                .style(theme::input)
                .on_input(Message::BulkNewPropertyChanged),
        ]
        .spacing(theme::spacing::XS)
        .into()
    } else {
        column![
            text(i18n.tr("bulk-property"))
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
            picker,
        ]
        .spacing(theme::spacing::XS)
        .into()
    }
}

fn bulk_value_controls<'a>(model: &'a ShellModel, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    let value_types = [
        BulkEditValueType::String,
        BulkEditValueType::Integer,
        BulkEditValueType::Float,
        BulkEditValueType::Boolean,
        BulkEditValueType::Null,
        BulkEditValueType::Relation,
    ];
    let types = value_types
        .into_iter()
        .map(|value_type| bulk_value_type_label(value_type, i18n))
        .collect::<Vec<_>>();
    let selected = Some(bulk_value_type_label(model.bulk_edit.value_type, i18n));
    let type_picker = pick_list(types, selected, |choice: String| {
        Message::BulkValueTypeSelected(match choice.as_str() {
            "Integer" | "Inteiro" => BulkEditValueType::Integer,
            "Float" | "Decimal" => BulkEditValueType::Float,
            "Boolean" | "Booleano" => BulkEditValueType::Boolean,
            "Null" | "Nulo" => BulkEditValueType::Null,
            "Relation" | "Relação" => BulkEditValueType::Relation,
            _ => BulkEditValueType::String,
        })
    })
    .width(Length::Fill);

    let input: Element<'_, Message> = match model.bulk_edit.value_type {
        BulkEditValueType::Boolean => row![
            button(text(i18n.tr("value-true")).size(theme::typography::LABEL))
                .height(28)
                .padding([0.0, theme::spacing::MD])
                .style(if model.bulk_edit.bool_value {
                    theme::button_selected
                } else {
                    theme::button_toolbar
                })
                .on_press(Message::BulkBoolValueSelected(true)),
            button(text(i18n.tr("value-false")).size(theme::typography::LABEL))
                .height(28)
                .padding([0.0, theme::spacing::MD])
                .style(if !model.bulk_edit.bool_value {
                    theme::button_selected
                } else {
                    theme::button_toolbar
                })
                .on_press(Message::BulkBoolValueSelected(false)),
        ]
        .spacing(theme::spacing::XS)
        .into(),
        BulkEditValueType::Null => text(i18n.tr("bulk-null-value"))
            .size(theme::typography::BODY)
            .style(theme::text_muted)
            .into(),
        BulkEditValueType::Relation => text_input(i18n.tr_static("bulk-target-placeholder"), &model.bulk_edit.value)
            .padding(theme::spacing::SM)
            .size(theme::typography::BODY)
            .style(theme::input)
            .on_input(Message::BulkValueChanged)
            .into(),
        _ => text_input(i18n.tr_static("bulk-value-placeholder"), &model.bulk_edit.value)
            .padding(theme::spacing::SM)
            .size(theme::typography::BODY)
            .style(theme::input)
            .on_input(Message::BulkValueChanged)
            .into(),
    };

    column![
        text(i18n.tr("bulk-type"))
            .size(theme::typography::LABEL)
            .style(theme::text_muted),
        type_picker,
        text(
            if model.bulk_edit.value_type == BulkEditValueType::Relation {
                i18n.tr("bulk-target")
            } else {
                i18n.tr("bulk-value")
            }
        )
        .size(theme::typography::LABEL)
        .style(theme::text_muted),
        input,
    ]
    .spacing(theme::spacing::XS)
    .into()
}

fn bulk_value_type_label(value_type: BulkEditValueType, i18n: &I18nCatalog) -> String {
    match value_type {
        BulkEditValueType::String => i18n.tr("value-type-string"),
        BulkEditValueType::Integer => i18n.tr("value-type-integer"),
        BulkEditValueType::Float => i18n.tr("value-type-float"),
        BulkEditValueType::Boolean => i18n.tr("value-type-boolean"),
        BulkEditValueType::Null => i18n.tr("value-type-null"),
        BulkEditValueType::Relation => i18n.tr("value-type-relation"),
    }
}

fn bulk_preview<'a>(
    model: &'a ShellModel,
    plan: &'a flokin_core::BulkEditPlan,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let summary = plan.summary();
    let mut list = column![].spacing(theme::spacing::SM);
    for change in &plan.changes {
        let status = match change.status {
            BulkEditChangeStatus::Changed => i18n.tr("change-status-changed"),
            BulkEditChangeStatus::NoChange => i18n.tr("change-status-no-change"),
            BulkEditChangeStatus::Blocked => i18n.tr("change-status-blocked"),
            BulkEditChangeStatus::Unsupported => i18n.tr("change-status-unsupported"),
        };
        let mut item = column![row![
            text(change.relative_path.display().to_string())
                .font(theme::mono())
                .size(theme::typography::LABEL)
                .style(theme::text_normal)
                .width(Length::Fill),
            text(status)
                .size(theme::typography::LABEL)
                .style(match change.status {
                    BulkEditChangeStatus::Changed => theme::text_accent,
                    BulkEditChangeStatus::NoChange => theme::text_muted,
                    BulkEditChangeStatus::Blocked | BulkEditChangeStatus::Unsupported => {
                        theme::text_warning
                    }
                }),
        ]
        .align_y(Alignment::Center)]
        .spacing(theme::spacing::XXS);
        if change.property_changes.is_empty() {
            if let Some(before) = change.before.as_ref() {
                item = item.push(
                    text(format!("- {before}"))
                        .font(theme::mono())
                        .size(theme::typography::LABEL)
                        .style(theme::text_warning),
                );
            }
            if let Some(after) = change.after.as_ref() {
                item = item.push(
                    text(format!("+ {after}"))
                        .font(theme::mono())
                        .size(theme::typography::LABEL)
                        .style(theme::text_accent),
                );
            }
        } else {
            for property_change in &change.property_changes {
                if let Some(before) = property_change.before.as_ref() {
                    item = item.push(
                        text(format!("- {before}"))
                            .font(theme::mono())
                            .size(theme::typography::LABEL)
                            .style(theme::text_warning),
                    );
                }
                if let Some(after) = property_change.after.as_ref() {
                    item = item.push(
                        text(format!("+ {after}"))
                            .font(theme::mono())
                            .size(theme::typography::LABEL)
                            .style(theme::text_accent),
                    );
                }
            }
        }
        if let Some(reason) = change.reason.as_ref() {
            item = item.push(
                text(reason)
                    .size(theme::typography::LABEL)
                    .style(theme::text_muted),
            );
        }
        list = list.push(
            container(item)
                .padding(theme::spacing::SM)
                .style(theme::surface),
        );
    }

    for warning in &plan.warnings {
        list = list.push(
            text(warning)
                .size(theme::typography::BODY)
                .style(theme::text_warning),
        );
    }

    let stale_message: Element<'_, Message> = if model.bulk_edit.stale {
        text(i18n.tr("error-stale-preview"))
            .size(theme::typography::BODY)
            .style(theme::text_warning)
            .into()
    } else {
        container("").height(0).into()
    };
    let error_message: Element<'_, Message> = if let Some(error) = model.bulk_edit.error.as_deref()
    {
        text(error)
            .size(theme::typography::BODY)
            .style(theme::text_error)
            .into()
    } else {
        container("").height(0).into()
    };

    column![
        text(i18n.tr("bulk-review-changes"))
            .size(theme::typography::TITLE)
            .style(theme::text_accent),
        row![
            text(i18n.tr_with("bulk-selected-count", &[("count", summary.selected.into())]))
                .size(theme::typography::LABEL),
            text(i18n.tr_with("changes-will-change", &[("count", summary.changed.into())]))
                .size(theme::typography::LABEL),
            text(i18n.tr_with("changes-no-change", &[("count", summary.no_change.into())]))
                .size(theme::typography::LABEL),
            text(i18n.tr_with(
                "changes-blocked",
                &[("count", (summary.blocked + summary.unsupported).into())],
            ))
            .size(theme::typography::LABEL),
        ]
        .spacing(theme::spacing::MD),
        stale_message,
        error_message,
        scrollable(list).height(Length::Fill),
    ]
    .spacing(theme::spacing::MD)
    .height(Length::Fill)
    .into()
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
    markdown_editor_scroll_y: f32,
    markdown_preview: &'a [markdown::Item],
    app_theme: AppTheme,
    _model: &'a ShellModel,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let file_name = tab
        .relative_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(tab.title.as_str());
    let header = container(
        row![
            row![
                widgets::icon(theme::Icon::FileText, theme::icons::META, true),
                text(file_name)
                    .font(theme::mono())
                    .size(theme::typography::BODY)
                    .wrapping(iced::widget::text::Wrapping::None)
                    .style(theme::text_normal)
            ]
            .spacing(theme::spacing::SM)
            .align_y(Alignment::Center)
            .width(Length::Fill),
            if tab.kind == EditorTabKind::Markdown {
                editor_view_mode_controls(tab.view_mode, i18n)
            } else {
                row![].into()
            },
            save_button(tab, i18n),
        ]
        .spacing(theme::spacing::MD)
        .align_y(Alignment::Center),
    )
    .height(theme::sizes::DOCUMENT_HEADER_HEIGHT)
    .padding([0.0, theme::spacing::LG])
    .style(theme::document_header);

    let mut content = column![header].spacing(0).height(Length::Fill);

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
        content = content.push(external_conflict_banner(conflict, i18n));
    }

    content = content.push(markdown_document_body(
        tab,
        markdown_editor,
        markdown_editor_scroll_y,
        markdown_preview,
        app_theme,
        i18n,
    ));

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding {
            top: 0.0,
            right: theme::spacing::LG,
            bottom: theme::spacing::LG,
            left: theme::spacing::LG,
        })
        .into()
}

fn editor_view_mode_controls<'a>(
    active: EditorViewMode,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    container(
        row![
            editor_view_mode_button(i18n.tr("editor-mode-edit"), EditorViewMode::Edit, active),
            editor_view_mode_button(i18n.tr("editor-mode-split"), EditorViewMode::Split, active),
            editor_view_mode_button(
                i18n.tr("editor-mode-preview"),
                EditorViewMode::Preview,
                active
            ),
        ]
        .spacing(0)
        .align_y(Alignment::Center),
    )
    .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT)
    .padding(2.0)
    .style(theme::segmented_control)
    .into()
}

fn editor_view_mode_button<'a>(
    label: String,
    mode: EditorViewMode,
    active: EditorViewMode,
) -> Element<'a, Message> {
    button(
        container(widgets::button_label(label))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center),
    )
    .width(86)
    .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT - 4.0)
    .padding([0.0, theme::spacing::MD])
    .style(if mode == active {
        theme::button_selected
    } else {
        theme::button_toolbar
    })
    .on_press(Message::EditorViewModeSelected(mode))
    .into()
}

fn save_button<'a>(tab: &'a EditorTab, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    let control = button(
        container(widgets::icon_text(
            theme::Icon::Save,
            i18n.tr("action-save"),
            theme::icons::TOOLBAR,
            !tab.dirty,
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center),
    )
    .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT)
    .padding([0.0, 13.0])
    .style(if tab.dirty {
        theme::button_selected
    } else {
        theme::button_accent_outline
    });

    let control = if tab.dirty {
        control.on_press(Message::EditorSaveRequested)
    } else {
        control
    };

    iced::widget::tooltip(
        control,
        widgets::tooltip_text(i18n.tr("editor-save-tooltip")),
        iced::widget::tooltip::Position::Bottom,
    )
    .style(theme::tooltip)
    .into()
}

fn external_conflict_banner<'a>(
    conflict: &'a EditorExternalConflict,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let message = match conflict {
        EditorExternalConflict::Modified(_) => i18n.tr("editor-conflict-modified"),
        EditorExternalConflict::Deleted => i18n.tr("editor-conflict-deleted"),
    };

    container(
        row![
            text(message)
                .size(theme::typography::BODY)
                .style(theme::text_warning)
                .width(Length::Fill),
            button(text(i18n.tr("editor-reload-disk")))
                .padding([5.0, 10.0])
                .style(theme::button_toolbar)
                .on_press(Message::EditorExternalReload),
            button(text(i18n.tr("editor-keep-local")))
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
    markdown_editor_scroll_y: f32,
    markdown_preview: &'a [markdown::Item],
    app_theme: AppTheme,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    match tab.view_mode {
        _ if tab.kind == EditorTabKind::Schema => {
            markdown_editor_body(tab, markdown_editor, markdown_editor_scroll_y, i18n)
        }
        EditorViewMode::Edit => {
            markdown_editor_body(tab, markdown_editor, markdown_editor_scroll_y, i18n)
        }
        EditorViewMode::Preview => markdown_preview_body(markdown_preview, app_theme, i18n),
        EditorViewMode::Split => markdown_split_body(
            tab,
            markdown_editor,
            markdown_editor_scroll_y,
            markdown_preview,
            app_theme,
            i18n,
        ),
    }
}

fn markdown_split_body<'a>(
    tab: &'a EditorTab,
    markdown_editor: &'a text_editor::Content,
    markdown_editor_scroll_y: f32,
    markdown_preview: &'a [markdown::Item],
    app_theme: AppTheme,
    i18n: &'a I18nCatalog,
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
            container(markdown_editor_body(
                tab,
                markdown_editor,
                markdown_editor_scroll_y,
                i18n,
            ))
            .width(left_width)
            .height(Length::Fill),
            markdown_splitter(),
            container(markdown_preview_body(markdown_preview, app_theme, i18n))
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
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let body: Element<'a, Message> = if markdown_preview.is_empty() {
        container(
            text(i18n.tr("editor-empty-preview"))
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
        scrollable(container(body).width(Length::Fill).padding([
            theme::spacing::XXL + theme::spacing::SM,
            theme::spacing::XXL + theme::spacing::SM,
        ]))
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
    markdown_editor_scroll_y: f32,
    _i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let frontmatter_range = frontmatter_line_range(markdown_editor);
    let line_count = markdown_editor.line_count();
    let editor = stack![
        editor_zebra_background(frontmatter_range, line_count, markdown_editor_scroll_y),
        text_editor(markdown_editor)
            .id(ACTIVE_MARKDOWN_EDITOR_ID)
            .placeholder("")
            .on_action(Message::MarkdownEditorAction)
            .key_binding(markdown_editor_key_binding)
            .font(theme::mono())
            .size(theme::typography::EDITOR)
            .line_height(LineHeight::Relative(theme::sizes::EDITOR_LINE_HEIGHT_RATIO,))
            .height(Length::Fill)
            .padding([theme::spacing::XL, theme::spacing::XXL])
            .wrapping(Wrapping::None)
            .style(theme::markdown_text_editor)
    ]
    .height(Length::Fill)
    .width(Length::Fill);

    container(
        row![
            line_number_gutter(line_count, frontmatter_range, markdown_editor_scroll_y),
            editor
        ]
        .spacing(0),
    )
    .style(theme::editor)
    .height(Length::Fill)
    .width(Length::Fill)
    .into()
}

fn editor_zebra_background<'a>(
    frontmatter_range: Option<(usize, usize)>,
    line_count: usize,
    scroll_y: f32,
) -> Element<'a, Message> {
    iced::widget::responsive(move |size| {
        let segments = visible_line_segments(line_count, scroll_y, size.height);
        let mut rows = column![].spacing(0);
        let mut cursor_y = 0.0;

        for segment in segments {
            if segment.y > cursor_y {
                rows = rows.push(Space::new().height(segment.y - cursor_y));
            }
            rows = rows.push(
                container("")
                    .width(Length::Fill)
                    .height(segment.height)
                    .style(move |theme| {
                        theme::editor_row_with_frontmatter(
                            theme,
                            segment.line_index,
                            is_frontmatter_line(frontmatter_range, segment.line_index),
                        )
                    }),
            );
            cursor_y = segment.y + segment.height;
        }

        container(rows)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

pub(crate) fn editor_line_height_px() -> f32 {
    theme::typography::EDITOR as f32 * theme::sizes::EDITOR_LINE_HEIGHT_RATIO
}

fn line_number_gutter<'a>(
    line_count: usize,
    frontmatter_range: Option<(usize, usize)>,
    scroll_y: f32,
) -> Element<'a, Message> {
    iced::widget::responsive(move |size| {
        let segments = visible_line_segments(line_count, scroll_y, size.height);
        let mut lines = column![].spacing(0);
        let mut cursor_y = 0.0;

        for segment in segments {
            if segment.y > cursor_y {
                lines = lines.push(Space::new().height(segment.y - cursor_y));
            }
            let line_number = segment.line_index + 1;
            let row_index = segment.line_index;
            lines = lines.push(
                container(
                    text(format!("{line_number:>4}"))
                        .font(theme::mono())
                        .size(theme::typography::EDITOR_LINE_NUMBER)
                        .line_height(LineHeight::Relative(theme::sizes::EDITOR_LINE_HEIGHT_RATIO))
                        .style(theme::text_muted),
                )
                .width(Length::Fill)
                .height(segment.height)
                .style(move |theme| {
                    theme::editor_row_with_frontmatter(
                        theme,
                        row_index,
                        is_frontmatter_line(frontmatter_range, row_index),
                    )
                }),
            );
            cursor_y = segment.y + segment.height;
        }

        container(lines)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([0.0, theme::spacing::SM])
            .style(theme::gutter)
            .into()
    })
    .width(theme::sizes::EDITOR_GUTTER_WIDTH)
    .height(Length::Fill)
    .into()
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LineBackgroundSegment {
    line_index: usize,
    y: f32,
    height: f32,
}

#[cfg(test)]
fn frontmatter_background_segments(
    frontmatter_range: Option<(usize, usize)>,
    line_count: usize,
    scroll_y: f32,
    viewport_height: f32,
) -> Vec<LineBackgroundSegment> {
    let Some((start, end)) = frontmatter_range else {
        return Vec::new();
    };
    visible_line_segments(line_count, scroll_y, viewport_height)
        .into_iter()
        .filter(|segment| (start..=end).contains(&segment.line_index))
        .collect()
}

fn visible_line_segments(
    line_count: usize,
    scroll_y: f32,
    viewport_height: f32,
) -> Vec<LineBackgroundSegment> {
    let line_height = editor_line_height_px();
    let line_count = line_count.max(1);
    let scroll_y = clamped_editor_scroll_y(scroll_y, line_count, viewport_height);
    let mut segments = Vec::new();

    for line_index in 0..line_count {
        let top = theme::spacing::XL + line_index as f32 * line_height - scroll_y;
        let bottom = top + line_height;
        if bottom <= 0.0 || top >= viewport_height {
            continue;
        }
        let y = top.max(0.0);
        let height = bottom.min(viewport_height) - y;
        if height > 0.0 {
            segments.push(LineBackgroundSegment {
                line_index,
                y,
                height,
            });
        }
    }

    segments
}

fn clamped_editor_scroll_y(scroll_y: f32, line_count: usize, viewport_height: f32) -> f32 {
    let line_height = editor_line_height_px();
    let visible_height = (viewport_height - theme::spacing::XL * 2.0).max(0.0);
    let content_height = line_count.max(1) as f32 * line_height;
    scroll_y.clamp(0.0, (content_height - visible_height).max(0.0))
}

fn frontmatter_line_range(content: &text_editor::Content) -> Option<(usize, usize)> {
    let mut lines = content.lines();
    let first = lines.next()?;
    if first.text.trim() != "---" {
        return None;
    }

    for (offset, line) in lines.enumerate() {
        if line.text.trim() == "---" {
            return Some((0, offset + 1));
        }
    }

    None
}

fn is_frontmatter_line(range: Option<(usize, usize)>, line: usize) -> bool {
    range.is_some_and(|(start, end)| (start..=end).contains(&line))
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

fn empty_document_area<'a>(i18n: &'a I18nCatalog) -> Element<'a, Message> {
    container(
        text(i18n.tr("editor-select-document"))
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

    use super::{
        editor_line_height_px, frontmatter_background_segments, frontmatter_line_range,
        is_frontmatter_line, markdown_editor_key_binding, sql_editor_key_binding,
    };
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

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.001,
            "expected {actual} to be close to {expected}"
        );
    }

    #[test]
    fn ctrl_enter_in_sql_editor_publishes_execute_message() {
        assert_eq!(
            sql_editor_key_binding(key_press(keyboard::Modifiers::CTRL)),
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
            sql_editor_key_binding(key_press(keyboard::Modifiers::NONE)),
            Some(text_editor::Binding::Enter)
        );
    }

    #[test]
    fn sql_editor_common_character_uses_default_text_editor_binding() {
        let press = latin_key_press('s', keyboard::Modifiers::NONE);

        assert_eq!(
            sql_editor_key_binding(press.clone()),
            text_editor::Binding::from_key_press(press)
        );
    }

    #[test]
    fn tab_is_not_captured_by_sql_completion() {
        let mut press = key_press(keyboard::Modifiers::NONE);
        press.key = Key::Named(keyboard::key::Named::Tab);
        press.modified_key = Key::Named(keyboard::key::Named::Tab);

        assert_eq!(
            sql_editor_key_binding(press.clone()),
            text_editor::Binding::from_key_press(press)
        );
    }

    #[test]
    fn arrows_and_escape_are_not_captured_by_sql_completion() {
        let mut down = key_press(keyboard::Modifiers::NONE);
        down.key = Key::Named(keyboard::key::Named::ArrowDown);
        down.modified_key = Key::Named(keyboard::key::Named::ArrowDown);
        assert_eq!(
            sql_editor_key_binding(down.clone()),
            text_editor::Binding::from_key_press(down)
        );

        let mut up = key_press(keyboard::Modifiers::NONE);
        up.key = Key::Named(keyboard::key::Named::ArrowUp);
        up.modified_key = Key::Named(keyboard::key::Named::ArrowUp);
        assert_eq!(
            sql_editor_key_binding(up.clone()),
            text_editor::Binding::from_key_press(up)
        );

        let mut escape = key_press(keyboard::Modifiers::NONE);
        escape.key = Key::Named(keyboard::key::Named::Escape);
        escape.modified_key = Key::Named(keyboard::key::Named::Escape);
        assert_eq!(
            sql_editor_key_binding(escape.clone()),
            text_editor::Binding::from_key_press(escape)
        );
    }

    #[test]
    fn frontmatter_range_includes_delimiters_when_document_starts_with_yaml() {
        let content =
            text_editor::Content::with_text("---\ntitle: CARF\nstatus: active\n---\n# Body\n");

        assert_eq!(frontmatter_line_range(&content), Some((0, 3)));
        assert!(is_frontmatter_line(Some((0, 3)), 0));
        assert!(is_frontmatter_line(Some((0, 3)), 2));
        assert!(is_frontmatter_line(Some((0, 3)), 3));
        assert!(!is_frontmatter_line(Some((0, 3)), 4));
    }

    #[test]
    fn frontmatter_range_expands_and_contracts_with_current_buffer() {
        let expanded = text_editor::Content::with_text(
            "---\ntitle: CARF\nstatus: active\npriority: 10\n---\nBody\n",
        );
        let contracted = text_editor::Content::with_text("---\ntitle: CARF\n---\nBody\n");

        assert_eq!(frontmatter_line_range(&expanded), Some((0, 4)));
        assert_eq!(frontmatter_line_range(&contracted), Some((0, 2)));
    }

    #[test]
    fn body_delimiters_and_unclosed_yaml_are_not_frontmatter() {
        let body_separator = text_editor::Content::with_text("# Body\n---\nnot frontmatter\n---\n");
        let unclosed = text_editor::Content::with_text("---\ntitle: CARF\n");

        assert_eq!(frontmatter_line_range(&body_separator), None);
        assert_eq!(frontmatter_line_range(&unclosed), None);
    }

    #[test]
    fn frontmatter_background_starts_at_document_top_without_scroll() {
        let line_height = editor_line_height_px();
        let segments = frontmatter_background_segments(Some((0, 4)), 20, 0.0, 240.0);

        assert_eq!(segments.len(), 5);
        assert_eq!(segments[0].line_index, 0);
        assert_close(segments[0].y, crate::theme::spacing::XL);
        assert_close(segments[0].height, line_height);
        assert_eq!(segments[4].line_index, 4);
    }

    #[test]
    fn frontmatter_background_moves_with_full_line_scroll() {
        let line_height = editor_line_height_px();
        let segments = frontmatter_background_segments(Some((0, 4)), 20, line_height * 2.0, 240.0);

        assert_eq!(segments[0].line_index, 1);
        assert_close(segments[0].y, 0.0);
        assert_eq!(segments[1].line_index, 2);
        assert_close(segments[1].y, crate::theme::spacing::XL);
    }

    #[test]
    fn frontmatter_background_moves_with_partial_scroll() {
        let line_height = editor_line_height_px();
        let segments = frontmatter_background_segments(Some((0, 2)), 20, 8.0, 240.0);

        assert_eq!(segments[0].line_index, 0);
        assert_close(segments[0].y, crate::theme::spacing::XL - 8.0);
        assert_close(segments[0].height, line_height);
    }

    #[test]
    fn frontmatter_background_disappears_when_scrolled_out_of_view() {
        let line_height = editor_line_height_px();
        let segments =
            frontmatter_background_segments(Some((0, 4)), 200, line_height * 50.0, 240.0);

        assert!(segments.is_empty());
    }

    #[test]
    fn frontmatter_background_reappears_after_returning_to_top() {
        let line_height = editor_line_height_px();
        let away = frontmatter_background_segments(Some((0, 4)), 200, line_height * 50.0, 240.0);
        let top = frontmatter_background_segments(Some((0, 4)), 200, 0.0, 240.0);

        assert!(away.is_empty());
        assert_eq!(top.len(), 5);
        assert_eq!(top[0].line_index, 0);
    }

    #[test]
    fn document_without_frontmatter_has_no_background_segments() {
        let segments = frontmatter_background_segments(None, 20, 0.0, 240.0);

        assert!(segments.is_empty());
    }
}
