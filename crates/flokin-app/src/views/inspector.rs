use flokin_core::{InspectorField, InspectorModel, InspectorValue, ShellModel};
use iced::widget::{
    column, container, row, scrollable, text,
    text::{LineHeight, Wrapping},
};
use iced::{Alignment, Element, Length};

use crate::{message::Message, theme, widgets};

pub fn view(model: &ShellModel, width: f32) -> Element<'_, Message> {
    match model.document_inspector() {
        InspectorModel::Empty { title, description } => empty_state(title, description, width),
        InspectorModel::Document(inspector) => document_inspector(inspector, width),
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
    .padding(theme::spacing::LG)
    .style(theme::panel)
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

    if !inspector.tags.is_empty() {
        content = content
            .push(subtle_divider())
            .push(section_header("TAGS", theme::Icon::Tag));

        let mut tags = column![].spacing(theme::spacing::SM);
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

    container(scrollable(content))
        .width(width)
        .height(Length::Fill)
        .padding(theme::spacing::LG)
        .style(theme::panel)
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
    .padding([3.0, 0.0])
    .into()
}

fn warning_row(warning: String) -> Element<'static, Message> {
    container(
        row![
            text("⚠")
                .size(theme::typography::BODY)
                .style(theme::text_warning),
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
