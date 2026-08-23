# Architecture

FlokinMD starts as a simple Tauri 2 application:

- `src/`: React + TypeScript frontend.
- `src-tauri/`: Tauri 2 Rust host.
- `docs/`: product, architecture, design-system, and roadmap notes.

MDB-001 intentionally keeps the architecture small. The current Rust layer only boots the Tauri application and does not expose product commands.

## Current Boundary

The UI is a static desktop shell. It does not read Markdown files, scan folders, watch the filesystem, create SQLite indexes, or call external services.

## Future Direction

As the product grows, the intended structure is:

```text
apps/
crates/
docs/
fixtures/
```

The future Rust core should live outside the GUI boundary and should be usable without the Tauri frontend. GUI code may depend on core APIs, but core code must not depend on React, webview details, or Tauri window concerns.

## Data Principle

Markdown files are the source of truth. Future SQLite storage is only a discardable index/cache that can be rebuilt from the Markdown files.
