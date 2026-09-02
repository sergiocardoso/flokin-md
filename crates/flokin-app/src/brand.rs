use iced::widget::{container, row, svg};
use iced::{Alignment, Element, Length};

use crate::{message::Message, theme::AppTheme};

const LOGO_ICON: &[u8] = include_bytes!("../../../assets/logo-icon.svg");
const LOGO_ICON_BLACK: &[u8] = include_bytes!("../../../assets/logo-icon-black.svg");
const LOGO_TEXT: &[u8] = include_bytes!("../../../assets/logo-text.svg");
const LOGO_TEXT_DARK: &[u8] = include_bytes!("../../../assets/logo-text-dark.svg");

const ICON_SIZE: f32 = 26.0;
const WORDMARK_HEIGHT: f32 = 20.0;
const WORDMARK_WIDTH: f32 = 103.2;
const LOCKUP_GAP: f32 = 8.0;
const LOCKUP_WIDTH: f32 = ICON_SIZE + LOCKUP_GAP + WORDMARK_WIDTH;

pub fn lockup(app_theme: AppTheme) -> Element<'static, Message> {
    row![
        svg(svg::Handle::from_memory(LOGO_ICON.to_vec()))
            .width(ICON_SIZE)
            .height(ICON_SIZE),
        wordmark(app_theme),
    ]
    .spacing(LOCKUP_GAP)
    .align_y(Alignment::Center)
    .into()
}

pub fn placeholder() -> Element<'static, Message> {
    container("")
        .width(Length::Fixed(LOCKUP_WIDTH))
        .height(Length::Fixed(ICON_SIZE))
        .into()
}

pub fn watermark() -> Element<'static, Message> {
    container(
        svg(svg::Handle::from_memory(LOGO_ICON_BLACK.to_vec()))
            .width(360.0)
            .height(360.0)
            .opacity(0.07_f32),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Right)
    .align_y(iced::alignment::Vertical::Bottom)
    .into()
}

fn wordmark(app_theme: AppTheme) -> Element<'static, Message> {
    let data = match app_theme {
        AppTheme::Light => LOGO_TEXT,
        AppTheme::Dark => LOGO_TEXT_DARK,
    };

    svg(svg::Handle::from_memory(data.to_vec()))
        .width(WORDMARK_WIDTH)
        .height(WORDMARK_HEIGHT)
        .into()
}
