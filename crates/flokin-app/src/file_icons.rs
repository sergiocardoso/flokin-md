use std::path::Path;

use iced::{Color, Font};

use crate::theme::{self, AppTheme};

#[derive(Debug, Clone, PartialEq)]
pub struct FileIconInfo {
    pub glyph: char,
    pub color: Color,
    pub fallback_label: String,
    pub font: Option<Font>,
}

pub fn icon_for_path(path: &Path, app_theme: AppTheme) -> FileIconInfo {
    let icon = devicons::icon_for_file(path, &Some(devicons_theme(app_theme)));
    let fallback = fallback_color(app_theme);
    let color = parse_hex_color(icon.color)
        .filter(|color| has_adequate_contrast(*color, background_color(app_theme)))
        .unwrap_or(fallback);

    FileIconInfo {
        glyph: icon.icon,
        color,
        fallback_label: fallback_label(path),
        font: file_icon_font(),
    }
}

fn devicons_theme(app_theme: AppTheme) -> devicons::Theme {
    match app_theme {
        AppTheme::Dark => devicons::Theme::Dark,
        AppTheme::Light => devicons::Theme::Light,
    }
}

fn file_icon_font() -> Option<Font> {
    // rust-devicons glyphs require Nerd Fonts. FlokinMD does not bundle one yet,
    // so rendering currently uses a stable colored text fallback.
    None
}

fn fallback_label(path: &Path) -> String {
    if let Some(file_name) = path.file_name().and_then(|file_name| file_name.to_str()) {
        match file_name.to_ascii_lowercase().as_str() {
            "cargo.toml" => return String::from("RS"),
            "dockerfile" => return String::from("DO"),
            "makefile" => return String::from("MK"),
            "package.json" => return String::from("JS"),
            "readme.md" => return String::from("MD"),
            _ => {}
        }
    }

    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.chars().take(4).collect::<String>().to_uppercase())
        .filter(|label| !label.is_empty())
        .unwrap_or_else(|| String::from("FILE"))
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return None;
    }

    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some(Color::from_rgb8(red, green, blue))
}

fn has_adequate_contrast(foreground: Color, background: Color) -> bool {
    contrast_ratio(foreground, background) >= 3.0
}

fn contrast_ratio(left: Color, right: Color) -> f32 {
    let left = relative_luminance(left);
    let right = relative_luminance(right);
    let lighter = left.max(right);
    let darker = left.min(right);

    (lighter + 0.05) / (darker + 0.05)
}

fn relative_luminance(color: Color) -> f32 {
    fn channel(value: f32) -> f32 {
        if value <= 0.039_28 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
}

fn background_color(app_theme: AppTheme) -> Color {
    match app_theme {
        AppTheme::Dark => theme::Palette::DARK.panel,
        AppTheme::Light => theme::Palette::LIGHT.panel,
    }
}

fn fallback_color(app_theme: AppTheme) -> Color {
    match app_theme {
        AppTheme::Dark => theme::Palette::DARK.text_muted,
        AppTheme::Light => theme::Palette::LIGHT.text_muted,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{file_icons::icon_for_path, theme::AppTheme};

    #[test]
    fn resolves_readme_md() {
        let icon = icon_for_path(Path::new("README.md"), AppTheme::Dark);

        assert_ne!(icon.glyph, '\0');
        assert_eq!(icon.fallback_label, "MD");
    }

    #[test]
    fn resolves_cargo_toml() {
        let icon = icon_for_path(Path::new("Cargo.toml"), AppTheme::Dark);

        assert_ne!(icon.glyph, '\0');
        assert_eq!(icon.fallback_label, "RS");
    }

    #[test]
    fn resolves_rust_file() {
        let icon = icon_for_path(Path::new("foo.rs"), AppTheme::Dark);

        assert_ne!(icon.glyph, '\0');
        assert_eq!(icon.fallback_label, "RS");
    }

    #[test]
    fn resolves_markdown_file() {
        let icon = icon_for_path(Path::new("foo.md"), AppTheme::Dark);

        assert_ne!(icon.glyph, '\0');
        assert_eq!(icon.fallback_label, "MD");
    }

    #[test]
    fn resolves_unknown_name() {
        let icon = icon_for_path(Path::new("unknown-file"), AppTheme::Dark);

        assert_ne!(icon.glyph, '\0');
        assert_eq!(icon.fallback_label, "FILE");
    }

    #[test]
    fn resolves_light_theme() {
        let icon = icon_for_path(Path::new("foo.rs"), AppTheme::Light);

        assert_ne!(icon.glyph, '\0');
    }

    #[test]
    fn resolves_dark_theme() {
        let icon = icon_for_path(Path::new("foo.rs"), AppTheme::Dark);

        assert_ne!(icon.glyph, '\0');
    }
}
