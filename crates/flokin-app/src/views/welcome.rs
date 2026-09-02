use iced::widget::{button, column, container};
use iced::{Alignment, Element, Length};

use crate::{brand, i18n::I18nCatalog, message::Message, theme, theme::AppTheme, widgets};

pub fn view<'a>(
    app_theme: AppTheme,
    i18n: &'a I18nCatalog,
    notice: Option<&'a str>,
) -> Element<'a, Message> {
    let mut content = column![
        brand::lockup(app_theme),
        iced::widget::text(i18n.tr("welcome-title"))
            .size(theme::typography::TITLE)
            .style(theme::text_muted),
        button(widgets::icon_text(
            theme::Icon::Folder,
            i18n.tr("welcome-open-folder"),
            theme::icons::TOOLBAR,
            false,
        ))
        .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT)
        .padding([0.0, theme::spacing::LG])
        .style(theme::button_primary)
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

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(theme::editor)
        .into()
}
