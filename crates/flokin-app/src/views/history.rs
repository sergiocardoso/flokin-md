use chrono::{DateTime, Local};
use flokin_core::{
    BulkEditChangeStatus, BulkEditPlan, HistoryFileChange, HistoryState, MutationHistoryEntry,
    ShellModel,
};
use iced::widget::{
    button, column, container, mouse_area, row, scrollable, stack, text,
    text::{LineHeight, Wrapping},
};
use iced::{alignment, Alignment, Element, Length};

use crate::{
    message::Message,
    theme::{self},
    widgets,
};

pub fn view(model: &ShellModel) -> Element<'_, Message> {
    let page = if model.current_workspace.is_none() {
        centered_empty(
            "Abra um workspace para ver o histórico.",
            "O histórico é local e isolado por workspace.",
        )
    } else if let Some(plan) = model.history.undo_plan.as_ref() {
        undo_preview(model, plan)
    } else {
        history_page(model)
    };

    if model.history.clear_confirm {
        stack![page, clear_history_dialog()].into()
    } else {
        page
    }
}

fn history_page(model: &ShellModel) -> Element<'_, Message> {
    let header = row![
        column![
            text("Histórico")
                .size(theme::typography::TITLE)
                .style(theme::text_accent),
            text("Operações de Bulk Edit, SQL Update e Undo registradas localmente.")
                .size(theme::typography::BODY)
                .style(theme::text_muted),
        ]
        .spacing(theme::spacing::XXS)
        .width(Length::Fill),
        clear_button(!model.history.entries.is_empty()),
    ]
    .spacing(theme::spacing::MD)
    .align_y(Alignment::Center);

    let body = if model.history.entries.is_empty() {
        centered_empty(
            "Nenhuma alteração registrada ainda.",
            "Operações de Bulk Edit e SQL Update aparecerão aqui.",
        )
    } else {
        row![history_list(model), history_detail(model)]
            .spacing(theme::spacing::MD)
            .height(Length::Fill)
            .into()
    };

    let mut content = column![header].spacing(theme::spacing::MD);
    if let Some(result) = model.history.last_result.as_ref() {
        content = content.push(
            text(result.as_str())
                .size(theme::typography::BODY)
                .style(theme::text_accent),
        );
    }
    if let Some(error) = model.history.error.as_ref() {
        content = content.push(
            text(error.as_str())
                .size(theme::typography::BODY)
                .wrapping(Wrapping::Word)
                .style(theme::text_warning),
        );
    }
    content = content.push(body);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(theme::spacing::XXL)
        .style(theme::editor)
        .into()
}

fn history_list(model: &ShellModel) -> Element<'_, Message> {
    let mut rows = column![].spacing(theme::spacing::XS);
    let mut last_day = String::new();
    for entry in &model.history.entries {
        let day = day_label(entry.created_at_unix);
        if day != last_day {
            rows = rows.push(
                text(day.clone())
                    .size(theme::typography::LABEL)
                    .style(theme::text_muted),
            );
            last_day = day;
        }
        rows = rows.push(history_row(
            &model.history,
            entry,
            model.history.selected_entry_id.as_deref() == Some(entry.id.as_str()),
        ));
    }

    container(scrollable(rows))
        .width(Length::Fixed(340.0))
        .height(Length::Fill)
        .style(theme::surface)
        .padding(theme::spacing::SM)
        .into()
}

fn history_row<'a>(
    history: &'a HistoryState,
    entry: &'a MutationHistoryEntry,
    selected: bool,
) -> Element<'a, Message> {
    let status = history.entry_status_label(entry);
    let sql = entry
        .sql
        .as_ref()
        .map(|sql| sql_preview(sql))
        .unwrap_or_default();
    let mut content = column![
        row![
            text(time_label(entry.created_at_unix))
                .font(theme::mono())
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
            text(entry.source.label_pt())
                .size(theme::typography::LABEL)
                .style(theme::text_accent),
            text(entry.file_count_label())
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center),
        text(entry.summary.as_str())
            .size(theme::typography::BODY)
            .wrapping(Wrapping::Word)
            .style(theme::text_normal),
        text(status)
            .size(theme::typography::LABEL)
            .style(theme::text_muted),
    ]
    .spacing(theme::spacing::XXS);
    if !sql.is_empty() {
        content = content.push(
            text(sql)
                .font(theme::mono())
                .size(theme::typography::LABEL)
                .wrapping(Wrapping::Word)
                .style(theme::text_muted),
        );
    }

    button(container(content).padding(theme::spacing::SM))
        .width(Length::Fill)
        .padding(0)
        .style(if selected {
            theme::button_tree_selected
        } else {
            theme::button_tree
        })
        .on_press(Message::HistoryEntrySelected(entry.id.clone()))
        .into()
}

fn history_detail(model: &ShellModel) -> Element<'_, Message> {
    let Some(entry) = model.history.selected_entry() else {
        return centered_empty("Selecione uma operação.", "Os detalhes aparecerão aqui.");
    };

    let mut files = column![].spacing(theme::spacing::SM);
    for file in &entry.files {
        files = files.push(file_diff(file));
    }

    let mut metadata = column![
        text(entry.source.label_pt())
            .size(theme::typography::TITLE)
            .style(theme::text_accent),
        text(format!(
            "{} • {} • {}",
            timestamp_label(entry.created_at_unix),
            entry.file_count_label(),
            model.history.entry_status_label(entry)
        ))
        .size(theme::typography::BODY)
        .style(theme::text_muted),
        text(entry.summary.as_str())
            .size(theme::typography::BODY)
            .wrapping(Wrapping::Word)
            .style(theme::text_normal),
    ]
    .spacing(theme::spacing::XS);
    if let Some(sql) = entry.sql.as_ref() {
        metadata = metadata.push(
            container(
                text(sql.as_str())
                    .font(theme::mono())
                    .size(theme::typography::LABEL)
                    .wrapping(Wrapping::Word)
                    .style(theme::text_normal),
            )
            .padding(theme::spacing::SM)
            .style(theme::surface),
        );
    }
    if let Some(original_id) = entry.original_entry_id.as_ref() {
        metadata = metadata.push(
            text(format!("Operação original: {original_id}"))
                .font(theme::mono())
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
        );
    }

    let header = if model.history.is_entry_undoable(entry) {
        row![
            metadata.width(Length::Fill),
            button(widgets::icon_text(
                theme::Icon::Reset,
                "Desfazer alteração",
                theme::icons::TOOLBAR,
                false,
            ))
            .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT)
            .padding([0.0, theme::spacing::MD])
            .style(theme::button_selected)
            .on_press(Message::HistoryUndoRequested)
        ]
    } else {
        row![metadata.width(Length::Fill)]
    }
    .align_y(Alignment::Start);

    container(
        column![
            header,
            container("")
                .height(1)
                .width(Length::Fill)
                .style(theme::divider),
            scrollable(files).height(Length::Fill),
        ]
        .spacing(theme::spacing::MD),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(theme::spacing::MD)
    .style(theme::surface)
    .into()
}

fn undo_preview<'a>(_model: &'a ShellModel, plan: &'a BulkEditPlan) -> Element<'a, Message> {
    let summary = plan.summary();
    let mut files = column![].spacing(theme::spacing::SM);
    for change in &plan.changes {
        let mut item = column![text(change.relative_path.display().to_string())
            .font(theme::mono())
            .size(theme::typography::LABEL)
            .style(theme::text_normal)]
        .spacing(theme::spacing::XXS);
        for property_change in &change.property_changes {
            if let Some(before) = property_change.before.as_ref() {
                item = item.push(diff_line("-", before, true));
            }
            if let Some(after) = property_change.after.as_ref() {
                item = item.push(diff_line("+", after, false));
            }
        }
        if change.property_changes.is_empty() {
            item = item.push(
                text("Conteúdo completo será restaurado.")
                    .size(theme::typography::LABEL)
                    .style(theme::text_muted),
            );
        }
        if let Some(reason) = change.reason.as_ref() {
            item = item.push(
                text(reason.as_str())
                    .size(theme::typography::LABEL)
                    .style(theme::text_warning),
            );
        }
        files = files.push(
            container(item)
                .padding(theme::spacing::SM)
                .style(theme::surface),
        );
    }

    let label = if summary.changed == 1 {
        String::from("Desfazer 1 alteração")
    } else {
        format!("Desfazer {} alterações", summary.changed)
    };
    let apply = button(text(label).size(theme::typography::LABEL))
        .height(34)
        .padding([0.0, theme::spacing::MD])
        .style(if plan.can_apply() {
            theme::button_selected
        } else {
            theme::button_toolbar
        });
    let apply = if plan.can_apply() {
        apply.on_press(Message::HistoryUndoApplyRequested)
    } else {
        apply
    };

    let footer = row![
        button(text("Cancelar").size(theme::typography::LABEL))
            .height(34)
            .padding([0.0, theme::spacing::MD])
            .style(theme::button_toolbar)
            .on_press(Message::HistoryUndoPreviewCanceled),
        iced::widget::Space::new().width(Length::Fill),
        apply,
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    container(
        column![
            text("Revisar desfazer")
                .size(theme::typography::TITLE)
                .style(theme::text_accent),
            text(format!("{} arquivos serão restaurados", summary.changed))
                .size(theme::typography::BODY)
                .style(theme::text_muted),
            scrollable(files).height(Length::Fill),
            footer,
        ]
        .spacing(theme::spacing::MD),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(theme::spacing::XXL)
    .style(theme::editor)
    .into()
}

fn file_diff(file: &HistoryFileChange) -> Element<'_, Message> {
    let mut item = column![text(file.relative_path.display().to_string())
        .font(theme::mono())
        .size(theme::typography::LABEL)
        .style(theme::text_normal)]
    .spacing(theme::spacing::XXS);

    for property_change in &file.property_changes {
        if let Some(before) = property_change.before.as_ref() {
            item = item.push(diff_line("-", before, true));
        }
        if let Some(after) = property_change.after.as_ref() {
            item = item.push(diff_line("+", after, false));
        }
    }
    if file.property_changes.is_empty() {
        item = item.push(
            text("Conteúdo completo registrado.")
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
        );
    }

    container(item)
        .width(Length::Fill)
        .padding(theme::spacing::SM)
        .style(theme::elevated)
        .into()
}

fn diff_line<'a>(prefix: &'static str, value: &'a str, removed: bool) -> Element<'a, Message> {
    text(format!("{prefix} {value}"))
        .font(theme::mono())
        .size(theme::typography::LABEL)
        .line_height(LineHeight::Relative(1.1))
        .wrapping(Wrapping::Word)
        .style(if removed {
            theme::text_warning
        } else {
            theme::text_accent
        })
        .into()
}

fn clear_button(enabled: bool) -> Element<'static, Message> {
    let button = button(text("Limpar histórico").size(theme::typography::LABEL))
        .height(34)
        .padding([0.0, theme::spacing::MD])
        .style(if enabled {
            theme::button_toolbar
        } else {
            theme::button_ghost
        });
    if enabled {
        button.on_press(Message::HistoryClearRequested).into()
    } else {
        button.into()
    }
}

fn centered_empty<'a>(title: &'a str, subtitle: &'a str) -> Element<'a, Message> {
    container(
        column![
            text(title)
                .size(theme::typography::TITLE)
                .style(theme::text_normal),
            text(subtitle)
                .size(theme::typography::BODY)
                .style(theme::text_muted),
        ]
        .spacing(theme::spacing::SM)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(alignment::Horizontal::Center)
    .align_y(alignment::Vertical::Center)
    .style(theme::editor)
    .into()
}

fn clear_history_dialog<'a>() -> Element<'a, Message> {
    stack![
        mouse_area(
            container("")
                .width(Length::Fill)
                .height(Length::Fill)
                .style(theme::overlay_backdrop)
        )
        .on_press(Message::HistoryClearCanceled),
        container(
            container(
                column![
                    text("Limpar histórico")
                        .size(theme::typography::TITLE)
                        .style(theme::text_accent),
                    text("Isso removerá o histórico de operações deste workspace. Os arquivos Markdown não serão alterados.")
                        .size(theme::typography::BODY)
                        .wrapping(Wrapping::Word)
                        .style(theme::text_normal),
                    row![
                        button(text("Cancelar").size(theme::typography::BODY))
                            .padding([7.0, 12.0])
                            .style(theme::button_toolbar)
                            .on_press(Message::HistoryClearCanceled),
                        button(text("Limpar histórico").size(theme::typography::BODY))
                            .padding([7.0, 12.0])
                            .style(theme::button_selected)
                            .on_press(Message::HistoryClearConfirmed),
                    ]
                    .spacing(theme::spacing::SM)
                    .align_y(Alignment::Center)
                ]
                .spacing(theme::spacing::MD)
            )
            .width(theme::sizes::DIALOG_WIDTH)
            .padding(theme::spacing::LG)
            .style(theme::overlay_panel)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn day_label(timestamp: i64) -> String {
    let date = local_time(timestamp);
    let today = Local::now().date_naive();
    let entry_date = date.date_naive();
    if entry_date == today {
        String::from("Hoje")
    } else if entry_date == today.pred_opt().unwrap_or(today) {
        String::from("Ontem")
    } else {
        date.format("%d/%m/%Y").to_string()
    }
}

fn time_label(timestamp: i64) -> String {
    local_time(timestamp).format("%H:%M").to_string()
}

fn timestamp_label(timestamp: i64) -> String {
    local_time(timestamp).format("%d/%m/%Y %H:%M").to_string()
}

fn local_time(timestamp: i64) -> DateTime<Local> {
    DateTime::from_timestamp(timestamp, 0)
        .map(|time| time.with_timezone(&Local))
        .unwrap_or_else(Local::now)
}

fn sql_preview(sql: &str) -> String {
    let compact = sql.split_whitespace().collect::<Vec<_>>().join(" ");
    const MAX: usize = 96;
    if compact.chars().count() <= MAX {
        compact
    } else {
        format!("{}...", compact.chars().take(MAX).collect::<String>())
    }
}

#[allow(dead_code)]
fn _status_label(status: BulkEditChangeStatus) -> &'static str {
    match status {
        BulkEditChangeStatus::Changed => "Alterado",
        BulkEditChangeStatus::NoChange => "Sem alteração",
        BulkEditChangeStatus::Blocked => "Bloqueado",
        BulkEditChangeStatus::Unsupported => "Não suportado",
    }
}
