use flokin_core::{mock_shell, ScanError, ShellModel};
use iced::{application, window, Element, Size, Subscription, Task, Theme};

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
}

impl FlokinApp {
    fn new() -> Self {
        Self {
            model: mock_shell(),
            theme: AppTheme::Dark,
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
                        Ok(update) => self.model.workspace_update_completed(update),
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
        file_watcher::subscription(self.model.current_workspace.clone())
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

    use super::FlokinApp;
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
}
