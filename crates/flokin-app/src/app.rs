use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    time::{Duration, Instant},
};

use flokin_core::{
    apply_bulk_edit_plan, build_undo_plan, bulk_history_entry, clamp_graph_zoom, complete_sql,
    default_query, document_node_id, fit_graph_viewport, graph_bounds, graph_collections_map,
    initial_graph_layout, mock_shell, replace_sql_completion, save_markdown_file,
    sql_history_entry, undo_history_entry, workspace_identity, BulkEditApplyError, BulkEditPlan,
    GraphNodeId, GraphProjection, MutationHistoryEntry, MutationHistoryStore, ScanError,
    ShellModel, SqlCompletionItem, SqlExplorerMode, SqlWritePlan, WorkspaceEvent,
    DEFAULT_SQL_COMPLETION_LIMIT,
};
use iced::{
    advanced::widget::{self as advanced_widget, operate},
    application, event, keyboard,
    keyboard::{key::Named, Key},
    widget::{markdown, text_editor},
    window, Element, Size, Subscription, Task, Theme,
};

use crate::{
    i18n::{AppLanguage, I18nCatalog},
    message::{AppMode, MenuAction, Message, SplitterKind},
    services::{file_dialog, file_watcher, settings},
    theme::{self, AppTheme},
    views,
    views::graph::GraphViewState,
};

#[derive(Debug)]
pub struct FlokinApp {
    model: ShellModel,
    theme: AppTheme,
    language: AppLanguage,
    i18n: I18nCatalog,
    search_needs_refresh: bool,
    search_debounce_target: Option<Instant>,
    sql_editor: text_editor::Content,
    markdown_editors: HashMap<PathBuf, text_editor::Content>,
    empty_markdown_editor: text_editor::Content,
    markdown_previews: HashMap<PathBuf, MarkdownPreviewCache>,
    sql_completion: SqlCompletionPopup,
    graph: GraphViewState,
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
    schema_create_dialog_open: bool,
    schema_create_error: Option<String>,
    left_width: f32,
    inspector_width: f32,
    schema_width: f32,
    sql_editor_height: f32,
    splitter: Option<(SplitterKind, f32, f32)>,
    cursor: (f32, f32),
    left_visible: bool,
    right_visible: bool,
    mode: AppMode,
}

fn toggle_menu(
    open: Option<crate::message::MenuId>,
    menu: crate::message::MenuId,
) -> Option<crate::message::MenuId> {
    (open != Some(menu)).then_some(menu)
}

fn hover_menu(
    open: Option<crate::message::MenuId>,
    menu: crate::message::MenuId,
) -> Option<crate::message::MenuId> {
    open.map(|_| menu)
}

impl FlokinApp {
    fn new() -> Self {
        #[cfg(test)]
        let theme = AppTheme::Dark;
        #[cfg(not(test))]
        let theme = initial_theme();
        #[cfg(test)]
        let language = AppLanguage::PortugueseBrazil;
        #[cfg(not(test))]
        let language = initial_language();

        Self {
            model: mock_shell(),
            theme,
            language,
            i18n: I18nCatalog::new(language),
            search_needs_refresh: false,
            search_debounce_target: None,
            sql_editor: text_editor::Content::new(),
            markdown_editors: HashMap::new(),
            empty_markdown_editor: text_editor::Content::new(),
            markdown_previews: HashMap::new(),
            sql_completion: SqlCompletionPopup::closed(),
            graph: GraphViewState::default(),
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
            schema_create_dialog_open: false,
            schema_create_error: None,
            left_width: crate::theme::sizes::SIDEBAR_DEFAULT_WIDTH,
            inspector_width: crate::theme::sizes::INSPECTOR_DEFAULT_WIDTH,
            schema_width: crate::theme::sizes::SCHEMA_DEFAULT_WIDTH,
            sql_editor_height: crate::theme::sizes::SQL_EDITOR_DEFAULT_HEIGHT,
            splitter: None,
            cursor: (0.0, 0.0),
            left_visible: true,
            right_visible: true,
            mode: AppMode::Files,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AppModeSelected(mode) => {
                self.mode = mode;
                if mode == AppMode::Graph {
                    self.sync_graph_projection(true);
                }
                self.model.select_activity(match mode {
                    AppMode::Sql => flokin_core::Activity::Terminal,
                    AppMode::Settings => flokin_core::Activity::Settings,
                    AppMode::Graph => flokin_core::Activity::Relations,
                    AppMode::Health => flokin_core::Activity::Health,
                    AppMode::History => flokin_core::Activity::History,
                    AppMode::Files | AppMode::Data => flokin_core::Activity::Explorer,
                });
                if mode == AppMode::Sql {
                    self.model.open_sql_explorer();
                } else {
                    self.model.sql_explorer.open = false;
                    self.sql_completion.close();
                    if mode == AppMode::Files {
                        self.restore_active_document_selection();
                    }
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
                            self.sync_graph_projection(true);
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
                            self.model.mark_bulk_preview_stale_for_paths(&changed_paths);
                            self.model.mark_sql_preview_stale_for_paths(&changed_paths);
                            self.model.workspace_update_completed(update);
                            self.sync_graph_projection(false);
                            self.sync_markdown_editors_for_paths(&changed_paths);
                            self.sync_markdown_previews_for_paths(&changed_paths);
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
            Message::CollectionPanelSelected(panel) => {
                self.model.select_collection_panel(panel);
            }
            Message::SchemaFieldSelected {
                collection_id,
                field_name,
            } => {
                self.model.select_schema_field(collection_id, field_name);
            }
            Message::HealthFilterSelected(filter) => {
                self.model.select_health_filter(filter);
            }
            Message::HealthQueryChanged(query) => {
                self.model.update_health_query(query);
            }
            Message::HealthIssueSelected(issue_id) => {
                self.model.select_health_issue(issue_id);
            }
            Message::HealthIssueOpened(issue_id) => {
                if let Some(path) = self
                    .model
                    .health
                    .issues
                    .iter()
                    .find(|issue| issue.id == issue_id)
                    .and_then(|issue| issue.document_path.clone())
                {
                    self.open_or_activate_document(path);
                }
            }
            Message::SchemaCreateRequested => {
                self.schema_create_error = None;
                self.schema_create_dialog_open = true;
            }
            Message::SchemaCreateCanceled => {
                self.schema_create_dialog_open = false;
                self.schema_create_error = None;
            }
            Message::SchemaCreateConfirmed => {
                let Some(workspace) = self.model.current_workspace.clone() else {
                    self.schema_create_error = Some(self.i18n.tr("error-no-workspace-schema"));
                    return Task::none();
                };
                let generated =
                    match flokin_core::generate_explicit_schema(&self.model.schema_catalog) {
                        Ok(generated) => generated,
                        Err(flokin_core::SchemaGenerationError::Empty) => {
                            self.schema_create_error = Some(self.i18n.tr("error-schema-empty"));
                            return Task::none();
                        }
                        Err(flokin_core::SchemaGenerationError::Serialize(error)) => {
                            self.schema_create_error = Some(self.i18n.tr_with(
                                "error-schema-generate",
                                &[("error", error.to_string().into())],
                            ));
                            return Task::none();
                        }
                    };
                return create_schema_file_task(workspace, generated.yaml);
            }
            Message::SchemaCreateCompleted(result) => match result {
                Ok(path) => {
                    self.schema_create_dialog_open = false;
                    self.schema_create_error = None;
                    if let Some(workspace) = self.model.current_workspace.clone() {
                        return self.enqueue_workspace_events(
                            workspace,
                            vec![WorkspaceEvent::Upsert(path)],
                        );
                    }
                }
                Err(error) => {
                    self.schema_create_error = Some(error);
                    self.schema_create_dialog_open = true;
                }
            },
            Message::SchemaOpenRequested => match self.model.open_schema_tab() {
                Ok(true) => {
                    self.mode = AppMode::Files;
                    self.model.sql_explorer.open = false;
                    self.sql_completion.close();
                    self.ensure_markdown_editor_for_active();
                    self.ensure_markdown_preview_for_active();
                }
                Ok(false) => {}
                Err(error) => {
                    self.schema_create_error = Some(error);
                    self.schema_create_dialog_open = true;
                }
            },
            Message::TableHeaderSelected(column_id) => {
                self.model.toggle_collection_sort(column_id);
            }
            Message::BulkSelectionToggled(path) => {
                self.model.toggle_bulk_selection(path);
            }
            Message::BulkSelectAllVisible(select_all) => {
                self.model
                    .set_bulk_selection_for_current_collection(select_all);
            }
            Message::BulkSelectionCleared => {
                self.model.clear_bulk_selection();
            }
            Message::BulkEditOpened => {
                self.model.open_bulk_edit();
            }
            Message::BulkEditCanceled => {
                self.model.close_bulk_edit();
            }
            Message::BulkEditBackToConfigure => {
                self.model.return_to_bulk_configuration();
            }
            Message::BulkNewPropertyRequested => {
                self.model.set_bulk_property(String::new());
            }
            Message::BulkOperationSelected(kind) => {
                self.model.set_bulk_operation_kind(kind);
            }
            Message::BulkPropertySelected(property) => {
                self.model.set_bulk_property(property);
            }
            Message::BulkNewPropertyChanged(property) => {
                self.model.set_bulk_new_property(property);
            }
            Message::BulkValueTypeSelected(value_type) => {
                self.model.set_bulk_value_type(value_type);
            }
            Message::BulkValueChanged(value) => {
                self.model.set_bulk_value(value);
            }
            Message::BulkBoolValueSelected(value) => {
                self.model.set_bulk_bool_value(value);
            }
            Message::BulkPreviewRequested => {
                self.model.build_bulk_preview();
            }
            Message::BulkApplyRequested => {
                if self.model.bulk_edit.stale {
                    self.model.bulk_edit.error = Some(self.i18n.tr("error-stale-preview"));
                    return Task::none();
                }
                let Some(plan) = self.model.bulk_edit.plan.clone() else {
                    self.model.build_bulk_preview();
                    return Task::none();
                };
                if !plan.can_apply() {
                    return Task::none();
                }
                let Some(workspace) = self.model.current_workspace.clone() else {
                    return Task::none();
                };
                return apply_bulk_edit_task(workspace, plan);
            }
            Message::BulkApplyCompleted(result) => match result {
                Ok((paths, count, warning)) => {
                    self.model.bulk_apply_completed(Ok(count));
                    if let Some(warning) = warning {
                        self.model.bulk_edit.error = Some(warning);
                    }
                    if let Some(workspace) = self.model.current_workspace.clone() {
                        return Task::batch([
                            self.load_history_task(workspace.clone()),
                            self.enqueue_workspace_events(
                                workspace,
                                paths.into_iter().map(WorkspaceEvent::Upsert).collect(),
                            ),
                        ]);
                    }
                }
                Err(error) => {
                    self.model.bulk_apply_completed(Err(error));
                }
            },
            Message::MarkdownSelected(path) => {
                self.open_or_activate_document(path);
            }
            Message::GraphFitRequested => {
                self.fit_graph();
            }
            Message::GraphFocusSelected => {
                self.focus_selected_graph_node();
            }
            Message::GraphZoomIn => {
                self.zoom_graph_by(0.18);
            }
            Message::GraphZoomOut => {
                self.zoom_graph_by(-0.18);
            }
            Message::GraphZoomReset => {
                self.reset_graph_zoom();
            }
            Message::GraphViewportChanged(width, height) => {
                self.graph.viewport = Size::new(width, height);
            }
            Message::GraphNodeSelected(node) => {
                if let GraphNodeId::Document(path) = node {
                    self.model.select_document_without_opening(path);
                }
            }
            Message::GraphNodeOpened(node) => {
                if let GraphNodeId::Document(path) = node {
                    self.open_or_activate_document(path);
                }
            }
            Message::GraphPanBy(dx, dy) => {
                self.graph.pan = iced::Vector::new(self.graph.pan.x + dx, self.graph.pan.y + dy);
            }
            Message::GraphZoomAt { x, y, delta } => {
                self.zoom_graph_at(x, y, delta);
            }
            Message::GraphNodeDragged { node, dx, dy } => {
                if let Some(position) = self.graph.positions.get_mut(&node) {
                    position.x += dx;
                    position.y += dy;
                }
            }
            Message::EditorTabSelected(path) => {
                if self.model.activate_editor_tab(path) {
                    self.ensure_markdown_editor_for_active();
                    self.ensure_markdown_preview_for_active();
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
                    self.ensure_markdown_preview_for_path(&path);
                }
            }
            Message::EditorViewModeSelected(mode) => {
                if self.model.set_active_editor_view_mode(mode) {
                    self.ensure_markdown_preview_for_active();
                }
            }
            Message::MarkdownLinkClicked(_uri) => {}
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
                    self.ensure_markdown_preview_for_active();
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
            Message::WindowFocused(focused) => {
                if !focused {
                    self.clear_focus_transients();
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
            Message::SqlModeSelected(mode) => {
                self.model.set_sql_mode(mode);
            }
            Message::SqlExecute => {
                self.sql_completion.close();
                self.model.update_sql_query(self.sql_editor.text());
                self.model.sql_execution_started();
                return match self.model.sql_explorer.mode {
                    SqlExplorerMode::Query => execute_sql_task(
                        self.model.documents.clone(),
                        self.model.collections.clone(),
                        self.model.sql_explorer.query.clone(),
                    ),
                    SqlExplorerMode::Update => preview_sql_update_task(
                        self.model.documents.clone(),
                        self.model.collections.clone(),
                        self.model.editor.clone(),
                        self.model.schema_catalog.clone(),
                        self.model.sql_explorer.query.clone(),
                    ),
                };
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
            Message::SqlUpdatePreviewCompleted(result) => {
                self.model.sql_update_preview_completed(result);
            }
            Message::SqlUpdateBackToEditor => {
                self.model.sql_explorer.write_plan = None;
                self.model.sql_explorer.error = None;
                self.model.sql_explorer.stale = false;
            }
            Message::SqlUpdatePreviewCanceled => {
                self.model.sql_explorer.write_plan = None;
                self.model.sql_explorer.error = None;
                self.model.sql_explorer.stale = false;
            }
            Message::SqlUpdateApplyRequested => {
                if self.model.sql_explorer.stale {
                    self.model.sql_explorer.error = Some(self.i18n.tr("error-stale-preview"));
                    return Task::none();
                }
                let Some(plan) = self.model.sql_explorer.write_plan.clone() else {
                    return Task::none();
                };
                if !plan.mutation_plan.can_apply() {
                    return Task::none();
                }
                let Some(workspace) = self.model.current_workspace.clone() else {
                    return Task::none();
                };
                return apply_sql_update_task(workspace, plan);
            }
            Message::SqlUpdateApplyCompleted(result) => match result {
                Ok((paths, count, warning)) => {
                    self.model.sql_update_apply_completed(Ok(count));
                    if let Some(warning) = warning {
                        self.model.sql_explorer.error = Some(warning);
                    }
                    if let Some(workspace) = self.model.current_workspace.clone() {
                        return Task::batch([
                            self.load_history_task(workspace.clone()),
                            self.enqueue_workspace_events(
                                workspace,
                                paths.into_iter().map(WorkspaceEvent::Upsert).collect(),
                            ),
                        ]);
                    }
                }
                Err(error) => {
                    self.model.sql_update_apply_completed(Err(error));
                }
            },
            Message::HistoryLoaded(workspace, result) => {
                if self.model.current_workspace.as_ref() == Some(&workspace) {
                    self.model.history_loaded(result);
                }
            }
            Message::HistoryEntrySelected(id) => {
                self.model.select_history_entry(id);
            }
            Message::HistoryUndoRequested => {
                let Some(entry) = self.model.selected_history_entry().cloned() else {
                    return Task::none();
                };
                if !self.model.history.is_entry_undoable(&entry) {
                    return Task::none();
                }
                let Some(workspace) = self.model.current_workspace.clone() else {
                    return Task::none();
                };
                let result = build_undo_plan(&workspace, &entry, &self.model.editor)
                    .map_err(|error| error.to_string());
                self.model.undo_preview_completed(result);
            }
            Message::HistoryUndoPreviewCanceled => {
                self.model.cancel_undo_preview();
            }
            Message::HistoryUndoApplyRequested => {
                let Some(plan) = self.model.history.undo_plan.clone() else {
                    return Task::none();
                };
                if !plan.can_apply() {
                    return Task::none();
                }
                let Some(workspace) = self.model.current_workspace.clone() else {
                    return Task::none();
                };
                let Some(entry) = self.model.selected_history_entry().cloned() else {
                    return Task::none();
                };
                if !self.model.history.is_entry_undoable(&entry) {
                    return Task::none();
                }
                return apply_history_undo_task(workspace, entry, plan);
            }
            Message::HistoryUndoApplyCompleted(result) => match result {
                Ok((paths, count, warning)) => {
                    self.model.undo_apply_completed(Ok(count));
                    if let Some(warning) = warning {
                        self.model.history.error = Some(warning);
                    }
                    if let Some(workspace) = self.model.current_workspace.clone() {
                        return Task::batch([
                            self.load_history_task(workspace.clone()),
                            self.enqueue_workspace_events(
                                workspace,
                                paths.into_iter().map(WorkspaceEvent::Upsert).collect(),
                            ),
                        ]);
                    }
                }
                Err(error) => {
                    self.model.undo_apply_completed(Err(error));
                }
            },
            Message::HistoryClearRequested => {
                self.model.request_clear_history();
            }
            Message::HistoryClearCanceled => {
                self.model.cancel_clear_history();
            }
            Message::HistoryClearConfirmed => {
                let Some(workspace) = self.model.current_workspace.clone() else {
                    self.model.clear_history_completed(Err(self
                        .i18n
                        .tr("error-history-clear-no-workspace")));
                    return Task::none();
                };
                return clear_history_task(workspace);
            }
            Message::HistoryClearCompleted(result) => {
                self.model.clear_history_completed(result);
            }
            Message::KeyboardEvent(event) => {
                if let Some(message) = keyboard_message(
                    event,
                    self.model.search.open,
                    self.open_menu.is_some(),
                    self.model.bulk_edit.editor_open,
                ) {
                    return self.update(message);
                }
            }
            Message::ThemeToggled => {
                self.theme = self.theme.toggled();
                return persist_theme_task(self.theme);
            }
            Message::ThemeSelected(light) => {
                self.theme = if light {
                    AppTheme::Light
                } else {
                    AppTheme::Dark
                };
                return persist_theme_task(self.theme);
            }
            Message::ThemePersisted(_result) => {}
            Message::LanguageSelected(language) => {
                self.language = language;
                self.i18n = I18nCatalog::new(language);
                return persist_language_task(language);
            }
            Message::LanguagePersisted(_result) => {}
            Message::MenuToggled(menu) => self.open_menu = toggle_menu(self.open_menu, menu),
            Message::MenuHovered(menu) => self.open_menu = hover_menu(self.open_menu, menu),
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
                    MenuAction::Graph => {
                        return self.update(Message::AppModeSelected(AppMode::Graph))
                    }
                    MenuAction::Health => {
                        return self.update(Message::AppModeSelected(AppMode::Health))
                    }
                    MenuAction::SqlExplorer => {
                        return self.update(Message::AppModeSelected(AppMode::Sql))
                    }
                    MenuAction::History => {
                        return self.update(Message::AppModeSelected(AppMode::History))
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
                    SplitterKind::MarkdownPreview => self
                        .model
                        .editor
                        .active_tab()
                        .map(|tab| f32::from(tab.split_ratio) / 1000.0)
                        .unwrap_or(0.5),
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
                    } else if kind == SplitterKind::MarkdownPreview {
                        (x - origin) / 1000.0
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
                        SplitterKind::MarkdownPreview => {
                            self.model.set_active_editor_split_ratio(initial + delta);
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
                self.reset_graph_layout();
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
            self.active_markdown_preview_items(),
            &self.sql_completion.items,
            &self.graph,
            self.sql_completion.selected,
            self.sql_completion.open,
            self.left_width,
            self.inspector_width,
            self.schema_width,
            self.sql_editor_height,
            self.open_menu,
            self.about_open,
            self.schema_create_dialog_open,
            self.schema_create_error.as_deref(),
            self.left_visible,
            self.right_visible,
            self.mode,
            &self.i18n,
            self.language,
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            file_watcher::subscription(self.model.current_workspace.clone()),
            window::close_requests().map(Message::WindowCloseRequested),
            keyboard::listen().map(Message::KeyboardEvent),
            event::listen_with(|event, _status, _window| match event {
                iced::Event::Window(window::Event::Focused) => Some(Message::WindowFocused(true)),
                iced::Event::Window(window::Event::Unfocused) => {
                    Some(Message::WindowFocused(false))
                }
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

    fn clear_focus_transients(&mut self) {
        self.open_menu = None;
        self.splitter = None;
        self.sql_completion.close();
        self.model.close_search();
        self.search_needs_refresh = false;
        self.search_debounce_target = None;
    }
}

#[derive(Debug, Clone)]
struct MarkdownPreviewCache {
    source: String,
    items: Vec<markdown::Item>,
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
        self.ensure_markdown_preview_for_active();
    }

    fn restore_active_document_selection(&mut self) {
        let Some(path) = self.model.editor.active_path.clone().or_else(|| {
            self.model
                .editor
                .tabs
                .iter()
                .find(|tab| tab.kind == flokin_core::EditorTabKind::Markdown)
                .map(|tab| tab.document_path.clone())
        }) else {
            return;
        };

        if self.model.activate_editor_tab(path) {
            self.ensure_markdown_editor_for_active();
            self.ensure_markdown_preview_for_active();
        }
    }

    fn switch_workspace(&mut self, path: std::path::PathBuf) -> Task<Message> {
        self.workspace_generation = self.workspace_generation.wrapping_add(1);
        let generation = self.workspace_generation;
        self.model.workspace_selected(Some(path.clone()));
        self.sql_editor = text_editor::Content::new();
        self.markdown_editors.clear();
        self.markdown_previews.clear();
        self.empty_markdown_editor = text_editor::Content::new();
        self.sql_completion.close();
        self.workspace_update_running = false;
        self.pending_workspace_events.clear();
        self.close_window_after_dialog = None;
        self.pending_window_save = None;
        self.pending_workspace_switch = None;
        self.pending_workspace_save = None;
        self.pending_reindex = false;
        self.schema_create_dialog_open = false;
        self.schema_create_error = None;
        Task::batch([
            scan_workspace_task(generation, path.clone()),
            self.load_history_task(path),
        ])
    }

    fn load_history_task(&self, workspace: std::path::PathBuf) -> Task<Message> {
        load_history_task(workspace)
    }

    fn active_markdown_preview_items(&self) -> &[markdown::Item] {
        let Some(path) = self.model.editor.active_path.as_ref() else {
            return &[];
        };
        self.markdown_previews
            .get(path)
            .map(|cache| cache.items.as_slice())
            .unwrap_or(&[])
    }

    fn ensure_markdown_preview_for_active(&mut self) {
        let Some(path) = self.model.editor.active_path.clone() else {
            return;
        };
        self.ensure_markdown_preview_for_path(&path);
    }

    fn ensure_markdown_preview_for_path(&mut self, path: &std::path::Path) {
        let Some(tab) = self.model.editor.tab(path) else {
            return;
        };
        let source = flokin_core::markdown_body_without_frontmatter(&tab.buffer).to_owned();
        if self
            .markdown_previews
            .get(path)
            .is_some_and(|cache| cache.source == source)
        {
            return;
        }
        let items = markdown::parse(&source).collect();
        self.markdown_previews
            .insert(path.to_path_buf(), MarkdownPreviewCache { source, items });
    }

    fn sync_graph_projection(&mut self, keep_positions: bool) {
        let collections = graph_collections_map(&self.model.collections);
        let projection = GraphProjection::build_with_collections(
            &self.model.documents,
            &collections,
            &self.model.relation_index,
        );
        let mut positions = if keep_positions {
            self.graph.positions.clone()
        } else {
            BTreeMap::new()
        };
        let layout = initial_graph_layout(&projection);
        positions.retain(|node, _| projection.nodes.iter().any(|current| &current.id == node));
        for (node, position) in layout {
            positions.entry(node).or_insert(position);
        }
        self.graph.projection = projection;
        self.graph.positions = positions;
        if self.graph.viewport.width > 0.0 && self.graph.viewport.height > 0.0 {
            self.fit_graph();
        }
    }

    fn reset_graph_layout(&mut self) {
        self.graph.positions = initial_graph_layout(&self.graph.projection);
        self.fit_graph();
    }

    fn fit_graph(&mut self) {
        let viewport_width = if self.graph.viewport.width > 0.0 {
            self.graph.viewport.width
        } else {
            900.0
        };
        let viewport_height = if self.graph.viewport.height > 0.0 {
            self.graph.viewport.height
        } else {
            600.0
        };
        let bounds = graph_bounds(
            &self.graph.positions,
            crate::theme::sizes::GRAPH_NODE_WIDTH,
            crate::theme::sizes::GRAPH_NODE_HEIGHT,
        );
        let viewport = fit_graph_viewport(bounds, viewport_width, viewport_height, 72.0);
        self.graph.pan = iced::Vector::new(viewport.pan_x, viewport.pan_y);
        self.graph.zoom = viewport.zoom;
    }

    fn zoom_graph_by(&mut self, delta: f32) {
        let x = graph_viewport_width(self.graph.viewport) / 2.0;
        let y = graph_viewport_height(self.graph.viewport) / 2.0;
        self.zoom_graph_at(x, y, delta);
    }

    fn zoom_graph_at(&mut self, x: f32, y: f32, delta: f32) {
        let old_zoom = self.graph.zoom;
        let new_zoom = clamp_graph_zoom(old_zoom * (1.0 + delta));
        if (new_zoom - old_zoom).abs() <= f32::EPSILON {
            return;
        }
        let world_x = (x - self.graph.pan.x) / old_zoom;
        let world_y = (y - self.graph.pan.y) / old_zoom;
        self.graph.zoom = new_zoom;
        self.graph.pan = iced::Vector::new(x - world_x * new_zoom, y - world_y * new_zoom);
    }

    fn reset_graph_zoom(&mut self) {
        let center_x = graph_viewport_width(self.graph.viewport) / 2.0;
        let center_y = graph_viewport_height(self.graph.viewport) / 2.0;
        let old_zoom = self.graph.zoom;
        if (old_zoom - 1.0).abs() <= f32::EPSILON {
            return;
        }
        let world_x = (center_x - self.graph.pan.x) / old_zoom;
        let world_y = (center_y - self.graph.pan.y) / old_zoom;
        self.graph.zoom = 1.0;
        self.graph.pan = iced::Vector::new(center_x - world_x, center_y - world_y);
    }

    fn focus_selected_graph_node(&mut self) {
        let Some(path) = self.model.selected_document_path.as_ref() else {
            return;
        };
        let Some(position) = self.graph.positions.get(&document_node_id(path)) else {
            return;
        };
        let viewport_width = graph_viewport_width(self.graph.viewport);
        let viewport_height = graph_viewport_height(self.graph.viewport);
        self.graph.pan = iced::Vector::new(
            viewport_width / 2.0
                - (position.x + crate::theme::sizes::GRAPH_NODE_WIDTH / 2.0) * self.graph.zoom,
            viewport_height / 2.0
                - (position.y + crate::theme::sizes::GRAPH_NODE_HEIGHT / 2.0) * self.graph.zoom,
        );
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

    fn sync_markdown_previews_for_paths(&mut self, paths: &[std::path::PathBuf]) {
        for path in paths {
            if self.model.editor.tab(path).is_some() {
                self.ensure_markdown_preview_for_path(path);
            }
        }
    }

    fn cleanup_markdown_editors(&mut self) {
        self.markdown_editors
            .retain(|path, _| self.model.editor.tab(path).is_some());
        self.markdown_previews
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

fn graph_viewport_width(viewport: Size) -> f32 {
    if viewport.width > 0.0 {
        viewport.width
    } else {
        900.0
    }
}

fn graph_viewport_height(viewport: Size) -> f32 {
    if viewport.height > 0.0 {
        viewport.height
    } else {
        600.0
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

fn create_schema_file_task(workspace: std::path::PathBuf, content: String) -> Task<Message> {
    Task::perform(
        async move { create_schema_file_if_absent(&workspace, &content) },
        Message::SchemaCreateCompleted,
    )
}

fn apply_bulk_edit_task(workspace: std::path::PathBuf, plan: BulkEditPlan) -> Task<Message> {
    Task::perform(
        async move {
            apply_bulk_edit_plan(&plan)
                .map(|result| {
                    let count = result.changed_paths.len();
                    let warning = record_history(bulk_history_entry(
                        workspace_identity(&workspace),
                        &plan,
                        &result.changed_paths,
                    ));
                    (result.changed_paths, count, warning)
                })
                .map_err(format_bulk_apply_error)
        },
        Message::BulkApplyCompleted,
    )
}

fn apply_history_undo_task(
    workspace: std::path::PathBuf,
    original: MutationHistoryEntry,
    plan: BulkEditPlan,
) -> Task<Message> {
    Task::perform(
        async move {
            apply_bulk_edit_plan(&plan)
                .map(|result| {
                    let count = result.changed_paths.len();
                    let changed = result
                        .changed_paths
                        .iter()
                        .collect::<std::collections::BTreeSet<_>>();
                    let changed_relative_paths = plan
                        .changes
                        .iter()
                        .filter(|change| changed.contains(&change.path))
                        .map(|change| change.relative_path.clone())
                        .collect::<Vec<_>>();
                    let warning = record_history(undo_history_entry(
                        workspace_identity(&workspace),
                        &original,
                        &changed_relative_paths,
                    ));
                    (result.changed_paths, count, warning)
                })
                .map_err(format_bulk_apply_error)
        },
        Message::HistoryUndoApplyCompleted,
    )
}

fn format_bulk_apply_error(error: BulkEditApplyError) -> String {
    match error {
        BulkEditApplyError::StalePreview { .. } => String::from(
            "O workspace mudou desde a geração do preview. Revise as alterações novamente.",
        ),
        BulkEditApplyError::Preflight { path, message } => {
            format!("Preflight falhou para {}: {message}", path.display())
        }
        BulkEditApplyError::Stage { path, message } => {
            format!("Não foi possível preparar {}: {message}", path.display())
        }
        BulkEditApplyError::Commit {
            path,
            message,
            rollback_failed,
        } => {
            if rollback_failed.is_empty() {
                format!("Bulk edit falhou ao substituir {}: {message}. Arquivos já alterados foram restaurados.", path.display())
            } else {
                let paths = rollback_failed
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "Bulk edit falhou e {} arquivo(s) não puderam ser restaurados: {paths}",
                    rollback_failed.len()
                )
            }
        }
    }
}

fn create_schema_file_if_absent(
    workspace: &std::path::Path,
    content: &str,
) -> Result<std::path::PathBuf, String> {
    let path = flokin_core::schema_path(workspace);
    if path.exists() {
        return Err(String::from(
            "Já existe um flokin.schema.yaml neste workspace.",
        ));
    }
    save_markdown_file(&path, content).map_err(|error| {
        format!(
            "Não foi possível criar {}: {error}",
            flokin_core::SCHEMA_FILE_NAME
        )
    })?;
    Ok(path)
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

fn preview_sql_update_task(
    documents: Vec<flokin_core::Document>,
    collections: Vec<flokin_core::Collection>,
    editor: flokin_core::EditorState,
    schema_catalog: flokin_core::SchemaCatalog,
    query: String,
) -> Task<Message> {
    Task::perform(
        async move {
            flokin_core::SqlProjection::preview_update(
                &query,
                &documents,
                &collections,
                &editor,
                &schema_catalog,
            )
        },
        Message::SqlUpdatePreviewCompleted,
    )
}

fn apply_sql_update_task(workspace: std::path::PathBuf, plan: SqlWritePlan) -> Task<Message> {
    Task::perform(
        async move {
            apply_bulk_edit_plan(&plan.mutation_plan)
                .map(|result| {
                    let count = result.changed_paths.len();
                    let warning = record_history(sql_history_entry(
                        workspace_identity(&workspace),
                        &plan,
                        &result.changed_paths,
                    ));
                    (result.changed_paths, count, warning)
                })
                .map_err(format_bulk_apply_error)
        },
        Message::SqlUpdateApplyCompleted,
    )
}

fn load_history_task(workspace: std::path::PathBuf) -> Task<Message> {
    Task::perform(
        async move {
            let workspace_id = workspace_identity(&workspace);
            let result = MutationHistoryStore::open(history_storage_path())
                .and_then(|store| store.load_workspace(&workspace_id));
            (workspace, result)
        },
        |(workspace, result)| Message::HistoryLoaded(workspace, result),
    )
}

fn clear_history_task(workspace: std::path::PathBuf) -> Task<Message> {
    Task::perform(
        async move {
            let workspace_id = workspace_identity(&workspace);
            let mut store = MutationHistoryStore::open(history_storage_path())?;
            store.clear_workspace(&workspace_id)
        },
        Message::HistoryClearCompleted,
    )
}

fn record_history(entry: Result<MutationHistoryEntry, String>) -> Option<String> {
    let entry = match entry {
        Ok(entry) => entry,
        Err(error) => {
            return Some(format!(
                "Alterações aplicadas, mas não foi possível registrar o histórico: {error}"
            ));
        }
    };
    let mut store = match MutationHistoryStore::open(history_storage_path()) {
        Ok(store) => store,
        Err(error) => {
            return Some(format!(
                "Alterações aplicadas, mas não foi possível registrar o histórico: {error}"
            ));
        }
    };
    store.save_entry(&entry).err().map(|error| {
        format!("Alterações aplicadas, mas não foi possível registrar o histórico: {error}")
    })
}

fn history_storage_path() -> std::path::PathBuf {
    app_data_dir().join("history.sqlite3")
}

fn settings_storage_path() -> std::path::PathBuf {
    settings::settings_path(&app_data_dir())
}

#[cfg(not(test))]
fn initial_theme() -> AppTheme {
    theme_from_settings_path(&settings_storage_path())
}

fn theme_from_settings_path(path: &std::path::Path) -> AppTheme {
    settings::load_theme(path).unwrap_or(AppTheme::Dark)
}

#[cfg(not(test))]
fn initial_language() -> AppLanguage {
    language_from_settings_path(&settings_storage_path())
}

fn language_from_settings_path(path: &std::path::Path) -> AppLanguage {
    match settings::load_language(path) {
        settings::LanguageLoad::Language(language) => language,
        settings::LanguageLoad::MissingLanguage => AppLanguage::PortugueseBrazil,
        settings::LanguageLoad::MissingSettings | settings::LanguageLoad::Invalid => {
            AppLanguage::from_os_locale(sys_locale::get_locale().as_deref())
        }
    }
}

fn persist_theme_task(theme: AppTheme) -> Task<Message> {
    Task::perform(
        async move { settings::save_theme(&settings_storage_path(), theme) },
        Message::ThemePersisted,
    )
}

fn persist_language_task(language: AppLanguage) -> Task<Message> {
    Task::perform(
        async move { settings::save_language(&settings_storage_path(), language) },
        Message::LanguagePersisted,
    )
}

fn app_data_dir() -> std::path::PathBuf {
    if let Some(value) = std::env::var_os("FLOKINMD_APP_DATA") {
        return std::path::PathBuf::from(value);
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(value) = std::env::var_os("APPDATA") {
            return std::path::PathBuf::from(value).join("FlokinMD");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(value) = std::env::var_os("HOME") {
            return std::path::PathBuf::from(value)
                .join("Library")
                .join("Application Support")
                .join("FlokinMD");
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(value) = std::env::var_os("XDG_DATA_HOME") {
            return std::path::PathBuf::from(value).join("flokinmd");
        }
        if let Some(value) = std::env::var_os("HOME") {
            return std::path::PathBuf::from(value)
                .join(".local")
                .join("share")
                .join("flokinmd");
        }
    }
    std::env::temp_dir().join("flokinmd")
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

fn keyboard_message(
    event: keyboard::Event,
    search_open: bool,
    menu_open: bool,
    bulk_open: bool,
) -> Option<Message> {
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

    if bulk_open && matches!(key, Key::Named(Named::Escape)) {
        return Some(Message::BulkEditCanceled);
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
    use flokin_core::{
        scan_workspace, workspace_update_from_events, Activity, ScanResult, ScanState,
        SqlCompletionItem, SqlCompletionKind, SqlError, WorkspaceEvent,
    };
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
        time::{Instant, SystemTime, UNIX_EPOCH},
    };

    use super::{
        create_schema_file_if_absent, hover_menu, keyboard_message, language_from_settings_path,
        theme_from_settings_path, toggle_menu, FlokinApp,
    };
    use crate::{
        i18n::AppLanguage,
        message::{AppMode, MenuId, Message, SplitterKind},
        services::{file_watcher::WatcherMessage, settings},
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
    fn menu_clicks_have_single_toggle_state() {
        assert_eq!(toggle_menu(None, MenuId::File), Some(MenuId::File));
        assert_eq!(toggle_menu(Some(MenuId::File), MenuId::File), None);
        assert_eq!(
            toggle_menu(Some(MenuId::File), MenuId::View),
            Some(MenuId::View)
        );
    }

    #[test]
    fn menu_hover_only_switches_an_open_menu() {
        assert_eq!(hover_menu(None, MenuId::View), None);
        assert_eq!(
            hover_menu(Some(MenuId::File), MenuId::View),
            Some(MenuId::View)
        );
    }

    #[test]
    fn focus_lost_clears_only_transient_input_state() {
        let mut app = FlokinApp::new();
        app.open_menu = Some(MenuId::File);
        app.model.open_search();
        app.search_needs_refresh = true;
        app.search_debounce_target = Some(Instant::now());
        app.splitter = Some((SplitterKind::LeftSidebar, 100.0, app.left_width));
        app.sql_completion.set_items(vec![SqlCompletionItem {
            label: String::from("projects"),
            insert_text: String::from("projects"),
            kind: SqlCompletionKind::Table,
            detail: String::from("table"),
            replacement_start: 0,
            replacement_end: 0,
        }]);
        app.left_width = 360.0;
        app.mode = AppMode::Data;

        let _ = app.update(Message::WindowFocused(false));

        assert_eq!(app.open_menu, None);
        assert!(!app.model.search.open);
        assert!(!app.search_needs_refresh);
        assert_eq!(app.search_debounce_target, None);
        assert_eq!(app.splitter, None);
        assert!(!app.sql_completion.open);
        assert_eq!(app.left_width, 360.0);
        assert_eq!(app.mode, AppMode::Data);
    }

    #[test]
    fn focus_gained_does_not_reset_interactive_state() {
        let mut app = FlokinApp::new();
        app.open_menu = Some(MenuId::Data);
        app.model.open_search();
        app.splitter = Some((SplitterKind::Inspector, 800.0, app.inspector_width));
        app.left_width = 340.0;
        app.mode = AppMode::Graph;

        let _ = app.update(Message::WindowFocused(true));

        assert_eq!(app.open_menu, Some(MenuId::Data));
        assert!(app.model.search.open);
        assert!(app.splitter.is_some());
        assert_eq!(app.left_width, 340.0);
        assert_eq!(app.mode, AppMode::Graph);
    }

    #[test]
    fn input_messages_still_apply_after_focus_cycle() {
        let mut app = FlokinApp::new();
        let _ = app.update(Message::WindowFocused(false));
        let _ = app.update(Message::WindowFocused(true));

        let _ = app.update(Message::MenuToggled(MenuId::File));
        assert_eq!(app.open_menu, Some(MenuId::File));

        let event = Event::KeyPressed {
            key: Key::Character("k".into()),
            modified_key: Key::Character("k".into()),
            physical_key: Physical::Code(Code::KeyK),
            location: Location::Standard,
            modifiers: Modifiers::CTRL,
            text: None,
            repeat: false,
        };
        let _ = app.update(Message::KeyboardEvent(event));

        assert!(app.model.search.open);
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
    fn saved_theme_is_used_for_startup_theme() {
        let workspace = TempWorkspace::new();
        let path = settings::settings_path(workspace.path());
        settings::save_theme(&path, AppTheme::Light).unwrap();

        assert_eq!(theme_from_settings_path(&path), AppTheme::Light);
    }

    #[test]
    fn saved_language_is_used_for_startup_language() {
        let workspace = TempWorkspace::new();
        let path = settings::settings_path(workspace.path());
        settings::save_language(&path, AppLanguage::English).unwrap();

        assert_eq!(language_from_settings_path(&path), AppLanguage::English);
    }

    #[test]
    fn existing_settings_without_language_migrate_to_portuguese() {
        let workspace = TempWorkspace::new();
        let path = settings::settings_path(workspace.path());
        fs::write(&path, "version=1\ntheme=dark\n").unwrap();

        assert_eq!(
            language_from_settings_path(&path),
            AppLanguage::PortugueseBrazil
        );
    }

    #[test]
    fn runtime_language_switch_preserves_workspace_dirty_tab_and_sql_text() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "# A\n");
        let mut app = app_from_workspace(&workspace);
        let path = workspace.path().join("a.md");
        app.open_or_activate_document(path.clone());
        app.model
            .update_active_editor_buffer(String::from("# A dirty\n"));
        app.sql_editor = text_editor::Content::with_text("SELECT * FROM projects;");
        app.model
            .update_sql_query(String::from("SELECT * FROM projects;"));
        let workspace_before = app.model.current_workspace.clone();

        let _ = app.update(Message::LanguageSelected(AppLanguage::English));

        assert_eq!(app.language, AppLanguage::English);
        assert_eq!(app.model.current_workspace, workspace_before);
        assert_eq!(app.model.editor.active_path, Some(path.clone()));
        assert!(app.model.editor.tab(&path).unwrap().dirty);
        assert_eq!(app.sql_editor.text(), "SELECT * FROM projects;");
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

        assert_eq!(keyboard_message(event.clone(), false, false, false), None);
        assert_eq!(
            keyboard_message(event, true, false, false),
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
            keyboard_message(event.clone(), false, true, false),
            Some(Message::MenuClosed)
        );
        assert_eq!(
            keyboard_message(event, false, false, true),
            Some(Message::BulkEditCanceled)
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
        assert_eq!(
            app.left_width,
            crate::theme::sizes::SIDEBAR_DEFAULT_WIDTH + 68.0
        );
    }

    #[test]
    fn app_modes_are_mutually_exclusive_and_reset_restores_layout() {
        let mut app = FlokinApp::new();
        let _ = app.update(Message::AppModeSelected(AppMode::Data));
        assert_eq!(app.mode, AppMode::Data);
        assert!(!app.model.sql_explorer.open);
        let _ = app.update(Message::AppModeSelected(AppMode::Graph));
        assert_eq!(app.mode, AppMode::Graph);
        assert!(!app.model.sql_explorer.open);
        let _ = app.update(Message::AppModeSelected(AppMode::Health));
        assert_eq!(app.mode, AppMode::Health);
        assert_eq!(app.model.active_activity, flokin_core::Activity::Health);
        assert!(!app.model.sql_explorer.open);
        let _ = app.update(Message::AppModeSelected(AppMode::Sql));
        assert_eq!(app.mode, AppMode::Sql);
        assert!(app.model.sql_explorer.open);
        app.left_width = 360.0;
        let _ = app.update(Message::ResetLayout);
        assert_eq!(app.left_width, crate::theme::sizes::SIDEBAR_DEFAULT_WIDTH);
        assert!(app.left_visible && app.right_visible);
    }

    #[test]
    fn returning_to_files_restores_active_document_context() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "carf-daily.md",
            "---\ntitle: CARF Daily\n---\n# CARF Daily\n",
        );
        let mut app = app_from_workspace(&workspace);
        let path = workspace.path().join("carf-daily.md");

        let _ = app.update(Message::MarkdownSelected(path.clone()));
        assert_eq!(app.model.editor.active_path.as_ref(), Some(&path));
        assert_eq!(app.model.selected_document_path.as_ref(), Some(&path));

        let _ = app.update(Message::AppModeSelected(AppMode::Sql));
        assert!(app.model.sql_explorer.open);
        assert!(app.model.editor.active_path.is_none());
        assert!(app.model.selected_document_path.is_none());

        let _ = app.update(Message::AppModeSelected(AppMode::Files));
        assert_eq!(app.model.editor.active_path.as_ref(), Some(&path));
        assert_eq!(app.model.selected_document_path.as_ref(), Some(&path));
        assert!(app.markdown_editors.contains_key(&path));
        assert!(app.markdown_previews.contains_key(&path));
    }

    #[test]
    fn explicit_schema_creation_writes_once_and_never_overwrites() {
        let workspace = TempWorkspace::new();
        let content = "version: 1\ncollections: {}\n";

        let path = create_schema_file_if_absent(workspace.path(), content).unwrap();

        assert_eq!(path, workspace.path().join(flokin_core::SCHEMA_FILE_NAME));
        assert_eq!(fs::read_to_string(&path).unwrap(), content);

        let error = create_schema_file_if_absent(workspace.path(), "version: 1\ncollections: []\n")
            .unwrap_err();

        assert!(error.contains("Já existe um flokin.schema.yaml"));
        assert_eq!(fs::read_to_string(&path).unwrap(), content);
    }

    #[test]
    fn graph_single_click_selects_document_without_opening_tab() {
        let workspace = TempWorkspace::new();
        workspace.write("carf.md", "---\ntitle: CARF\n---\n");
        let mut app = app_from_workspace(&workspace);
        let path = workspace.path().join("carf.md");

        let _ = app.update(Message::AppModeSelected(AppMode::Graph));
        let _ = app.update(Message::GraphNodeSelected(flokin_core::document_node_id(
            &path,
        )));

        assert_eq!(app.mode, AppMode::Graph);
        assert_eq!(app.model.selected_document_path, Some(path));
        assert!(app.model.editor.tabs.is_empty());
    }

    #[test]
    fn graph_double_click_reuses_document_opening_flow() {
        let workspace = TempWorkspace::new();
        workspace.write("daily.md", "---\ntitle: CARF Daily\n---\n# Daily\n");
        let mut app = app_from_workspace(&workspace);
        let path = workspace.path().join("daily.md");

        let _ = app.update(Message::AppModeSelected(AppMode::Graph));
        let _ = app.update(Message::GraphNodeOpened(flokin_core::document_node_id(
            &path,
        )));

        assert_eq!(app.mode, AppMode::Files);
        assert_eq!(app.model.editor.active_path, Some(path.clone()));
        assert_eq!(
            app.markdown_editors.get(&path).unwrap().text(),
            "---\ntitle: CARF Daily\n---\n# Daily\n"
        );
    }

    #[test]
    fn graph_mode_does_not_disturb_dirty_editor_buffers() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "A saved\n");
        let mut app = app_from_workspace(&workspace);
        let path = workspace.path().join("a.md");
        app.open_or_activate_document(path.clone());
        app.model
            .update_active_editor_buffer(String::from("A dirty\n"));
        app.markdown_editors
            .insert(path.clone(), text_editor::Content::with_text("A dirty\n"));

        let _ = app.update(Message::AppModeSelected(AppMode::Graph));
        let _ = app.update(Message::AppModeSelected(AppMode::Files));

        assert_eq!(app.model.editor.tab(&path).unwrap().buffer, "A dirty\n");
        assert!(app.model.editor.tab(&path).unwrap().dirty);
        assert_eq!(app.markdown_editors.get(&path).unwrap().text(), "A dirty\n");
    }

    #[test]
    fn markdown_preview_cache_uses_unsaved_live_buffer_and_strips_frontmatter() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "---\ntitle: A\n---\n# Saved\n");
        let mut app = app_from_workspace(&workspace);
        let path = workspace.path().join("a.md");
        app.open_or_activate_document(path.clone());

        app.model.update_active_editor_buffer(String::from(
            "---\ntitle: A\n---\n## Teste preview\n\n- item\n",
        ));
        app.ensure_markdown_preview_for_active();

        let cache = app.markdown_previews.get(&path).unwrap();
        assert_eq!(cache.source, "## Teste preview\n\n- item\n");
        assert!(app.model.editor.tab(&path).unwrap().dirty);
    }

    #[test]
    fn markdown_preview_cache_updates_for_clean_external_change() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "# A\n");
        let mut app = app_from_workspace(&workspace);
        let path = workspace.path().join("a.md");
        app.open_or_activate_document(path.clone());
        app.ensure_markdown_preview_for_active();

        workspace.write("a.md", "# A external\n");
        let update =
            workspace_update_from_events(workspace.path(), &[WorkspaceEvent::Upsert(path.clone())])
                .unwrap();
        app.model.workspace_update_completed(update);
        app.sync_markdown_editors_for_paths(std::slice::from_ref(&path));
        app.sync_markdown_previews_for_paths(std::slice::from_ref(&path));

        assert_eq!(
            app.markdown_previews.get(&path).unwrap().source,
            "# A external\n"
        );
        assert!(!app.model.editor.tab(&path).unwrap().dirty);
    }

    #[test]
    fn markdown_preview_cache_keeps_local_buffer_during_dirty_conflict() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "# A\n");
        let mut app = app_from_workspace(&workspace);
        let path = workspace.path().join("a.md");
        app.open_or_activate_document(path.clone());

        app.model
            .update_active_editor_buffer(String::from("# A local\n"));
        app.ensure_markdown_preview_for_active();
        workspace.write("a.md", "# A external\n");
        let update =
            workspace_update_from_events(workspace.path(), &[WorkspaceEvent::Upsert(path.clone())])
                .unwrap();
        app.model.workspace_update_completed(update);
        app.sync_markdown_previews_for_paths(std::slice::from_ref(&path));

        assert_eq!(
            app.markdown_previews.get(&path).unwrap().source,
            "# A local\n"
        );
        assert!(app
            .model
            .editor
            .tab(&path)
            .unwrap()
            .external_conflict
            .is_some());
    }

    #[test]
    fn graph_state_survives_opening_document_and_returning_to_graph() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "---\ntitle: A\n---\n");
        let mut app = app_from_workspace(&workspace);
        let path = workspace.path().join("a.md");
        let node = flokin_core::document_node_id(&path);
        let _ = app.update(Message::AppModeSelected(AppMode::Graph));
        let _ = app.update(Message::GraphNodeDragged {
            node: node.clone(),
            dx: 42.0,
            dy: 11.0,
        });
        let position = *app.graph.positions.get(&node).unwrap();

        let _ = app.update(Message::GraphNodeOpened(node.clone()));
        let _ = app.update(Message::AppModeSelected(AppMode::Graph));

        assert_eq!(app.graph.positions.get(&node).copied(), Some(position));
    }

    #[test]
    fn graph_zoom_controls_update_and_reset_zoom() {
        let workspace = TempWorkspace::new();
        workspace.write("a.md", "---\ntitle: A\n---\n");
        let mut app = app_from_workspace(&workspace);
        let _ = app.update(Message::AppModeSelected(AppMode::Graph));
        app.graph.viewport = iced::Size::new(800.0, 600.0);
        app.graph.zoom = 1.0;

        let _ = app.update(Message::GraphZoomIn);
        assert!(app.graph.zoom > 1.0);
        let _ = app.update(Message::GraphZoomOut);
        assert!(app.graph.zoom < 1.18);
        let _ = app.update(Message::GraphZoomReset);
        assert_eq!(app.graph.zoom, 1.0);
    }

    fn app_from_workspace(workspace: &TempWorkspace) -> FlokinApp {
        let mut app = FlokinApp::new();
        app.model
            .workspace_selected(Some(workspace.path().to_path_buf()));
        app.model
            .scan_completed(scan_workspace(workspace.path()).unwrap());
        app.sync_graph_projection(false);
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
