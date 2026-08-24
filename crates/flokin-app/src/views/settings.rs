use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::{message::Message, theme, widgets};

pub fn view<'a>(
    app_theme: theme::AppTheme,
    left_visible: bool,
    right_visible: bool,
) -> Element<'a, Message> {
    let theme_row = row![
        text("Tema").width(Length::Fill),
        button(text("Light"))
            .style(if app_theme == theme::AppTheme::Light {
                theme::button_selected
            } else {
                theme::button_toolbar
            })
            .on_press(Message::ThemeSelected(true)),
        button(text("Dark"))
            .style(if app_theme == theme::AppTheme::Dark {
                theme::button_selected
            } else {
                theme::button_toolbar
            })
            .on_press(Message::ThemeSelected(false)),
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    let left_label = if left_visible {
        "Ocultar barra lateral esquerda"
    } else {
        "Mostrar barra lateral esquerda"
    };
    let right_label = if right_visible {
        "Ocultar barra lateral direita"
    } else {
        "Mostrar barra lateral direita"
    };
    let content = column![
        widgets::section_title("APARÊNCIA"),
        theme_row,
        widgets::section_title("LAYOUT"),
        button(row![
            text(left_label).width(Length::Fill),
            text(if left_visible { "ON" } else { "OFF" })
                .font(theme::mono())
                .style(theme::text_muted)
        ])
        .width(Length::Fill)
        .style(theme::button_toolbar)
        .on_press(Message::ToggleLeftSidebar),
        button(row![
            text(right_label).width(Length::Fill),
            text(if right_visible { "ON" } else { "OFF" })
                .font(theme::mono())
                .style(theme::text_muted)
        ])
        .width(Length::Fill)
        .style(theme::button_toolbar)
        .on_press(Message::ToggleRightSidebar),
        button(text("Restaurar layout padrão"))
            .style(theme::button_toolbar)
            .on_press(Message::ResetLayout),
    ]
    .spacing(theme::spacing::MD)
    .max_width(560);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(theme::spacing::XXL)
        .style(theme::editor)
        .into()
}
