use flokin_core::{
    HealthIssue, HealthSeverity, InspectorField, InspectorModel, InspectorRelation,
    InspectorRelationStatus, InspectorValue, ShellModel,
};
use iced::widget::{
    button, column, container, row, scrollable,
    scrollable::{Direction, Scrollbar},
    text,
    text::{LineHeight, Wrapping},
};
use iced::{Alignment, Element, Length};

use crate::{message::Message, theme, widgets};

pub fn view(model: &ShellModel, width: f32) -> Element<'_, Message> {
    match model.document_inspector() {
        InspectorModel::Empty { title, description } => empty_state(title, description, width),
        InspectorModel::Document(inspector) => document_inspector(inspector, width),
        InspectorModel::HealthIssue(inspector) => health_issue_inspector(inspector.issue, width),
    }
}

fn empty_state(title: String, description: String, width: f32) -> Element<'static, Message> {
    container(scrollable(
        column![
            section_header("PROPRIEDADES", theme::Icon::Settings),
            column![
                wrapped_text(title, theme::typography::BODY, false),
                wrapped_text(description, theme::typography::BODY, true),
            ]
            .spacing(theme::spacing::SM),
        ]
        .spacing(theme::spacing::MD),
    ))
    .width(width)
    .height(Length::Fill)
    .padding(theme::spacing::XL)
    .style(theme::inspector_panel)
    .into()
}

fn document_inspector(
    inspector: flokin_core::DocumentInspector,
    width: f32,
) -> Element<'static, Message> {
    let mut content =
        column![section_header("PROPRIEDADES", theme::Icon::Settings)].spacing(theme::spacing::MD);

    for field in inspector.properties {
        content = content.push(field_row(field));
    }

    if !inspector.outgoing_relations.is_empty() {
        content = content
            .push(subtle_divider())
            .push(section_header("RELAÇÕES", theme::Icon::FileText));

        for relation in inspector.outgoing_relations {
            content = content.push(relation_row(relation, true));
        }
    }

    if !inspector.incoming_relations.is_empty() {
        content = content
            .push(subtle_divider())
            .push(section_header("REFERENCIADO POR", theme::Icon::Tag));

        for relation in inspector.incoming_relations {
            content = content.push(relation_row(relation, false));
        }
    }

    if !inspector.tags.is_empty() {
        content = content
            .push(subtle_divider())
            .push(section_header("TAGS", theme::Icon::Tag));

        let mut tags = column![].spacing(theme::spacing::XS);
        for tag in inspector.tags {
            tags = tags.push(chip(tag));
        }
        content = content.push(tags);
    }

    if !inspector.warnings.is_empty() {
        content = content
            .push(subtle_divider())
            .push(section_header("WARNINGS", theme::Icon::Clock));

        for warning in inspector.warnings {
            content = content.push(warning_row(warning));
        }
    }

    content = content
        .push(subtle_divider())
        .push(section_header("METADADOS", theme::Icon::FileText));

    for field in inspector.metadata {
        content = content.push(field_row(field));
    }

    container(scrollable(content).direction(Direction::Vertical(
        Scrollbar::default().width(4).scroller_width(4).spacing(8),
    )))
    .width(width)
    .height(Length::Fill)
    .padding(theme::spacing::XL)
    .style(theme::inspector_panel)
    .into()
}

fn health_issue_inspector(issue: HealthIssue, width: f32) -> Element<'static, Message> {
    let mut content =
        column![section_header("ISSUE", theme::Icon::Health)].spacing(theme::spacing::MD);

    content = content
        .push(health_field(
            "Severity",
            issue.severity.label().to_owned(),
            issue.severity,
        ))
        .push(health_field(
            "Category",
            issue.category.label().to_owned(),
            issue.severity,
        ))
        .push(health_field(
            "Problem",
            issue.message.clone(),
            issue.severity,
        ));

    if let Some(path) = issue.relative_path.as_ref() {
        content = content.push(health_field(
            "Document",
            path.display().to_string(),
            issue.severity,
        ));
    }
    if let Some(property) = issue.property.as_ref() {
        content = content.push(health_field("Property", property.clone(), issue.severity));
    }
    if let Some(expected) = issue.expected {
        content = content.push(health_field(
            "Expected",
            expected.label().to_owned(),
            issue.severity,
        ));
    }
    if let Some(found) = issue.found {
        content = content.push(health_field(
            "Found",
            found.label().to_owned(),
            issue.severity,
        ));
    }

    if !issue.details.is_empty() {
        content = content
            .push(subtle_divider())
            .push(section_header("DETALHES", theme::Icon::FileText));
        for detail in issue.details {
            content = content.push(
                text(detail)
                    .size(theme::typography::BODY)
                    .style(theme::text_muted)
                    .wrapping(Wrapping::WordOrGlyph),
            );
        }
    }

    if issue.document_path.is_some() {
        content = content.push(subtle_divider()).push(
            button(
                row![
                    widgets::icon(theme::Icon::FileText, theme::icons::META, true),
                    text("Abrir documento").size(theme::typography::BODY),
                ]
                .spacing(theme::spacing::SM)
                .align_y(Alignment::Center),
            )
            .padding([5.0, theme::spacing::MD])
            .style(theme::button_toolbar)
            .on_press(Message::HealthIssueOpened(issue.id)),
        );
    }

    container(scrollable(content).direction(Direction::Vertical(
        Scrollbar::default().width(4).scroller_width(4).spacing(8),
    )))
    .width(width)
    .height(Length::Fill)
    .padding(theme::spacing::XL)
    .style(theme::inspector_panel)
    .into()
}

fn health_field(
    label: &'static str,
    value: String,
    severity: HealthSeverity,
) -> Element<'static, Message> {
    let style = match severity {
        HealthSeverity::Error => theme::text_error,
        HealthSeverity::Warning => theme::text_warning,
        HealthSeverity::Info => theme::text_normal,
    };
    container(
        column![
            text(label)
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
            text(value)
                .size(theme::typography::BODY)
                .style(style)
                .wrapping(Wrapping::WordOrGlyph)
                .width(Length::Fill),
        ]
        .spacing(theme::spacing::XXS),
    )
    .width(Length::Fill)
    .padding([3.0, 0.0])
    .into()
}

fn relation_row(relation: InspectorRelation, outgoing: bool) -> Element<'static, Message> {
    let property = relation.property;
    let label = relation.label;
    let status = relation.status;
    let target_path = relation.target_path;
    let candidates = relation.candidates;

    let status_text = match status {
        InspectorRelationStatus::Resolved => None,
        InspectorRelationStatus::Unresolved => Some(String::from("Não resolvido")),
        InspectorRelationStatus::Ambiguous(count) => {
            Some(format!("Ambíguo — {count} documentos correspondem"))
        }
    };

    let target: Element<'static, Message> = if let Some(path) = target_path {
        button(
            row![
                text(label)
                    .size(theme::typography::BODY)
                    .style(theme::text_accent)
                    .width(Length::Fill),
                text("→")
                    .size(theme::typography::BODY)
                    .style(theme::text_muted),
            ]
            .spacing(theme::spacing::SM)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .height(32)
        .padding([0.0, theme::spacing::SM])
        .style(theme::button_ghost)
        .on_press(Message::MarkdownSelected(path))
        .into()
    } else {
        row![
            widgets::icon(theme::Icon::Health, theme::icons::META, true),
            text(label)
                .size(theme::typography::BODY)
                .style(theme::text_normal)
                .width(Length::Fill),
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center)
        .into()
    };

    let mut details = column![
        text(property)
            .size(theme::typography::LABEL)
            .style(theme::text_muted),
        target,
    ]
    .spacing(theme::spacing::XS);

    if let Some(status_text) = status_text {
        details = details.push(
            text(status_text)
                .size(theme::typography::LABEL)
                .style(theme::text_muted)
                .wrapping(Wrapping::WordOrGlyph),
        );
    }

    if !outgoing && matches!(status, InspectorRelationStatus::Resolved) {
        details = details.push(
            text("referência estruturada")
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
        );
    }

    for candidate in candidates.into_iter().take(4) {
        details = details.push(
            text(candidate.relative_path.display().to_string())
                .font(theme::mono())
                .size(theme::typography::LABEL)
                .style(theme::text_muted)
                .wrapping(Wrapping::WordOrGlyph),
        );
    }

    container(details)
        .width(Length::Fill)
        .padding([4.0, 0.0])
        .into()
}

fn section_header<'a>(title: &'a str, icon: theme::Icon) -> Element<'a, Message> {
    row![
        widgets::icon(icon, theme::icons::META, true),
        widgets::section_title(title)
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center)
    .into()
}

fn field_row(field: InspectorField) -> Element<'static, Message> {
    let value = field.value;
    let is_mono = matches!(
        value,
        InspectorValue::Number(_) | InspectorValue::Bool(_) | InspectorValue::Empty
    );
    let muted = matches!(value, InspectorValue::Empty);

    container(
        column![
            text(field.label)
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
            text(value.display_value())
                .size(theme::typography::BODY)
                .font(if is_mono {
                    theme::mono()
                } else {
                    theme::typography::UI
                })
                .style(if muted {
                    theme::text_muted
                } else {
                    theme::text_normal
                })
                .wrapping(Wrapping::WordOrGlyph)
                .line_height(LineHeight::Relative(1.25))
                .width(Length::Fill),
        ]
        .spacing(theme::spacing::XXS),
    )
    .width(Length::Fill)
    .padding([4.0, 0.0])
    .into()
}

fn warning_row(warning: String) -> Element<'static, Message> {
    container(
        row![
            widgets::icon(theme::Icon::Health, theme::icons::META, true),
            text(warning)
                .size(theme::typography::BODY)
                .style(theme::text_warning)
                .wrapping(Wrapping::WordOrGlyph)
                .width(Length::Fill),
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Start),
    )
    .padding([3.0, 0.0])
    .into()
}

fn chip(label: String) -> Element<'static, Message> {
    container(
        text(label)
            .size(theme::typography::LABEL)
            .style(theme::text_accent)
            .wrapping(Wrapping::WordOrGlyph),
    )
    .padding([3.0, 8.0])
    .style(theme::chip)
    .into()
}

fn wrapped_text(value: String, size: u32, muted: bool) -> Element<'static, Message> {
    text(value)
        .size(size)
        .style(if muted {
            theme::text_muted
        } else {
            theme::text_normal
        })
        .wrapping(Wrapping::WordOrGlyph)
        .line_height(LineHeight::Relative(1.25))
        .width(Length::Fill)
        .into()
}

fn subtle_divider<'a>() -> Element<'a, Message> {
    container("")
        .height(1)
        .width(Length::Fill)
        .style(theme::divider)
        .into()
}
