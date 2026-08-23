# Design System

MDB-001B defines a token-based design system in Rust at `crates/flokin-app/src/theme.rs`.

The application supports two in-memory themes:

- Dark
- Light

## Colors

- `background`
- `surface`
- `surface_hover`
- `surface_selected`
- `surface_active`
- `elevated_surface`
- `panel`
- `editor_background`
- `editor_gutter`
- `border`
- `border_subtle`
- `text`
- `text_muted`
- `accent`
- `accent_hover`
- `accent_soft`
- `success`
- `warning`
- `danger`
- `selected_text`

## Spacing

- `XXS`
- `XS`
- `SM`
- `MD`
- `LG`
- `XL`
- `XXL`

## Radius

- `XS`
- `SM`
- `MD`
- `LG`

## Typography

- UI font: system default.
- Mono font: system monospace fallback.
- Sizes: menu, label, body, editor, title.

## Icons

Icons are centralized as inline SVG line icons through `theme::Icon` and `widgets::icon`.
They are intentionally consistent in weight and size and avoid adding a large icon dependency during this shell milestone.

## Usage Rules

New screens should reuse these tokens before introducing new values. If a new token is needed, document why it belongs in the shared Rust theme instead of hard-coding one-off colors or dimensions.

The visual direction is a native desktop tool with high information density, clear panes, restrained borders, minimal rounding, technical typography, and a purple accent. Dark mode remains the default; light mode should feel equally refined and readable.
