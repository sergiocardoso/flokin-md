use flokin_core::ShellModel;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::{message::Message, theme, widgets};

pub fn view(model: &ShellModel) -> Element<'_, Message> {
    let workspace = model.workspace_display();
    let header = column![
        widgets::section_title("EXPLORER"),
        row![
            widgets::icon(theme::Icon::Database, theme::icons::META, true),
            text(workspace.name).size(theme::typography::TITLE)
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center),
        text(workspace.path)
            .size(theme::typography::LABEL)
            .font(theme::mono())
            .style(theme::text_muted),
    ]
    .spacing(theme::spacing::SM);

    let tree = if model.current_workspace.is_some() {
        workspace_pending()
    } else {
        no_workspace()
    };

    let filters = filters();

    container(
        column![header, scrollable(tree).height(Length::Fill), filters].spacing(theme::spacing::XL),
    )
    .width(272)
    .height(Length::Fill)
    .padding(theme::spacing::LG)
    .style(theme::panel)
    .into()
}

fn workspace_pending<'a>() -> iced::widget::Column<'a, Message> {
    column![container(
        row![
            widgets::icon(theme::Icon::Folder, theme::icons::TREE, false),
            text("O conteúdo será analisado na próxima etapa")
                .size(theme::typography::BODY)
                .style(theme::text_muted)
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center),
    )
    .padding([5.0, 8.0])]
    .spacing(theme::spacing::XXS)
}

fn no_workspace<'a>() -> iced::widget::Column<'a, Message> {
    column![
        text("Nenhuma pasta aberta")
            .size(theme::typography::BODY)
            .style(theme::text_muted),
        button(
            row![
                widgets::icon(theme::Icon::Folder, theme::icons::TOOLBAR, false),
                text("Abrir pasta").size(theme::typography::BODY)
            ]
            .spacing(theme::spacing::SM)
            .align_y(Alignment::Center)
        )
        .padding([6.0, 9.0])
        .style(theme::button_toolbar)
        .on_press(Message::OpenFolder)
    ]
    .spacing(theme::spacing::MD)
}

fn filters<'a>() -> Element<'a, Message> {
    let list = column![
        widgets::section_title("FILTROS"),
        text("Disponíveis após indexação")
            .size(theme::typography::BODY)
            .style(theme::text_muted)
    ]
    .spacing(theme::spacing::MD);

    container(list)
        .padding([theme::spacing::LG, theme::spacing::XS])
        .into()
}
