use flokin_core::{EditorDialog, ExplicitSchemaState, SchemaType, ShellModel, SqlCompletionItem};
use iced::widget::{
    button, column, container, mouse_area, row, scrollable, stack, text, text_input,
};
use iced::widget::{markdown, text_editor};
use iced::{alignment, mouse, Alignment, Element, Length};

use crate::{
    brand,
    i18n::{AppLanguage, I18nCatalog},
    message::{AppMode, MenuAction, MenuId, Message, SplitterKind},
    theme::{self, AppTheme},
    views,
    views::graph::GraphViewState,
    widgets,
};

const LOGO_TOP_OFFSET: f32 = 5.0;
const MENU_TOP_OFFSET: f32 = 20.0;
const SEARCH_TOP_OFFSET: f32 = 2.0;
const SEARCH_LEFT_OFFSET: f32 = 30.0;

#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    model: &'a ShellModel,
    app_theme: AppTheme,
    sql_editor: &'a text_editor::Content,
    markdown_editor: &'a text_editor::Content,
    markdown_preview: &'a [markdown::Item],
    sql_completion_items: &'a [SqlCompletionItem],
    graph_state: &'a GraphViewState,
    sql_completion_selected: usize,
    sql_completion_open: bool,
    left_width: f32,
    inspector_width: f32,
    schema_width: f32,
    sql_editor_height: f32,
    open_menu: Option<MenuId>,
    about_open: bool,
    schema_create_dialog_open: bool,
    schema_create_error: Option<&'a str>,
    left_visible: bool,
    right_visible: bool,
    mode: AppMode,
    i18n: &'a I18nCatalog,
    language: AppLanguage,
) -> Element<'a, Message> {
    let content = if mode == AppMode::Settings {
        row![
            activity_bar(mode, i18n),
            panel_gutter(),
            views::settings::view(app_theme, language, i18n, left_visible, right_visible)
        ]
        .height(Length::Fill)
    } else if mode == AppMode::Sql {
        let mut content = row![activity_bar(mode, i18n), panel_gutter()].height(Length::Fill);
        if left_visible {
            content = content
                .push(views::explorer::sql_schema_view(model, schema_width, i18n))
                .push(splitter(SplitterKind::SqlSchema, false));
        }
        content = content.push(workspace(
            model,
            app_theme,
            sql_editor,
            markdown_editor,
            markdown_preview,
            sql_completion_items,
            sql_completion_selected,
            sql_completion_open,
            sql_editor_height,
            i18n,
        ));
        if right_visible {
            content = content
                .push(splitter(SplitterKind::Inspector, false))
                .push(views::inspector::view(model, inspector_width));
        }
        content
    } else if mode == AppMode::Graph {
        let mut content = row![activity_bar(mode, i18n), panel_gutter()].height(Length::Fill);
        if left_visible {
            content = content
                .push(views::graph::sidebar(graph_state, left_width, i18n))
                .push(splitter(SplitterKind::LeftSidebar, false));
        }
        content = content.push(views::graph::view(
            graph_state,
            model.selected_document_path.as_ref(),
            i18n,
        ));
        if right_visible {
            content = content
                .push(splitter(SplitterKind::Inspector, false))
                .push(views::inspector::view(model, inspector_width));
        }
        content
    } else if mode == AppMode::Health {
        let mut content = row![activity_bar(mode, i18n), panel_gutter()].height(Length::Fill);
        content = content.push(views::health::view(model, i18n));
        if right_visible {
            content = content
                .push(splitter(SplitterKind::Inspector, false))
                .push(views::inspector::view(model, inspector_width));
        }
        content
    } else if mode == AppMode::History {
        row![
            activity_bar(mode, i18n),
            panel_gutter(),
            views::history::view(model, i18n)
        ]
        .height(Length::Fill)
    } else {
        let mut content = row![activity_bar(mode, i18n), panel_gutter()].height(Length::Fill);
        if left_visible {
            if mode == AppMode::Data {
                content = content
                    .push(views::explorer::data_view(model, left_width, i18n))
                    .push(splitter(SplitterKind::LeftSidebar, false));
            } else {
                content = content
                    .push(views::explorer::view(model, app_theme, left_width, i18n))
                    .push(splitter(SplitterKind::LeftSidebar, false));
            }
        }
        content = content.push(workspace(
            model,
            app_theme,
            sql_editor,
            markdown_editor,
            markdown_preview,
            sql_completion_items,
            sql_completion_selected,
            sql_completion_open,
            sql_editor_height,
            i18n,
        ));
        if right_visible {
            content = content
                .push(splitter(SplitterKind::Inspector, false))
                .push(views::inspector::view(model, inspector_width));
        }
        content
    };

    let shell = column![
        top_shell(
            model,
            app_theme,
            left_visible,
            right_visible,
            open_menu,
            i18n
        ),
        content_frame(content),
        views::status_bar::view(model, i18n),
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    let shell = if model.search.open {
        stack![shell, search_backdrop(), search_overlay(model, i18n)]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        shell.into()
    };

    let shell = if let Some(menu) = open_menu {
        stack![shell, menu_overlay(menu, i18n)].into()
    } else {
        shell
    };

    let shell = if about_open {
        stack![shell, about_overlay(i18n)].into()
    } else {
        shell
    };

    if let Some(dialog) = model.editor.dialog.as_ref() {
        stack![shell, editor_dialog_overlay(dialog, model, i18n)].into()
    } else if schema_create_dialog_open {
        stack![
            shell,
            schema_create_dialog_overlay(model, schema_create_error, i18n)
        ]
        .into()
    } else {
        shell
    }
}

fn top_shell<'a>(
    model: &'a ShellModel,
    app_theme: AppTheme,
    left_visible: bool,
    right_visible: bool,
    open_menu: Option<MenuId>,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let items = [
        (i18n.tr("menu-file"), MenuId::File),
        (i18n.tr("menu-view"), MenuId::View),
        (i18n.tr("menu-navigate"), MenuId::Navigate),
        (i18n.tr("menu-data"), MenuId::Data),
        (i18n.tr("menu-help"), MenuId::Help),
    ];

    let mut left = row![container(brand::lockup(app_theme)).padding(iced::Padding {
        top: LOGO_TOP_OFFSET,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    })]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    for (item, id) in items {
        left = left.push(menu_trigger(item, id, open_menu));
    }

    let input = text_input(
        i18n.tr_static("search-placeholder"),
        model.search.query.as_str(),
    )
    .padding([0, 4])
    .size(theme::typography::BODY)
    .width(theme::sizes::TOOLBAR_SEARCH_WIDTH)
    .style(theme::input_embedded);

    let search_message = if open_menu.is_some() {
        Message::MenuAction(MenuAction::Search)
    } else {
        Message::SearchOpened
    };
    let search = container(
        mouse_area(
            container(
                row![
                    widgets::icon(theme::Icon::Search, theme::icons::TOOLBAR, false),
                    input,
                    text("Ctrl+K")
                        .size(theme::typography::LABEL)
                        .font(theme::mono())
                        .line_height(iced::widget::text::LineHeight::Relative(1.0))
                        .style(theme::text_muted)
                ]
                .spacing(theme::spacing::SM)
                .align_y(Alignment::Center)
                .height(Length::Fill),
            )
            .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT)
            .padding([0.0, theme::spacing::MD])
            .style(theme::search_surface),
        )
        .on_press(search_message),
    )
    .padding(iced::Padding {
        top: SEARCH_TOP_OFFSET,
        right: 0.0,
        bottom: 0.0,
        left: SEARCH_LEFT_OFFSET,
    });

    let left_toggle = top_icon_button(
        theme::Icon::PanelLeft,
        left_visible,
        if open_menu.is_some() {
            Message::MenuAction(MenuAction::ToggleLeftSidebar)
        } else {
            Message::ToggleLeftSidebar
        },
        if left_visible {
            i18n.tr("tooltip-hide-left-sidebar")
        } else {
            i18n.tr("tooltip-show-left-sidebar")
        },
    );
    let right_toggle = top_icon_button(
        theme::Icon::Split,
        right_visible,
        if open_menu.is_some() {
            Message::MenuAction(MenuAction::ToggleRightSidebar)
        } else {
            Message::ToggleRightSidebar
        },
        if right_visible {
            i18n.tr("tooltip-hide-right-sidebar")
        } else {
            i18n.tr("tooltip-show-right-sidebar")
        },
    );

    let layout_group =
        container(row![left_toggle, right_toggle].spacing(theme::spacing::XS)).padding(2.0);

    let theme_button = iced::widget::tooltip(
        button(
            container(widgets::icon_text(
                theme::Icon::Settings,
                app_theme.label(),
                theme::icons::TOOLBAR,
                false,
            ))
            .height(Length::Fill)
            .align_y(alignment::Vertical::Center),
        )
        .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT)
        .padding([0.0, 14.0])
        .style(theme::button_toolbar)
        .on_press(if open_menu.is_some() {
            Message::MenuAction(MenuAction::ToggleTheme)
        } else {
            Message::ThemeToggled
        }),
        widgets::tooltip_text(i18n.tr("tooltip-toggle-theme")),
        iced::widget::tooltip::Position::Bottom,
    )
    .style(theme::tooltip);

    let right = row![
        iced::widget::Space::new().width(Length::Fill),
        layout_group,
        theme_button,
    ]
    .spacing(theme::spacing::MD)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    container(
        row![left.width(Length::Fill), search, right]
            .spacing(theme::spacing::LG)
            .align_y(Alignment::Center),
    )
    .height(theme::sizes::MENU_BAR_HEIGHT)
    .padding([0.0, theme::spacing::XL])
    .style(theme::top_bar)
    .into()
}

fn menu_trigger<'a>(
    label: String,
    menu: MenuId,
    open_menu: Option<MenuId>,
) -> Element<'a, Message> {
    mouse_area(
        container(
            button(text(label).size(theme::typography::MENU))
                .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT)
                .padding([0.0, 8.0])
                .style(if open_menu == Some(menu) {
                    theme::button_selected
                } else {
                    theme::button_ghost
                })
                .on_press(Message::MenuToggled(menu)),
        )
        .padding(iced::Padding {
            top: MENU_TOP_OFFSET,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }),
    )
    .on_move(move |_| Message::MenuHovered(menu))
    .into()
}

fn menu_trigger_placeholder<'a>(label: String) -> Element<'a, Message> {
    container(
        button(
            text(label)
                .size(theme::typography::MENU)
                .style(|_| iced::widget::text::Style {
                    color: Some(iced::Color::TRANSPARENT),
                }),
        )
        .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT)
        .padding([0.0, 8.0])
        .style(|_, _| button::Style {
            text_color: iced::Color::TRANSPARENT,
            ..button::Style::default()
        }),
    )
    .padding(iced::Padding {
        top: MENU_TOP_OFFSET,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    })
    .into()
}

fn top_icon_button<'a>(
    icon: theme::Icon,
    selected: bool,
    message: Message,
    tooltip: String,
) -> Element<'a, Message> {
    iced::widget::tooltip(
        button(widgets::icon(icon, theme::icons::TOOLBAR, selected))
            .width(theme::sizes::TOOLBAR_BUTTON_HEIGHT - 4.0)
            .height(theme::sizes::TOOLBAR_BUTTON_HEIGHT - 4.0)
            .padding(0)
            .style(theme::button_toolbar)
            .on_press(message),
        widgets::tooltip_text(tooltip),
        iced::widget::tooltip::Position::Bottom,
    )
    .style(theme::tooltip)
    .into()
}

fn content_frame<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(iced::Padding {
            top: theme::spacing::SM,
            right: theme::spacing::MD,
            bottom: theme::spacing::MD,
            left: theme::spacing::MD,
        })
        .into()
}

fn menu_overlay<'a>(menu: MenuId, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    let items = [
        (i18n.tr("menu-file"), MenuId::File),
        (i18n.tr("menu-view"), MenuId::View),
        (i18n.tr("menu-navigate"), MenuId::Navigate),
        (i18n.tr("menu-data"), MenuId::Data),
        (i18n.tr("menu-help"), MenuId::Help),
    ];

    let mut anchor_prefix = row![brand::placeholder()]
        .spacing(theme::spacing::SM)
        .align_y(Alignment::Center);
    for (label, id) in items {
        if id == menu {
            break;
        }
        anchor_prefix = anchor_prefix.push(menu_trigger_placeholder(label));
    }

    stack![
        column![
            iced::widget::Space::new().height(theme::sizes::MENU_TOP_OFFSET),
            mouse_area(container("").height(Length::Fill).width(Length::Fill))
                .on_press(Message::MenuClosed),
        ]
        .width(Length::Fill)
        .height(Length::Fill),
        column![
            iced::widget::Space::new().height(theme::sizes::MENU_TOP_OFFSET),
            row![anchor_prefix, menu_items(menu, i18n)].spacing(theme::spacing::SM),
        ]
        .padding([0.0, theme::spacing::XL])
        .width(Length::Fill)
        .height(Length::Fill),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn menu_items<'a>(menu: MenuId, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    let entries: Vec<(String, Option<&str>, MenuAction)> = match menu {
        MenuId::File => vec![
            (i18n.tr("menu-open-folder"), None, MenuAction::OpenFolder),
            (i18n.tr("menu-reindex"), None, MenuAction::Reindex),
        ],
        MenuId::View => vec![
            (i18n.tr("menu-toggle-theme"), None, MenuAction::ToggleTheme),
            (
                i18n.tr("menu-left-sidebar"),
                None,
                MenuAction::ToggleLeftSidebar,
            ),
            (
                i18n.tr("menu-right-sidebar"),
                None,
                MenuAction::ToggleRightSidebar,
            ),
        ],
        MenuId::Navigate => vec![
            (i18n.tr("menu-files"), None, MenuAction::Explorer),
            (i18n.tr("menu-data"), None, MenuAction::Data),
            (i18n.tr("menu-graph"), None, MenuAction::Graph),
            (i18n.tr("menu-health"), None, MenuAction::Health),
            (i18n.tr("menu-sql-explorer"), None, MenuAction::SqlExplorer),
            (i18n.tr("menu-history"), None, MenuAction::History),
            (i18n.tr("menu-settings"), None, MenuAction::Settings),
            (i18n.tr("menu-search"), Some("Ctrl+K"), MenuAction::Search),
        ],
        MenuId::Data => vec![
            (i18n.tr("menu-open-data"), None, MenuAction::Data),
            (i18n.tr("menu-open-graph"), None, MenuAction::Graph),
            (i18n.tr("menu-health"), None, MenuAction::Health),
            (i18n.tr("menu-sql-explorer"), None, MenuAction::SqlExplorer),
            (i18n.tr("menu-history"), None, MenuAction::History),
            (
                i18n.tr("menu-run-query"),
                Some("Ctrl+Enter"),
                MenuAction::ExecuteSql,
            ),
        ],
        MenuId::Help => vec![(i18n.tr("menu-about"), None, MenuAction::About)],
    };
    let mut items = column![];
    for (label, shortcut, action) in entries {
        let mut content = row![text(label)
            .size(theme::typography::BODY)
            .wrapping(iced::widget::text::Wrapping::None)
            .width(Length::Fill)];
        if let Some(shortcut) = shortcut {
            content = content.push(
                text(shortcut)
                    .font(theme::mono())
                    .size(theme::typography::LABEL)
                    .wrapping(iced::widget::text::Wrapping::None)
                    .style(theme::text_muted),
            );
        }
        items = items.push(
            button(content.align_y(Alignment::Center))
                .width(theme::sizes::MENU_WIDTH - theme::spacing::SM)
                .height(theme::sizes::MENU_ITEM_HEIGHT)
                .padding([theme::sizes::MENU_PADDING_Y, theme::sizes::MENU_PADDING_X])
                .style(theme::button_menu)
                .on_press(Message::MenuAction(action)),
        );
    }
    container(items)
        .padding(theme::sizes::MENU_POPUP_PADDING)
        .style(theme::overlay_panel)
        .into()
}

fn about_overlay<'a>(i18n: &'a I18nCatalog) -> Element<'a, Message> {
    mouse_area(
        container(
            column![
                text("FlokinMD")
                    .size(theme::typography::TITLE)
                    .style(theme::text_accent),
                text(i18n.tr("about-description")).style(theme::text_muted),
                button(text(i18n.tr("action-close")))
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
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let (title, description, save_label, discard_label) = match dialog {
        EditorDialog::CloseDirty { document_path } => {
            let file_name = model
                .editor
                .tab(document_path)
                .map(|tab| tab.title.as_str())
                .unwrap_or("arquivo.md");
            (
                i18n.tr_with("dirty-close-title", &[("file", file_name.into())]),
                i18n.tr("dirty-close-description"),
                i18n.tr("action-save"),
                i18n.tr("action-do-not-save"),
            )
        }
        EditorDialog::CloseWorkspaceDirty { dirty_count } => (
            i18n.tr_with("dirty-workspace-title", &[("count", (*dirty_count).into())]),
            i18n.tr("dirty-workspace-description"),
            i18n.tr("action-save-all"),
            i18n.tr("action-discard"),
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
                        button(text(i18n.tr("action-cancel")))
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

fn schema_create_dialog_overlay<'a>(
    model: &'a ShellModel,
    error: Option<&'a str>,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    let existing_schema = !matches!(
        model.schema_catalog.explicit_schema,
        ExplicitSchemaState::Absent
    );
    let available_collections = model
        .schema_catalog
        .collections
        .iter()
        .filter(|collection| collection.document_count > 0)
        .collect::<Vec<_>>();
    let mixed_fields = available_collections
        .iter()
        .flat_map(|collection| {
            collection
                .fields
                .iter()
                .filter(|field| field.field_type == SchemaType::Mixed)
                .map(move |field| format!("{} · {}", collection.display_name, field.name))
        })
        .collect::<Vec<_>>();

    let mut content = column![
        text(i18n.tr("schema-create-title"))
            .size(theme::typography::TITLE)
            .style(theme::text_accent),
        text(i18n.tr_with(
            "schema-create-description",
            &[("file", flokin_core::SCHEMA_FILE_NAME.into())],
        ))
        .size(theme::typography::BODY)
        .style(theme::text_normal),
    ]
    .spacing(theme::spacing::SM);

    if existing_schema {
        content = content.push(
            text(i18n.tr("schema-exists-warning"))
                .size(theme::typography::BODY)
                .style(theme::text_warning),
        );
    } else if available_collections.is_empty() {
        content = content.push(
            text(i18n.tr("schema-none-available"))
                .size(theme::typography::BODY)
                .style(theme::text_muted),
        );
    } else {
        content = content
            .push(
                text(i18n.tr("schema-from-inferred"))
                    .size(theme::typography::BODY)
                    .style(theme::text_muted),
            )
            .push(text(i18n.tr("schema-detected-collections")).size(theme::typography::BODY));
        for collection in &available_collections {
            content = content.push(
                text(i18n.tr_with(
                    "schema-collection-count",
                    &[
                        ("name", collection.display_name.as_str().into()),
                        ("count", collection.document_count.into()),
                    ],
                ))
                .size(theme::typography::BODY)
                .style(theme::text_muted),
            );
        }
        if !mixed_fields.is_empty() {
            content = content.push(
                text(i18n.tr_with(
                    "schema-mixed-fields-omitted",
                    &[("fields", mixed_fields.join(", ").into())],
                ))
                .size(theme::typography::BODY)
                .wrapping(iced::widget::text::Wrapping::Word)
                .style(theme::text_warning),
            );
        }
    }

    if let Some(error) = error {
        content = content.push(
            text(error)
                .size(theme::typography::BODY)
                .wrapping(iced::widget::text::Wrapping::Word)
                .style(theme::text_warning),
        );
    }

    let mut actions = row![
        button(text(i18n.tr("action-cancel")).size(theme::typography::BODY))
            .padding([6.0, 12.0])
            .style(theme::button_toolbar)
            .on_press(Message::SchemaCreateCanceled)
    ]
    .spacing(theme::spacing::SM)
    .align_y(Alignment::Center);

    if existing_schema {
        actions = actions.push(
            button(text(i18n.tr("schema-open")).size(theme::typography::BODY))
                .padding([6.0, 12.0])
                .style(theme::button_selected)
                .on_press(Message::SchemaOpenRequested),
        );
    } else {
        let create = button(text(i18n.tr("schema-create")).size(theme::typography::BODY))
            .padding([6.0, 12.0])
            .style(if available_collections.is_empty() {
                theme::button_ghost
            } else {
                theme::button_selected
            });
        actions = actions.push(if available_collections.is_empty() {
            create
        } else {
            create.on_press(Message::SchemaCreateConfirmed)
        });
    }

    let dialog = container(content.push(actions))
        .width(theme::sizes::DIALOG_WIDTH)
        .padding(theme::spacing::LG)
        .style(theme::overlay_panel);

    stack![
        mouse_area(
            container("")
                .width(Length::Fill)
                .height(Length::Fill)
                .style(theme::overlay_backdrop)
        )
        .on_press(Message::SchemaCreateCanceled),
        container(dialog)
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

fn panel_gutter<'a>() -> Element<'a, Message> {
    container("")
        .width(theme::sizes::SPLITTER_HIT_AREA)
        .height(Length::Fill)
        .style(theme::splitter)
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

fn search_overlay<'a>(model: &'a ShellModel, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    let query = model.search.query.trim();
    let body: Element<'_, Message> = if query.is_empty() {
        container(
            text(i18n.tr("search-type-to-search"))
                .size(theme::typography::BODY)
                .style(theme::text_muted),
        )
        .padding(theme::spacing::MD)
        .width(Length::Fill)
        .into()
    } else if model.search.results.is_empty() {
        container(
            text(i18n.tr_with("search-no-results", &[("query", query.into())]))
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
        i18n.tr_with(
            "search-results-limited",
            &[("count", model.search.results.len().into())],
        )
    } else {
        i18n.tr_with(
            "search-results",
            &[("count", model.search.total_matches.into())],
        )
    };

    let overlay_input = text_input(
        i18n.tr_static("search-placeholder"),
        model.search.query.as_str(),
    )
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

fn activity_bar<'a>(mode: AppMode, i18n: &'a I18nCatalog) -> Element<'a, Message> {
    let entries = [
        (
            AppMode::Files,
            theme::Icon::Folder,
            i18n.tr("activity-files"),
        ),
        (
            AppMode::Data,
            theme::Icon::Database,
            i18n.tr("activity-data"),
        ),
        (
            AppMode::Graph,
            theme::Icon::Graph,
            i18n.tr("activity-graph"),
        ),
        (
            AppMode::Health,
            theme::Icon::Health,
            i18n.tr("activity-health"),
        ),
        (AppMode::Sql, theme::Icon::Terminal, i18n.tr("activity-sql")),
        (
            AppMode::History,
            theme::Icon::Clock,
            i18n.tr("activity-history"),
        ),
    ];
    let mut top = column![]
        .spacing(theme::spacing::SM)
        .width(Length::Fill)
        .align_x(Alignment::Center);
    for (entry, icon, label) in entries {
        top = top.push(activity_button(entry, icon, label, mode == entry));
    }
    let bottom = activity_button(
        AppMode::Settings,
        theme::Icon::Settings,
        i18n.tr("activity-settings"),
        mode == AppMode::Settings,
    );

    container(column![
        container(top).width(Length::Fill),
        iced::widget::Space::new().height(Length::Fill),
        container(bottom)
            .width(Length::Fill)
            .align_x(alignment::Horizontal::Center),
    ])
    .width(theme::sizes::ACTIVITY_BAR_WIDTH)
    .height(Length::Fill)
    .padding([theme::spacing::LG, theme::spacing::XS])
    .style(theme::activity_bar)
    .into()
}

fn activity_button(
    mode: AppMode,
    icon: theme::Icon,
    label: String,
    selected: bool,
) -> Element<'static, Message> {
    let control = button(
        container(widgets::icon(icon, theme::icons::ACTIVITY, selected))
            .height(Length::Fill)
            .align_y(alignment::Vertical::Center),
    )
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
        widgets::tooltip_text(label),
        iced::widget::tooltip::Position::Right,
    )
    .style(theme::tooltip)
    .into()
}

#[allow(clippy::too_many_arguments)]
fn workspace<'a>(
    model: &'a ShellModel,
    app_theme: AppTheme,
    sql_editor: &'a text_editor::Content,
    markdown_editor: &'a text_editor::Content,
    markdown_preview: &'a [markdown::Item],
    sql_completion_items: &'a [SqlCompletionItem],
    sql_completion_selected: usize,
    sql_completion_open: bool,
    sql_editor_height: f32,
    i18n: &'a I18nCatalog,
) -> Element<'a, Message> {
    container(
        column![
            views::editor::tabs(model),
            views::editor::view(
                model,
                app_theme,
                sql_editor,
                markdown_editor,
                markdown_preview,
                sql_completion_items,
                sql_completion_selected,
                sql_completion_open,
                sql_editor_height,
                i18n,
            )
        ]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::document_surface)
    .into()
}
