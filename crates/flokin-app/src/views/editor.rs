use flokin_core::{BottomTab, ShellModel, WorkspaceTab};
use iced::widget::{column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::{message::Message, theme, widgets};

pub fn tabs(model: &ShellModel) -> Element<'_, Message> {
    let mut tabs = row![]
        .spacing(theme::spacing::XXS)
        .align_y(Alignment::Center);

    for tab in WorkspaceTab::ALL {
        tabs = tabs.push(widgets::tab_button(
            tab.title(),
            tab == model.selected_tab,
            Message::WorkspaceTabSelected(tab),
        ));
    }

    tabs = tabs.push(widgets::tab_icon_button(
        theme::Icon::Plus,
        Message::MockAction,
    ));

    container(tabs)
        .height(38)
        .padding([0.0, theme::spacing::SM])
        .style(theme::surface)
        .into()
}

pub fn view(model: &ShellModel) -> Element<'_, Message> {
    column![breadcrumb(), editor_area(model), bottom_panel(model)]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn breadcrumb<'a>() -> Element<'a, Message> {
    container(
        row![
            text("Projects")
                .size(theme::typography::BODY)
                .style(theme::text_muted),
            text("›")
                .size(theme::typography::BODY)
                .style(theme::text_muted),
            text("carf.md").size(theme::typography::BODY),
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center),
    )
    .height(32)
    .padding([0.0, theme::spacing::MD])
    .style(theme::elevated)
    .into()
}

fn editor_area(model: &ShellModel) -> Element<'_, Message> {
    let mut lines = column![].spacing(0);

    for (index, line) in model.document.content.lines().enumerate() {
        lines = lines.push(editor_line(index + 1, line));
    }

    container(scrollable(lines).height(Length::Fill))
        .height(Length::FillPortion(3))
        .style(theme::editor)
        .into()
}

fn editor_line(line_number: usize, line: &str) -> Element<'_, Message> {
    let is_heading = line.starts_with('#');
    let is_active = line_number == 1;
    let line_text = if line.is_empty() { " " } else { line };
    let code = text(line_text)
        .font(theme::mono())
        .size(theme::typography::EDITOR)
        .style(if is_heading {
            theme::text_accent
        } else {
            theme::text_normal
        });

    let line_row = row![
        container(
            text(format!("{line_number:>3}"))
                .font(theme::mono())
                .size(theme::typography::LABEL)
                .style(theme::text_muted)
        )
        .width(62)
        .padding([3.0, theme::spacing::LG])
        .style(theme::gutter),
        container(code)
            .width(Length::Fill)
            .padding([3.0, theme::spacing::LG])
    ]
    .height(24);

    if is_active {
        container(line_row).style(theme::active_line).into()
    } else {
        line_row.into()
    }
}

fn bottom_panel(model: &ShellModel) -> Element<'_, Message> {
    let mut tabs = row![]
        .spacing(theme::spacing::XXS)
        .align_y(Alignment::Center);

    for tab in BottomTab::ALL {
        tabs = tabs.push(widgets::tab_button(
            tab.title(),
            tab == model.bottom_tab,
            Message::BottomTabSelected(tab),
        ));
    }

    let preview = column![
        row![
            widgets::tab_button("Prévia", true, Message::MockAction),
            widgets::tab_button("Código-fonte", false, Message::MockAction),
        ]
        .spacing(theme::spacing::SM),
        container(
            column![
                text("CARF").size(18).style(theme::text_accent),
                text("Conselho Administrativo de Recursos Fiscais.").size(theme::typography::BODY),
                text("Visão Geral").size(theme::typography::TITLE),
                text("• Instância administrativa").size(theme::typography::BODY),
                text("• Julgamento de recursos fiscais").size(theme::typography::BODY),
            ]
            .spacing(theme::spacing::SM)
        )
        .padding(theme::spacing::MD)
        .width(Length::Fill)
        .style(theme::elevated)
    ]
    .spacing(theme::spacing::SM);

    container(column![tabs, preview].spacing(theme::spacing::SM))
        .height(Length::FillPortion(1))
        .padding(theme::spacing::MD)
        .style(theme::panel)
        .into()
}
