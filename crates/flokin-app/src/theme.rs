use iced::border::Radius;
pub mod tokens;

use iced::widget::{
    button, container, markdown, svg, text, text_editor as iced_text_editor, text_input,
};
use iced::{theme as iced_theme, Background, Border, Color, Font, Padding, Shadow, Theme};

pub use tokens::{ColorTokens, ThemeTokens};

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

pub mod spacing {
    use super::tokens;

    pub const XXS: f32 = tokens::SPACING.xxs;
    pub const XS: f32 = tokens::SPACING.xs;
    pub const SM: f32 = tokens::SPACING.sm;
    pub const MD: f32 = tokens::SPACING.md;
    pub const LG: f32 = tokens::SPACING.lg;
    pub const XL: f32 = tokens::SPACING.xl;
    pub const XXL: f32 = tokens::SPACING.xxl;
}

pub mod radius {
    use super::tokens;

    pub const XS: f32 = 2.0;
    pub const SM: f32 = tokens::RADIUS.small;
    pub const MD: f32 = tokens::RADIUS.medium;
    pub const LG: f32 = tokens::RADIUS.large;
}

pub mod typography {
    use super::tokens;

    pub const UI: iced::Font = tokens::TYPOGRAPHY.ui_font;
    pub const MONO: iced::Font = tokens::TYPOGRAPHY.mono_font;
    pub const MENU: u32 = tokens::TYPOGRAPHY.font_size_menu;
    #[allow(dead_code)]
    pub const SMALL: u32 = tokens::TYPOGRAPHY.font_size_small;
    pub const LABEL: u32 = tokens::TYPOGRAPHY.font_size_label;
    pub const BODY: u32 = tokens::TYPOGRAPHY.font_size_body;
    pub const EDITOR: u32 = tokens::TYPOGRAPHY.font_size_editor;
    pub const EDITOR_LINE_NUMBER: u32 = tokens::TYPOGRAPHY.font_size_editor_line_number;
    #[allow(dead_code)]
    pub const GRID: u32 = tokens::TYPOGRAPHY.font_size_grid;
    pub const TITLE: u32 = tokens::TYPOGRAPHY.font_size_heading;
}

pub mod icons {
    use super::tokens;

    pub const TREE: f32 = 16.0;
    pub const TOOLBAR: f32 = tokens::SIZES.toolbar_icon_size;
    pub const ACTIVITY: f32 = tokens::SIZES.activity_icon_size;
    pub const META: f32 = 16.0;
}

pub mod sizes {
    use super::tokens;

    pub const CONTROL_HEIGHT_SMALL: f32 = tokens::SIZES.control_height_small;
    pub const CONTROL_HEIGHT_MEDIUM: f32 = tokens::SIZES.control_height_medium;
    pub const CONTROL_HEIGHT_LARGE: f32 = tokens::SIZES.control_height_large;
    pub const ICON_SLOT_SMALL: f32 = tokens::SIZES.icon_slot_small;
    pub const ICON_SLOT_MEDIUM: f32 = tokens::SIZES.icon_slot_medium;
    pub const ICON_SLOT_LARGE: f32 = tokens::SIZES.icon_slot_large;
    pub const ACTIVITY_BAR_WIDTH: f32 = tokens::SIZES.activity_bar_width;
    pub const ACTIVITY_BUTTON_SIZE: f32 = CONTROL_HEIGHT_LARGE - 2.0;
    pub const TOOLBAR_HEIGHT: f32 = tokens::SIZES.toolbar_height;
    #[allow(dead_code)]
    pub const TOOLBAR_BUTTON_WIDTH: f32 = tokens::SIZES.toolbar_button_width;
    pub const TOOLBAR_BUTTON_HEIGHT: f32 = CONTROL_HEIGHT_MEDIUM;
    pub const TOOLBAR_SEARCH_WIDTH: f32 = tokens::SIZES.toolbar_search_width;
    pub const TAB_HEIGHT: f32 = tokens::SIZES.tab_height;
    pub const TAB_BUTTON_HEIGHT: f32 = tokens::SIZES.tab_button_height;
    pub const TAB_CLOSE_WIDTH: f32 = CONTROL_HEIGHT_SMALL - 6.0;
    pub const TAB_ICON_SIZE: f32 = tokens::SIZES.tab_icon_size;
    pub const DOCUMENT_HEADER_HEIGHT: f32 = tokens::SIZES.document_header_height;
    pub const EDITOR_LINE_HEIGHT_RATIO: f32 = tokens::SIZES.editor_line_height_ratio;
    pub const EDITOR_GUTTER_WIDTH: f32 = tokens::SIZES.editor_gutter_width;
    pub const DATA_GRID_ROW_HEIGHT: f32 = tokens::SIZES.data_grid_row_height;
    pub const DATA_GRID_HEADER_HEIGHT: f32 = tokens::SIZES.data_grid_header_height;
    pub const DATA_GRID_GUTTER_WIDTH: f32 = tokens::SIZES.data_grid_gutter_width;
    pub const SIDEBAR_DEFAULT_WIDTH: f32 = tokens::SIZES.sidebar_default_width;
    pub const INSPECTOR_DEFAULT_WIDTH: f32 = tokens::SIZES.inspector_default_width;
    pub const SCHEMA_DEFAULT_WIDTH: f32 = tokens::SIZES.schema_default_width;
    pub const SQL_EDITOR_DEFAULT_HEIGHT: f32 = tokens::SIZES.sql_editor_default_height;
    pub const SIDEBAR_MIN_WIDTH: f32 = tokens::SIZES.sidebar_min_width;
    pub const SIDEBAR_MAX_WIDTH: f32 = tokens::SIZES.sidebar_max_width;
    pub const INSPECTOR_MIN_WIDTH: f32 = tokens::SIZES.inspector_min_width;
    pub const INSPECTOR_MAX_WIDTH: f32 = tokens::SIZES.inspector_max_width;
    pub const SCHEMA_MIN_WIDTH: f32 = tokens::SIZES.schema_min_width;
    pub const SCHEMA_MAX_WIDTH: f32 = tokens::SIZES.schema_max_width;
    pub const SQL_EDITOR_MIN_HEIGHT: f32 = tokens::SIZES.sql_editor_min_height;
    pub const SQL_EDITOR_MAX_HEIGHT: f32 = tokens::SIZES.sql_editor_max_height;
    pub const SPLITTER_HIT_AREA: f32 = tokens::SIZES.splitter_hit_area;
    pub const MENU_BAR_HEIGHT: f32 = tokens::SIZES.menu_bar_height;
    pub const MENU_WIDTH: f32 = tokens::SIZES.menu_width;
    pub const MENU_TOP_OFFSET: f32 = tokens::SIZES.menu_top_offset;
    pub const MENU_ITEM_HEIGHT: f32 = tokens::SIZES.menu_item_height;
    pub const MENU_PADDING_Y: f32 = tokens::SIZES.menu_padding_y;
    pub const MENU_PADDING_X: f32 = tokens::SIZES.menu_padding_x;
    pub const MENU_POPUP_PADDING: f32 = tokens::SIZES.menu_popup_padding;
    pub const SEARCH_OVERLAY_WIDTH: f32 = tokens::SIZES.search_overlay_width;
    pub const SEARCH_OVERLAY_HEIGHT: f32 = tokens::SIZES.search_overlay_height;
    pub const SEARCH_RESULTS_HEIGHT: f32 = tokens::SIZES.search_results_height;
    pub const DIALOG_WIDTH: f32 = tokens::SIZES.dialog_width;
    pub const GRAPH_NODE_WIDTH: f32 = tokens::SIZES.graph_node_width;
    pub const GRAPH_NODE_HEIGHT: f32 = tokens::SIZES.graph_node_height;
    pub const GRAPH_TOOLBAR_BUTTON_SIZE: f32 = tokens::SIZES.graph_toolbar_button_size;
    pub const GRAPH_ZOOM_BADGE_WIDTH: f32 = tokens::SIZES.graph_zoom_badge_width;
    pub const GRAPH_NODE_FONT_SIZE: u32 = tokens::SIZES.graph_node_font_size;
    pub const GRAPH_EDGE_LABEL_FONT_SIZE: u32 = tokens::SIZES.graph_edge_label_font_size;
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
    Focus,
    Frame,
    Graph,
    Health,
    Minus,
    PanelLeft,
    Plus,
    Refresh,
    Reset,
    Save,
    Search,
    Settings,
    Split,
    Tag,
    Terminal,
    X,
}

pub fn tokens(theme: &Theme) -> &'static ThemeTokens {
    if theme.palette().background.r > 0.5 {
        &tokens::LIGHT
    } else {
        &tokens::DARK
    }
}

pub fn palette(theme: &Theme) -> ColorTokens {
    tokens(theme).colors
}

pub fn application_style(theme: &Theme) -> iced_theme::Style {
    let palette = palette(theme);

    iced_theme::Style {
        background_color: palette.app_background,
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
        color: Some(palette(theme).accent_text),
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

#[allow(dead_code)]
pub fn text_error(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(palette(theme).error),
    }
}

pub fn markdown_preview_settings(app_theme: AppTheme) -> markdown::Settings {
    let iced_theme = app_theme.iced();
    let palette = palette(&iced_theme);
    let style = markdown::Style {
        font: typography::UI,
        inline_code_highlight: markdown::Highlight {
            background: Background::Color(palette.preview_code_background),
            border: iced::border::rounded(radius::SM),
        },
        inline_code_padding: Padding::from([0.0, 3.0]),
        inline_code_color: palette.preview_text,
        inline_code_font: typography::MONO,
        code_block_font: typography::MONO,
        link_color: palette.preview_link,
    };
    let mut settings = markdown::Settings::with_text_size(typography::BODY + 1, style);
    settings.h1_size = 31.0.into();
    settings.h2_size = 24.0.into();
    settings.h3_size = 19.0.into();
    settings.h4_size = typography::TITLE.into();
    settings.h5_size = typography::BODY.into();
    settings.h6_size = typography::BODY.into();
    settings.code_size = typography::BODY.into();
    settings.spacing = spacing::MD.into();
    settings
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

pub fn icon_inverse_style(theme: &Theme, _status: svg::Status) -> svg::Style {
    svg::Style {
        color: Some(palette(theme).text_inverse),
    }
}

pub fn text_editor(theme: &Theme, status: iced_text_editor::Status) -> iced_text_editor::Style {
    text_editor_with_background(theme, status, palette(theme).editor_background)
}

pub fn markdown_text_editor(
    theme: &Theme,
    status: iced_text_editor::Status,
) -> iced_text_editor::Style {
    text_editor_with_background(theme, status, Color::TRANSPARENT)
}

pub fn markdown_preview(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(
        palette.preview_background,
        Some(palette.border_subtle),
        radius::SM,
    )
}

fn text_editor_with_background(
    theme: &Theme,
    status: iced_text_editor::Status,
    background: Color,
) -> iced_text_editor::Style {
    let palette = palette(theme);
    let border_color = match status {
        iced_text_editor::Status::Focused { .. } => palette.accent,
        iced_text_editor::Status::Hovered => palette.border,
        iced_text_editor::Status::Active | iced_text_editor::Status::Disabled => {
            palette.border_subtle
        }
    };

    iced_text_editor::Style {
        background: Background::Color(background),
        border: border(border_color, 1.0, radius::SM),
        placeholder: palette.text_muted,
        value: palette.text,
        selection: palette.editor_selection,
    }
}

pub fn panel(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(
        palette.sidebar_background,
        Some(palette.border_subtle),
        radius::LG,
    )
}

pub fn elevated(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(
        palette.surface_elevated,
        Some(palette.border_subtle),
        radius::MD,
    )
}

pub fn top_bar(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(palette.top_bar_background, Some(palette.border_subtle), 0.0)
}

pub fn search_surface(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(
        palette.search_background,
        Some(palette.border_subtle),
        radius::SM,
    )
}

pub fn activity_bar(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(palette.activity_bar_background, None, radius::LG)
}

pub fn inspector_panel(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(
        palette.inspector_background,
        Some(palette.border_subtle),
        radius::LG,
    )
}

pub fn document_surface(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(
        palette.content_background,
        Some(palette.border_subtle),
        radius::LG,
    )
}

pub fn document_header(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(palette.content_background, None, radius::MD)
}

pub fn segmented_control(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(palette.surface, Some(palette.border_subtle), radius::MD)
}

pub fn status_bar(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(
        palette.status_bar_background,
        Some(palette.border_subtle),
        0.0,
    )
}

pub fn status_dot(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(palette.success, None, radius::LG)
}

pub fn overlay_panel(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container::Style {
        text_color: Some(palette.text),
        background: Some(Background::Color(palette.menu_background)),
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

pub fn tooltip(theme: &Theme) -> container::Style {
    overlay_panel(theme)
}

pub fn search_overlay_panel(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container::Style {
        text_color: Some(palette.text),
        background: Some(Background::Color(palette.surface_elevated)),
        border: border(palette.border_strong, 1.0, radius::LG),
        shadow: Shadow {
            color: Color {
                a: 0.36,
                ..Color::BLACK
            },
            offset: iced::Vector::new(0.0, 14.0),
            blur_radius: 30.0,
        },
        ..container::Style::default()
    }
}

pub fn sql_completion_popup(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container::Style {
        text_color: Some(palette.text),
        background: Some(Background::Color(palette.surface_elevated)),
        border: border(palette.border, 1.0, radius::SM),
        shadow: Shadow {
            color: Color {
                a: 0.22,
                ..Color::BLACK
            },
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 18.0,
        },
        ..container::Style::default()
    }
}

pub fn overlay_backdrop(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container::Style {
        background: Some(Background::Color(palette.overlay_backdrop)),
        ..container::Style::default()
    }
}

pub fn surface(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(palette.surface, Some(palette.border_subtle), radius::SM)
}

pub fn editor(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(
        palette.editor_background,
        Some(palette.border_subtle),
        radius::MD,
    )
}

pub fn graph_panel(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(palette.graph_background, None, 0.0)
}

pub fn graph_toolbar_group(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(
        palette.graph_zoom_badge_background,
        Some(palette.graph_toolbar_border),
        radius::SM,
    )
}

pub fn graph_zoom_badge(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(
        palette.graph_zoom_badge_background,
        Some(palette.graph_toolbar_border),
        radius::SM,
    )
}

pub fn gutter(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container::Style {
        text_color: Some(palette.text_muted),
        background: Some(Background::Color(palette.editor_gutter_background)),
        border: Border::default(),
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}

pub fn editor_row(theme: &Theme, row_index: usize) -> container::Style {
    let palette = palette(theme);
    let background = if row_index.is_multiple_of(2) {
        palette.editor_row_odd
    } else {
        palette.editor_row_even
    };

    container_style(background, None, 0.0)
}

pub fn chip(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(palette.accent_soft, None, radius::LG)
}

pub fn divider(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(palette.border_subtle, Some(palette.border_subtle), 0.0)
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
        palette.grid_row_even
    } else {
        palette.grid_row_odd
    };

    container_style(background, Some(palette.grid_separator), 0.0)
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
        palette.grid_row_even
    } else {
        palette.grid_row_odd
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
    container_style(palette.grid_gutter, Some(palette.grid_separator), 0.0)
}

pub fn data_header(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container_style(
        palette.grid_header,
        Some(palette.grid_separator),
        radius::XS,
    )
}

pub fn data_cell(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container::Style {
        border: Border {
            color: palette.grid_separator,
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
        text_input::Status::Focused { .. } => palette.focus_ring,
        text_input::Status::Hovered => palette.border,
        _ => palette.border_subtle,
    };

    text_input::Style {
        background: Background::Color(palette.surface_elevated),
        border: border(border_color, 1.0, radius::SM),
        icon: palette.text_muted,
        placeholder: palette.text_muted,
        value: palette.text,
        selection: palette.accent_soft,
    }
}

pub fn input_embedded(theme: &Theme, _status: text_input::Status) -> text_input::Style {
    let palette = palette(theme);
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
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
    let palette = palette(theme);
    let background = match status {
        button::Status::Hovered => Some(Background::Color(palette.surface_hover)),
        button::Status::Pressed => Some(Background::Color(palette.surface_pressed)),
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

pub fn button_graph_toolbar(theme: &Theme, status: button::Status) -> button::Style {
    let palette = palette(theme);
    let background = match status {
        button::Status::Active => palette.graph_toolbar_button,
        button::Status::Hovered => palette.graph_toolbar_button_hover,
        button::Status::Pressed => palette.graph_toolbar_button_active,
        button::Status::Disabled => palette.graph_toolbar_button_disabled,
    };
    let text_color = if matches!(status, button::Status::Disabled) {
        palette.text_disabled
    } else {
        palette.text
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: border(palette.graph_toolbar_border, 1.0, radius::SM),
        shadow: Shadow::default(),
        ..button::Style::default()
    }
}

pub fn button_selected(theme: &Theme, status: button::Status) -> button::Style {
    let palette = palette(theme);
    let background = match status {
        button::Status::Hovered => palette.surface_selected,
        button::Status::Pressed => palette.surface_pressed,
        _ => palette.accent_soft,
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: palette.accent_text,
        border: Border::default(),
        shadow: Shadow::default(),
        ..button::Style::default()
    }
}

pub fn button_accent_outline(theme: &Theme, status: button::Status) -> button::Style {
    let palette = palette(theme);
    let background = match status {
        button::Status::Hovered => palette.accent_soft,
        button::Status::Pressed => palette.surface_pressed,
        _ => Color::TRANSPARENT,
    };

    button::Style {
        background: if background == Color::TRANSPARENT {
            None
        } else {
            Some(Background::Color(background))
        },
        text_color: palette.accent_text,
        border: border(palette.accent_border, 1.0, radius::SM),
        shadow: Shadow::default(),
        ..button::Style::default()
    }
}

pub fn button_primary(theme: &Theme, status: button::Status) -> button::Style {
    let palette = palette(theme);
    let background = match status {
        button::Status::Hovered => palette.accent_hover,
        button::Status::Pressed => palette.accent_pressed,
        button::Status::Disabled => palette.surface_active,
        button::Status::Active => palette.accent,
    };
    let text_color = if matches!(status, button::Status::Disabled) {
        palette.text_disabled
    } else {
        palette.text_inverse
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color,
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
        background: Some(Background::Color(palette.app_background)),
        ..container::Style::default()
    }
}

#[allow(dead_code)]
pub fn button_tree(theme: &Theme, status: button::Status) -> button::Style {
    let palette = palette(theme);
    let background = match status {
        button::Status::Hovered => Some(Background::Color(palette.surface_hover)),
        button::Status::Pressed => Some(Background::Color(palette.surface_pressed)),
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
        button::Status::Hovered => palette.surface_selected,
        button::Status::Pressed => palette.surface_pressed,
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

pub fn sql_completion_button(
    theme: &Theme,
    selected: bool,
    status: button::Status,
) -> button::Style {
    let palette = palette(theme);
    let background = if selected {
        Some(Background::Color(palette.accent_soft))
    } else if matches!(status, button::Status::Hovered) {
        Some(Background::Color(palette.surface_hover))
    } else {
        None
    };

    button::Style {
        background,
        text_color: palette.text,
        border: Border::default(),
        shadow: Shadow::default(),
        ..button::Style::default()
    }
}

pub fn button_table_header(theme: &Theme, status: button::Status) -> button::Style {
    let palette = palette(theme);
    let background = match status {
        button::Status::Hovered => palette.surface_hover,
        _ => palette.surface_elevated,
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
        button::Status::Pressed => palette.surface_pressed,
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

pub fn button_tab_selected(theme: &Theme, status: button::Status) -> button::Style {
    let palette = palette(theme);
    let background = match status {
        button::Status::Hovered => palette.surface_selected,
        button::Status::Pressed => palette.surface_pressed,
        _ => palette.accent_soft,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: palette.accent_text,
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
        (false, button::Status::Pressed) => palette.surface_pressed,
        (false, _) => Color::TRANSPARENT,
    };
    let text_color = if selected {
        palette.accent_text
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
        Icon::Focus => {
            r#"<svg viewBox="0 0 24 24"><path d="M12 5v4"/><path d="M12 15v4"/><path d="M5 12h4"/><path d="M15 12h4"/><circle cx="12" cy="12" r="3"/></svg>"#
        }
        Icon::Frame => {
            r#"<svg viewBox="0 0 24 24"><path d="M8 4H4v4"/><path d="M16 4h4v4"/><path d="M20 16v4h-4"/><path d="M8 20H4v-4"/><rect x="8" y="8" width="8" height="8" rx="1"/></svg>"#
        }
        Icon::Graph => {
            r#"<svg viewBox="0 0 24 24"><circle cx="6" cy="7" r="2.5"/><circle cx="18" cy="6" r="2.5"/><circle cx="8" cy="18" r="2.5"/><circle cx="17" cy="16" r="2.5"/><path d="M8.4 7.8 15.6 6.4"/><path d="M7 9.2 7.7 15.6"/><path d="M10.3 17.4 14.7 16.5"/></svg>"#
        }
        Icon::Health => {
            r#"<svg viewBox="0 0 24 24"><path d="M20 11a8 8 0 1 1-16 0 8 8 0 0 1 16 0Z"/><path d="M7 12h3l1.5-4 2 7 1.5-3h2"/></svg>"#
        }
        Icon::Minus => r#"<svg viewBox="0 0 24 24"><path d="M5 12h14"/></svg>"#,
        Icon::PanelLeft => {
            r#"<svg viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16"/></svg>"#
        }
        Icon::Plus => r#"<svg viewBox="0 0 24 24"><path d="M12 5v14"/><path d="M5 12h14"/></svg>"#,
        Icon::Refresh => {
            r#"<svg viewBox="0 0 24 24"><path d="M20 12a8 8 0 0 1-14 5"/><path d="M4 12a8 8 0 0 1 14-5"/><path d="M18 3v4h-4"/><path d="M6 21v-4h4"/></svg>"#
        }
        Icon::Save => {
            r#"<svg viewBox="0 0 24 24"><path d="M5 4h12l2 2v14H5z"/><path d="M8 4v6h8"/><path d="M8 20v-6h8v6"/></svg>"#
        }
        Icon::Reset => {
            r#"<svg viewBox="0 0 24 24"><path d="M5 8v5h5"/><path d="M6.5 16A7 7 0 1 0 5 12.5"/><path d="M5 13l3.5-3.5"/></svg>"#
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
        Icon::X => r#"<svg viewBox="0 0 24 24"><path d="M6 6l12 12"/><path d="M18 6 6 18"/></svg>"#,
    }
}

fn border(color: Color, width: f32, radius: f32) -> Border {
    Border {
        color,
        width,
        radius: Radius::from(radius),
    }
}

#[cfg(test)]
mod tests {
    use super::{sizes, tokens};

    #[test]
    fn light_and_dark_tokens_include_subtle_editor_zebra_rows() {
        assert_ne!(
            tokens::DARK.colors.editor_row_odd,
            tokens::DARK.colors.editor_row_even
        );
        assert_ne!(
            tokens::LIGHT.colors.editor_row_odd,
            tokens::LIGHT.colors.editor_row_even
        );
        assert_eq!(
            tokens::DARK.colors.editor_row_odd,
            tokens::DARK.colors.editor_background
        );
        assert_eq!(
            tokens::LIGHT.colors.editor_row_odd,
            tokens::LIGHT.colors.editor_background
        );
    }

    #[test]
    fn exported_size_tokens_match_central_values() {
        assert_eq!(sizes::TAB_HEIGHT, tokens::SIZES.tab_height);
        assert_eq!(
            sizes::EDITOR_LINE_HEIGHT_RATIO,
            tokens::SIZES.editor_line_height_ratio
        );
        assert_eq!(
            sizes::ACTIVITY_BUTTON_SIZE,
            tokens::SIZES.activity_button_size
        );
    }
}
