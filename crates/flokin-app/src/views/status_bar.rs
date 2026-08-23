use flokin_core::{ScanState, ShellModel};
use iced::widget::{container, row, text};
use iced::{Alignment, Element, Length};

use crate::{message::Message, theme};

pub fn view(model: &ShellModel) -> Element<'_, Message> {
    let workspace = model.workspace_display();
    let mut status_items = vec![if workspace.is_open {
        workspace.name
    } else {
        String::from("Nenhuma pasta aberta")
    }];

    match &model.scan_state {
        ScanState::Idle => {}
        ScanState::Scanning => status_items.push(String::from("Analisando documentos...")),
        ScanState::Updating {
            documents,
            collections,
            warnings,
            ..
        } => {
            status_items.push(String::from("Atualizando..."));
            status_items.push(format!("{documents} documentos"));
            status_items.push(format!("{collections} collections"));
            if *warnings > 0 {
                status_items.push(format!("{warnings} warnings"));
            }
        }
        ScanState::Completed {
            documents,
            collections,
            warnings,
            ..
        } => {
            status_items.push(format!("{documents} documentos"));
            status_items.push(format!("{collections} collections"));
            if *warnings > 0 {
                status_items.push(format!("{warnings} warnings"));
            }
        }
        ScanState::Failed(_) => status_items.push(String::from("Falha ao analisar workspace")),
    }

    if workspace.is_open && !matches!(model.scan_state, ScanState::Scanning | ScanState::Failed(_))
    {
        status_items.push(String::from("Workspace monitorado"));
    }
    status_items.push(String::from("Markdown"));

    let mut row = row![]
        .spacing(theme::spacing::LG)
        .align_y(Alignment::Center);
    let mut is_first = true;

    for item in status_items {
        let style = if item == "Analisando documentos..."
            || item == "Atualizando..."
            || item == "Falha ao analisar workspace"
            || item.ends_with("warnings")
        {
            theme::text_warning
        } else {
            theme::text_muted
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
