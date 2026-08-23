use iced::widget::{button, column, container, row, svg, text};
use iced::{Alignment, Element, Length};

use crate::{
    message::Message,
    theme::{self, Icon},
};

pub fn section_title<'a>(label: &'a str) -> Element<'a, Message> {
    text(label)
        .size(theme::typography::LABEL)
        .font(theme::typography::UI)
        .style(theme::text_muted)
        .into()
}

pub fn toolbar_button(label: &str, icon_id: Icon) -> button::Button<'_, Message> {
    button(
        row![
            icon(icon_id, theme::icons::TOOLBAR, false),
            text(label).size(theme::typography::BODY)
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center),
    )
    .padding([6.0, 9.0])
    .style(theme::button_toolbar)
    .on_press(Message::MockAction)
}

pub fn icon_button(icon_id: Icon) -> button::Button<'static, Message> {
    button(icon(icon_id, theme::icons::TOOLBAR, false))
        .width(32)
        .height(30)
        .padding(0)
        .style(theme::button_toolbar)
        .on_press(Message::MockAction)
}

pub fn tab_button<'a>(label: &'a str, selected: bool, on_press: Message) -> Element<'a, Message> {
    let style = if selected {
        theme::button_tab_selected
    } else {
        theme::button_tab
    };

    let underline = if selected {
        container("")
            .height(2)
            .width(Length::Fill)
            .style(theme::tab_underline)
    } else {
        container("").height(2).width(Length::Fill)
    };

    column![
        button(text(label).size(theme::typography::BODY))
            .padding([7.0, 12.0])
            .style(style)
            .on_press(on_press),
        underline
    ]
    .spacing(0)
    .into()
}

pub fn tab_icon_button(icon_id: Icon, on_press: Message) -> Element<'static, Message> {
    column![
        button(icon(icon_id, theme::icons::TOOLBAR, false))
            .padding([7.0, 10.0])
            .style(theme::button_tab)
            .on_press(on_press),
        container("").height(2).width(Length::Fill)
    ]
    .spacing(0)
    .into()
}

pub fn divider<'a>() -> Element<'a, Message> {
    container("")
        .width(1)
        .height(Length::Fill)
        .style(theme::divider)
        .into()
}

pub fn field_row<'a>(label: &'a str, value: &'a str) -> Element<'a, Message> {
    container(
        row![
            text(label)
                .size(theme::typography::BODY)
                .style(theme::text_muted)
                .width(94),
            text(value)
                .size(theme::typography::BODY)
                .style(theme::text_normal)
                .width(Length::Fill)
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center),
    )
    .padding([3.0, 0.0])
    .into()
}

pub fn icon(icon: Icon, size: f32, accent: bool) -> Element<'static, Message> {
    let body = theme::icon_svg(icon)
        .trim_start_matches(r#"<svg viewBox="0 0 24 24">"#)
        .trim_end_matches("</svg>");
    let data = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">{}</svg>"##,
        body
    )
    .into_bytes();
    let style = if accent {
        theme::icon_accent_style
    } else {
        theme::icon_style
    };

    svg(svg::Handle::from_memory(data))
        .width(size)
        .height(size)
        .style(style)
        .into()
}

pub fn chip<'a>(label: &'a str) -> Element<'a, Message> {
    container(
        text(label)
            .size(theme::typography::LABEL)
            .style(theme::text_accent),
    )
    .padding([3.0, 8.0])
    .style(theme::chip)
    .into()
}
