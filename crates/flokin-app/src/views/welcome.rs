use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::{brand, i18n::I18nCatalog, message::Message, theme, theme::AppTheme, widgets};

pub fn view<'a>(
    app_theme: AppTheme,
    i18n: &'a I18nCatalog,
    notice: Option<&'a str>,
) -> Element<'a, Message> {
    let mut content = column![
        brand::welcome_lockup(app_theme),
        iced::widget::text(i18n.tr("welcome-title"))
            .size(theme::typography::TITLE)
            .style(theme::text_muted),
        button(
            container(
                row![
                    widgets::icon_inverse(theme::Icon::Folder, theme::icons::TOOLBAR),
                    text(i18n.tr("welcome-open-folder")).size(theme::typography::BODY),
                ]
                .spacing(theme::spacing::SM)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
        )
        .width(Length::Fixed(220.0))
        .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT)
        .padding([0.0, theme::spacing::LG])
        .style(theme::welcome_button)
        .on_press(Message::OpenFolder),
    ]
    .spacing(theme::spacing::LG)
    .align_x(Alignment::Center);

    if let Some(notice) = notice {
        content = content.push(
            iced::widget::text(notice)
                .size(theme::typography::BODY)
                .style(theme::text_warning),
        );
    }

    container(
        iced::widget::stack![
            brand::watermark(),
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::editor)
    .into()
}
