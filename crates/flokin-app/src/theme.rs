use iced::border::Radius;
use iced::widget::{button, container, svg, text, text_editor as iced_text_editor, text_input};
use iced::{color, theme as iced_theme, Background, Border, Color, Font, Shadow, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTheme {
    Dark,
    Light,
}

impl AppTheme {
    pub const fn toggled(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Dark => "Light",
            Self::Light => "Dark",
        }
    }

    pub const fn iced(self) -> Theme {
        match self {
            Self::Dark => Theme::TokyoNight,
            Self::Light => Theme::TokyoNightLight,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub background: Color,
    pub surface: Color,
    pub surface_hover: Color,
    pub surface_selected: Color,
    pub surface_active: Color,
    pub elevated_surface: Color,
    pub panel: Color,
    pub editor_background: Color,
    pub editor_gutter: Color,
    pub data_row_odd: Color,
    pub data_row_even: Color,
    pub data_gutter: Color,
    pub data_separator: Color,
    pub border: Color,
    pub border_subtle: Color,
    pub text: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub accent_hover: Color,
    pub accent_soft: Color,
    #[allow(dead_code)]
    pub success: Color,
    pub warning: Color,
    #[allow(dead_code)]
    pub danger: Color,
    #[allow(dead_code)]
    pub selected_text: Color,
}

impl Palette {
    pub const DARK: Self = Self {
        background: color!(0x090d14),
        surface: color!(0x0f151f),
        surface_hover: color!(0x1b2534),
        surface_selected: color!(0x241d3f),
        surface_active: color!(0x202b3b),
        elevated_surface: color!(0x151d2a),
        panel: color!(0x101720),
        editor_background: color!(0x0b1018),
        editor_gutter: color!(0x111925),
        data_row_odd: color!(0x0b1018),
        data_row_even: color!(0x0f151f),
        data_gutter: color!(0x111925),
        data_separator: color!(0x182231),
        border: color!(0x253144),
        border_subtle: color!(0x1a2433),
        text: color!(0xe8eef8),
        text_muted: color!(0xa4afc1),
        accent: color!(0x9b7cff),
        accent_hover: color!(0xb39aff),
        accent_soft: color!(0x282044),
        success: color!(0x43d18b),
        warning: color!(0xf0a742),
        danger: color!(0xf26464),
        selected_text: color!(0xf7f3ff),
    };

    pub const LIGHT: Self = Self {
        background: color!(0xf4f6fb),
        surface: color!(0xfafbfd),
        surface_hover: color!(0xeef2f8),
        surface_selected: color!(0xeee8ff),
        surface_active: color!(0xe5eaf3),
        elevated_surface: color!(0xffffff),
        panel: color!(0xf9fafc),
        editor_background: color!(0xffffff),
        editor_gutter: color!(0xf1f3f8),
        data_row_odd: color!(0xffffff),
        data_row_even: color!(0xf8f9fc),
        data_gutter: color!(0xf1f3f8),
        data_separator: color!(0xe4e8f0),
        border: color!(0xd8deea),
        border_subtle: color!(0xe8ecf3),
        text: color!(0x182030),
        text_muted: color!(0x657084),
        accent: color!(0x6f45e8),
        accent_hover: color!(0x5f35d9),
        accent_soft: color!(0xeee8ff),
        success: color!(0x147a4d),
        warning: color!(0xa96813),
        danger: color!(0xc43f3f),
        selected_text: color!(0x241452),
    };
}

pub mod spacing {
    pub const XXS: f32 = 2.0;
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 20.0;
    pub const XXL: f32 = 24.0;
}

pub mod radius {
    pub const XS: f32 = 2.0;
    pub const SM: f32 = 4.0;
    pub const MD: f32 = 6.0;
    pub const LG: f32 = 8.0;
}

pub mod typography {
    use iced::Font;

    pub const UI: Font = Font::DEFAULT;
    pub const MONO: Font = Font::MONOSPACE;
    pub const MENU: u32 = 13;
    pub const LABEL: u32 = 11;
    pub const BODY: u32 = 13;
    pub const EDITOR: u32 = 14;
    pub const TITLE: u32 = 15;
}

pub mod icons {
    pub const TREE: f32 = 15.0;
    pub const TOOLBAR: f32 = 16.0;
    pub const ACTIVITY: f32 = 20.0;
    pub const META: f32 = 15.0;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    #[allow(dead_code)]
    ChevronRight,
    #[allow(dead_code)]
    ChevronDown,
    Clock,
    Database,
    FileText,
    Folder,
    PanelLeft,
    Plus,
    Refresh,
    Search,
    Settings,
    Split,
    Tag,
    Terminal,
}

pub fn palette(theme: &Theme) -> Palette {
    if theme.palette().background.r > 0.5 {
        Palette::LIGHT
    } else {
        Palette::DARK
    }
}

pub fn application_style(theme: &Theme) -> iced_theme::Style {
    let palette = palette(theme);

    iced_theme::Style {
        background_color: palette.background,
        text_color: palette.text,
    }
}

pub fn text_normal(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(palette(theme).text),
    }
}

pub fn text_muted(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(palette(theme).text_muted),
    }
}

pub fn text_accent(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(palette(theme).accent),
    }
}

#[allow(dead_code)]
pub fn text_success(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(palette(theme).success),
    }
}

pub fn text_warning(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(palette(theme).warning),
    }
}

pub fn icon_style(theme: &Theme, _status: svg::Status) -> svg::Style {
    svg::Style {
        color: Some(palette(theme).text_muted),
    }
}

pub fn icon_accent_style(theme: &Theme, _status: svg::Status) -> svg::Style {
    svg::Style {
        color: Some(palette(theme).accent),
    }
}

pub fn text_editor(theme: &Theme, status: iced_text_editor::Status) -> iced_text_editor::Style {
    let palette = palette(theme);
    let border_color = match status {
        iced_text_editor::Status::Focused { .. } => palette.accent,
        iced_text_editor::Status::Hovered => palette.border,
        iced_text_editor::Status::Active | iced_text_editor::Status::Disabled => {
            palette.border_subtle
        }
    };

    iced_text_editor::Style {
        background: Background::Color(palette.editor_background),
        border: border(border_color, 1.0, radius::SM),
        placeholder: palette.text_muted,
        value: palette.text,
        selection: palette.accent_soft,
    }
}

pub fn panel(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(palette.panel, None, radius::XS)
}

pub fn elevated(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(palette.elevated_surface, None, radius::SM)
}

pub fn overlay_panel(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container::Style {
        text_color: Some(palette.text),
        background: Some(Background::Color(palette.elevated_surface)),
        border: border(palette.border, 1.0, radius::LG),
        shadow: Shadow {
            color: Color {
                a: 0.28,
                ..Color::BLACK
            },
            offset: iced::Vector::new(0.0, 12.0),
            blur_radius: 28.0,
        },
        ..container::Style::default()
    }
}

pub fn overlay_backdrop(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    let backdrop = if palette.background.r > 0.5 {
        Color {
            a: 0.22,
            ..palette.border
        }
    } else {
        Color {
            a: 0.34,
            ..Color::BLACK
        }
    };

    container::Style {
        background: Some(Background::Color(backdrop)),
        ..container::Style::default()
    }
}

pub fn surface(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(palette.surface, None, radius::SM)
}

pub fn editor(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(
        palette.editor_background,
        Some(palette.border_subtle),
        radius::MD,
    )
}

pub fn gutter(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container::Style {
        text_color: Some(palette.text_muted),
        background: Some(Background::Color(palette.editor_gutter)),
        border: Border::default(),
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}

pub fn chip(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(palette.accent_soft, None, radius::LG)
}

pub fn divider(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(palette.border_subtle, Some(palette.border_subtle), 0.0)
}

pub fn tab_underline(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(palette.accent, None, 0.0)
}

pub fn active_line(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(palette.surface_selected, None, 0.0)
}

pub fn table_row(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(palette.editor_background, Some(palette.border_subtle), 0.0)
}

pub fn data_row(theme: &Theme, row_index: usize, selected: bool) -> container::Style {
    let palette = palette(theme);
    let background = if selected {
        palette.surface_selected
    } else if row_index.is_multiple_of(2) {
        palette.data_row_even
    } else {
        palette.data_row_odd
    };

    container_style(background, Some(palette.data_separator), 0.0)
}

pub fn data_row_button(
    theme: &Theme,
    row_index: usize,
    selected: bool,
    status: button::Status,
) -> button::Style {
    let palette = palette(theme);
    let background = if selected {
        palette.surface_selected
    } else if matches!(status, button::Status::Hovered) {
        palette.surface_hover
    } else if row_index.is_multiple_of(2) {
        palette.data_row_even
    } else {
        palette.data_row_odd
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: palette.text,
        border: Border::default(),
        shadow: Shadow::default(),
        ..button::Style::default()
    }
}

pub fn data_gutter(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(palette.data_gutter, Some(palette.data_separator), 0.0)
}

pub fn data_header(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(
        palette.elevated_surface,
        Some(palette.data_separator),
        radius::XS,
    )
}

pub fn data_cell(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container::Style {
        border: Border {
            color: palette.data_separator,
            width: 0.5,
            radius: Radius::default(),
        },
        ..container::Style::default()
    }
}

pub fn table_row_selected(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(palette.surface_selected, Some(palette.accent_soft), 0.0)
}

pub fn input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = palette(theme);
    let border_color = match status {
        text_input::Status::Focused { .. } => palette.accent,
        text_input::Status::Hovered => palette.border,
        _ => palette.border_subtle,
    };

    text_input::Style {
        background: Background::Color(palette.elevated_surface),
        border: border(border_color, 1.0, radius::SM),
        icon: palette.text_muted,
        placeholder: palette.text_muted,
        value: palette.text,
        selection: palette.accent_soft,
    }
}

pub fn button_toolbar(theme: &Theme, status: button::Status) -> button::Style {
    button_chrome(theme, status, false)
}

pub fn button_activity(theme: &Theme, status: button::Status) -> button::Style {
    button_chrome(theme, status, false)
}

pub fn button_selected(theme: &Theme, status: button::Status) -> button::Style {
    let palette = palette(theme);
    let background = match status {
        button::Status::Hovered => palette.surface_active,
        _ => palette.accent_soft,
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: palette.accent,
        border: Border::default(),
        shadow: Shadow::default(),
        ..button::Style::default()
    }
}

pub fn button_ghost(theme: &Theme, status: button::Status) -> button::Style {
    let palette = palette(theme);
    let background = match status {
        button::Status::Hovered => Some(Background::Color(palette.surface_hover)),
        _ => None,
    };

    button::Style {
        background,
        text_color: palette.text_muted,
        border: Border::default(),
        shadow: Shadow::default(),
        ..button::Style::default()
    }
}

pub fn button_menu(theme: &Theme, status: button::Status) -> button::Style {
    button_chrome(theme, status, false)
}

pub fn splitter(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container::Style {
        background: Some(Background::Color(palette.border_subtle)),
        ..container::Style::default()
    }
}

#[allow(dead_code)]
pub fn button_tree(theme: &Theme, status: button::Status) -> button::Style {
    let palette = palette(theme);
    let background = match status {
        button::Status::Hovered => Some(Background::Color(palette.surface_hover)),
        _ => None,
    };

    button::Style {
        background,
        text_color: palette.text,
        border: Border::default(),
        shadow: Shadow::default(),
        ..button::Style::default()
    }
}

#[allow(dead_code)]
pub fn button_tree_selected(theme: &Theme, status: button::Status) -> button::Style {
    let palette = palette(theme);
    let background = match status {
        button::Status::Hovered => palette.surface_active,
        _ => palette.surface_selected,
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: palette.selected_text,
        border: Border::default(),
        shadow: Shadow::default(),
        ..button::Style::default()
    }
}

pub fn button_table_header(theme: &Theme, status: button::Status) -> button::Style {
    let palette = palette(theme);
    let background = match status {
        button::Status::Hovered => palette.surface_hover,
        _ => palette.elevated_surface,
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: palette.text,
        border: Border::default(),
        shadow: Shadow::default(),
        ..button::Style::default()
    }
}

pub fn button_tab(theme: &Theme, status: button::Status) -> button::Style {
    let palette = palette(theme);
    let background = match status {
        button::Status::Hovered => palette.surface_hover,
        _ => Color::TRANSPARENT,
    };

    button::Style {
        background: if background == Color::TRANSPARENT {
            None
        } else {
            Some(Background::Color(background))
        },
        text_color: palette.text_muted,
        border: Border::default(),
        shadow: Shadow::default(),
        ..button::Style::default()
    }
}

pub fn button_tab_selected(theme: &Theme, _status: button::Status) -> button::Style {
    let palette = palette(theme);
    button::Style {
        background: None,
        text_color: palette.text,
        border: Border::default(),
        shadow: Shadow::default(),
        ..button::Style::default()
    }
}

pub const fn mono() -> Font {
    typography::MONO
}

fn button_chrome(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let palette = palette(theme);
    let background = match (selected, status) {
        (true, button::Status::Hovered) => palette.accent_hover,
        (true, _) => palette.accent,
        (false, button::Status::Hovered) => palette.surface_hover,
        (false, button::Status::Pressed) => palette.surface_active,
        (false, _) => Color::TRANSPARENT,
    };
    let text_color = if selected {
        palette.accent
    } else {
        palette.text
    };

    button::Style {
        background: if background == Color::TRANSPARENT {
            None
        } else {
            Some(Background::Color(background))
        },
        text_color,
        border: Border::default(),
        shadow: Shadow::default(),
        ..button::Style::default()
    }
}

fn container_style(
    background: Color,
    border_color: Option<Color>,
    radius: f32,
) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(Background::Color(background)),
        border: border_color.map_or_else(Border::default, |color| border(color, 1.0, radius)),
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}

pub const fn icon_svg(icon: Icon) -> &'static str {
    match icon {
        Icon::ChevronRight => r#"<svg viewBox="0 0 24 24"><path d="m9 6 6 6-6 6"/></svg>"#,
        Icon::ChevronDown => r#"<svg viewBox="0 0 24 24"><path d="m6 9 6 6 6-6"/></svg>"#,
        Icon::Clock => {
            r#"<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="8"/><path d="M12 8v5l3 2"/></svg>"#
        }
        Icon::Database => {
            r#"<svg viewBox="0 0 24 24"><ellipse cx="12" cy="5" rx="7" ry="3"/><path d="M5 5v6c0 1.7 3.1 3 7 3s7-1.3 7-3V5"/><path d="M5 11v6c0 1.7 3.1 3 7 3s7-1.3 7-3v-6"/></svg>"#
        }
        Icon::FileText => {
            r#"<svg viewBox="0 0 24 24"><path d="M7 3h7l4 4v14H7z"/><path d="M14 3v5h5"/><path d="M9 12h6"/><path d="M9 16h6"/></svg>"#
        }
        Icon::Folder => {
            r#"<svg viewBox="0 0 24 24"><path d="M3 6h7l2 2h9v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/></svg>"#
        }
        Icon::PanelLeft => {
            r#"<svg viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16"/></svg>"#
        }
        Icon::Plus => r#"<svg viewBox="0 0 24 24"><path d="M12 5v14"/><path d="M5 12h14"/></svg>"#,
        Icon::Refresh => {
            r#"<svg viewBox="0 0 24 24"><path d="M20 12a8 8 0 0 1-14 5"/><path d="M4 12a8 8 0 0 1 14-5"/><path d="M18 3v4h-4"/><path d="M6 21v-4h4"/></svg>"#
        }
        Icon::Search => {
            r#"<svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="7"/><path d="m16 16 4 4"/></svg>"#
        }
        Icon::Settings => {
            r#"<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 0 0-.1-1l2-1.6-2-3.4-2.4 1a7 7 0 0 0-1.7-1L14.5 3h-5l-.3 3a7 7 0 0 0-1.7 1l-2.4-1-2 3.4 2 1.6a7 7 0 0 0 0 2l-2 1.6 2 3.4 2.4-1a7 7 0 0 0 1.7 1l.3 3h5l.3-3a7 7 0 0 0 1.7-1l2.4 1 2-3.4-2-1.6c.1-.3.1-.7.1-1z"/></svg>"#
        }
        Icon::Split => {
            r#"<svg viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M12 4v16"/></svg>"#
        }
        Icon::Tag => {
            r#"<svg viewBox="0 0 24 24"><path d="M4 12V5h7l9 9-6 6z"/><circle cx="8" cy="8" r="1"/></svg>"#
        }
        Icon::Terminal => {
            r#"<svg viewBox="0 0 24 24"><path d="m5 7 5 5-5 5"/><path d="M12 17h7"/></svg>"#
        }
    }
}

fn border(color: Color, width: f32, radius: f32) -> Border {
    Border {
        color,
        width,
        radius: Radius::from(radius),
    }
}
