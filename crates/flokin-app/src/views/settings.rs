use iced::widget::{button, column, container, pick_list, row, text};
use iced::{Alignment, Element, Length};

use crate::{
    i18n::{AppLanguage, I18nCatalog},
    message::Message,
    theme, widgets,
};

pub fn view<'a>(
    app_theme: theme::AppTheme,
    language: AppLanguage,
    i18n: &'a I18nCatalog,
    left_visible: bool,
    right_visible: bool,
) -> Element<'a, Message> {
    let language_row = row![
        text(i18n.tr("settings-language")).width(Length::Fill),
        pick_list(AppLanguage::all(), Some(language), Message::LanguageSelected)
            .width(Length::Fixed(220.0))
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    let theme_row = row![
        text(i18n.tr("settings-theme")).width(Length::Fill),
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
        i18n.tr("settings-hide-left-sidebar")
    } else {
        i18n.tr("settings-show-left-sidebar")
    };
    let right_label = if right_visible {
        i18n.tr("settings-hide-right-sidebar")
    } else {
        i18n.tr("settings-show-right-sidebar")
    };
    let content = column![
        widgets::section_title(i18n.tr("settings-section-interface")),
        language_row,
        widgets::section_title(i18n.tr("settings-section-appearance")),
        theme_row,
        widgets::section_title(i18n.tr("settings-section-layout")),
        button(row![
            text(left_label).width(Length::Fill),
            text(if left_visible {
                i18n.tr("state-on")
            } else {
                i18n.tr("state-off")
            })
                .font(theme::mono())
                .style(theme::text_muted)
        ])
        .width(Length::Fill)
        .style(theme::button_toolbar)
        .on_press(Message::ToggleLeftSidebar),
        button(row![
            text(right_label).width(Length::Fill),
            text(if right_visible {
                i18n.tr("state-on")
            } else {
                i18n.tr("state-off")
            })
                .font(theme::mono())
                .style(theme::text_muted)
        ])
        .width(Length::Fill)
        .style(theme::button_toolbar)
        .on_press(Message::ToggleRightSidebar),
        button(text(i18n.tr("settings-reset-layout")))
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
