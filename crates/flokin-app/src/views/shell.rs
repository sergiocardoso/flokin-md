use flokin_core::{Activity, ShellModel};
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Alignment, Element, Length};

use crate::{
    message::Message,
    theme::{self, AppTheme},
    views, widgets,
};

pub fn view(model: &ShellModel, app_theme: AppTheme) -> Element<'_, Message> {
    column![
        menu_bar(),
        toolbar(app_theme),
        row![
            activity_bar(model),
            views::explorer::view(model, app_theme),
            widgets::divider(),
            workspace(model),
            widgets::divider(),
            views::inspector::view(model),
        ]
        .height(Length::Fill),
        views::status_bar::view(model),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn menu_bar<'a>() -> Element<'a, Message> {
    const ITEMS: [&str; 7] = [
        "Arquivo",
        "Editar",
        "Exibir",
        "Navegar",
        "Dados",
        "Ferramentas",
        "Ajuda",
    ];

    let mut row = row![text("FlokinMD")
        .size(theme::typography::TITLE)
        .style(theme::text_accent)]
    .spacing(theme::spacing::LG)
    .align_y(Alignment::Center);

    for item in ITEMS {
        row = row.push(
            button(text(item).size(theme::typography::MENU))
                .padding([3, 6])
                .style(theme::button_ghost)
                .on_press(Message::MockAction),
        );
    }

    container(row)
        .height(32)
        .padding([0.0, theme::spacing::MD])
        .style(theme::panel)
        .into()
}

fn toolbar(app_theme: AppTheme) -> Element<'static, Message> {
    let search = row![
        widgets::icon(theme::Icon::Search, theme::icons::TOOLBAR, false),
        text_input("Buscar...", "")
            .padding([4, 8])
            .size(theme::typography::BODY)
            .width(250)
            .style(theme::input),
        text("Ctrl+K")
            .size(theme::typography::LABEL)
            .font(theme::mono())
            .style(theme::text_muted)
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    let left = row![
        widgets::toolbar_button("Abrir pasta", theme::Icon::Folder, Message::OpenFolder),
        widgets::toolbar_button("Reindexar", theme::Icon::Refresh, Message::ReindexWorkspace),
        widgets::toolbar_button("Novo", theme::Icon::Plus, Message::MockAction),
        container("").width(1).height(22).style(theme::divider),
        search
    ]
    .spacing(theme::spacing::MD)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    let right = row![
        widgets::icon_button(theme::Icon::PanelLeft),
        widgets::icon_button(theme::Icon::Split),
        button(
            row![
                widgets::icon(theme::Icon::Settings, theme::icons::TOOLBAR, false),
                text(app_theme.label()).size(theme::typography::BODY)
            ]
            .spacing(theme::spacing::SM)
            .align_y(Alignment::Center)
        )
        .padding([5.0, 10.0])
        .style(theme::button_toolbar)
        .on_press(Message::ThemeToggled)
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    container(row![left, right].align_y(Alignment::Center))
        .height(50)
        .padding([0.0, theme::spacing::LG])
        .style(theme::elevated)
        .into()
}

fn activity_bar(model: &ShellModel) -> Element<'_, Message> {
    let mut items = column![]
        .spacing(theme::spacing::SM)
        .align_x(Alignment::Center);

    for activity in flokin_core::Activity::ALL {
        let selected = activity == model.active_activity;
        let style = if selected {
            theme::button_selected
        } else {
            theme::button_activity
        };

        items = items.push(
            button(widgets::icon(
                activity_icon(activity),
                theme::icons::ACTIVITY,
                selected,
            ))
            .width(40)
            .height(40)
            .padding(0)
            .style(style)
            .on_press(Message::ActivitySelected(activity)),
        );
    }

    container(items)
        .width(56)
        .height(Length::Fill)
        .padding([theme::spacing::LG, theme::spacing::SM])
        .style(theme::panel)
        .into()
}

fn workspace(model: &ShellModel) -> Element<'_, Message> {
    column![views::editor::tabs(model), views::editor::view(model)]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn activity_icon(activity: Activity) -> theme::Icon {
    match activity {
        Activity::Explorer => theme::Icon::Database,
        Activity::Relations => theme::Icon::GitBranch,
        Activity::Links => theme::Icon::Link,
        Activity::Tags => theme::Icon::Tag,
        Activity::Calendar => theme::Icon::Calendar,
        Activity::Favorites => theme::Icon::Heart,
        Activity::History => theme::Icon::Clock,
        Activity::Terminal => theme::Icon::Terminal,
        Activity::Settings => theme::Icon::Settings,
    }
}
