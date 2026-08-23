use flokin_core::{BottomTab, Document, PropertyValue, ShellModel, WorkspaceTab};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::{message::Message, theme, widgets};

pub fn tabs(model: &ShellModel) -> Element<'_, Message> {
    if let Some(collection) = model.selected_collection() {
        return container(row![widgets::tab_button(
            collection.display_name.as_str(),
            true,
            Message::MockAction,
        )])
        .height(38)
        .padding([0.0, theme::spacing::SM])
        .style(theme::surface)
        .into();
    }

    if let Some(document) = model.selected_document() {
        return container(row![widgets::tab_button(
            document.title.as_str(),
            true,
            Message::MockAction,
        )])
        .height(38)
        .padding([0.0, theme::spacing::SM])
        .style(theme::surface)
        .into();
    }

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
    if let Some(collection) = model.selected_collection() {
        return collection_view(model, collection.id.as_str());
    }

    if let Some(document) = model.selected_document() {
        return document_selection_view(document);
    }

    column![breadcrumb(), editor_area(model), bottom_panel(model)]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn collection_view<'a>(model: &'a ShellModel, collection_id: &'a str) -> Element<'a, Message> {
    let Some(collection) = model.selected_collection() else {
        return container("").into();
    };
    let documents = model.collection_documents(collection_id);
    let mut list = column![
        text(collection.display_name.as_str())
            .size(22)
            .style(theme::text_accent),
        text(format!("{} documentos", collection.document_count))
            .size(theme::typography::BODY)
            .style(theme::text_muted),
    ]
    .spacing(theme::spacing::MD);

    for document in documents {
        list = list.push(document_row(document));
    }

    container(scrollable(list).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(theme::spacing::XXL)
        .style(theme::editor)
        .into()
}

fn document_row(document: &Document) -> Element<'_, Message> {
    let mut meta = row![text(document.relative_path.display().to_string())
        .font(theme::mono())
        .size(theme::typography::LABEL)
        .style(theme::text_muted)]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    for (key, value) in simple_properties(document).into_iter().take(3) {
        meta = meta.push(property_chip(format!("{key}: {}", property_preview(value))));
    }

    button(
        column![
            text(document.title.as_str())
                .size(theme::typography::TITLE)
                .style(theme::text_normal),
            meta,
        ]
        .spacing(theme::spacing::XS),
    )
    .width(Length::Fill)
    .padding(theme::spacing::MD)
    .style(theme::button_tree)
    .on_press(Message::MarkdownSelected(document.path.clone()))
    .into()
}

fn document_selection_view(document: &Document) -> Element<'_, Message> {
    container(
        column![
            text(document.title.as_str())
                .size(22)
                .style(theme::text_accent),
            text(document.relative_path.display().to_string())
                .font(theme::mono())
                .size(theme::typography::BODY)
                .style(theme::text_muted),
            text("Conteúdo real será aberto em milestone futura.")
                .size(theme::typography::BODY)
                .style(theme::text_muted),
        ]
        .spacing(theme::spacing::MD),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(theme::spacing::XXL)
    .style(theme::editor)
    .into()
}

fn simple_properties(document: &Document) -> Vec<(&str, &PropertyValue)> {
    document
        .properties
        .iter()
        .filter(|(key, _)| key.as_str() != "title" && key.as_str() != "type")
        .map(|(key, value)| (key.as_str(), value))
        .collect()
}

fn property_preview(value: &PropertyValue) -> String {
    match value {
        PropertyValue::Null => String::from("null"),
        PropertyValue::Bool(value) => value.to_string(),
        PropertyValue::Number(value) | PropertyValue::String(value) => value.clone(),
        PropertyValue::Array(values) => format!("{} itens", values.len()),
        PropertyValue::Object(values) => format!("{} campos", values.len()),
    }
}

fn property_chip(label: String) -> Element<'static, Message> {
    container(
        text(label)
            .size(theme::typography::LABEL)
            .style(theme::text_accent),
    )
    .padding([3.0, 8.0])
    .style(theme::chip)
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
