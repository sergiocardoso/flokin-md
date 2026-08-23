use flokin_core::ShellModel;
use iced::widget::{container, row, text};
use iced::{Alignment, Element, Length};

use crate::{message::Message, theme};

pub fn view(model: &ShellModel) -> Element<'_, Message> {
    let workspace = model.workspace_display();
    let status_items = [
        if workspace.is_open {
            format!("{} ({})", workspace.name, workspace.path)
        } else {
            String::from("Nenhuma pasta aberta")
        },
        String::from("Não indexado"),
        String::from("Markdown"),
        String::from("Ln 1, Col 1"),
    ];

    let mut row = row![]
        .spacing(theme::spacing::LG)
        .align_y(Alignment::Center);
    let mut is_first = true;

    for item in status_items {
        let style = match item.as_str() {
            "Não indexado" => theme::text_warning,
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
