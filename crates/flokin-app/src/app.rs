use flokin_core::{mock_shell, ShellModel};
use iced::{application, window, Element, Size, Task, Theme};

use crate::{
    message::Message,
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
                self.model.toggle_explorer_node(id);
            }
            Message::WorkspaceTabSelected(tab) => {
                self.model.select_workspace_tab(tab);
            }
            Message::BottomTabSelected(tab) => {
                self.model.select_bottom_tab(tab);
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
}

pub fn run() -> iced::Result {
    application(FlokinApp::new, FlokinApp::update, FlokinApp::view)
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
    use flokin_core::{Activity, BottomTab, ExplorerNodeId, WorkspaceTab};

    use super::FlokinApp;
    use crate::{message::Message, theme::AppTheme};

    #[test]
    fn starts_with_native_shell_defaults() {
        let app = FlokinApp::new();

        assert_eq!(app.model.active_activity, Activity::Explorer);
        assert_eq!(app.model.selected_tab, WorkspaceTab::Carf);
        assert_eq!(app.model.bottom_tab, BottomTab::View);
        assert_eq!(app.theme, AppTheme::Dark);
    }

    #[test]
    fn update_selects_tabs_and_toggles_tree() {
        let mut app = FlokinApp::new();

        let _ = app.update(Message::WorkspaceTabSelected(WorkspaceTab::Cvm));
        let _ = app.update(Message::BottomTabSelected(BottomTab::Backlinks));
        let _ = app.update(Message::ExplorerNodeToggled(ExplorerNodeId(2)));

        assert_eq!(app.model.selected_tab, WorkspaceTab::Cvm);
        assert_eq!(app.model.bottom_tab, BottomTab::Backlinks);
        assert!(!app.model.explorer[0].children[0].expanded);
    }

    #[test]
    fn update_toggles_theme_in_memory() {
        let mut app = FlokinApp::new();

        let _ = app.update(Message::ThemeToggled);
        assert_eq!(app.theme, AppTheme::Light);

        let _ = app.update(Message::ThemeToggled);
        assert_eq!(app.theme, AppTheme::Dark);
    }
}
