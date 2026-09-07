use iced::widget::{button, container, text};
use iced::{alignment, Element, Length};

use crate::{message::Message, theme};

pub const GUTTER_WIDTH: f32 = theme::sizes::DATA_GRID_GUTTER_WIDTH;
pub const ROW_HEIGHT: f32 = theme::sizes::DATA_GRID_ROW_HEIGHT;
pub const HEADER_HEIGHT: f32 = theme::sizes::DATA_GRID_HEADER_HEIGHT;

pub fn header_gutter() -> Element<'static, Message> {
    container(text("#").font(theme::mono()).size(theme::typography::LABEL))
        .width(GUTTER_WIDTH)
        .height(HEADER_HEIGHT)
        .padding([7.0, theme::spacing::SM])
        .align_y(alignment::Vertical::Center)
        .style(theme::data_gutter)
        .into()
}

pub fn row_gutter(row_index: usize, selected: bool) -> Element<'static, Message> {
    container(
        text((row_index + 1).to_string())
            .font(theme::mono())
            .size(theme::typography::LABEL)
            .style(theme::text_muted),
    )
    .width(GUTTER_WIDTH)
    .height(ROW_HEIGHT)
    .padding([7.0, theme::spacing::SM])
    .align_x(alignment::Horizontal::Right)
    .align_y(alignment::Vertical::Center)
    .style(move |theme| {
        if selected {
            theme::data_row(theme, row_index, true)
        } else {
            theme::data_gutter(theme)
        }
    })
    .into()
}

pub fn cell<'a>(
    content: impl Into<Element<'a, Message>>,
    width: f32,
    alignment: alignment::Horizontal,
) -> Element<'a, Message> {
    container(content)
        .width(width)
        .height(ROW_HEIGHT)
        .padding([6.0, theme::spacing::SM])
        .align_x(alignment)
        .align_y(alignment::Vertical::Center)
        .style(theme::data_cell)
        .into()
}

pub fn header_cell<'a>(
    content: impl Into<Element<'a, Message>>,
    width: f32,
) -> Element<'a, Message> {
    container(content)
        .width(width)
        .height(HEADER_HEIGHT)
        .padding([7.0, theme::spacing::SM])
        .align_y(alignment::Vertical::Center)
        .style(theme::data_header)
        .into()
}

pub fn selection_header<'a>(label: &'a str, width: f32, message: Message) -> Element<'a, Message> {
    container(
        button(
            container(
                text(label)
                    .font(theme::mono())
                    .size(theme::typography::LABEL),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center),
        )
        .width(width)
        .height(HEADER_HEIGHT)
        .padding(0)
        .style(theme::button_table_header)
        .on_press(message),
    )
    .width(width)
    .height(HEADER_HEIGHT)
    .style(theme::data_header)
    .into()
}

pub fn selection_cell<'a>(
    label: &'a str,
    width: f32,
    message: Message,
    selected: bool,
    row_index: usize,
) -> Element<'a, Message> {
    container(
        button(
            container(
                text(label)
                    .font(theme::mono())
                    .size(theme::typography::LABEL),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center),
        )
        .width(width)
        .height(ROW_HEIGHT)
        .padding(0)
        .style(theme::button_ghost)
        .on_press(message),
    )
    .width(width)
    .height(ROW_HEIGHT)
    .align_x(alignment::Horizontal::Center)
    .style(move |theme| theme::data_row(theme, row_index, selected))
    .into()
}

pub fn grid_width(gutter: bool, columns: impl Iterator<Item = f32>) -> f32 {
    columns.sum::<f32>() + if gutter { GUTTER_WIDTH } else { 0.0 }
}
