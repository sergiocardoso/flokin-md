use iced::widget::{button, column, container, row, scrollable, text};
use iced::{alignment, Alignment, Element, Length};

use crate::{
    brand,
    i18n::I18nCatalog,
    message::Message,
    services::external_links::{AboutContactLink, AUTHOR_EMAIL},
    theme::{self, AppTheme},
    widgets,
};

pub fn view<'a>(app_theme: AppTheme, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    let content = column![
        header(app_theme, i18n),
        divider(),
        section(
            i18n.tr("about-motivation-title"),
            vec![
                i18n.tr("about-motivation-paragraph-1"),
                i18n.tr("about-motivation-paragraph-2"),
                i18n.tr("about-motivation-paragraph-3"),
                i18n.tr("about-motivation-paragraph-4"),
            ],
        ),
        highlight(i18n.tr("about-context-highlight")),
        divider(),
        creator_section(i18n),
        divider(),
        footer(i18n),
        row![button(text(i18n.tr("action-close")))
            .style(theme::button_toolbar)
            .on_press(Message::AboutClosed)]
        .align_y(Alignment::Center)
    ]
    .spacing(theme::spacing::LG)
    .max_width(760);

    container(scrollable(
        container(content)
            .width(Length::Fill)
            .padding([theme::spacing::XXL, theme::spacing::XL])
            .align_x(alignment::Horizontal::Center),
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(alignment::Horizontal::Center)
    .into()
}

fn header<'a>(app_theme: AppTheme, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    column![
        brand::welcome_lockup(app_theme),
        text(i18n.tr_with(
            "about-version",
            &[("version", env!("CARGO_PKG_VERSION").into())]
        ))
        .size(theme::typography::LABEL)
        .style(theme::text_muted),
        text(i18n.tr("about-tagline"))
            .size(theme::typography::TITLE)
            .style(theme::text_normal)
            .align_x(alignment::Horizontal::Center),
    ]
    .spacing(theme::spacing::SM)
    .align_x(Alignment::Center)
    .into()
}

fn section<'a>(title: String, paragraphs: Vec<String>) -> Element<'a, Message> {
    let mut body = column![section_title(title)].spacing(theme::spacing::SM);
    for paragraph in paragraphs {
        body = body.push(paragraph_text(paragraph));
    }
    body.into()
}

fn creator_section<'a>(i18n: &'a I18nCatalog) -> Element<'a, Message> {
    column![
        section_title(i18n.tr("about-creator-title")),
        text("Sérgio Cardoso")
            .size(theme::typography::TITLE)
            .style(theme::text_normal),
        text(i18n.tr("about-creator-role"))
            .size(theme::typography::BODY)
            .style(theme::text_muted),
        paragraph_text(i18n.tr("about-creator-paragraph-1")),
        paragraph_text(i18n.tr("about-creator-paragraph-2")),
        paragraph_text(i18n.tr("about-flokin-project")),
        contact_buttons(i18n),
        text(AUTHOR_EMAIL)
            .size(theme::typography::LABEL)
            .style(theme::text_muted)
    ]
    .spacing(theme::spacing::SM)
    .into()
}

fn contact_buttons<'a>(i18n: &'a I18nCatalog) -> Element<'a, Message> {
    row![
        contact_button(
            theme::Icon::ExternalLink,
            i18n.tr("about-linkedin"),
            AboutContactLink::LinkedIn,
        ),
        contact_button(
            theme::Icon::Globe,
            i18n.tr("about-website"),
            AboutContactLink::Website,
        ),
        contact_button(
            theme::Icon::Mail,
            i18n.tr("about-email"),
            AboutContactLink::Email,
        ),
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center)
    .into()
}

fn contact_button<'a>(
    icon: theme::Icon,
    label: String,
    link: AboutContactLink,
) -> Element<'a, Message> {
    button(
        container(widgets::icon_text(
            icon,
            label,
            theme::icons::TOOLBAR,
            false,
        ))
        .height(Length::Fill)
        .align_y(alignment::Vertical::Center),
    )
    .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT)
    .padding([0.0, 12.0])
    .style(theme::button_toolbar)
    .on_press(Message::AboutContactOpened(link))
    .into()
}

fn footer<'a>(i18n: &'a I18nCatalog) -> Element<'a, Message> {
    column![
        row![
            text(i18n.tr("about-open-source"))
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
            text("·").size(theme::typography::LABEL).style(theme::text_muted),
            text(i18n.tr("about-built-with"))
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
            text("·").size(theme::typography::LABEL).style(theme::text_muted),
            text(i18n.tr("about-flokin-project-short"))
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
        ]
        .spacing(theme::spacing::XS)
        .align_y(Alignment::Center),
        text(i18n.tr("about-manifesto"))
            .size(theme::typography::TITLE)
            .style(theme::text_accent)
            .align_x(alignment::Horizontal::Center),
    ]
    .spacing(theme::spacing::SM)
    .align_x(Alignment::Center)
    .into()
}

fn section_title<'a>(title: String) -> Element<'a, Message> {
    text(title.to_uppercase())
        .size(theme::typography::LABEL)
        .style(theme::text_accent)
        .into()
}

fn paragraph_text<'a>(value: String) -> Element<'a, Message> {
    text(value)
        .size(theme::typography::BODY)
        .style(theme::text_normal)
        .wrapping(iced::widget::text::Wrapping::Word)
        .into()
}

fn highlight<'a>(value: String) -> Element<'a, Message> {
    container(
        text(value)
            .size(theme::typography::BODY)
            .style(theme::text_accent)
            .wrapping(iced::widget::text::Wrapping::Word),
    )
    .padding([theme::spacing::SM, 0.0])
    .into()
}

fn divider<'a>() -> Element<'a, Message> {
    container("").height(1).width(Length::Fill).style(theme::divider).into()
}

pub const fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::app_version;

    #[test]
    fn about_version_comes_from_cargo_metadata() {
        assert_eq!(app_version(), env!("CARGO_PKG_VERSION"));
    }
}
