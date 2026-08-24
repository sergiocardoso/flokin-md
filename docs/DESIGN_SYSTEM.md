# Design System

UI-TOKENS-001 centralizes the visual source of truth in Rust at
`crates/flokin-app/src/theme/tokens.rs`. `theme.rs` exposes compatibility
helpers and widget style functions that consume those tokens.

The application supports two in-memory themes:

- Dark
- Light

## Colors

Color tokens live under `ThemeTokens.colors` and include application surfaces,
text colors, accent states, status colors, editor colors, grid colors, menu
backgrounds, and overlay backdrop colors. Dark and Light palettes are defined
side by side as `tokens::DARK` and `tokens::LIGHT`.

## Spacing

Spacing uses a compact scale under `ThemeTokens.spacing`: `xxs`, `xs`, `sm`,
`md`, `lg`, `xl`, and `xxl`.

## Radius

Radius tokens live under `ThemeTokens.radius`: `small`, `medium`, and `large`.

## Typography

- UI font: system default.
- Mono font: system monospace fallback.
- Sizes: body, small, label, heading, editor, editor line number, grid, and menu.

## Sizes

Important dimensions live under `ThemeTokens.sizes`, including activity bar and
icon sizes, toolbar height, tab height, editor line height, editor gutter width,
data grid density, sidebar defaults, splitter hit area, menu sizes, search
overlay sizes, and dialog width.

## Icons

Icons are centralized as inline SVG line icons through `theme::Icon` and `widgets::icon`.
They are intentionally consistent in weight and size and avoid adding a large icon dependency during this shell milestone.

## Usage Rules

New screens should reuse these tokens before introducing new values. If a new token is needed, document why it belongs in the shared Rust theme instead of hard-coding one-off colors or dimensions.

The visual direction is a native desktop tool with high information density, clear panes, restrained borders, minimal rounding, technical typography, and a purple accent. Dark mode remains the default; light mode should feel equally refined and readable.

## Data Grids

Collection tables and SQL result grids share the view-level data grid language in `views::data_grid`:

- compact 28px rows with a subtle odd/even alternation;
- a fixed, muted row-number gutter that is not part of the data model;
- compact headers with a subtle bottom separator;
- type-aware alignment: text left, numbers right, booleans centered;
- `—` for NULL or missing values and `✓`/`✕` for booleans;
- hover and selected-row surfaces take precedence over zebra backgrounds.

Vertical separators remain lighter than horizontal row separators. Data grid tokens are defined once in `theme/tokens.rs` and must work in both Dark and Light themes.

## Markdown Editor

The editor consumes `editor_background`, `editor_row_odd`,
`editor_row_even`, `editor_gutter`, `editor_current_line`, and
`editor_selection` tokens. UI-TOKENS-001 renders a subtle viewport-row
background behind the Markdown `text_editor` and mirrors it in the line-number
gutter while keeping Iced's editor widget intact for cursor, selection,
copy/paste, undo/redo, scrolling, and per-tab state. Iced 0.14 does not expose
a public text-editor scroll offset, so the zebra layer is intentionally a safe
visual reading aid rather than a document-line-aware renderer after arbitrary
internal scrolling.

## Relation Graph

MDB-013 adds graph-specific tokens under `ThemeTokens.colors` and
`ThemeTokens.sizes` for the native canvas view. The Graph view consumes
centralized values for background, node surfaces, selected/hover states, node
borders, edge colors, unresolved/ambiguous warnings, node dimensions, node font
size, and edge label font size. Graph views should not hard-code RGB colors in
view code.

MDB-013A extends those tokens for navigation polish: graph toolbar surfaces and
button states, disabled button color, zoom badge background/text, canvas dot
grid color, node shadow color, toolbar button size, zoom badge width, and grid
step are centralized in `crates/flokin-app/src/theme/tokens.rs`.
