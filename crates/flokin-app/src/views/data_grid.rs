use iced::widget::{container, text};
use iced::{alignment, Element};

use crate::{message::Message, theme};

pub const GUTTER_WIDTH: f32 = 42.0;
pub const ROW_HEIGHT: f32 = 28.0;
pub const HEADER_HEIGHT: f32 = 30.0;

pub fn header_gutter() -> Element<'static, Message> {
    container(text("#").font(theme::mono()).size(theme::typography::LABEL))
        .width(GUTTER_WIDTH)
        .height(HEADER_HEIGHT)
        .padding([7.0, theme::spacing::SM])
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
        .style(theme::data_header)
        .into()
}

pub fn grid_width(gutter: bool, columns: impl Iterator<Item = f32>) -> f32 {
    columns.sum::<f32>() + if gutter { GUTTER_WIDTH } else { 0.0 }
}
