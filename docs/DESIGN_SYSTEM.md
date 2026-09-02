# Design System

UI-POLISH-001 keeps the Rust theme as the visual source of truth. Tokens live in
`crates/flokin-app/src/theme/tokens.rs`; `theme.rs` exposes semantic helpers and
widget styles that views consume.

FlokinMD is a dense native desktop tool. The interface should feel calm,
structured, local-first, and productive rather than like a marketing website.
Markdown files remain the product's source of truth; the visual system must not
imply features that do not exist.

## Visual Reference Principles

The UI-POLISH reference images informed polish principles, not branding:

- compact activity/navigation bars with centered line icons;
- one top shell with brand, menus, central search, and right-side controls;
- a simple FlokinMD mark beside the product name, drawn in the app icon system;
- panels differentiated by subtle surface levels before visible borders;
- selected states using a soft purple surface, not heavy outlines;
- document tabs with enough height and horizontal padding to read as product UI;
- editor and preview panes that feel connected in split mode;
- inspector sections with muted labels, primary values, thin separators, and
  relation rows that feel clickable without becoming web-style links;
- status bar density with small separators and status indicators.

## Color Tokens

`ThemeTokens.colors` is semantic. Prefer the role over the literal shade:

- backgrounds: `app_background`, `activity_bar_background`,
  `sidebar_background`, `content_background`, `inspector_background`,
  `top_bar_background`, `status_bar_background`;
- surfaces: `surface`, `surface_elevated`, `surface_hover`,
  `surface_pressed`, `surface_selected`, `surface_active`;
- borders: `border_subtle`, `border`, `border_strong`, `focus_ring`;
- text: `text`, `text_muted`, `text_disabled`, `text_inverse`;
- accent: `accent`, `accent_hover`, `accent_pressed`, `accent_soft`,
  `accent_border`, `accent_text`;
- status: `success`, `success_soft`, `warning`, `warning_soft`, `error`,
  `error_soft`, `info`, `info_soft`;
- editor/preview/grid/graph groups remain specialized where the rendering
  surface needs distinct behavior.

Light mode uses off-white app chrome, white content, subtle blue-gray borders,
and restrained violet selection. Dark mode uses charcoal layers, low-contrast
borders, and the same violet accent family. Neither theme should be treated as
an inverted copy of the other.

## Spacing

Spacing uses a compact scale: `xxs`, `xs`, `sm`, `md`, `lg`, `xl`, `xxl`.
Use smaller values inside controls and rows; use `lg` or `xl` only to separate
major regions. Avoid adding one-off padding in views unless the value is tied to
a fixed-format widget.

## Radius

Radius is intentionally limited:

- `small`: tight controls and badges;
- `medium`: panels, inputs, selected rows, segmented controls;
- `large`: overlays and chips.

Do not introduce arbitrary radii per view. The app should remain desktop-like,
not mobile or SaaS-card heavy.

## Typography

Roles are intentionally few:

- `TITLE`: pane titles, document titles, important view headings;
- `BODY`: default UI copy and row labels;
- `SMALL`/`LABEL`: metadata, section labels, counters, shortcuts;
- `EDITOR`: Markdown and SQL editors;
- `EDITOR_LINE_NUMBER`: gutter line numbers;
- `GRID`: dense tabular data;
- `MENU`: menu bar and menu rows.

Use color, spacing, and weight implied by the widget before adding new sizes.
Monospace is for paths, code, SQL, line numbers, counters, and tabular values.

## Sizes

Structural dimensions live under `ThemeTokens.sizes`: activity bar width,
activity/icon buttons, top toolbar height, search width, tab height, document
header height, editor gutter width, data grid row/header heights, sidebar
defaults and limits, splitters, overlay sizes, graph node sizes, and SQL pane
height bounds.

These sizes protect desktop density and make resizing predictable across
1280x720, 1440x900, and 1920x1080 windows.

## Components

Shared components should be preferred before local styling:

- `widgets::toolbar_button` for labeled toolbar actions;
- `widgets::tab_button` for simple tabs;
- `widgets::section_title` for muted uppercase section labels;
- `widgets::icon` for all app UI line icons;
- theme styles for `button_primary`, `button_toolbar`, `button_selected`,
  `button_accent_outline`, `button_ghost`, `button_tree`,
  `button_tree_selected`, `button_tab`, `button_tab_selected`,
  `segmented_control`, `document_surface`, `document_header`,
  `inspector_panel`, `top_bar`, `activity_bar`, `status_bar`, and
  `status_dot`.

Icon-only controls must include tooltips where the meaning is not obvious.
Avoid emoji as UI icons; use the centralized SVG icon system.

## Shell

The shell hierarchy is:

1. top app chrome and toolbar;
2. activity bar and resizable panels;
3. workspace/document content;
4. inspector;
5. compact status bar.

Activity items are icon-only with centered alignment, soft selected background,
and bottom settings. Search belongs visually to the center of the top shell and
keeps the `Ctrl+K` shortcut hint inside the same rounded field.

## Explorer

Explorer rows are readable rather than compressed, indented by tree depth, and
use soft hover/selected states. Workspace name is primary, path and counts are
secondary. Toolbar actions are compact and should only expose real
functionality.

## Editor And Preview

Tabs are compact with soft selected state and subdued close controls. The
document header groups file icon, filename, path, view mode segmented control,
and Save. Editor panes preserve the real Iced editor; zebra rows and gutters
are visual aids only. Preview uses readable padding and Markdown typography
while staying inside the desktop app surface.

## Inspector

Inspector sections use a section heading, property rows, and subtle dividers.
Property labels are muted; values are primary. Relation values use accent text
and a small arrow affordance, with hover through `button_ghost`.

## Data, Schema, Health, Graph, SQL

DataGrid and SQL results share grid row, gutter, header, zebra, hover, and
selection tokens. Schema and Health use the same document surface and compact
toolbar patterns. Graph keeps its canvas engine but aligns toolbar groups,
buttons, badges, canvas, nodes, and sidebar with the shared palette.

## Iced Constraints

Iced 0.14 does not expose every low-level style hook used by web UIs. In
particular, Markdown editor zebra rows are viewport-based because the public
text editor does not expose scroll offset. Use safe visual layers and shared
styles rather than reimplementing editor behavior.
