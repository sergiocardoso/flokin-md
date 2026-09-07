use flokin_core::{ScanState, ShellModel};
use iced::widget::{container, row, text};
use iced::{Alignment, Element, Length};

use crate::{i18n::I18nCatalog, message::Message, theme};

pub fn view<'a>(model: &'a ShellModel, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    let workspace = model.workspace_display();
    let mut status_items = vec![if workspace.is_open {
        workspace.name
    } else {
        i18n.tr("status-no-workspace")
    }];

    match &model.scan_state {
        ScanState::Idle => {}
        ScanState::Scanning => status_items.push(i18n.tr("status-scanning")),
        ScanState::Updating {
            documents,
            collections,
            warnings,
            ..
        } => {
            status_items.push(i18n.tr("status-updating"));
            status_items.push(i18n.tr_with("status-documents", &[("count", (*documents).into())]));
            status_items
                .push(i18n.tr_with("status-collections", &[("count", (*collections).into())]));
            if *warnings > 0 {
                status_items
                    .push(i18n.tr_with("status-warnings", &[("count", (*warnings).into())]));
            }
        }
        ScanState::Completed {
            documents,
            collections,
            warnings,
            ..
        } => {
            status_items.push(i18n.tr_with("status-documents", &[("count", (*documents).into())]));
            status_items
                .push(i18n.tr_with("status-collections", &[("count", (*collections).into())]));
            if *warnings > 0 {
                status_items
                    .push(i18n.tr_with("status-warnings", &[("count", (*warnings).into())]));
            }
        }
        ScanState::Failed(_) => status_items.push(i18n.tr("status-scan-failed")),
    }

    if workspace.is_open && !matches!(model.scan_state, ScanState::Scanning | ScanState::Failed(_))
    {
        status_items.push(i18n.tr("status-workspace-watched"));
    }
    if model.sql_explorer.open {
        status_items.push(i18n.tr("status-sqlite-memory"));
        status_items.push(i18n.tr("status-read-only"));
    }
    status_items.push(i18n.tr("status-markdown"));

    let mut row = row![container("").width(6).height(6).style(theme::status_dot)]
        .spacing(theme::spacing::MD)
        .align_y(Alignment::Center);
    let mut is_first = true;

    for item in status_items {
        let style = if item == i18n.tr("status-scanning")
            || item == i18n.tr("status-updating")
            || item == i18n.tr("status-scan-failed")
        {
            theme::text_warning
        } else {
            theme::text_muted
        };

        if is_first {
            is_first = false;
        } else {
            row = row.push(
                text("·")
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
        .style(theme::status_bar)
        .into()
}
