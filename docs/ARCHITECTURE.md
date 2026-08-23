# Architecture

FlokinMD is a native Rust desktop application built with Iced.

- `crates/flokin-app/`: Iced 0.14 desktop GUI.
- `crates/flokin-core/`: product state, mock data, and domain logic that can be tested without a window.
- `docs/`: product, architecture, design-system, and roadmap notes.
- `assets/`: future non-code assets.

MDB-001R intentionally keeps the architecture small. The application renders a native shell and mock state only.

## Current Boundary

The UI is a native desktop shell. It does not read Markdown files, scan folders, watch the filesystem, create SQLite indexes, or call external services.

The app crate may depend on the core crate. The core crate must not depend on Iced, windowing, rendering, filesystem dialogs, or other presentation concerns.

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
