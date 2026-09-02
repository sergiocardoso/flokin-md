use iced::widget::{
    button, column, container, row, svg, text,
    text::{LineHeight, Wrapping},
};
use iced::{alignment, Alignment, Element};

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

#[allow(dead_code)]
pub fn toolbar_button(
    label: &str,
    icon_id: Icon,
    on_press: Message,
) -> button::Button<'_, Message> {
    button(icon_text(icon_id, label, theme::icons::TOOLBAR, false))
        .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT)
        .padding([0.0, 10.0])
        .style(theme::button_toolbar)
        .on_press(on_press)
}

pub fn tab_button<'a>(label: &'a str, selected: bool, on_press: Message) -> Element<'a, Message> {
    let style = if selected {
        theme::button_tab_selected
    } else {
        theme::button_tab
    };

    column![button(text(label).size(theme::typography::BODY))
        .padding([0.0, 16.0])
        .height(theme::sizes::TAB_BUTTON_HEIGHT)
        .style(style)
        .on_press(on_press)]
    .spacing(0)
    .into()
}

pub fn icon(icon: Icon, size: f32, accent: bool) -> Element<'static, Message> {
    icon_slot(icon, size, default_icon_slot(size), accent)
}

pub fn icon_slot(icon: Icon, size: f32, slot: f32, accent: bool) -> Element<'static, Message> {
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

    container(
        svg(svg::Handle::from_memory(data))
            .width(size)
            .height(size)
            .style(style),
    )
    .width(slot)
    .height(slot)
    .align_x(alignment::Horizontal::Center)
    .align_y(alignment::Vertical::Center)
    .into()
}

pub fn button_label<'a>(label: &'a str) -> Element<'a, Message> {
    text(label)
        .size(theme::typography::BODY)
        .line_height(LineHeight::Relative(1.0))
        .wrapping(Wrapping::None)
        .into()
}

pub fn icon_text<'a>(
    icon_id: Icon,
    label: &'a str,
    icon_size: f32,
    accent: bool,
) -> Element<'a, Message> {
    row![icon(icon_id, icon_size, accent), button_label(label)]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center)
        .into()
}

fn default_icon_slot(size: f32) -> f32 {
    if size <= theme::icons::TREE {
        theme::sizes::ICON_SLOT_SMALL
    } else if size < theme::icons::ACTIVITY {
        theme::sizes::ICON_SLOT_MEDIUM
    } else {
        theme::sizes::ICON_SLOT_LARGE
    }
}
