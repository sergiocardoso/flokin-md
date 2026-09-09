use flokin_core::UpdateStatus;
use iced::widget::{button, column, container, mouse_area, opaque, row, stack, text};
use iced::{alignment, Alignment, Element, Length};

use crate::{i18n::I18nCatalog, message::Message, theme, widgets};

/// Non-modal banner shown after an automatic check finds a compatible newer release.
/// Never appears at startup on its own; it is only inserted into the shell once a
/// check has actually completed with a result the user has not already dismissed/skipped.
pub fn banner<'a>(
    update: &'a flokin_core::UpdateInfo,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let message = i18n.tr_with(
        "update-banner-available",
        &[
            ("version", update.latest_version.to_string().into()),
            ("current", update.current_version.to_string().into()),
        ],
    );

    let actions = row![
        button(text(i18n.tr("update-action-release-notes")).size(theme::typography::LABEL))
            .padding([5.0, 10.0])
            .style(theme::button_toolbar)
            .on_press(Message::UpdateLinkOpened(update.release_url.clone())),
        button(text(i18n.tr("update-action-download")).size(theme::typography::LABEL))
            .padding([5.0, 10.0])
            .style(theme::button_primary)
            .on_press(Message::UpdateLinkOpened(update.release_url.clone())),
        button(text(i18n.tr("update-action-skip")).size(theme::typography::LABEL))
            .padding([5.0, 10.0])
            .style(theme::button_ghost)
            .on_press(Message::UpdateBannerSkipped),
        button(text(i18n.tr("update-action-remind-later")).size(theme::typography::LABEL))
            .padding([5.0, 10.0])
            .style(theme::button_ghost)
            .on_press(Message::UpdateBannerDismissed),
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    container(
        row![
            widgets::icon(theme::Icon::Refresh, theme::icons::TOOLBAR, true),
            text(message)
                .size(theme::typography::BODY)
                .style(theme::text_normal)
                .width(Length::Fill),
            actions,
        ]
        .spacing(theme::spacing::MD)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([theme::spacing::SM, theme::spacing::LG])
    .style(theme::update_banner)
    .into()
}

/// Centered dialog driven by the explicit "Check for Updates..." action. Unlike the
/// automatic banner, this always reports a definite outcome, including failures.
pub fn check_dialog_overlay<'a>(
    status: &'a UpdateStatus,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let mut content = column![text(i18n.tr("update-check-title"))
        .size(theme::typography::TITLE)
        .style(theme::text_accent)]
    .spacing(theme::spacing::SM);

    content = match status {
        UpdateStatus::Idle | UpdateStatus::Checking => content.push(
            text(i18n.tr("update-status-checking"))
                .size(theme::typography::BODY)
                .style(theme::text_muted),
        ),
        UpdateStatus::UpToDate => content.push(
            text(i18n.tr("update-status-up-to-date"))
                .size(theme::typography::BODY)
                .style(theme::text_success),
        ),
        UpdateStatus::Available(update) => content
            .push(
                text(i18n.tr_with(
                    "update-banner-available",
                    &[
                        ("version", update.latest_version.to_string().into()),
                        ("current", update.current_version.to_string().into()),
                    ],
                ))
                .size(theme::typography::BODY)
                .style(theme::text_normal),
            )
            .push(
                row![
                    button(
                        text(i18n.tr("update-action-release-notes")).size(theme::typography::BODY)
                    )
                    .padding([6.0, 12.0])
                    .style(theme::button_toolbar)
                    .on_press(Message::UpdateLinkOpened(update.release_url.clone())),
                    button(text(i18n.tr("update-action-download")).size(theme::typography::BODY))
                        .padding([6.0, 12.0])
                        .style(theme::button_primary)
                        .on_press(Message::UpdateLinkOpened(update.release_url.clone())),
                ]
                .spacing(theme::spacing::SM),
            ),
        UpdateStatus::Failed(error) => content.push(
            text(i18n.tr_with("update-status-failed", &[("error", error.as_str().into())]))
                .size(theme::typography::BODY)
                .wrapping(iced::widget::text::Wrapping::Word)
                .style(theme::text_warning),
        ),
    };

    let actions = row![
        button(text(i18n.tr("action-close")).size(theme::typography::BODY))
            .padding([6.0, 12.0])
            .style(theme::button_toolbar)
            .on_press(Message::UpdateCheckDialogClosed)
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    let dialog = container(content.push(actions))
        .width(theme::sizes::DIALOG_WIDTH)
        .padding(theme::spacing::LG)
        .style(theme::overlay_panel);

    opaque(
        stack![
            mouse_area(
                container("")
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(theme::overlay_backdrop)
            )
            .on_press(Message::UpdateCheckDialogClosed),
            container(dialog)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center),
        ]
        .width(Length::Fill)
        .height(Length::Fill),
    )
}
