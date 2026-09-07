use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{i18n::AppLanguage, theme::AppTheme};

const SETTINGS_VERSION: &str = "1";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppSettings {
    pub theme: Option<AppTheme>,
    pub language: Option<AppLanguage>,
    pub last_workspace_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageLoad {
    MissingSettings,
    MissingLanguage,
    Language(AppLanguage),
    Invalid,
}

pub fn load_settings(path: &Path) -> Option<AppSettings> {
    let content = fs::read_to_string(path).ok()?;
    parse_settings(&content)
}

pub fn load_theme(path: &Path) -> Option<AppTheme> {
    load_settings(path).and_then(|settings| settings.theme)
}

pub fn load_language(path: &Path) -> LanguageLoad {
    if !path.exists() {
        return LanguageLoad::MissingSettings;
    }
    let Some(settings) = load_settings(path) else {
        return LanguageLoad::Invalid;
    };
    settings
        .language
        .map(LanguageLoad::Language)
        .unwrap_or(LanguageLoad::MissingLanguage)
}

pub fn save_theme(path: &Path, theme: AppTheme) -> Result<(), String> {
    let mut settings = load_settings(path).unwrap_or_default();
    settings.theme = Some(theme);
    save_settings(path, settings)
}

pub fn save_language(path: &Path, language: AppLanguage) -> Result<(), String> {
    let mut settings = load_settings(path).unwrap_or_default();
    settings.language = Some(language);
    save_settings(path, settings)
}

pub fn load_last_workspace_path(path: &Path) -> Option<PathBuf> {
    load_settings(path).and_then(|settings| settings.last_workspace_path)
}

pub fn save_last_workspace_path(path: &Path, workspace: PathBuf) -> Result<(), String> {
    let mut settings = load_settings(path).unwrap_or_default();
    settings.last_workspace_path = Some(workspace);
    save_settings(path, settings)
}

pub fn clear_last_workspace_path(path: &Path) -> Result<(), String> {
    let mut settings = load_settings(path).unwrap_or_default();
    settings.last_workspace_path = None;
    save_settings(path, settings)
}

pub fn save_settings(path: &Path, settings: AppSettings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Nao foi possivel criar diretorio de configuracoes {}: {error}",
                parent.display()
            )
        })?;
    }
    let temp = path.with_extension("conf.tmp");
    fs::write(&temp, serialize_settings(settings)).map_err(|error| {
        format!(
            "Nao foi possivel salvar configuracoes em {}: {error}",
            temp.display()
        )
    })?;
    fs::rename(&temp, path).map_err(|error| {
        format!(
            "Nao foi possivel salvar configuracoes em {}: {error}",
            path.display()
        )
    })
}

pub fn settings_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("settings.conf")
}

fn parse_settings(content: &str) -> Option<AppSettings> {
    let mut version_ok = false;
    let mut theme = None;
    let mut language = None;
    let mut last_workspace_path = None;

    for line in content.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "version" => version_ok = value.trim() == SETTINGS_VERSION,
            "theme" => {
                theme = match value.trim() {
                    "light" => Some(AppTheme::Light),
                    "dark" => Some(AppTheme::Dark),
                    _ => None,
                }
            }
            "language" => language = AppLanguage::from_setting(value.trim()),
            "last_workspace_path" => {
                let value = value.trim();
                if !value.is_empty() {
                    last_workspace_path = Some(PathBuf::from(value));
                }
            }
            _ => {}
        }
    }

    version_ok.then_some(AppSettings {
        theme,
        language,
        last_workspace_path,
    })
}

fn serialize_settings(settings: AppSettings) -> String {
    let mut content = format!("version={SETTINGS_VERSION}\n");
    if let Some(theme) = settings.theme {
        let value = match theme {
            AppTheme::Dark => "dark",
            AppTheme::Light => "light",
        };
        content.push_str("theme=");
        content.push_str(value);
        content.push('\n');
    }
    if let Some(language) = settings.language {
        content.push_str("language=");
        content.push_str(language.locale());
        content.push('\n');
    }
    if let Some(path) = settings.last_workspace_path {
        content.push_str("last_workspace_path=");
        content.push_str(&path.display().to_string());
        content.push('\n');
    }
    content
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_loads_theme() {
        let temp = temp_dir();
        let path = settings_path(&temp);

        save_theme(&path, AppTheme::Light).unwrap();

        assert_eq!(load_theme(&path), Some(AppTheme::Light));
    }

    #[test]
    fn saves_and_loads_language_without_dropping_theme() {
        let temp = temp_dir();
        let path = settings_path(&temp);

        save_theme(&path, AppTheme::Light).unwrap();
        save_language(&path, AppLanguage::English).unwrap();

        assert_eq!(load_theme(&path), Some(AppTheme::Light));
        assert_eq!(
            load_language(&path),
            LanguageLoad::Language(AppLanguage::English)
        );
    }

    #[test]
    fn saves_and_loads_last_workspace_without_dropping_theme_or_language() {
        let temp = temp_dir();
        let path = settings_path(&temp);
        let workspace = temp.join("My Workspace");

        save_theme(&path, AppTheme::Light).unwrap();
        save_language(&path, AppLanguage::English).unwrap();
        save_last_workspace_path(&path, workspace.clone()).unwrap();

        let settings = load_settings(&path).unwrap();
        assert_eq!(settings.theme, Some(AppTheme::Light));
        assert_eq!(settings.language, Some(AppLanguage::English));
        assert_eq!(settings.last_workspace_path, Some(workspace));
    }

    #[test]
    fn clears_last_workspace_without_dropping_theme_or_language() {
        let temp = temp_dir();
        let path = settings_path(&temp);
        let workspace = temp.join("My Workspace");

        save_theme(&path, AppTheme::Light).unwrap();
        save_language(&path, AppLanguage::English).unwrap();
        save_last_workspace_path(&path, workspace).unwrap();
        clear_last_workspace_path(&path).unwrap();

        let settings = load_settings(&path).unwrap();
        assert_eq!(settings.theme, Some(AppTheme::Light));
        assert_eq!(settings.language, Some(AppLanguage::English));
        assert_eq!(settings.last_workspace_path, None);
    }

    #[test]
    fn existing_settings_without_language_are_detected_for_migration() {
        let temp = temp_dir();
        let path = settings_path(&temp);
        fs::write(&path, "version=1\ntheme=dark\n").unwrap();

        assert_eq!(load_language(&path), LanguageLoad::MissingLanguage);
    }

    #[test]
    fn missing_or_malformed_settings_fall_back_to_default_path() {
        let temp = temp_dir();
        let path = settings_path(&temp);

        assert_eq!(load_theme(&path), None);
        assert_eq!(load_language(&path), LanguageLoad::MissingSettings);
        fs::write(&path, "version=1\ntheme=banana\n").unwrap();
        assert_eq!(load_theme(&path), None);
        fs::write(&path, "theme=light\n").unwrap();
        assert_eq!(load_theme(&path), None);
        assert_eq!(load_language(&path), LanguageLoad::Invalid);
    }

    fn temp_dir() -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "flokinmd-settings-test-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }
}
