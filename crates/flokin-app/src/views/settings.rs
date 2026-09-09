use flokin_core::UpdateChannel;
use iced::widget::{button, column, container, pick_list, row, text};
use iced::{Alignment, Element, Length};

use crate::{
    brand,
    i18n::{AppLanguage, I18nCatalog},
    message::Message,
    theme, widgets,
};

use super::about::app_version;

#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    app_theme: theme::AppTheme,
    language: AppLanguage,
    i18n: &'a I18nCatalog,
    left_visible: bool,
    right_visible: bool,
    workspace_open: bool,
    update_auto_check_enabled: bool,
    update_channel: UpdateChannel,
) -> Element<'a, Message> {
    let language_row = row![
        text(i18n.tr("settings-language")).width(Length::Fill),
        pick_list(
            AppLanguage::all(),
            Some(language),
            Message::LanguageSelected
        )
        .width(Length::Fixed(220.0))
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    let theme_row = row![
        text(i18n.tr("settings-theme")).width(Length::Fill),
        button(text(i18n.tr("theme-light")))
            .style(if app_theme == theme::AppTheme::Light {
                theme::button_selected
            } else {
                theme::button_toolbar
            })
            .on_press(Message::ThemeSelected(true)),
        button(text(i18n.tr("theme-dark")))
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

    let installed_version_row = row![
        text(i18n.tr("settings-update-installed-version")).width(Length::Fill),
        text(app_version())
            .font(theme::mono())
            .style(theme::text_muted),
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    let auto_check_row = button(row![
        text(i18n.tr("settings-update-auto-check")).width(Length::Fill),
        text(if update_auto_check_enabled {
            i18n.tr("state-on")
        } else {
            i18n.tr("state-off")
        })
        .font(theme::mono())
        .style(theme::text_muted)
    ])
    .width(Length::Fill)
    .style(theme::button_toolbar)
    .on_press(Message::UpdateAutoCheckToggled(!update_auto_check_enabled));

    let channel_row = row![
        text(i18n.tr("settings-update-channel")).width(Length::Fill),
        button(text(i18n.tr("settings-update-channel-stable")))
            .style(if update_channel == UpdateChannel::Stable {
                theme::button_selected
            } else {
                theme::button_toolbar
            })
            .on_press(Message::UpdateChannelSelected(UpdateChannel::Stable)),
        button(text(i18n.tr("settings-update-channel-prerelease")))
            .style(if update_channel == UpdateChannel::Prerelease {
                theme::button_selected
            } else {
                theme::button_toolbar
            })
            .on_press(Message::UpdateChannelSelected(UpdateChannel::Prerelease)),
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    let channel_description = text(match update_channel {
        UpdateChannel::Stable => i18n.tr("settings-update-channel-stable-description"),
        UpdateChannel::Prerelease => i18n.tr("settings-update-channel-prerelease-description"),
    })
    .size(theme::typography::LABEL)
    .style(theme::text_muted);

    let check_now_row = row![
        text(i18n.tr("settings-update-check-now")).width(Length::Fill),
        button(text(i18n.tr("menu-check-updates")))
            .style(theme::button_toolbar)
            .on_press(Message::UpdateCheckRequested(true)),
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    let mut content = column![
        widgets::section_title(i18n.tr("settings-section-interface")),
        language_row,
        widgets::section_title(i18n.tr("settings-section-appearance")),
        theme_row,
        widgets::section_title(i18n.tr("settings-section-updates")),
        installed_version_row,
        auto_check_row,
        channel_row,
        channel_description,
        check_now_row,
    ]
    .spacing(theme::spacing::MD)
    .max_width(560);

    if workspace_open {
        content = content
            .push(widgets::section_title(i18n.tr("settings-section-layout")))
            .push(
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
            )
            .push(
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
            )
            .push(
                button(text(i18n.tr("settings-reset-layout")))
                    .style(theme::button_toolbar)
                    .on_press(Message::ResetLayout),
            );
    }

    container(
        iced::widget::stack![
            brand::watermark(),
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(theme::spacing::XXL)
        ]
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::editor)
    .into()
}

#[cfg(test)]
mod tests {
    use super::app_version;

    #[test]
    fn installed_version_row_reads_cargo_pkg_version_not_a_hardcoded_string() {
        // The Settings > Updates "Installed version" row renders `app_version()`.
        // This proves that value is sourced from Cargo/build metadata at compile
        // time, not a literal string someone could forget to update on release.
        assert_eq!(app_version(), env!("CARGO_PKG_VERSION"));
    }
}
