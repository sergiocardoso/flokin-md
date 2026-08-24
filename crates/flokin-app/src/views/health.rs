use flokin_core::{ExplicitSchemaState, HealthFilter, HealthIssue, HealthSeverity, ShellModel};
use iced::widget::{
    button, column, container, row, scrollable,
    scrollable::{Direction, Scrollbar},
    text,
    text::Wrapping,
    text_input,
};
use iced::{Alignment, Element, Length};

use crate::{message::Message, theme, views::data_grid, widgets};

pub fn view(model: &ShellModel) -> Element<'_, Message> {
    let summary = &model.health.summary;
    let issues = model.filtered_health_issues();

    container(
        column![
            row![
                column![
                    text("Database Health")
                        .size(theme::typography::TITLE)
                        .style(theme::text_normal),
                    text(format!("{} documentos", summary.total_documents))
                        .size(theme::typography::BODY)
                        .style(theme::text_muted),
                ]
                .spacing(theme::spacing::XS)
                .width(Length::Fill),
                summary_counter("Errors", summary.errors, HealthSeverity::Error),
                summary_counter("Warnings", summary.warnings, HealthSeverity::Warning),
                summary_counter("Healthy", summary.healthy_documents, HealthSeverity::Info),
            ]
            .spacing(theme::spacing::MD)
            .align_y(Alignment::Center),
            row![
                filter_button("All", HealthFilter::All, model),
                filter_button("Errors", HealthFilter::Errors, model),
                filter_button("Warnings", HealthFilter::Warnings, model),
                text_input("Filtrar issues...", model.health_query.as_str())
                    .on_input(Message::HealthQueryChanged)
                    .size(theme::typography::BODY)
                    .padding([5.0, theme::spacing::SM])
                    .width(220)
                    .style(theme::input),
            ]
            .spacing(theme::spacing::SM)
            .align_y(Alignment::Center),
            health_schema_hint(model),
            if issues.is_empty() {
                empty_state()
            } else {
                issues_grid(model, &issues)
            }
        ]
        .spacing(theme::spacing::LG),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(theme::spacing::LG)
    .style(theme::document_surface)
    .into()
}

fn health_schema_hint<'a>(model: &'a ShellModel) -> Element<'a, Message> {
    match &model.schema_catalog.explicit_schema {
        ExplicitSchemaState::Absent => container(
            row![
                column![
                    text("Schema explícito não configurado.")
                        .size(theme::typography::BODY)
                        .style(theme::text_normal),
                    text("O banco está usando somente a estrutura inferida.")
                        .size(theme::typography::BODY)
                        .style(theme::text_muted),
                ]
                .spacing(theme::spacing::XXS)
                .width(Length::Fill),
                button(text("Criar schema").size(theme::typography::BODY))
                    .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT)
                    .padding([0.0, 10.0])
                    .style(theme::button_primary)
                    .on_press(Message::SchemaCreateRequested),
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
                text("Há um problema em flokin.schema.yaml.")
                    .size(theme::typography::BODY)
                    .style(theme::text_warning)
                    .width(Length::Fill),
                button(text("Abrir schema").size(theme::typography::BODY))
                    .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT)
                    .padding([0.0, 10.0])
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
        ExplicitSchemaState::Loaded(_) => container(row![]).height(0).into(),
    }
}

fn summary_counter<'a>(
    label: &'static str,
    count: usize,
    severity: HealthSeverity,
) -> Element<'a, Message> {
    let style = match severity {
        HealthSeverity::Error => theme::text_error,
        HealthSeverity::Warning => theme::text_warning,
        HealthSeverity::Info => theme::text_success,
    };
    column![
        text(label)
            .size(theme::typography::LABEL)
            .style(theme::text_muted),
        text(count.to_string())
            .font(theme::mono())
            .size(theme::typography::TITLE)
            .style(style),
    ]
    .spacing(theme::spacing::XXS)
    .align_x(Alignment::End)
    .into()
}

fn filter_button(
    label: &'static str,
    filter: HealthFilter,
    model: &ShellModel,
) -> Element<'static, Message> {
    button(text(label).size(theme::typography::LABEL))
        .height(theme::sizes::TAB_BUTTON_HEIGHT)
        .padding([0.0, theme::spacing::MD])
        .style(if model.health_filter == filter {
            theme::button_selected
        } else {
            theme::button_toolbar
        })
        .on_press(Message::HealthFilterSelected(filter))
        .into()
}

fn empty_state<'a>() -> Element<'a, Message> {
    container(
        column![
            widgets::icon(theme::Icon::Health, theme::icons::META, true),
            text("Nenhuma issue encontrada.")
                .size(theme::typography::BODY)
                .style(theme::text_muted),
        ]
        .spacing(theme::spacing::SM)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn issues_grid<'a>(model: &'a ShellModel, issues: &[&'a HealthIssue]) -> Element<'a, Message> {
    let widths = [92.0, 110.0, 260.0, 140.0, 360.0];
    let width = data_grid::grid_width(true, widths.into_iter());
    let mut rows = column![header(widths, width)].spacing(0);

    for (row_index, issue) in issues.iter().enumerate() {
        let selected = model.selected_health_issue_id.as_ref() == Some(&issue.id);
        let mut cells = row![data_grid::row_gutter(row_index, selected)]
            .spacing(0)
            .align_y(Alignment::Center);
        cells = cells.push(cell(
            severity_label(issue.severity),
            widths[0],
            selected,
            issue.severity,
        ));
        cells = cells.push(cell(
            issue.category.label().to_owned(),
            widths[1],
            selected,
            issue.severity,
        ));
        cells = cells.push(cell(
            issue
                .relative_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| String::from("workspace")),
            widths[2],
            selected,
            issue.severity,
        ));
        cells = cells.push(cell(
            issue.property.clone().unwrap_or_else(|| String::from("—")),
            widths[3],
            selected,
            issue.severity,
        ));
        cells = cells.push(cell(
            issue.message.clone(),
            widths[4],
            selected,
            issue.severity,
        ));

        rows = rows.push(
            button(container(cells).width(width))
                .width(width)
                .height(data_grid::ROW_HEIGHT)
                .padding(0)
                .style(move |theme, status| {
                    theme::data_row_button(theme, row_index, selected, status)
                })
                .on_press(Message::HealthIssueSelected(issue.id.clone())),
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

fn header<'a>(widths: [f32; 5], width: f32) -> Element<'a, Message> {
    let mut header = row![data_grid::header_gutter()]
        .spacing(0)
        .align_y(Alignment::Center);
    for (label, width) in [
        ("SEVERITY", widths[0]),
        ("CATEGORY", widths[1]),
        ("DOCUMENT", widths[2]),
        ("PROPERTY", widths[3]),
        ("PROBLEM", widths[4]),
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

fn cell<'a>(
    value: String,
    width: f32,
    selected: bool,
    severity: HealthSeverity,
) -> Element<'a, Message> {
    let style = if selected {
        theme::text_accent
    } else {
        match severity {
            HealthSeverity::Error => theme::text_error,
            HealthSeverity::Warning => theme::text_warning,
            HealthSeverity::Info => theme::text_normal,
        }
    };
    data_grid::cell(
        text(value)
            .size(theme::typography::BODY)
            .wrapping(Wrapping::None)
            .style(style),
        width,
        iced::alignment::Horizontal::Left,
    )
}

fn severity_label(severity: HealthSeverity) -> String {
    match severity {
        HealthSeverity::Error => String::from("Error"),
        HealthSeverity::Warning => String::from("Warning"),
        HealthSeverity::Info => String::from("Info"),
    }
}
