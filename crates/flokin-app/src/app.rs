use std::time::{Duration, Instant};

use flokin_core::{mock_shell, ScanError, ShellModel};
use iced::{
    advanced::widget::{self as advanced_widget, operate},
    application, keyboard,
    keyboard::{key::Named, Key},
    window, Element, Size, Subscription, Task, Theme,
};

use crate::{
    message::Message,
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
}

impl FlokinApp {
    fn new() -> Self {
        Self {
            model: mock_shell(),
            theme: AppTheme::Dark,
            search_needs_refresh: false,
            search_debounce_target: None,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ActivitySelected(activity) => {
                self.model.select_activity(activity);
            }
            Message::ExplorerNodeToggled(id) => {
                if !self.model.toggle_explorer_node(id) {
                    self.model.select_explorer_node(id);
                }
            }
            Message::WorkspaceTabSelected(tab) => {
                self.model.select_workspace_tab(tab);
            }
            Message::BottomTabSelected(tab) => {
                self.model.select_bottom_tab(tab);
            }
            Message::OpenFolder => {
                return Task::perform(file_dialog::pick_folder(), Message::FolderSelected);
            }
            Message::FolderSelected(path) => {
                if let Some(path) = path {
                    self.model.workspace_selected(Some(path.clone()));
                    return scan_workspace_task(path);
                }
            }
            Message::ScanCompleted(path, result) => {
                if self.model.current_workspace.as_ref() == Some(&path) {
                    match result {
                        Ok(result) => self.model.scan_completed(result),
                        Err(message) => self.model.scan_failed(message),
                    }
                    self.search_needs_refresh = false;
                }
            }
            Message::ReindexWorkspace => {
                if let Some(path) = self.model.current_workspace.clone() {
                    self.model.workspace_selected(Some(path.clone()));
                    return scan_workspace_task(path);
                }
            }
            Message::WorkspaceWatcher(message) => match message {
                file_watcher::WatcherMessage::Events { workspace, events } => {
                    if self.model.current_workspace.as_ref() == Some(&workspace) {
                        self.model.workspace_update_started();
                        return Task::perform(
                            async move {
                                let result =
                                    flokin_core::workspace_update_from_events(&workspace, &events)
                                        .map_err(|error| error.to_string());
                                (workspace, result)
                            },
                            |(path, result)| Message::WorkspaceUpdateCompleted(path, result),
                        );
                    }
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
            Message::WorkspaceUpdateCompleted(path, result) => {
                if self.model.current_workspace.as_ref() == Some(&path) {
                    match result {
                        Ok(update) if update.needs_rescan => return scan_workspace_task(path),
                        Ok(update) => {
                            self.model.workspace_update_completed(update);
                            self.search_needs_refresh = false;
                        }
                        Err(message) => self
                            .model
                            .workspace_update_failed(ScanError { path, message }),
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
                self.model.select_markdown_path(path);
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
                self.model.activate_selected_search_result();
                self.search_needs_refresh = false;
                self.search_debounce_target = None;
            }
            Message::SearchResultSelected(path) => {
                self.model.select_search_result_path(path);
                self.search_needs_refresh = false;
                self.search_debounce_target = None;
            }
            Message::KeyboardEvent(event) => {
                if let Some(message) = keyboard_message(event, self.model.search.open) {
                    return self.update(message);
                }
            }
            Message::ThemeToggled => {
                self.theme = self.theme.toggled();
            }
            Message::MockAction => {}
        }

        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        views::shell::view(&self.model, self.theme)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            file_watcher::subscription(self.model.current_workspace.clone()),
            keyboard::listen().map(Message::KeyboardEvent),
        ])
    }
}

pub fn run() -> iced::Result {
    application(FlokinApp::new, FlokinApp::update, FlokinApp::view)
        .subscription(FlokinApp::subscription)
        .title(title)
        .theme(app_theme)
        .style(app_style)
        .window(window::Settings {
            size: Size::new(1440.0, 900.0),
            min_size: Some(Size::new(1100.0, 700.0)),
            resizable: true,
            ..window::Settings::default()
        })
        .run()
}

fn scan_workspace_task(path: std::path::PathBuf) -> Task<Message> {
    Task::perform(
        async move {
            let result = flokin_core::scan_workspace(&path).map_err(|error| error.to_string());
            (path, result)
        },
        |(path, result)| Message::ScanCompleted(path, result),
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

fn keyboard_message(event: keyboard::Event, search_open: bool) -> Option<Message> {
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
    use flokin_core::{Activity, BottomTab, ScanState, WorkspaceEvent, WorkspaceTab};
    use iced::keyboard::{
        key::{Code, Named, Physical},
        Event, Key, Location, Modifiers,
    };

    use super::{keyboard_message, FlokinApp};
    use crate::{message::Message, services::file_watcher::WatcherMessage, theme::AppTheme};

    #[test]
    fn starts_with_native_shell_defaults() {
        let app = FlokinApp::new();

        assert_eq!(app.model.active_activity, Activity::Explorer);
        assert_eq!(app.model.current_workspace, None);
        assert_eq!(app.model.selected_tab, WorkspaceTab::Carf);
        assert_eq!(app.model.bottom_tab, BottomTab::View);
        assert_eq!(app.theme, AppTheme::Dark);
    }

    #[test]
    fn update_selects_tabs_and_toggles_tree() {
        let mut app = FlokinApp::new();

        let _ = app.update(Message::WorkspaceTabSelected(WorkspaceTab::Cvm));
        let _ = app.update(Message::BottomTabSelected(BottomTab::Backlinks));

        assert_eq!(app.model.selected_tab, WorkspaceTab::Cvm);
        assert_eq!(app.model.bottom_tab, BottomTab::Backlinks);
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

        assert_eq!(keyboard_message(event.clone(), false), None);
        assert_eq!(keyboard_message(event, true), Some(Message::SearchNext));
    }
}
