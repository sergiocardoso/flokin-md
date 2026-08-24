use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};

use flokin_core::{
    complete_sql, default_query, mock_shell, replace_sql_completion, save_markdown_file, ScanError,
    ShellModel, SqlCompletionItem, WorkspaceEvent, DEFAULT_SQL_COMPLETION_LIMIT,
};
use iced::{
    advanced::widget::{self as advanced_widget, operate},
    application, event, keyboard,
    keyboard::{key::Named, Key},
    widget::text_editor,
    window, Element, Size, Subscription, Task, Theme,
};

use crate::{
    message::{AppMode, MenuAction, Message, SplitterKind},
    services::{file_dialog, file_watcher},
    theme::{self, AppTheme},
    views,
};

#[derive(Debug)]
pub struct FlokinApp {
    model: ShellModel,
    theme: AppTheme,
    search_needs_refresh: bool,
    search_debounce_target: Option<Instant>,
    sql_editor: text_editor::Content,
    markdown_editors: HashMap<PathBuf, text_editor::Content>,
    empty_markdown_editor: text_editor::Content,
    sql_completion: SqlCompletionPopup,
    workspace_update_running: bool,
    pending_workspace_events: Vec<WorkspaceEvent>,
    close_window_after_dialog: Option<window::Id>,
    pending_window_save: Option<(window::Id, Vec<std::path::PathBuf>)>,
    pending_workspace_switch: Option<std::path::PathBuf>,
    pending_workspace_save: Option<Vec<std::path::PathBuf>>,
    pending_reindex: bool,
    workspace_generation: u64,
    open_menu: Option<crate::message::MenuId>,
    about_open: bool,
    left_width: f32,
    inspector_width: f32,
    schema_width: f32,
    sql_editor_height: f32,
    splitter: Option<(SplitterKind, f32, f32)>,
    cursor: (f32, f32),
    menu_anchor_x: f32,
    left_visible: bool,
    right_visible: bool,
    mode: AppMode,
}

impl FlokinApp {
    fn new() -> Self {
        Self {
            model: mock_shell(),
            theme: AppTheme::Dark,
            search_needs_refresh: false,
            search_debounce_target: None,
            sql_editor: text_editor::Content::new(),
            markdown_editors: HashMap::new(),
            empty_markdown_editor: text_editor::Content::new(),
            sql_completion: SqlCompletionPopup::closed(),
            workspace_update_running: false,
            pending_workspace_events: Vec::new(),
            close_window_after_dialog: None,
            pending_window_save: None,
            pending_workspace_switch: None,
            pending_workspace_save: None,
            pending_reindex: false,
            workspace_generation: 0,
            open_menu: None,
            about_open: false,
            left_width: crate::theme::sizes::SIDEBAR_DEFAULT_WIDTH,
            inspector_width: crate::theme::sizes::INSPECTOR_DEFAULT_WIDTH,
            schema_width: crate::theme::sizes::SCHEMA_DEFAULT_WIDTH,
            sql_editor_height: crate::theme::sizes::SQL_EDITOR_DEFAULT_HEIGHT,
            splitter: None,
            cursor: (0.0, 0.0),
            menu_anchor_x: 0.0,
            left_visible: true,
            right_visible: true,
            mode: AppMode::Files,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AppModeSelected(mode) => {
                self.mode = mode;
                self.model.select_activity(match mode {
                    AppMode::Sql => flokin_core::Activity::Terminal,
                    AppMode::Settings => flokin_core::Activity::Settings,
                    AppMode::Files | AppMode::Data => flokin_core::Activity::Explorer,
                });
                if mode == AppMode::Sql {
                    self.model.open_sql_explorer();
                } else {
                    self.model.sql_explorer.open = false;
                    self.sql_completion.close();
                }
            }
            Message::ExplorerNodeToggled(id) => {
                if !self.model.toggle_explorer_node(id) && self.model.select_explorer_node(id) {
                    self.finish_document_open_or_activate();
                }
            }
            Message::OpenFolder => {
                return Task::perform(
                    async { file_dialog::pick_folder() },
                    Message::FolderSelected,
                );
            }
            Message::FolderSelected(path) => {
                if let Some(path) = path {
                    if self.model.editor.has_dirty_tabs() {
                        self.pending_workspace_switch = Some(path);
                        self.model.request_close_workspace();
                        return Task::none();
                    }
                    return self.switch_workspace(path);
                }
            }
            Message::ScanCompleted(generation, path, result) => {
                if generation == self.workspace_generation
                    && self.model.current_workspace.as_ref() == Some(&path)
                {
                    match result {
                        Ok(result) => {
                            self.model.scan_completed(result);
                            return rebuild_sql_projection_task(
                                generation,
                                path,
                                self.model.documents.clone(),
                                self.model.collections.clone(),
                            );
                        }
                        Err(message) => self.model.scan_failed(message),
                    }
                    self.search_needs_refresh = false;
                }
            }
            Message::ReindexWorkspace => {
                if let Some(path) = self.model.current_workspace.clone() {
                    if self.model.editor.has_dirty_tabs() {
                        self.pending_reindex = true;
                        self.model.request_close_workspace();
                        return Task::none();
                    }
                    return self.switch_workspace(path);
                }
            }
            Message::WorkspaceWatcher(message) => match message {
                file_watcher::WatcherMessage::Events { workspace, events } => {
                    return self.enqueue_workspace_events(workspace, events);
                }
                file_watcher::WatcherMessage::Failed { workspace, message } => {
                    if self.model.current_workspace.as_ref() == Some(&workspace) {
                        self.model.workspace_update_failed(ScanError {
                            path: workspace,
                            message,
                        });
                    }
                }
            },
            Message::WorkspaceUpdateCompleted(generation, path, result) => {
                if generation == self.workspace_generation
                    && self.model.current_workspace.as_ref() == Some(&path)
                {
                    match result {
                        Ok(update) if update.needs_rescan => {
                            self.workspace_update_running = false;
                            return scan_workspace_task(self.workspace_generation, path);
                        }
                        Ok(update) => {
                            let changed_paths = update.changed_paths();
                            self.model.workspace_update_completed(update);
                            self.sync_markdown_editors_for_paths(&changed_paths);
                            self.cleanup_markdown_editors();
                            self.workspace_update_running = false;
                            self.search_needs_refresh = false;
                            return Task::batch([
                                rebuild_sql_projection_task(
                                    self.workspace_generation,
                                    path.clone(),
                                    self.model.documents.clone(),
                                    self.model.collections.clone(),
                                ),
                                self.start_next_workspace_update(path),
                            ]);
                        }
                        Err(message) => {
                            self.workspace_update_running = false;
                            self.model.workspace_update_failed(ScanError {
                                path: path.clone(),
                                message,
                            });
                            return self.start_next_workspace_update(path);
                        }
                    }
                }
            }
            Message::CollectionSelected(collection_id) => {
                self.model.select_collection(collection_id);
            }
            Message::TableHeaderSelected(column_id) => {
                self.model.toggle_collection_sort(column_id);
            }
            Message::MarkdownSelected(path) => {
                self.open_or_activate_document(path);
            }
            Message::EditorTabSelected(path) => {
                if self.model.activate_editor_tab(path) {
                    self.ensure_markdown_editor_for_active();
                }
            }
            Message::EditorTabCloseRequested(path) => {
                self.model.request_close_editor_tab(path);
                self.cleanup_markdown_editors();
                self.ensure_markdown_editor_for_active();
            }
            Message::MarkdownEditorAction(action) => {
                let Some(path) = self.model.editor.active_path.clone() else {
                    return Task::none();
                };
                self.ensure_markdown_editor_for_path(&path);
                if let Some(content) = self.markdown_editors.get_mut(&path) {
                    content.perform(action);
                    self.model.update_active_editor_buffer(content.text());
                }
            }
            Message::EditorSaveRequested => {
                return self.save_editor_paths(self.model.pending_save_paths());
            }
            Message::EditorSaveCompleted(path, content, result) => {
                let saved = result.is_ok();
                self.model.editor_save_completed(&path, &content, result);
                if saved {
                    self.model.close_saved_dialog_tab(&path);
                    if let Some((window_id, paths)) = self.pending_window_save.clone() {
                        if paths.contains(&path)
                            && paths.iter().all(|path| {
                                self.model
                                    .editor
                                    .tab(path)
                                    .map(|tab| !tab.dirty)
                                    .unwrap_or(true)
                            })
                        {
                            self.pending_window_save = None;
                            return window::close(window_id);
                        }
                    }
                    if let Some(paths) = self.pending_workspace_save.clone() {
                        if paths.contains(&path)
                            && paths.iter().all(|path| {
                                self.model
                                    .editor
                                    .tab(path)
                                    .map(|tab| !tab.dirty)
                                    .unwrap_or(true)
                            })
                        {
                            self.pending_workspace_save = None;
                            self.model.editor.dialog = None;
                            if let Some(path) = self.pending_workspace_switch.take() {
                                return self.switch_workspace(path);
                            }
                            if self.pending_reindex {
                                self.pending_reindex = false;
                                if let Some(path) = self.model.current_workspace.clone() {
                                    return self.switch_workspace(path);
                                }
                            }
                        }
                    }
                    if let Some(workspace) = self.model.current_workspace.clone() {
                        return self.enqueue_workspace_events(
                            workspace,
                            vec![WorkspaceEvent::Upsert(path)],
                        );
                    }
                }
            }
            Message::EditorCloseActiveRequested => {
                self.model.request_close_active_editor_tab();
                self.cleanup_markdown_editors();
                self.ensure_markdown_editor_for_active();
            }
            Message::EditorDialogCancel => {
                self.model.cancel_editor_dialog();
                self.close_window_after_dialog = None;
                self.pending_window_save = None;
                self.pending_workspace_switch = None;
                self.pending_workspace_save = None;
                self.pending_reindex = false;
            }
            Message::EditorDialogDiscard => {
                self.model.discard_dialog_changes();
                self.cleanup_markdown_editors();
                self.ensure_markdown_editor_for_active();
                if let Some(window_id) = self.close_window_after_dialog.take() {
                    return window::close(window_id);
                }
                if let Some(path) = self.pending_workspace_switch.take() {
                    return self.switch_workspace(path);
                }
                if self.pending_reindex {
                    self.pending_reindex = false;
                    if let Some(path) = self.model.current_workspace.clone() {
                        return self.switch_workspace(path);
                    }
                }
            }
            Message::EditorDialogSave => {
                let paths = self.model.pending_save_paths();
                if let Some(window_id) = self.close_window_after_dialog {
                    self.pending_window_save = Some((window_id, paths.clone()));
                } else if self.pending_workspace_switch.is_some() || self.pending_reindex {
                    self.pending_workspace_save = Some(paths.clone());
                }
                return self.save_editor_paths(paths);
            }
            Message::EditorExternalReload => {
                if self.model.reload_external_editor_change() {
                    self.sync_markdown_editor_for_active_from_model();
                }
            }
            Message::EditorExternalKeep => {
                self.model.keep_local_editor_changes();
            }
            Message::WindowCloseRequested(window_id) => {
                if self.model.request_close_workspace() {
                    self.close_window_after_dialog = Some(window_id);
                } else {
                    return window::close(window_id);
                }
            }
            Message::SearchOpened => {
                self.model.open_search();
                self.search_needs_refresh = false;
                self.search_debounce_target = None;
                return focus_search_task();
            }
            Message::SearchClosed => {
                self.model.close_search();
                self.search_needs_refresh = false;
                self.search_debounce_target = None;
            }
            Message::SearchQueryChanged(query) => {
                self.model.open_search();
                self.model.update_search_query(query);
                self.search_needs_refresh = true;
                let target = Instant::now() + Duration::from_millis(150);
                self.search_debounce_target = Some(target);
                return debounce_search_task(target);
            }
            Message::SearchDebounceElapsed(target) => {
                if self.search_needs_refresh && self.search_debounce_target == Some(target) {
                    self.model.refresh_search_results();
                    self.search_needs_refresh = false;
                    self.search_debounce_target = None;
                }
            }
            Message::SearchNext => {
                self.model.select_next_search_result();
            }
            Message::SearchPrevious => {
                self.model.select_previous_search_result();
            }
            Message::SearchActivated => {
                if self.model.activate_selected_search_result() {
                    self.finish_document_open_or_activate();
                }
                self.search_needs_refresh = false;
                self.search_debounce_target = None;
            }
            Message::SearchResultSelected(path) => {
                self.open_search_document(path);
                self.search_needs_refresh = false;
                self.search_debounce_target = None;
            }
            Message::SqlExplorerOpened => {
                self.mode = AppMode::Sql;
                self.model.select_activity(flokin_core::Activity::Terminal);
                self.model.open_sql_explorer();
                if self.model.sql_explorer.query.is_empty() {
                    if let Some(catalog) = self.model.sql_explorer.catalog.as_ref() {
                        let query =
                            default_query(catalog, self.model.selected_collection.as_deref());
                        self.model.update_sql_query(query.clone());
                        self.sql_editor = text_editor::Content::with_text(&query);
                    }
                }
            }
            Message::SqlSchemaTableToggled(table) => {
                self.model.toggle_sql_schema_table(table);
            }
            Message::SqlEditorAction(action) => {
                self.sql_editor.perform(action);
                self.model.update_sql_query(self.sql_editor.text());
                self.refresh_sql_completion(false);
            }
            Message::SqlCompletionRequested => {
                self.refresh_sql_completion(true);
            }
            Message::SqlCompletionNext => {
                self.sql_completion.select_next();
            }
            Message::SqlCompletionPrevious => {
                self.sql_completion.select_previous();
            }
            Message::SqlCompletionAccepted => {
                self.accept_sql_completion();
            }
            Message::SqlCompletionSelected(index) => {
                self.sql_completion.selected =
                    index.min(self.sql_completion.items.len().saturating_sub(1));
                self.accept_sql_completion();
            }
            Message::SqlCompletionClosed => {
                self.sql_completion.close();
            }
            Message::SqlExecute => {
                self.sql_completion.close();
                self.model.update_sql_query(self.sql_editor.text());
                self.model.sql_execution_started();
                return execute_sql_task(
                    self.model.documents.clone(),
                    self.model.collections.clone(),
                    self.model.sql_explorer.query.clone(),
                );
            }
            Message::SqlProjectionCompleted(generation, path, result) => {
                if generation != self.workspace_generation
                    || self.model.current_workspace.as_ref() != Some(&path)
                {
                    return Task::none();
                }
                let should_fill_query = self.model.sql_explorer.query.is_empty();
                self.model.sql_projection_completed(result);
                self.refresh_sql_completion(false);
                if should_fill_query {
                    if let Some(catalog) = self.model.sql_explorer.catalog.as_ref() {
                        let query =
                            default_query(catalog, self.model.selected_collection.as_deref());
                        self.model.update_sql_query(query.clone());
                        self.sql_editor = text_editor::Content::with_text(&query);
                    }
                }
            }
            Message::SqlQueryCompleted(result) => {
                self.model.sql_execution_completed(result);
            }
            Message::KeyboardEvent(event) => {
                if let Some(message) =
                    keyboard_message(event, self.model.search.open, self.open_menu.is_some())
                {
                    return self.update(message);
                }
            }
            Message::ThemeToggled => {
                self.theme = self.theme.toggled();
            }
            Message::ThemeSelected(light) => {
                self.theme = if light {
                    AppTheme::Light
                } else {
                    AppTheme::Dark
                };
            }
            Message::MenuToggled(menu) => {
                self.open_menu = if self.open_menu == Some(menu) {
                    None
                } else {
                    Some(menu)
                };
            }
            Message::MenuTriggerMoved(_menu, local_x) => {
                self.menu_anchor_x = (self.cursor.0 - local_x).max(0.0);
            }
            Message::MenuAction(action) => {
                self.open_menu = None;
                match action {
                    MenuAction::OpenFolder => return self.update(Message::OpenFolder),
                    MenuAction::Reindex => return self.update(Message::ReindexWorkspace),
                    MenuAction::ToggleTheme => return self.update(Message::ThemeToggled),
                    MenuAction::ToggleLeftSidebar => {
                        return self.update(Message::ToggleLeftSidebar)
                    }
                    MenuAction::ToggleRightSidebar => {
                        return self.update(Message::ToggleRightSidebar)
                    }
                    MenuAction::Explorer => {
                        return self.update(Message::AppModeSelected(AppMode::Files))
                    }
                    MenuAction::Data => {
                        return self.update(Message::AppModeSelected(AppMode::Data))
                    }
                    MenuAction::SqlExplorer => {
                        return self.update(Message::AppModeSelected(AppMode::Sql))
                    }
                    MenuAction::Settings => {
                        return self.update(Message::AppModeSelected(AppMode::Settings))
                    }
                    MenuAction::Search => return self.update(Message::SearchOpened),
                    MenuAction::ExecuteSql => return self.update(Message::SqlExecute),
                    MenuAction::About => self.about_open = true,
                }
            }
            Message::MenuClosed => self.open_menu = None,
            Message::AboutClosed => self.about_open = false,
            Message::SplitterPressed(kind, _position) => {
                let value = match kind {
                    SplitterKind::LeftSidebar => self.left_width,
                    SplitterKind::Inspector => self.inspector_width,
                    SplitterKind::SqlSchema => self.schema_width,
                    SplitterKind::SqlEditor => self.sql_editor_height,
                };
                let position = if kind == SplitterKind::SqlEditor {
                    self.cursor.1
                } else {
                    self.cursor.0
                };
                self.splitter = Some((kind, position, value));
            }
            Message::SplitterMoved(x, y) => {
                self.cursor = (x, y);
                if let Some((kind, origin, initial)) = self.splitter {
                    let delta = if kind == SplitterKind::SqlEditor {
                        y - origin
                    } else if kind == SplitterKind::Inspector {
                        origin - x
                    } else {
                        x - origin
                    };
                    match kind {
                        SplitterKind::LeftSidebar => {
                            self.left_width = (initial + delta).clamp(
                                crate::theme::sizes::SIDEBAR_MIN_WIDTH,
                                crate::theme::sizes::SIDEBAR_MAX_WIDTH,
                            )
                        }
                        SplitterKind::Inspector => {
                            self.inspector_width = (initial + delta).clamp(
                                crate::theme::sizes::INSPECTOR_MIN_WIDTH,
                                crate::theme::sizes::INSPECTOR_MAX_WIDTH,
                            )
                        }
                        SplitterKind::SqlSchema => {
                            self.schema_width = (initial + delta).clamp(
                                crate::theme::sizes::SCHEMA_MIN_WIDTH,
                                crate::theme::sizes::SCHEMA_MAX_WIDTH,
                            )
                        }
                        SplitterKind::SqlEditor => {
                            self.sql_editor_height = (initial + delta).clamp(
                                crate::theme::sizes::SQL_EDITOR_MIN_HEIGHT,
                                crate::theme::sizes::SQL_EDITOR_MAX_HEIGHT,
                            )
                        }
                    }
                }
            }
            Message::SplitterReleased => self.splitter = None,
            Message::ToggleLeftSidebar => self.left_visible = !self.left_visible,
            Message::ToggleRightSidebar => self.right_visible = !self.right_visible,
            Message::ResetLayout => {
                self.left_width = crate::theme::sizes::SIDEBAR_DEFAULT_WIDTH;
                self.inspector_width = crate::theme::sizes::INSPECTOR_DEFAULT_WIDTH;
                self.schema_width = crate::theme::sizes::SCHEMA_DEFAULT_WIDTH;
                self.sql_editor_height = crate::theme::sizes::SQL_EDITOR_DEFAULT_HEIGHT;
                self.left_visible = true;
                self.right_visible = true;
            }
            Message::MockAction => {}
        }

        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let markdown_editor = self
            .model
            .editor
            .active_path
            .as_ref()
            .and_then(|path| self.markdown_editors.get(path))
            .unwrap_or(&self.empty_markdown_editor);

        views::shell::view(
            &self.model,
            self.theme,
            &self.sql_editor,
            markdown_editor,
            &self.sql_completion.items,
            self.sql_completion.selected,
            self.sql_completion.open,
            self.left_width,
            self.inspector_width,
            self.schema_width,
            self.sql_editor_height,
            self.open_menu,
            self.menu_anchor_x,
            self.about_open,
            self.left_visible,
            self.right_visible,
            self.mode,
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            file_watcher::subscription(self.model.current_workspace.clone()),
            window::close_requests().map(Message::WindowCloseRequested),
            keyboard::listen().map(Message::KeyboardEvent),
            event::listen_with(|event, _status, _window| match event {
                iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                    Some(Message::SplitterMoved(position.x, position.y))
                }
                iced::Event::Mouse(iced::mouse::Event::ButtonReleased(_)) => {
                    Some(Message::SplitterReleased)
                }
                _ => None,
            }),
        ])
    }
}

#[derive(Debug, Clone, Default)]
struct SqlCompletionPopup {
    open: bool,
    items: Vec<SqlCompletionItem>,
    selected: usize,
}

impl SqlCompletionPopup {
    fn closed() -> Self {
        Self::default()
    }

    fn close(&mut self) {
        self.open = false;
        self.items.clear();
        self.selected = 0;
    }

    fn set_items(&mut self, items: Vec<SqlCompletionItem>) {
        self.open = !items.is_empty();
        self.items = items;
        self.selected = self.selected.min(self.items.len().saturating_sub(1));
    }

    fn select_next(&mut self) {
        if !self.items.is_empty() {
            self.selected = (self.selected + 1) % self.items.len();
        }
    }

    fn select_previous(&mut self) {
        if !self.items.is_empty() {
            self.selected = if self.selected == 0 {
                self.items.len() - 1
            } else {
                self.selected - 1
            };
        }
    }
}

impl FlokinApp {
    fn open_or_activate_document(&mut self, path: std::path::PathBuf) -> bool {
        if self.model.select_markdown_path(path) {
            self.finish_document_open_or_activate();
            true
        } else {
            false
        }
    }

    fn open_search_document(&mut self, path: std::path::PathBuf) -> bool {
        if self.model.select_search_result_path(path) {
            self.finish_document_open_or_activate();
            true
        } else {
            false
        }
    }

    fn finish_document_open_or_activate(&mut self) {
        self.mode = AppMode::Files;
        self.model.sql_explorer.open = false;
        self.sql_completion.close();
        self.ensure_markdown_editor_for_active();
    }

    fn switch_workspace(&mut self, path: std::path::PathBuf) -> Task<Message> {
        self.workspace_generation = self.workspace_generation.wrapping_add(1);
        let generation = self.workspace_generation;
        self.model.workspace_selected(Some(path.clone()));
        self.sql_editor = text_editor::Content::new();
        self.markdown_editors.clear();
        self.empty_markdown_editor = text_editor::Content::new();
        self.sql_completion.close();
        self.workspace_update_running = false;
        self.pending_workspace_events.clear();
        self.close_window_after_dialog = None;
        self.pending_window_save = None;
        self.pending_workspace_switch = None;
        self.pending_workspace_save = None;
        self.pending_reindex = false;
        scan_workspace_task(generation, path)
    }

    fn ensure_markdown_editor_for_active(&mut self) {
        let Some(path) = self.model.editor.active_path.clone() else {
            return;
        };
        self.ensure_markdown_editor_for_path(&path);
    }

    fn ensure_markdown_editor_for_path(&mut self, path: &std::path::Path) {
        if self.markdown_editors.contains_key(path) {
            return;
        }
        let Some(tab) = self.model.editor.tab(path) else {
            return;
        };
        self.markdown_editors.insert(
            path.to_path_buf(),
            text_editor::Content::with_text(&tab.buffer),
        );
    }

    fn sync_markdown_editor_for_active_from_model(&mut self) {
        let Some(path) = self.model.editor.active_path.clone() else {
            return;
        };
        self.sync_markdown_editors_for_paths(&[path]);
    }

    fn sync_markdown_editors_for_paths(&mut self, paths: &[std::path::PathBuf]) {
        for path in paths {
            let Some(tab) = self.model.editor.tab(path) else {
                continue;
            };
            let Some(content) = self.markdown_editors.get_mut(path) else {
                continue;
            };
            if content.text() != tab.buffer {
                *content = text_editor::Content::with_text(&tab.buffer);
            }
        }
    }

    fn cleanup_markdown_editors(&mut self) {
        self.markdown_editors
            .retain(|path, _| self.model.editor.tab(path).is_some());
    }

    fn enqueue_workspace_events(
        &mut self,
        workspace: std::path::PathBuf,
        events: Vec<WorkspaceEvent>,
    ) -> Task<Message> {
        if self.model.current_workspace.as_ref() != Some(&workspace) {
            return Task::none();
        }
        self.pending_workspace_events.extend(events);
        if self.workspace_update_running {
            Task::none()
        } else {
            self.start_next_workspace_update(workspace)
        }
    }

    fn start_next_workspace_update(&mut self, workspace: std::path::PathBuf) -> Task<Message> {
        if self.workspace_update_running || self.pending_workspace_events.is_empty() {
            return Task::none();
        }
        let events = std::mem::take(&mut self.pending_workspace_events);
        self.workspace_update_running = true;
        self.model.workspace_update_started();
        workspace_events_task(self.workspace_generation, workspace, events)
    }

    fn save_editor_paths(&self, paths: Vec<std::path::PathBuf>) -> Task<Message> {
        let tasks = paths
            .into_iter()
            .filter_map(|path| {
                let content = self.model.editor.tab(&path).map(|tab| tab.buffer.clone())?;
                Some(save_editor_tab_task(path, content))
            })
            .collect::<Vec<_>>();

        if tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(tasks)
        }
    }

    fn refresh_sql_completion(&mut self, manual: bool) {
        let Some(catalog) = self.model.sql_explorer.catalog.as_ref() else {
            self.sql_completion.close();
            return;
        };
        let query = self.sql_editor.text();
        let cursor = cursor_offset(&self.sql_editor);
        if !manual && !should_auto_trigger_completion(&query, cursor) {
            self.sql_completion.close();
            return;
        }
        let items = complete_sql(catalog, &query, cursor, DEFAULT_SQL_COMPLETION_LIMIT);
        self.sql_completion.set_items(items);
    }

    fn accept_sql_completion(&mut self) {
        let Some(item) = self
            .sql_completion
            .items
            .get(self.sql_completion.selected)
            .cloned()
        else {
            return;
        };
        let updated = replace_sql_completion(&self.sql_editor.text(), &item);
        let cursor = item.replacement_start + item.insert_text.len();
        self.sql_editor = text_editor::Content::with_text(&updated);
        move_editor_cursor_to_offset(&mut self.sql_editor, cursor);
        self.model.update_sql_query(updated);
        self.sql_completion.close();
    }
}

fn cursor_offset(content: &text_editor::Content) -> usize {
    let cursor = content.cursor().position;
    let mut offset = 0;
    for line_index in 0..cursor.line {
        let Some(line) = content.line(line_index) else {
            return content.text().len();
        };
        offset += line.text.len();
        offset += line.ending.as_str().len();
    }
    let Some(line) = content.line(cursor.line) else {
        return content.text().len();
    };
    offset + cursor.column.min(line.text.len())
}

fn move_editor_cursor_to_offset(content: &mut text_editor::Content, offset: usize) {
    let mut remaining = offset;
    for line_index in 0..content.line_count() {
        let Some(line) = content.line(line_index) else {
            break;
        };
        if remaining <= line.text.len() {
            content.move_to(text_editor::Cursor {
                position: text_editor::Position {
                    line: line_index,
                    column: remaining,
                },
                selection: None,
            });
            return;
        }
        remaining = remaining.saturating_sub(line.text.len());
        let ending_len = line.ending.as_str().len();
        if remaining <= ending_len {
            content.move_to(text_editor::Cursor {
                position: text_editor::Position {
                    line: (line_index + 1).min(content.line_count().saturating_sub(1)),
                    column: 0,
                },
                selection: None,
            });
            return;
        }
        remaining -= ending_len;
    }
    content.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
}

fn should_auto_trigger_completion(query: &str, cursor: usize) -> bool {
    let before = &query[..cursor.min(query.len())];
    let Some(character) = before.chars().next_back() else {
        return false;
    };
    character == '.' || character == '_' || character.is_alphanumeric()
}

pub fn run() -> iced::Result {
    application(FlokinApp::new, FlokinApp::update, FlokinApp::view)
        .subscription(FlokinApp::subscription)
        .title(title)
        .theme(app_theme)
        .style(app_style)
        .exit_on_close_request(false)
        .window(window::Settings {
            size: Size::new(1440.0, 900.0),
            min_size: Some(Size::new(1100.0, 700.0)),
            resizable: true,
            ..window::Settings::default()
        })
        .run()
}

fn scan_workspace_task(generation: u64, path: std::path::PathBuf) -> Task<Message> {
    Task::perform(
        async move {
            let result = flokin_core::scan_workspace(&path).map_err(|error| error.to_string());
            (path, result)
        },
        move |(path, result)| Message::ScanCompleted(generation, path, result),
    )
}

fn workspace_events_task(
    generation: u64,
    workspace: std::path::PathBuf,
    events: Vec<WorkspaceEvent>,
) -> Task<Message> {
    Task::perform(
        async move {
            let result = flokin_core::workspace_update_from_events(&workspace, &events)
                .map_err(|error| error.to_string());
            (workspace, result)
        },
        move |(path, result)| Message::WorkspaceUpdateCompleted(generation, path, result),
    )
}

fn save_editor_tab_task(path: std::path::PathBuf, content: String) -> Task<Message> {
    Task::perform(
        async move {
            let result = save_markdown_file(&path, &content)
                .map_err(|error| format!("Não foi possível salvar {}: {error}", path.display()));
            (path, content, result)
        },
        |(path, content, result)| Message::EditorSaveCompleted(path, content, result),
    )
}

fn rebuild_sql_projection_task(
    generation: u64,
    path: std::path::PathBuf,
    documents: Vec<flokin_core::Document>,
    collections: Vec<flokin_core::Collection>,
) -> Task<Message> {
    Task::perform(
        async move {
            let result = flokin_core::SqlProjection::build(&documents, &collections)
                .map(|projection| projection.catalog().clone());
            (path, result)
        },
        move |(path, result)| Message::SqlProjectionCompleted(generation, path, result),
    )
}

fn execute_sql_task(
    documents: Vec<flokin_core::Document>,
    collections: Vec<flokin_core::Collection>,
    query: String,
) -> Task<Message> {
    Task::perform(
        async move {
            let projection = flokin_core::SqlProjection::build(&documents, &collections)?;
            projection.execute_read(&query, flokin_core::DEFAULT_RESULT_LIMIT)
        },
        Message::SqlQueryCompleted,
    )
}

fn focus_search_task() -> Task<Message> {
    let id = advanced_widget::Id::new("search-overlay-input");
    Task::batch([
        operate::<Message>(advanced_widget::operation::focusable::focus(id.clone())),
        operate::<Message>(advanced_widget::operation::text_input::move_cursor_to_end(
            id,
        )),
    ])
}

fn debounce_search_task(target: Instant) -> Task<Message> {
    Task::perform(
        async move {
            std::thread::sleep(Duration::from_millis(150));
            target
        },
        Message::SearchDebounceElapsed,
    )
}

fn keyboard_message(event: keyboard::Event, search_open: bool, menu_open: bool) -> Option<Message> {
    let keyboard::Event::KeyPressed {
        key,
        modified_key,
        physical_key,
        modifiers,
        ..
    } = event
    else {
        return None;
    };

    if menu_open && matches!(key, Key::Named(Named::Escape)) {
        return Some(Message::MenuClosed);
    }

    if modifiers.command()
        && key
            .to_latin(physical_key)
            .or_else(|| modified_key.to_latin(physical_key))
            == Some('k')
    {
        return Some(Message::SearchOpened);
    }

    if !search_open {
        return None;
    }

    match key.as_ref() {
        Key::Named(Named::Escape) => Some(Message::SearchClosed),
        Key::Named(Named::ArrowDown) => Some(Message::SearchNext),
        Key::Named(Named::ArrowUp) => Some(Message::SearchPrevious),
        Key::Named(Named::Enter) => Some(Message::SearchActivated),
        _ => None,
    }
}

fn title(_state: &FlokinApp) -> String {
    String::from("FlokinMD")
}

fn app_theme(state: &FlokinApp) -> Theme {
    state.theme.iced()
}

fn app_style(_state: &FlokinApp, theme: &Theme) -> iced::theme::Style {
    theme::application_style(theme)
}

#[cfg(test)]
mod tests {
    use flokin_core::{scan_workspace, Activity, ScanResult, ScanState, SqlError, WorkspaceEvent};
    use iced::{
        keyboard::{
            key::{Code, Named, Physical},
            Event, Key, Location, Modifiers,
        },
        widget::text_editor,
    };
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{keyboard_message, FlokinApp};
    use crate::{
        message::{AppMode, Message, SplitterKind},
        services::file_watcher::WatcherMessage,
        theme::AppTheme,
    };

    #[test]
    fn starts_with_native_shell_defaults() {
        let app = FlokinApp::new();

        assert_eq!(app.model.active_activity, Activity::Explorer);
        assert_eq!(app.model.current_workspace, None);
        assert_eq!(app.theme, AppTheme::Dark);
    }

    #[test]
    fn update_ignores_mock_action() {
        let mut app = FlokinApp::new();

        let _ = app.update(Message::MockAction);

        assert_eq!(app.model.active_activity, Activity::Explorer);
    }

    #[test]
    fn workspace_events_are_serialized_and_coalesced_while_update_runs() {
        let workspace = PathBuf::from("/tmp/flokinmd-serialized");
        let mut app = FlokinApp::new();
        app.model.workspace_selected(Some(workspace.clone()));
        app.model.scan_completed(ScanResult {
            root: workspace.clone(),
            documents: Vec::new(),
            collections: Vec::new(),
            directories: Vec::new(),
            errors: Vec::new(),
            duration: std::time::Duration::ZERO,
        });

        let first = app.enqueue_workspace_events(
            workspace.clone(),
            vec![WorkspaceEvent::Upsert(workspace.join("a.md"))],
        );
        assert!(matches!(app.model.scan_state, ScanState::Updating { .. }));
        assert!(app.workspace_update_running);
        assert!(app.pending_workspace_events.is_empty());
        drop(first);

        let second = app.enqueue_workspace_events(
            workspace.clone(),
            vec![WorkspaceEvent::Upsert(workspace.join("b.md"))],
        );
        assert!(app.workspace_update_running);
        assert_eq!(app.pending_workspace_events.len(), 1);
        drop(second);

        app.workspace_update_running = false;
        let next = app.start_next_workspace_update(workspace);
        assert!(app.workspace_update_running);
        assert!(app.pending_workspace_events.is_empty());
        drop(next);
    }

    #[test]
    fn per_tab_text_editor_content_is_not_duplicated_or_shared() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "A\n");
        workspace.write("b.md", "B\n");
        let mut app = app_from_workspace(&workspace);
        let a = workspace.path().join("a.md");
        let b = workspace.path().join("b.md");

        let _ = app.update(Message::MarkdownSelected(a.clone()));
        app.model
            .update_active_editor_buffer(String::from("A local\n"));
        app.markdown_editors
            .insert(a.clone(), text_editor::Content::with_text("A local\n"));
        let _ = app.update(Message::MarkdownSelected(b.clone()));
        app.model
            .update_active_editor_buffer(String::from("B local\n"));
        app.markdown_editors
            .insert(b.clone(), text_editor::Content::with_text("B local\n"));
        let _ = app.update(Message::EditorTabSelected(a.clone()));

        assert_eq!(app.markdown_editors.len(), 2);
        assert_eq!(app.markdown_editors.get(&a).unwrap().text(), "A local\n");
        assert_eq!(app.markdown_editors.get(&b).unwrap().text(), "B local\n");
        assert_eq!(app.model.editor.tabs.len(), 2);
    }

    #[test]
    fn syncing_target_tab_does_not_rebuild_unrelated_active_content() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "A\n");
        workspace.write("b.md", "B\n");
        let mut app = app_from_workspace(&workspace);
        let a = workspace.path().join("a.md");
        let b = workspace.path().join("b.md");
        let _ = app.update(Message::MarkdownSelected(a.clone()));
        let _ = app.update(Message::MarkdownSelected(b.clone()));
        let _ = app.update(Message::EditorTabSelected(a.clone()));
        app.model
            .update_active_editor_buffer(String::from("A local\n"));
        app.markdown_editors
            .insert(a.clone(), text_editor::Content::with_text("A local\n"));

        app.model.activate_editor_tab(b.clone());
        app.model
            .update_active_editor_buffer(String::from("B external\n"));
        app.sync_markdown_editors_for_paths(std::slice::from_ref(&b));
        app.model.activate_editor_tab(a.clone());

        assert_eq!(app.markdown_editors.get(&a).unwrap().text(), "A local\n");
    }

    #[test]
    fn explorer_open_non_empty_file_initializes_editor_content_immediately() {
        let workspace = TempWorkspace::new();
        workspace.write("projects/b.md", "# B\nconteúdo real\n");
        let mut app = app_from_workspace(&workspace);
        let path = workspace.path().join("projects/b.md");
        let id = explorer_file_id(&app, &path);

        let _ = app.update(Message::ExplorerNodeToggled(id));

        assert_eq!(app.model.editor.active_path, Some(path.clone()));
        assert_eq!(
            app.markdown_editors.get(&path).unwrap().text(),
            "# B\nconteúdo real\n"
        );
    }

    #[test]
    fn explorer_open_does_not_need_editor_action_to_show_content() {
        let workspace = TempWorkspace::new();
        workspace.write("b.md", "B before focus\n");
        let mut app = app_from_workspace(&workspace);
        let path = workspace.path().join("b.md");

        let _ = app.update(Message::ExplorerNodeToggled(explorer_file_id(&app, &path)));

        assert_eq!(
            app.markdown_editors.get(&path).unwrap().text(),
            "B before focus\n"
        );
    }

    #[test]
    fn real_empty_file_initializes_empty_editor_content() {
        let workspace = TempWorkspace::new();
        workspace.write("empty.md", "");
        let mut app = app_from_workspace(&workspace);
        let path = workspace.path().join("empty.md");

        app.open_or_activate_document(path.clone());

        assert_eq!(app.model.editor.tab(&path).unwrap().buffer, "");
        assert_eq!(app.markdown_editors.get(&path).unwrap().text(), "");
    }

    #[test]
    fn missing_ui_state_is_initialized_from_existing_editor_buffer() {
        let workspace = TempWorkspace::new();
        workspace.write("b.md", "B buffer\n");
        let mut app = app_from_workspace(&workspace);
        let path = workspace.path().join("b.md");
        assert!(app.model.select_markdown_path(path.clone()));
        assert!(!app.markdown_editors.contains_key(&path));

        app.finish_document_open_or_activate();

        assert_eq!(
            app.markdown_editors.get(&path).unwrap().text(),
            "B buffer\n"
        );
    }

    #[test]
    fn opening_a_then_b_shows_b_in_first_render_state() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "A\n");
        workspace.write("b.md", "B\n");
        let mut app = app_from_workspace(&workspace);
        let a = workspace.path().join("a.md");
        let b = workspace.path().join("b.md");

        app.open_or_activate_document(a);
        app.open_or_activate_document(b.clone());

        assert_eq!(app.model.editor.active_path, Some(b.clone()));
        assert_eq!(app.markdown_editors.get(&b).unwrap().text(), "B\n");
    }

    #[test]
    fn switching_back_uses_existing_content_without_recreate() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "A\n");
        workspace.write("b.md", "B\n");
        let mut app = app_from_workspace(&workspace);
        let a = workspace.path().join("a.md");
        let b = workspace.path().join("b.md");
        app.open_or_activate_document(a.clone());
        app.markdown_editors
            .insert(a.clone(), text_editor::Content::with_text("A existing\n"));
        app.open_or_activate_document(b.clone());

        let _ = app.update(Message::EditorTabSelected(a.clone()));

        assert_eq!(app.markdown_editors.len(), 2);
        assert_eq!(app.markdown_editors.get(&a).unwrap().text(), "A existing\n");
        assert_eq!(app.model.editor.active_path, Some(a));
    }

    #[test]
    fn opening_existing_tab_does_not_recreate_ui_content() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "A\n");
        let mut app = app_from_workspace(&workspace);
        let a = workspace.path().join("a.md");
        app.open_or_activate_document(a.clone());
        app.markdown_editors
            .insert(a.clone(), text_editor::Content::with_text("A ui state\n"));

        app.open_or_activate_document(a.clone());

        assert_eq!(app.markdown_editors.get(&a).unwrap().text(), "A ui state\n");
    }

    #[test]
    fn dirty_buffer_and_ui_content_survive_tab_switch() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "A\n");
        workspace.write("b.md", "B\n");
        let mut app = app_from_workspace(&workspace);
        let a = workspace.path().join("a.md");
        let b = workspace.path().join("b.md");
        app.open_or_activate_document(a.clone());
        app.model
            .update_active_editor_buffer(String::from("A local\n"));
        app.markdown_editors
            .insert(a.clone(), text_editor::Content::with_text("A local\n"));
        app.open_or_activate_document(b);

        let _ = app.update(Message::EditorTabSelected(a.clone()));

        assert_eq!(app.model.editor.tab(&a).unwrap().buffer, "A local\n");
        assert_eq!(app.markdown_editors.get(&a).unwrap().text(), "A local\n");
    }

    #[test]
    fn relation_navigation_initializes_editor_immediately() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "---\ntitle: A\nrelated: \"[[B]]\"\n---\n");
        workspace.write("b.md", "---\ntitle: B\n---\n# B\n");
        let mut app = app_from_workspace(&workspace);
        let b = workspace.path().join("b.md");

        let _ = app.update(Message::MarkdownSelected(b.clone()));

        assert_eq!(
            app.markdown_editors.get(&b).unwrap().text(),
            "---\ntitle: B\n---\n# B\n"
        );
    }

    #[test]
    fn search_navigation_initializes_editor_immediately() {
        let workspace = TempWorkspace::new();
        workspace.write("b.md", "# B searchable\n");
        let mut app = app_from_workspace(&workspace);
        let b = workspace.path().join("b.md");

        let _ = app.update(Message::SearchResultSelected(b.clone()));

        assert_eq!(
            app.markdown_editors.get(&b).unwrap().text(),
            "# B searchable\n"
        );
    }

    #[test]
    fn update_toggles_theme_in_memory() {
        let mut app = FlokinApp::new();

        let _ = app.update(Message::ThemeToggled);
        assert_eq!(app.theme, AppTheme::Light);

        let _ = app.update(Message::ThemeToggled);
        assert_eq!(app.theme, AppTheme::Dark);
    }

    #[test]
    fn folder_selected_updates_workspace_state() {
        let mut app = FlokinApp::new();
        let path = std::path::PathBuf::from("/tmp/Conhecimento");

        let _ = app.update(Message::FolderSelected(Some(path.clone())));

        assert_eq!(app.model.current_workspace, Some(path));
        assert!(matches!(app.model.scan_state, ScanState::Scanning));
    }

    #[test]
    fn projection_failure_does_not_clear_selected_workspace() {
        let mut app = FlokinApp::new();
        let path = std::path::PathBuf::from("/tmp/flokinmd-sql-test");
        let _ = app.update(Message::FolderSelected(Some(path.clone())));

        let _ = app.update(Message::SqlProjectionCompleted(
            1,
            path.clone(),
            Err(SqlError {
                message: String::from("projection failed"),
            }),
        ));

        assert_eq!(app.model.current_workspace, Some(path));
        assert_eq!(
            app.model.sql_explorer.error.as_deref(),
            Some("projection failed")
        );
    }

    #[test]
    fn stale_scan_result_cannot_replace_new_workspace() {
        let mut app = FlokinApp::new();
        let first = std::path::PathBuf::from("/tmp/workspace-a");
        let second = std::path::PathBuf::from("/tmp/workspace-b");

        let _ = app.update(Message::FolderSelected(Some(first.clone())));
        let _ = app.update(Message::FolderSelected(Some(second.clone())));
        let empty_scan = || ScanResult {
            root: first.clone(),
            documents: Vec::new(),
            collections: Vec::new(),
            directories: Vec::new(),
            errors: Vec::new(),
            duration: std::time::Duration::ZERO,
        };

        let _ = app.update(Message::ScanCompleted(1, first.clone(), Ok(empty_scan())));

        assert_eq!(app.model.current_workspace, Some(second));
        assert!(matches!(app.model.scan_state, ScanState::Scanning));
    }

    #[test]
    fn canceling_folder_dialog_preserves_workspace_state() {
        let mut app = FlokinApp::new();
        let path = std::path::PathBuf::from("/tmp/Conhecimento");

        let _ = app.update(Message::FolderSelected(Some(path.clone())));
        let _ = app.update(Message::FolderSelected(None));

        assert_eq!(app.model.current_workspace, Some(path));
    }

    #[test]
    fn watcher_event_from_old_workspace_is_ignored() {
        let mut app = FlokinApp::new();
        let current = std::path::PathBuf::from("/tmp/current");
        let old = std::path::PathBuf::from("/tmp/old");
        let _ = app.update(Message::FolderSelected(Some(current.clone())));

        let _ = app.update(Message::WorkspaceWatcher(WatcherMessage::Events {
            workspace: old.clone(),
            events: vec![WorkspaceEvent::Upsert(old.join("stale.md"))],
        }));

        assert_eq!(app.model.current_workspace, Some(current));
        assert!(matches!(app.model.scan_state, ScanState::Scanning));
    }

    #[test]
    fn ctrl_k_opens_search() {
        let message = keyboard_message(
            Event::KeyPressed {
                key: Key::Character("k".into()),
                modified_key: Key::Character("k".into()),
                physical_key: Physical::Code(Code::KeyK),
                location: Location::Standard,
                modifiers: Modifiers::CTRL,
                text: None,
                repeat: false,
            },
            false,
            false,
        );

        assert_eq!(message, Some(Message::SearchOpened));
    }

    #[test]
    fn search_keyboard_navigation_only_runs_when_open() {
        let event = Event::KeyPressed {
            key: Key::Named(Named::ArrowDown),
            modified_key: Key::Named(Named::ArrowDown),
            physical_key: Physical::Code(Code::ArrowDown),
            location: Location::Standard,
            modifiers: Modifiers::NONE,
            text: None,
            repeat: false,
        };

        assert_eq!(keyboard_message(event.clone(), false, false), None);
        assert_eq!(
            keyboard_message(event, true, false),
            Some(Message::SearchNext)
        );
    }

    #[test]
    fn escape_closes_an_open_menu() {
        let event = Event::KeyPressed {
            key: Key::Named(Named::Escape),
            modified_key: Key::Named(Named::Escape),
            physical_key: Physical::Code(Code::Escape),
            location: Location::Standard,
            modifiers: Modifiers::NONE,
            text: None,
            repeat: false,
        };

        assert_eq!(
            keyboard_message(event, false, true),
            Some(Message::MenuClosed)
        );
    }

    #[test]
    fn splitter_width_is_clamped_to_keep_panels_visible() {
        let mut app = FlokinApp::new();
        let _ = app.update(Message::SplitterPressed(SplitterKind::LeftSidebar, 0.0));
        let _ = app.update(Message::SplitterMoved(-10_000.0, 0.0));
        assert_eq!(app.left_width, 220.0);
        let _ = app.update(Message::SplitterMoved(10_000.0, 0.0));
        assert_eq!(app.left_width, 420.0);
    }

    #[test]
    fn sidebar_visibility_does_not_change_custom_width() {
        let mut app = FlokinApp::new();
        let _ = app.update(Message::SplitterPressed(SplitterKind::LeftSidebar, 0.0));
        let _ = app.update(Message::SplitterMoved(68.0, 0.0));
        let _ = app.update(Message::ToggleLeftSidebar);
        let _ = app.update(Message::ToggleLeftSidebar);
        assert!(app.left_visible);
        assert_eq!(app.left_width, 340.0);
    }

    #[test]
    fn app_modes_are_mutually_exclusive_and_reset_restores_layout() {
        let mut app = FlokinApp::new();
        let _ = app.update(Message::AppModeSelected(AppMode::Data));
        assert_eq!(app.mode, AppMode::Data);
        assert!(!app.model.sql_explorer.open);
        let _ = app.update(Message::AppModeSelected(AppMode::Sql));
        assert_eq!(app.mode, AppMode::Sql);
        assert!(app.model.sql_explorer.open);
        app.left_width = 360.0;
        let _ = app.update(Message::ResetLayout);
        assert_eq!(app.left_width, 272.0);
        assert!(app.left_visible && app.right_visible);
    }

    fn app_from_workspace(workspace: &TempWorkspace) -> FlokinApp {
        let mut app = FlokinApp::new();
        app.model
            .workspace_selected(Some(workspace.path().to_path_buf()));
        app.model
            .scan_completed(scan_workspace(workspace.path()).unwrap());
        app
    }

    fn explorer_file_id(app: &FlokinApp, path: &Path) -> flokin_core::ExplorerNodeId {
        fn find(
            nodes: &[flokin_core::ExplorerNode],
            path: &Path,
        ) -> Option<flokin_core::ExplorerNodeId> {
            for node in nodes {
                if node.path == path && matches!(node.kind, flokin_core::ExplorerNodeKind::File) {
                    return Some(node.id);
                }
                if let Some(id) = find(&node.children, path) {
                    return Some(id);
                }
            }
            None
        }

        find(&app.model.explorer, path).unwrap_or_else(|| panic!("missing explorer node {path:?}"))
    }

    struct TempWorkspace {
        path: PathBuf,
    }

    impl TempWorkspace {
        fn new() -> Self {
            let mut path = std::env::temp_dir();
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            path.push(format!("flokin-md-app-{}-{unique}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn write(&self, relative_path: &str, content: &str) {
            let path = self.path.join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, content).unwrap();
        }
    }

    impl Drop for TempWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
