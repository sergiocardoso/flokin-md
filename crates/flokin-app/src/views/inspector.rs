use flokin_core::{
    HealthCategory, HealthIssue, HealthSeverity, InspectorField, InspectorModel, InspectorRelation,
    InspectorRelationStatus, InspectorValue, ShellModel,
};
use iced::widget::{
    button, column, container, row, scrollable,
    scrollable::{Direction, Scrollbar},
    text,
    text::{LineHeight, Wrapping},
};
use iced::{Alignment, Element, Length};

use crate::{
    i18n::I18nCatalog,
    message::Message,
    theme,
    views::health::{localized_health_issue_message, schema_type_label},
    widgets,
};

pub fn view<'a>(model: &'a ShellModel, width: f32, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    match model.document_inspector() {
        InspectorModel::Empty { title, description } => {
            empty_state(title, description, width, i18n)
        }
        InspectorModel::Document(inspector) => document_inspector(inspector, width, i18n),
        InspectorModel::HealthIssue(inspector) => {
            health_issue_inspector(inspector.issue, width, i18n)
        }
    }
}

fn empty_state<'a>(
    title: String,
    description: String,
    width: f32,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    container(scrollable(
        column![
            section_header(i18n.tr("inspector-properties"), theme::Icon::Settings),
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

fn document_inspector<'a>(
    inspector: flokin_core::DocumentInspector,
    width: f32,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let mut content = column![section_header(
        i18n.tr("inspector-properties"),
        theme::Icon::Settings
    )]
    .spacing(theme::spacing::MD);

    for field in inspector.properties {
        content = content.push(field_row(field));
    }

    if !inspector.outgoing_relations.is_empty() {
        content = content
            .push(subtle_divider())
            .push(section_header(i18n.tr("inspector-relations"), theme::Icon::FileText));

        for relation in inspector.outgoing_relations {
            content = content.push(relation_row(relation, true, i18n));
        }
    }

    if !inspector.incoming_relations.is_empty() {
        content = content
            .push(subtle_divider())
            .push(section_header(
                i18n.tr("inspector-referenced-by"),
                theme::Icon::Tag,
            ));

        for relation in inspector.incoming_relations {
            content = content.push(relation_row(relation, false, i18n));
        }
    }

    if !inspector.tags.is_empty() {
        content = content
            .push(subtle_divider())
            .push(section_header(i18n.tr("inspector-tags"), theme::Icon::Tag));

        let mut tags = column![].spacing(theme::spacing::XS);
        for tag in inspector.tags {
            tags = tags.push(chip(tag));
        }
        content = content.push(tags);
    }

    if !inspector.warnings.is_empty() {
        content = content
            .push(subtle_divider())
            .push(section_header(i18n.tr("inspector-warnings"), theme::Icon::Clock));

        for warning in inspector.warnings {
            content = content.push(warning_row(warning));
        }
    }

    content = content
        .push(subtle_divider())
        .push(section_header(i18n.tr("inspector-metadata"), theme::Icon::FileText));

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

fn health_issue_inspector<'a>(
    issue: HealthIssue,
    width: f32,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let mut content = column![section_header(
        i18n.tr("inspector-issue"),
        theme::Icon::Health
    )]
    .spacing(theme::spacing::MD);

    content = content
        .push(health_field(
            i18n.tr("health-severity"),
            severity_label(issue.severity, i18n),
            issue.severity,
        ))
        .push(health_field(
            i18n.tr("health-category"),
            health_category_label(issue.category, i18n),
            issue.severity,
        ))
        .push(health_field(
            i18n.tr("health-problem"),
            localized_health_issue_message(&issue, i18n),
            issue.severity,
        ));

    if let Some(path) = issue.relative_path.as_ref() {
        content = content.push(health_field(
            i18n.tr("health-document"),
            path.display().to_string(),
            issue.severity,
        ));
    }
    if let Some(property) = issue.property.as_ref() {
        content = content.push(health_field(
            i18n.tr("health-property"),
            property.clone(),
            issue.severity,
        ));
    }
    if let Some(expected) = issue.expected {
        content = content.push(health_field(
            i18n.tr("health-expected"),
            schema_type_label(expected, i18n),
            issue.severity,
        ));
    }
    if let Some(found) = issue.found {
        content = content.push(health_field(
            i18n.tr("health-found"),
            schema_type_label(found, i18n),
            issue.severity,
        ));
    }

    if !issue.details.is_empty() {
        content = content
            .push(subtle_divider())
            .push(section_header(i18n.tr("inspector-details"), theme::Icon::FileText));
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
                    text(i18n.tr("inspector-open-document")).size(theme::typography::BODY),
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
    label: String,
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

fn severity_label(severity: HealthSeverity, i18n: &I18nCatalog) -> String {
    match severity {
        HealthSeverity::Error => i18n.tr("health-severity-error"),
        HealthSeverity::Warning => i18n.tr("health-severity-warning"),
        HealthSeverity::Info => i18n.tr("health-severity-info"),
    }
}

fn health_category_label(category: HealthCategory, i18n: &I18nCatalog) -> String {
    match category {
        HealthCategory::Parsing => i18n.tr("health-category-parsing"),
        HealthCategory::Schema => i18n.tr("health-category-schema"),
        HealthCategory::Relations => i18n.tr("health-category-relations"),
        HealthCategory::Workspace => i18n.tr("health-category-workspace"),
    }
}

fn relation_row<'a>(
    relation: InspectorRelation,
    outgoing: bool,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let property = relation.property;
    let label = relation.label;
    let status = relation.status;
    let target_path = relation.target_path;
    let candidates = relation.candidates;

    let status_text = match status {
        InspectorRelationStatus::Resolved => None,
        InspectorRelationStatus::Unresolved => Some(i18n.tr("relation-unresolved")),
        InspectorRelationStatus::Ambiguous(count) => Some(i18n.tr_with(
            "relation-ambiguous-count",
            &[("count", count.into())],
        )),
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
            text(i18n.tr("relation-structured-reference"))
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

fn section_header<'a>(title: impl Into<String>, icon: theme::Icon) -> Element<'a, Message> {
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
