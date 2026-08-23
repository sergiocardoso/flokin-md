use flokin_core::ShellModel;
use iced::widget::{column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::{message::Message, theme, widgets};

pub fn view(model: &ShellModel) -> Element<'_, Message> {
    let mut properties = section_header("PROPRIEDADES", theme::Icon::Settings);
    for field in model.inspector.iter().take(5) {
        properties = properties.push(widgets::field_row(field.label, field.value));
    }

    let mut metadata = section_header("METADADOS", theme::Icon::FileText);
    for field in model.inspector.iter().skip(5) {
        metadata = metadata.push(widgets::field_row(field.label, field.value));
    }

    let mut tags = column![widgets::section_title("TAGS")].spacing(theme::spacing::MD);
    for tag in &model.tags {
        tags = tags.push(
            container(
                row![
                    widgets::chip(tag.label),
                    text(tag.count.to_string())
                        .size(theme::typography::BODY)
                        .font(theme::mono())
                        .style(theme::text_muted),
                ]
                .spacing(theme::spacing::SM)
                .align_y(Alignment::Center),
            )
            .padding([3.0, 0.0]),
        );
    }

    container(scrollable(
        column![
            properties,
            subtle_divider(),
            metadata,
            subtle_divider(),
            tags
        ]
        .spacing(theme::spacing::XXL),
    ))
    .width(300)
    .height(Length::Fill)
    .padding(theme::spacing::LG)
    .style(theme::panel)
    .into()
}

fn section_header<'a>(title: &'a str, icon: theme::Icon) -> iced::widget::Column<'a, Message> {
    column![row![
        widgets::icon(icon, theme::icons::META, true),
        widgets::section_title(title)
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center)]
    .spacing(theme::spacing::MD)
    .width(Length::Fill)
}

fn subtle_divider<'a>() -> Element<'a, Message> {
    container("")
        .height(1)
        .width(Length::Fill)
        .style(theme::divider)
        .into()
}
