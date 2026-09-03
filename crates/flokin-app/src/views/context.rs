use flokin_core::{
    build_context_projection, ContextArtifact, ContextProjection, ContextSection, PropertyValue,
    RelationStatus, SemanticKind, ShellModel,
};
use iced::widget::{
    button, column, container, row, scrollable,
    scrollable::{Direction, Scrollbar},
    text,
    text::{LineHeight, Wrapping},
};
use iced::{alignment, Alignment, Element, Length};

use crate::{i18n::I18nCatalog, message::Message, theme, widgets};

pub fn sidebar<'a>(
    model: &'a ShellModel,
    width: f32,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let projection = projection(model);
    let mut content = column![widgets::section_title(i18n.tr("context-sidebar-title"))]
        .spacing(theme::spacing::XS);

    for section in ContextSection::ALL {
        let count = if section == ContextSection::Overview {
            projection.artifacts.len()
        } else {
            projection.count_for_section(section)
        };
        content = content.push(section_row(
            section,
            section_label(section, i18n),
            count,
            model.context_section == section,
        ));
    }

    container(scrollable(content).direction(Direction::Vertical(
        Scrollbar::default().width(4).scroller_width(4).spacing(8),
    )))
    .width(width)
    .height(Length::Fill)
    .padding(theme::spacing::MD)
    .style(theme::panel)
    .into()
}

pub fn view<'a>(model: &'a ShellModel, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    let projection = projection(model);
    let title = if model.context_section == ContextSection::Overview {
        i18n.tr("context-title")
    } else {
        section_label(model.context_section, i18n)
    };
    let summary = i18n.tr_with(
        "context-artifact-count",
        &[(
            "count",
            projection
                .artifacts_for_section(model.context_section)
                .count()
                .into(),
        )],
    );

    let header = row![
        container(widgets::icon(theme::Icon::Split, theme::icons::META, true))
            .width(theme::sizes::ICON_SLOT_MEDIUM)
            .height(theme::sizes::ICON_SLOT_MEDIUM)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center),
        column![
            text(title)
                .size(theme::typography::TITLE)
                .style(theme::text_accent),
            text(summary)
                .size(theme::typography::BODY)
                .style(theme::text_muted),
        ]
        .spacing(theme::spacing::XXS)
        .width(Length::Fill),
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    let body = if projection
        .artifacts_for_section(model.context_section)
        .count()
        == 0
    {
        empty_state(model.context_section, i18n)
    } else if model.context_section == ContextSection::Overview {
        overview(model, projection, i18n)
    } else {
        artifacts_table(
            model,
            projection
                .artifacts_for_section(model.context_section)
                .cloned()
                .collect(),
            i18n,
        )
    };

    container(column![header, body].spacing(theme::spacing::LG))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(theme::spacing::XXL)
        .style(theme::editor)
        .into()
}

pub fn inspector<'a>(
    model: &'a ShellModel,
    width: f32,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let projection = projection(model);
    let Some(path) = model.selected_context_artifact.as_ref() else {
        return inspector_empty(width, i18n);
    };
    let Some(artifact) = projection.artifact_for_path(path).cloned() else {
        return inspector_empty(width, i18n);
    };

    let incoming = model.relation_index.incoming(&artifact.document_path);
    let outgoing = model.relation_index.outgoing(&artifact.document_path);

    let mut content = column![
        section_header(
            i18n.tr("context-inspector-title"),
            semantic_icon(artifact.semantic_kind)
        ),
        text(artifact.title.clone())
            .size(theme::typography::TITLE)
            .style(theme::text_normal)
            .wrapping(Wrapping::WordOrGlyph),
        field(
            i18n.tr("context-kind"),
            kind_label(artifact.semantic_kind, i18n),
            false
        ),
        field(
            i18n.tr("context-path"),
            artifact.relative_path.display().to_string(),
            true,
        ),
        actions(artifact.document_path.clone(), i18n),
    ]
    .spacing(theme::spacing::MD);

    if !incoming.is_empty() || !outgoing.is_empty() {
        content = content.push(divider()).push(section_header(
            i18n.tr("context-relations"),
            theme::Icon::Graph,
        ));
    }

    if !incoming.is_empty() {
        content = content.push(widgets::section_title(i18n.tr("context-referenced-by")));
        for relation in incoming {
            content = content.push(relation_row(
                relation.source_title.clone(),
                relation.source_relative_path.display().to_string(),
                relation.source_document.clone(),
            ));
        }
    }

    if !outgoing.is_empty() {
        content = content.push(widgets::section_title(i18n.tr("context-references")));
        for relation in outgoing {
            let (title, path, select_path) = match &relation.status {
                RelationStatus::Resolved(target) => (
                    target.title.clone(),
                    target.relative_path.display().to_string(),
                    target.path.clone(),
                ),
                RelationStatus::Unresolved => (
                    relation.target.display.clone(),
                    i18n.tr("context-unresolved"),
                    artifact.document_path.clone(),
                ),
                RelationStatus::Ambiguous(_) => (
                    relation.target.display.clone(),
                    i18n.tr("context-ambiguous"),
                    artifact.document_path.clone(),
                ),
            };
            content = content.push(relation_row(title, path, select_path));
        }
    }

    if !artifact.properties.is_empty() {
        content = content.push(divider()).push(section_header(
            i18n.tr("context-metadata"),
            theme::Icon::FileText,
        ));
        for (key, value) in artifact.properties.iter().take(12) {
            content = content.push(field(key.clone(), property_value_label(value), true));
        }
    }

    container(scrollable(content).direction(Direction::Vertical(
        Scrollbar::default().width(4).scroller_width(4).spacing(8),
    )))
    .width(width)
    .height(Length::Fill)
    .padding(theme::spacing::XL)
    .style(theme::inspector_panel)
    .into()
}

fn projection(model: &ShellModel) -> ContextProjection {
    build_context_projection(&model.documents, &model.relation_index)
}

fn overview<'a>(
    model: &'a ShellModel,
    projection: ContextProjection,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let counter_items = ContextSection::ALL
        .into_iter()
        .filter(|section| *section != ContextSection::Overview)
        .map(|section| (section, section_label(section, i18n)))
        .collect::<Vec<_>>();
    let mut counter_rows = column![].spacing(theme::spacing::SM);
    for (row_index, chunk) in counter_items.chunks(3).enumerate() {
        let mut counter_row = row![].spacing(theme::spacing::SM).width(Length::Fill);
        for (column_index, (section, label)) in chunk.iter().enumerate() {
            counter_row = counter_row.push(summary_button(
                *section,
                label.clone(),
                projection.count_for_section(*section),
                row_index * 3 + column_index,
            ));
        }
        counter_rows = counter_rows.push(counter_row);
    }

    let unconnected = i18n.tr_with(
        "context-unconnected-count",
        &[("count", projection.unconnected_count().into())],
    );
    let artifact_count = i18n.tr_with(
        "context-artifact-count",
        &[("count", projection.artifacts.len().into())],
    );

    column![
        container(counter_rows)
            .width(Length::Fill)
            .padding(theme::spacing::XS)
            .style(theme::surface),
        container(
            row![
                widgets::icon(theme::Icon::Graph, theme::icons::META, false),
                text(i18n.tr("context-unconnected"))
                    .size(theme::typography::BODY)
                    .style(theme::text_muted)
                    .width(Length::Fill),
                text(unconnected)
                    .size(theme::typography::BODY)
                    .font(theme::mono())
                    .style(theme::text_normal),
            ]
            .spacing(theme::spacing::SM)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([theme::spacing::XS, theme::spacing::SM])
        .style(theme::elevated),
        row![
            widgets::section_title(i18n.tr("context-artifacts")),
            iced::widget::Space::new().width(Length::Fill),
            text(artifact_count)
                .size(theme::typography::LABEL)
                .font(theme::mono())
                .style(theme::text_muted),
        ]
        .align_y(Alignment::Center),
        artifacts_table(model, projection.artifacts, i18n),
    ]
    .spacing(theme::spacing::SM)
    .into()
}

fn artifacts_table<'a>(
    model: &'a ShellModel,
    artifacts: Vec<ContextArtifact>,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let mut rows = column![table_header(i18n)].spacing(0);
    for (row_index, artifact) in artifacts.into_iter().enumerate() {
        let selected = model.selected_context_artifact.as_ref() == Some(&artifact.document_path);
        rows = rows.push(artifact_row(artifact, row_index, selected, i18n));
    }

    container(scrollable(rows).direction(Direction::Vertical(
        Scrollbar::default().width(4).scroller_width(4).spacing(8),
    )))
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::surface)
    .padding(theme::spacing::SM)
    .into()
}

fn table_header<'a>(i18n: &'a I18nCatalog) -> Element<'a, Message> {
    container(
        row![
            header_cell(i18n.tr("context-name"), Length::FillPortion(3)),
            header_cell(i18n.tr("context-kind"), Length::FillPortion(2)),
            header_cell(i18n.tr("context-path"), Length::FillPortion(4)),
            header_cell(i18n.tr("context-relations"), Length::Fixed(96.0)),
        ]
        .spacing(theme::spacing::SM)
        .padding([0.0, theme::spacing::SM])
        .align_y(Alignment::Center),
    )
    .height(theme::sizes::DATA_GRID_HEADER_HEIGHT)
    .style(theme::data_header)
    .into()
}

fn artifact_row<'a>(
    artifact: ContextArtifact,
    row_index: usize,
    selected: bool,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let row = row![
        row![
            widgets::icon(
                semantic_icon(artifact.semantic_kind),
                theme::icons::TREE,
                selected
            ),
            text(artifact.title.clone())
                .size(theme::typography::BODY)
                .width(Length::Fill)
                .wrapping(Wrapping::WordOrGlyph)
                .style(if selected {
                    theme::text_accent
                } else {
                    theme::text_normal
                }),
        ]
        .spacing(theme::spacing::XS)
        .align_y(Alignment::Center)
        .width(Length::FillPortion(3)),
        text(kind_label(artifact.semantic_kind, i18n))
            .size(theme::typography::BODY)
            .width(Length::FillPortion(2))
            .style(theme::text_muted),
        text(artifact.relative_path.display().to_string())
            .font(theme::mono())
            .size(theme::typography::LABEL)
            .width(Length::FillPortion(4))
            .wrapping(Wrapping::WordOrGlyph)
            .style(theme::text_muted),
        container(
            text(artifact.relations_count().to_string())
                .font(theme::mono())
                .size(theme::typography::BODY)
                .style(theme::text_normal),
        )
        .width(96)
        .align_x(alignment::Horizontal::Right),
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    button(
        container(row)
            .width(Length::Fill)
            .padding([8.0, theme::spacing::SM]),
    )
    .width(Length::Fill)
    .padding(0)
    .style(move |theme, status| theme::data_row_button(theme, row_index, selected, status))
    .on_press(Message::ContextArtifactSelected(
        artifact.document_path.clone(),
    ))
    .into()
}

fn section_row<'a>(
    section: ContextSection,
    label: String,
    count: usize,
    selected: bool,
) -> Element<'a, Message> {
    let count_label = if section == ContextSection::Overview {
        String::new()
    } else {
        count.to_string()
    };
    let row = row![
        widgets::icon(section_icon(section), theme::icons::TREE, selected),
        text(label)
            .size(theme::typography::BODY)
            .width(Length::Fill)
            .style(if selected {
                theme::text_accent
            } else {
                theme::text_normal
            }),
        text(count_label)
            .font(theme::mono())
            .size(theme::typography::LABEL)
            .style(theme::text_muted),
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    button(
        container(row)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .height(theme::sizes::CONTROL_HEIGHT_MEDIUM)
    .padding([0.0, theme::spacing::XS])
    .style(if selected {
        theme::button_tree_selected
    } else {
        theme::button_tree
    })
    .on_press(Message::ContextSectionSelected(section))
    .into()
}

fn summary_button<'a>(
    section: ContextSection,
    label: String,
    count: usize,
    metric: usize,
) -> Element<'a, Message> {
    button(
        container(
            column![
                text(label)
                    .size(theme::typography::LABEL)
                    .style(theme::text_muted),
                text(count.to_string())
                    .font(theme::mono())
                    .size(theme::typography::TITLE)
                    .style(theme::text_normal),
            ]
            .spacing(theme::spacing::XXS)
            .width(Length::Fill)
            .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(alignment::Vertical::Center)
        .padding(theme::spacing::SM),
    )
    .width(Length::FillPortion(1))
    .height(80.0)
    .padding(0)
    .style(move |theme, status| theme::context_metric(theme, metric, status))
    .on_press(Message::ContextSectionSelected(section))
    .into()
}

fn actions<'a>(document_path: std::path::PathBuf, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    row![
        button(widgets::icon_text(
            theme::Icon::FileText,
            i18n.tr("context-open-editor"),
            theme::icons::META,
            false,
        ))
        .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT)
        .padding([0.0, theme::spacing::SM])
        .style(theme::button_toolbar)
        .on_press(Message::ContextOpenInEditor(document_path.clone())),
        button(widgets::icon_text(
            theme::Icon::Graph,
            i18n.tr("context-show-graph"),
            theme::icons::META,
            false,
        ))
        .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT)
        .padding([0.0, theme::spacing::SM])
        .style(theme::button_toolbar)
        .on_press(Message::ContextShowInGraph(document_path)),
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center)
    .into()
}

fn relation_row<'a>(
    title: String,
    path: String,
    select_path: std::path::PathBuf,
) -> Element<'a, Message> {
    button(
        container(
            column![
                text(title)
                    .size(theme::typography::BODY)
                    .style(theme::text_normal)
                    .wrapping(Wrapping::WordOrGlyph),
                text(path)
                    .font(theme::mono())
                    .size(theme::typography::LABEL)
                    .style(theme::text_muted)
                    .wrapping(Wrapping::WordOrGlyph),
            ]
            .spacing(theme::spacing::XXS),
        )
        .width(Length::Fill)
        .padding([4.0, 0.0]),
    )
    .width(Length::Fill)
    .padding(0)
    .style(theme::button_tree)
    .on_press(Message::ContextArtifactSelected(select_path))
    .into()
}

fn inspector_empty<'a>(width: f32, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    container(scrollable(
        column![
            section_header(i18n.tr("context-inspector-title"), theme::Icon::Settings),
            text(i18n.tr("context-select-artifact"))
                .size(theme::typography::BODY)
                .style(theme::text_muted)
                .wrapping(Wrapping::WordOrGlyph),
        ]
        .spacing(theme::spacing::MD),
    ))
    .width(width)
    .height(Length::Fill)
    .padding(theme::spacing::XL)
    .style(theme::inspector_panel)
    .into()
}

fn empty_state<'a>(section: ContextSection, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    let key = match section {
        ContextSection::Overview => "context-empty",
        ContextSection::Agents => "context-no-agents",
        ContextSection::Skills => "context-no-skills",
        ContextSection::Specs => "context-no-specs",
        ContextSection::Ice => "context-no-ice",
        ContextSection::Contexts => "context-no-contexts",
        ContextSection::Prompts => "context-no-prompts",
        ContextSection::Rules => "context-no-rules",
        ContextSection::Memory => "context-no-memory",
        ContextSection::Mcp => "context-no-mcp",
    };
    container(
        column![
            widgets::icon(section_icon(section), 28.0, false),
            text(i18n.tr(key))
                .size(theme::typography::BODY)
                .style(theme::text_muted)
                .wrapping(Wrapping::WordOrGlyph),
        ]
        .spacing(theme::spacing::SM)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(alignment::Horizontal::Center)
    .align_y(alignment::Vertical::Center)
    .into()
}

fn header_cell<'a>(label: String, width: Length) -> Element<'a, Message> {
    text(label)
        .size(theme::typography::LABEL)
        .width(width)
        .style(theme::text_muted)
        .into()
}

fn field<'a>(label: String, value: String, mono: bool) -> Element<'a, Message> {
    column![
        text(label)
            .size(theme::typography::LABEL)
            .style(theme::text_muted),
        text(value)
            .font(if mono {
                theme::mono()
            } else {
                theme::typography::UI
            })
            .size(theme::typography::BODY)
            .line_height(LineHeight::Relative(1.25))
            .wrapping(Wrapping::WordOrGlyph)
            .style(theme::text_normal),
    ]
    .spacing(theme::spacing::XXS)
    .into()
}

fn section_header<'a>(label: String, icon: theme::Icon) -> Element<'a, Message> {
    row![
        widgets::icon(icon, theme::icons::META, true),
        text(label)
            .size(theme::typography::LABEL)
            .style(theme::text_muted),
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center)
    .into()
}

fn divider<'a>() -> Element<'a, Message> {
    container("")
        .height(1)
        .width(Length::Fill)
        .style(theme::divider)
        .into()
}

fn property_value_label(value: &PropertyValue) -> String {
    match value {
        PropertyValue::Null => String::from("null"),
        PropertyValue::Bool(value) => value.to_string(),
        PropertyValue::Number(value) | PropertyValue::String(value) => value.clone(),
        PropertyValue::Array(values) => format!("[{}]", values.len()),
        PropertyValue::Object(values) => format!("{{{}}}", values.len()),
    }
}

fn section_label(section: ContextSection, i18n: &I18nCatalog) -> String {
    i18n.tr(match section {
        ContextSection::Overview => "context-overview",
        ContextSection::Agents => "context-agents",
        ContextSection::Skills => "context-skills",
        ContextSection::Specs => "context-specs",
        ContextSection::Ice => "context-ice",
        ContextSection::Contexts => "context-contexts",
        ContextSection::Prompts => "context-prompts",
        ContextSection::Rules => "context-rules",
        ContextSection::Memory => "context-memory",
        ContextSection::Mcp => "context-mcp",
    })
}

fn kind_label(kind: SemanticKind, i18n: &I18nCatalog) -> String {
    i18n.tr(match kind {
        SemanticKind::Agent => "semantic-agent",
        SemanticKind::AgentInstructions => "semantic-agent-instructions",
        SemanticKind::Skill => "semantic-skill",
        SemanticKind::Spec => "semantic-spec",
        SemanticKind::Ice => "semantic-ice",
        SemanticKind::Context => "semantic-context",
        SemanticKind::Prompt => "semantic-prompt",
        SemanticKind::Rules => "semantic-rules",
        SemanticKind::Memory => "semantic-memory",
        SemanticKind::Mcp => "semantic-mcp",
    })
}

fn section_icon(section: ContextSection) -> theme::Icon {
    match section {
        ContextSection::Overview => theme::Icon::Frame,
        ContextSection::Agents => theme::Icon::Agent,
        ContextSection::Skills => theme::Icon::Puzzle,
        ContextSection::Specs | ContextSection::Ice | ContextSection::Rules => {
            theme::Icon::ScrollCheck
        }
        ContextSection::Contexts => theme::Icon::Split,
        ContextSection::Prompts => theme::Icon::Prompt,
        ContextSection::Memory => theme::Icon::Database,
        ContextSection::Mcp => theme::Icon::Terminal,
    }
}

fn semantic_icon(kind: SemanticKind) -> theme::Icon {
    match kind {
        SemanticKind::Agent | SemanticKind::AgentInstructions => theme::Icon::Agent,
        SemanticKind::Skill => theme::Icon::Puzzle,
        SemanticKind::Spec | SemanticKind::Ice | SemanticKind::Rules => theme::Icon::ScrollCheck,
        SemanticKind::Context => theme::Icon::Split,
        SemanticKind::Prompt => theme::Icon::Prompt,
        SemanticKind::Memory => theme::Icon::Database,
        SemanticKind::Mcp => theme::Icon::Terminal,
    }
}
