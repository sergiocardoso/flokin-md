# Architecture

FlokinMD is a native Rust desktop application built with Iced.

- `crates/flokin-app/`: Iced 0.14 desktop GUI.
- `crates/flokin-core/`: product state, Markdown workspace scanning, structured document metadata, collections, mock editor data, and domain logic that can be tested without a window.
- `docs/`: product, architecture, design-system, and roadmap notes.
- `assets/`: future non-code assets.

MDB-001R intentionally keeps the architecture small. The application renders a native shell and mock state only.

## Current Boundary

The UI is a native desktop shell with native folder selection, read-only Markdown discovery, YAML frontmatter metadata parsing, and collection grouping. It does not watch the filesystem, create SQLite indexes, or call external services.

The app crate may depend on the core crate. The core crate must not depend on Iced, windowing, rendering, filesystem dialogs, or other presentation concerns.

## Markdown Scanner

MDB-003 adds `scan_workspace` in `flokin-core`. The scanner recursively discovers real `.md` and `.markdown` files, does not follow symlinks by default, ignores technical directories such as `.git`, `target`, and `node_modules`, and returns structured paths and partial filesystem errors without reading Markdown contents.

## Document And Collection Pipeline

MDB-004 evolves `scan_workspace` into a read-only discovery and analysis pipeline:

```text
filesystem discovery
Markdown file
UTF-8 read
YAML frontmatter parse
Document
Collection
```

The core owns this pipeline. It resolves document titles from `frontmatter.title`, then first Markdown H1, then filename stem; preserves YAML properties in a domain `PropertyValue` enum; resolves logical document type from frontmatter with a small parent-folder fallback; and groups documents into normalized Collections with separate ids and display names. Per-document warnings, such as invalid YAML or invalid UTF-8, are retained without failing the whole workspace.

## File Icons

Explorer filetype metadata is resolved through the app-local file icon helper, which wraps `devicons` instead of calling it directly from views. `AppTheme::Dark` maps to `devicons::Theme::Dark`, and `AppTheme::Light` maps to `devicons::Theme::Light`.

`devicons` glyphs require Nerd Fonts. FlokinMD does not bundle a compatible font yet and must not require manual user font setup, so the current renderer uses a stable colored text fallback derived from the resolved file type while preserving the devicons glyph and color in `FileIconInfo`. A future focused change can bundle a compatible icon font and switch the helper to render the glyphs directly.

## Future Direction

As the product grows, the intended structure remains:

```text
crates/
docs/
assets/
fixtures/
```

The Rust core should remain usable without the GUI. GUI code may depend on core APIs, but core code must not depend on Iced or window concerns.

Explorer, workspace, and inspector are visually separated in MDB-001R. Interactive resizing is intentionally not implemented yet; it should be handled as a focused future UI infrastructure task.

## Data Principle

Markdown files are the source of truth. Future SQLite storage is only a discardable index/cache that can be rebuilt from the Markdown files.
