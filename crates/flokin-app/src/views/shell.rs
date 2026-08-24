use flokin_core::{EditorDialog, ShellModel, SqlCompletionItem};
use iced::widget::text_editor;
use iced::widget::{
    button, column, container, mouse_area, row, scrollable, stack, text, text_input,
};
use iced::{alignment, mouse, Alignment, Element, Length};

use crate::{
    message::{AppMode, MenuAction, MenuId, Message, SplitterKind},
    theme::{self, AppTheme},
    views,
    views::graph::GraphViewState,
    widgets,
};

#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    model: &'a ShellModel,
    app_theme: AppTheme,
    sql_editor: &'a text_editor::Content,
    markdown_editor: &'a text_editor::Content,
    sql_completion_items: &'a [SqlCompletionItem],
    graph_state: &'a GraphViewState,
    sql_completion_selected: usize,
    sql_completion_open: bool,
    left_width: f32,
    inspector_width: f32,
    schema_width: f32,
    sql_editor_height: f32,
    open_menu: Option<MenuId>,
    menu_anchor_x: f32,
    about_open: bool,
    left_visible: bool,
    right_visible: bool,
    mode: AppMode,
) -> Element<'a, Message> {
    let content = if mode == AppMode::Settings {
        row![
            activity_bar(mode),
            views::settings::view(app_theme, left_visible, right_visible)
        ]
        .height(Length::Fill)
    } else if mode == AppMode::Sql {
        let mut content = row![activity_bar(mode)].height(Length::Fill);
        if left_visible {
            content = content
                .push(views::explorer::sql_schema_view(model, schema_width))
                .push(splitter(SplitterKind::SqlSchema, false));
        }
        content = content.push(workspace(
            model,
            sql_editor,
            markdown_editor,
            sql_completion_items,
            sql_completion_selected,
            sql_completion_open,
            sql_editor_height,
        ));
        if right_visible {
            content = content
                .push(splitter(SplitterKind::Inspector, false))
                .push(views::inspector::view(model, inspector_width));
        }
        content
    } else if mode == AppMode::Graph {
        let mut content = row![activity_bar(mode)].height(Length::Fill);
        if left_visible {
            content = content
                .push(views::graph::sidebar(graph_state, left_width))
                .push(splitter(SplitterKind::LeftSidebar, false));
        }
        content = content.push(views::graph::view(
            graph_state,
            model.selected_document_path.as_ref(),
        ));
        if right_visible {
            content = content
                .push(splitter(SplitterKind::Inspector, false))
                .push(views::inspector::view(model, inspector_width));
        }
        content
    } else {
        let mut content = row![activity_bar(mode)].height(Length::Fill);
        if left_visible {
            if mode == AppMode::Data {
                content = content
                    .push(views::explorer::data_view(model, left_width))
                    .push(splitter(SplitterKind::LeftSidebar, false));
            } else {
                content = content
                    .push(views::explorer::view(model, app_theme, left_width))
                    .push(splitter(SplitterKind::LeftSidebar, false));
            }
        }
        content = content.push(workspace(
            model,
            sql_editor,
            markdown_editor,
            sql_completion_items,
            sql_completion_selected,
            sql_completion_open,
            sql_editor_height,
        ));
        if right_visible {
            content = content
                .push(splitter(SplitterKind::Inspector, false))
                .push(views::inspector::view(model, inspector_width));
        }
        content
    };

    let shell = column![
        menu_bar(),
        toolbar(model, app_theme, left_visible, right_visible),
        content,
        views::status_bar::view(model),
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    let shell = if model.search.open {
        stack![shell, search_backdrop(), search_overlay(model)]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        shell.into()
    };

    let shell = if let Some(menu) = open_menu {
        stack![shell, menu_overlay(menu, menu_anchor_x)].into()
    } else {
        shell
    };

    let shell = if about_open {
        stack![shell, about_overlay()].into()
    } else {
        shell
    };

    if let Some(dialog) = model.editor.dialog.as_ref() {
        stack![shell, editor_dialog_overlay(dialog, model)].into()
    } else {
        shell
    }
}

fn menu_bar<'a>() -> Element<'a, Message> {
    let items = [
        ("Arquivo", MenuId::File),
        ("Exibir", MenuId::View),
        ("Navegar", MenuId::Navigate),
        ("Dados", MenuId::Data),
        ("Ajuda", MenuId::Help),
    ];

    let mut row = row![text("FlokinMD")
        .size(theme::typography::TITLE)
        .style(theme::text_accent)]
    .spacing(theme::spacing::LG)
    .align_y(Alignment::Center);

    for (item, id) in items {
        row = row.push(
            mouse_area(
                button(text(item).size(theme::typography::MENU))
                    .padding([3, 6])
                    .style(theme::button_ghost)
                    .on_press(Message::MenuToggled(id)),
            )
            .on_move(move |point| Message::MenuTriggerMoved(id, point.x)),
        );
    }

    container(row)
        .height(theme::sizes::MENU_BAR_HEIGHT)
        .padding([0.0, theme::spacing::MD])
        .style(theme::panel)
        .into()
}

fn menu_overlay<'a>(menu: MenuId, anchor_x: f32) -> Element<'a, Message> {
    let menu_width = theme::sizes::MENU_WIDTH;
    stack![
        mouse_area(container("").width(Length::Fill).height(Length::Fill))
            .on_press(Message::MenuClosed),
        container(iced::widget::responsive(move |size| {
            let x = menu_left(anchor_x, size.width, menu_width);
            row![iced::widget::space().width(x), menu_items(menu)].into()
        }))
        .padding(iced::Padding {
            top: theme::sizes::MENU_TOP_OFFSET,
            ..iced::Padding::ZERO
        })
        .width(Length::Fill)
        .height(Length::Fill),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn menu_left(anchor_x: f32, window_width: f32, menu_width: f32) -> f32 {
    anchor_x.max(0.0).min((window_width - menu_width).max(0.0))
}

fn menu_items(menu: MenuId) -> Element<'static, Message> {
    let entries: Vec<(&str, Option<&str>, MenuAction)> = match menu {
        MenuId::File => vec![
            ("Abrir pasta", None, MenuAction::OpenFolder),
            ("Reindexar", None, MenuAction::Reindex),
        ],
        MenuId::View => vec![
            ("Alternar tema", None, MenuAction::ToggleTheme),
            (
                "Barra lateral esquerda",
                None,
                MenuAction::ToggleLeftSidebar,
            ),
            (
                "Barra lateral direita",
                None,
                MenuAction::ToggleRightSidebar,
            ),
        ],
        MenuId::Navigate => vec![
            ("Arquivos", None, MenuAction::Explorer),
            ("Dados", None, MenuAction::Data),
            ("Grafo", None, MenuAction::Graph),
            ("SQL Explorer", None, MenuAction::SqlExplorer),
            ("Configurações", None, MenuAction::Settings),
            ("Buscar", Some("Ctrl+K"), MenuAction::Search),
        ],
        MenuId::Data => vec![
            ("Abrir Dados", None, MenuAction::Data),
            ("Abrir Grafo", None, MenuAction::Graph),
            ("SQL Explorer", None, MenuAction::SqlExplorer),
            ("Executar query", Some("Ctrl+Enter"), MenuAction::ExecuteSql),
        ],
        MenuId::Help => vec![("Sobre FlokinMD", None, MenuAction::About)],
    };
    let mut items = column![].spacing(2);
    for (label, shortcut, action) in entries {
        let mut content = row![text(label)
            .size(theme::typography::BODY)
            .width(Length::Fill)];
        if let Some(shortcut) = shortcut {
            content = content.push(
                text(shortcut)
                    .font(theme::mono())
                    .size(theme::typography::LABEL)
                    .style(theme::text_muted),
            );
        }
        items = items.push(
            button(content)
                .width(theme::sizes::MENU_WIDTH - theme::spacing::SM)
                .padding([7.0, 10.0])
                .style(theme::button_menu)
                .on_press(Message::MenuAction(action)),
        );
    }
    container(items)
        .padding(4)
        .style(theme::overlay_panel)
        .into()
}

fn about_overlay<'a>() -> Element<'a, Message> {
    mouse_area(
        container(
            column![
                text("FlokinMD")
                    .size(theme::typography::TITLE)
                    .style(theme::text_accent),
                text("Markdown workspace com projeção SQL descartável.").style(theme::text_muted),
                button(text("Fechar"))
                    .style(theme::button_toolbar)
                    .on_press(Message::AboutClosed)
            ]
            .spacing(theme::spacing::SM),
        )
        .padding(theme::spacing::LG)
        .style(theme::overlay_panel),
    )
    .on_press(Message::AboutClosed)
    .into()
}

fn editor_dialog_overlay<'a>(
    dialog: &'a EditorDialog,
    model: &'a ShellModel,
) -> Element<'a, Message> {
    let (title, description, save_label, discard_label) = match dialog {
        EditorDialog::CloseDirty { document_path } => {
            let file_name = model
                .editor
                .tab(document_path)
                .map(|tab| tab.title.as_str())
                .unwrap_or("arquivo.md");
            (
                format!("Salvar alterações em {file_name}?"),
                String::from("Suas alterações ainda não foram gravadas no Markdown."),
                String::from("Salvar"),
                String::from("Não salvar"),
            )
        }
        EditorDialog::CloseWorkspaceDirty { dirty_count } => (
            format!("Existem {dirty_count} arquivos com alterações não salvas."),
            String::from("Escolha como lidar com as tabs sujas antes de continuar."),
            String::from("Salvar tudo"),
            String::from("Descartar alterações"),
        ),
    };

    stack![
        mouse_area(
            container("")
                .width(Length::Fill)
                .height(Length::Fill)
                .style(theme::overlay_backdrop)
        )
        .on_press(Message::EditorDialogCancel),
        container(
            container(
                column![
                    text(title)
                        .size(theme::typography::TITLE)
                        .style(theme::text_normal),
                    text(description)
                        .size(theme::typography::BODY)
                        .style(theme::text_muted),
                    row![
                        button(text("Cancelar"))
                            .padding([7.0, 12.0])
                            .style(theme::button_toolbar)
                            .on_press(Message::EditorDialogCancel),
                        button(text(discard_label))
                            .padding([7.0, 12.0])
                            .style(theme::button_toolbar)
                            .on_press(Message::EditorDialogDiscard),
                        button(text(save_label))
                            .padding([7.0, 12.0])
                            .style(theme::button_selected)
                            .on_press(Message::EditorDialogSave),
                    ]
                    .spacing(theme::spacing::SM)
                    .align_y(Alignment::Center)
                ]
                .spacing(theme::spacing::MD)
            )
            .width(theme::sizes::DIALOG_WIDTH)
            .padding(theme::spacing::LG)
            .style(theme::overlay_panel)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn splitter(kind: SplitterKind, vertical: bool) -> Element<'static, Message> {
    let size = Length::Fixed(theme::sizes::SPLITTER_HIT_AREA);
    let message_position = Message::SplitterPressed(kind, 0.0);
    mouse_area(
        container("")
            .width(if vertical { Length::Fill } else { size })
            .height(if vertical { size } else { Length::Fill })
            .style(theme::splitter),
    )
    .on_press(message_position)
    .interaction(if vertical {
        mouse::Interaction::ResizingVertically
    } else {
        mouse::Interaction::ResizingHorizontally
    })
    .into()
}

fn toolbar(
    model: &ShellModel,
    app_theme: AppTheme,
    left_visible: bool,
    right_visible: bool,
) -> Element<'_, Message> {
    let input = text_input("Buscar documentos...", model.search.query.as_str())
        .padding([4, 8])
        .size(theme::typography::BODY)
        .width(theme::sizes::TOOLBAR_SEARCH_WIDTH)
        .style(theme::input);

    let search = mouse_area(
        row![
            widgets::icon(theme::Icon::Search, theme::icons::TOOLBAR, false),
            input,
            text("Ctrl+K")
                .size(theme::typography::LABEL)
                .font(theme::mono())
                .style(theme::text_muted)
        ]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center),
    )
    .on_press(Message::SearchOpened);

    let left = row![
        iced::widget::tooltip(
            widgets::toolbar_button("Abrir pasta", theme::Icon::Folder, Message::OpenFolder),
            text("Abrir pasta"),
            iced::widget::tooltip::Position::Bottom
        ),
        iced::widget::tooltip(
            widgets::toolbar_button("Reindexar", theme::Icon::Refresh, Message::ReindexWorkspace),
            text("Reindexar workspace"),
            iced::widget::tooltip::Position::Bottom
        ),
        container("")
            .width(theme::sizes::DIVIDER_WIDTH)
            .height(theme::sizes::DIVIDER_HEIGHT)
            .style(theme::divider),
        search
    ]
    .spacing(theme::spacing::MD)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    let left_toggle = iced::widget::tooltip(
        button(widgets::icon(
            theme::Icon::PanelLeft,
            theme::icons::TOOLBAR,
            left_visible,
        ))
        .width(theme::sizes::TOOLBAR_BUTTON_WIDTH)
        .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT)
        .padding(0)
        .style(if left_visible {
            theme::button_selected
        } else {
            theme::button_toolbar
        })
        .on_press(Message::ToggleLeftSidebar),
        text(if left_visible {
            "Ocultar barra lateral esquerda"
        } else {
            "Mostrar barra lateral esquerda"
        }),
        iced::widget::tooltip::Position::Bottom,
    );
    let right_toggle = iced::widget::tooltip(
        button(widgets::icon(
            theme::Icon::Split,
            theme::icons::TOOLBAR,
            right_visible,
        ))
        .width(theme::sizes::TOOLBAR_BUTTON_WIDTH)
        .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT)
        .padding(0)
        .style(if right_visible {
            theme::button_selected
        } else {
            theme::button_toolbar
        })
        .on_press(Message::ToggleRightSidebar),
        text(if right_visible {
            "Ocultar barra lateral direita"
        } else {
            "Mostrar barra lateral direita"
        }),
        iced::widget::tooltip::Position::Bottom,
    );
    let right = row![
        left_toggle,
        right_toggle,
        iced::widget::tooltip(
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
            .on_press(Message::ThemeToggled),
            text("Alternar tema"),
            iced::widget::tooltip::Position::Bottom
        )
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    container(row![left, right].align_y(Alignment::Center))
        .height(theme::sizes::TOOLBAR_HEIGHT)
        .padding([0.0, theme::spacing::LG])
        .style(theme::elevated)
        .into()
}

fn search_backdrop<'a>() -> Element<'a, Message> {
    mouse_area(
        container("")
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::overlay_backdrop),
    )
    .on_press(Message::SearchClosed)
    .into()
}

fn search_overlay(model: &ShellModel) -> Element<'_, Message> {
    let query = model.search.query.trim();
    let body: Element<'_, Message> = if query.is_empty() {
        container(
            text("Digite para buscar documentos.")
                .size(theme::typography::BODY)
                .style(theme::text_muted),
        )
        .padding(theme::spacing::MD)
        .width(Length::Fill)
        .into()
    } else if model.search.results.is_empty() {
        container(
            text(format!("Nenhum documento encontrado para \"{query}\"."))
                .size(theme::typography::BODY)
                .style(theme::text_muted),
        )
        .padding(theme::spacing::MD)
        .width(Length::Fill)
        .into()
    } else {
        let mut rows = column![].spacing(0);
        for (index, result) in model.search.results.iter().enumerate() {
            rows = rows.push(search_result_row(
                result,
                model.search.selected_index == Some(index),
            ));
        }

        scrollable(rows)
            .height(theme::sizes::SEARCH_RESULTS_HEIGHT)
            .width(Length::Fill)
            .into()
    };

    let count = if model.search.is_limited() {
        format!("{}+ resultados", model.search.results.len())
    } else {
        format!("{} resultados", model.search.total_matches)
    };

    let overlay_input = text_input("Buscar documentos...", model.search.query.as_str())
        .id("search-overlay-input")
        .on_input(Message::SearchQueryChanged)
        .on_submit(Message::SearchActivated)
        .padding([8, 10])
        .size(theme::typography::BODY)
        .width(Length::Fill)
        .style(theme::input);

    let palette = mouse_area(
        container(
            column![
                row![
                    widgets::icon(theme::Icon::Search, theme::icons::TOOLBAR, false),
                    overlay_input,
                ]
                .spacing(theme::spacing::SM)
                .align_y(Alignment::Center),
                container("")
                    .height(1)
                    .width(Length::Fill)
                    .style(theme::divider),
                body,
                text(count)
                    .size(theme::typography::LABEL)
                    .style(theme::text_muted),
            ]
            .spacing(theme::spacing::SM),
        )
        .width(Length::Fill)
        .max_width(theme::sizes::SEARCH_OVERLAY_WIDTH)
        .max_height(theme::sizes::SEARCH_OVERLAY_HEIGHT)
        .padding(theme::spacing::MD)
        .style(theme::overlay_panel),
    )
    .on_press(Message::SearchOpened);

    container(palette)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([96.0, theme::spacing::LG])
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Top)
        .into()
}

fn search_result_row<'a>(
    result: &'a flokin_core::SearchResult,
    selected: bool,
) -> Element<'a, Message> {
    let style = if selected {
        theme::button_tree_selected
    } else {
        theme::button_tree
    };
    let row_style = if selected {
        theme::table_row_selected
    } else {
        theme::table_row
    };

    let mut content = column![
        text(result.title.as_str())
            .size(theme::typography::BODY)
            .style(if selected {
                theme::text_accent
            } else {
                theme::text_normal
            }),
        text(result.relative_path.display().to_string())
            .font(theme::mono())
            .size(theme::typography::LABEL)
            .style(theme::text_muted),
    ]
    .spacing(theme::spacing::XXS);

    if let Some(snippet) = result.snippet.as_deref() {
        content = content.push(
            text(snippet)
                .size(theme::typography::LABEL)
                .style(theme::text_muted),
        );
    }

    button(
        container(content)
            .padding([7.0, theme::spacing::MD])
            .style(row_style),
    )
    .width(Length::Fill)
    .padding(0)
    .style(style)
    .on_press(Message::SearchResultSelected(result.document_path.clone()))
    .into()
}

fn activity_bar(mode: AppMode) -> Element<'static, Message> {
    let entries = [
        (AppMode::Files, theme::Icon::Folder, "Arquivos"),
        (AppMode::Data, theme::Icon::Database, "Dados"),
        (AppMode::Graph, theme::Icon::Graph, "Grafo"),
        (AppMode::Sql, theme::Icon::Terminal, "SQL Explorer"),
    ];
    let mut top = column![]
        .spacing(theme::spacing::SM)
        .align_x(Alignment::Center);
    for (entry, icon, label) in entries {
        top = top.push(activity_button(entry, icon, label, mode == entry));
    }
    let bottom = activity_button(
        AppMode::Settings,
        theme::Icon::Settings,
        "Configurações",
        mode == AppMode::Settings,
    );

    container(column![
        top,
        iced::widget::Space::new().height(Length::Fill),
        bottom
    ])
    .width(theme::sizes::ACTIVITY_BAR_WIDTH)
    .height(Length::Fill)
    .padding([theme::spacing::LG, theme::spacing::SM])
    .style(theme::panel)
    .into()
}

fn activity_button(
    mode: AppMode,
    icon: theme::Icon,
    label: &'static str,
    selected: bool,
) -> Element<'static, Message> {
    let control = button(widgets::icon(icon, theme::icons::ACTIVITY, selected))
        .width(theme::sizes::ACTIVITY_BUTTON_SIZE)
        .height(theme::sizes::ACTIVITY_BUTTON_SIZE)
        .padding(0)
        .style(if selected {
            theme::button_selected
        } else {
            theme::button_activity
        })
        .on_press(Message::AppModeSelected(mode));
    iced::widget::tooltip(
        control,
        container(text(label).size(theme::typography::LABEL))
            .padding([4.0, 7.0])
            .style(theme::overlay_panel),
        iced::widget::tooltip::Position::Right,
    )
    .into()
}

fn workspace<'a>(
    model: &'a ShellModel,
    sql_editor: &'a text_editor::Content,
    markdown_editor: &'a text_editor::Content,
    sql_completion_items: &'a [SqlCompletionItem],
    sql_completion_selected: usize,
    sql_completion_open: bool,
    sql_editor_height: f32,
) -> Element<'a, Message> {
    column![
        views::editor::tabs(model),
        views::editor::view(
            model,
            sql_editor,
            markdown_editor,
            sql_completion_items,
            sql_completion_selected,
            sql_completion_open,
            sql_editor_height,
        )
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
