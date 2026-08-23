use flokin_core::ShellModel;
use iced::widget::{container, row, text};
use iced::{Alignment, Element, Length};

use crate::{message::Message, theme};

pub fn view(model: &ShellModel) -> Element<'_, Message> {
    let status_items = [
        format!("{} ({})", model.root_name, model.root_path),
        String::from("127 documentos"),
        String::from("✓ Indexado"),
        String::from("SQLite"),
        String::from("UTF-8"),
        String::from("Markdown"),
        String::from("Ln 1, Col 1"),
        String::from("main"),
    ];

    let mut row = row![]
        .spacing(theme::spacing::LG)
        .align_y(Alignment::Center);
    let mut is_first = true;

    for item in status_items {
        let style = match item.as_str() {
            "✓ Indexado" => theme::text_success,
            "SQLite" => theme::text_warning,
            "main" => theme::text_accent,
            _ => theme::text_muted,
        };

        if is_first {
            is_first = false;
        } else {
            row = row.push(
                text("│")
                    .size(theme::typography::LABEL)
                    .style(theme::text_muted),
            );
        }

        row = row.push(
            text(item)
                .size(theme::typography::LABEL)
                .font(theme::mono())
                .style(style),
        );
    }

    container(row)
        .height(26)
        .width(Length::Fill)
        .padding([0.0, theme::spacing::MD])
        .style(theme::elevated)
        .into()
}
