# Architecture

FlokinMD is a native Rust desktop application built with Iced.

- `crates/flokin-app/`: Iced 0.14 desktop GUI.
- `crates/flokin-core/`: product state, Markdown workspace scanning, structured document metadata, collections, editor tab/buffer state, and domain logic that can be tested without a window.
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

The loaded `Document` keeps both the parsed Markdown body used by search and the full source text used by the read-only Document Viewer. The GUI does not reread files during rendering; watcher updates replace the in-memory `Document`, which updates the viewer, inspector, search input data, table projections, and relations from the same selected document identity.

## Table Projection

MDB-005 adds a read-only `TableModel` projection in `flokin-core`. It derives deterministic columns and typed cells from in-memory `Document` values for a selected Collection, keeps `Title` as the first column, skips redundant internal properties such as `title` and `type`, infers predominant column types, and applies basic typed sorting without depending on Iced or any database cache.

## Document Inspector Projection

MDB-006 adds a read-only `DocumentInspector` projection in `flokin-core`. The GUI stores only the selected Markdown document path as the document selection identity and resolves the current `Document` from in-memory scan results. The Inspector projection renders real title, typed frontmatter properties, tags, parser warnings, and lightweight filesystem metadata without coupling the core crate to Iced.

## Relations

MDB-011 adds a disposable relation index derived from the loaded `Document` values.

```text
Documents
RelationIndex
outgoing / incoming
Inspector
```

Relations are explicit only: a frontmatter string or array item must use wikilink syntax such as `[[CARF]]` or `[[projects/carf.md]]`. Plain strings such as `project: CARF` remain normal string values and are not interpreted as relations.

The frontmatter property name is the relation type. Relation targets resolve first by explicit relative path when the wikilink clearly names a path, then by exact document title. A unique title match is resolved, no match is unresolved, and duplicate title matches are ambiguous; the core never chooses an arbitrary document for ambiguous relations.

`RelationIndex` is kept in `flokin-core` and contains outgoing and incoming lookups for future Graph and Database Health work, but it is not persisted and does not write Markdown. Watcher updates, full scans, and workspace changes rebuild the index from the current in-memory Document Store. The Iced Inspector only renders and navigates relation projections using the existing selected document identity.

## Markdown Editor And Tabs

MDB-012 replaces the read-only center source viewer with real Markdown tabs and editable buffers.

```text
Document
↕
EditorBuffer
↓ Save
Filesystem
↓ watcher / parser
Document Store
```

`flokin-core` owns `EditorState` and `EditorTab` so tab identity, active document selection, dirty state, close confirmation, and external-change conflicts can be tested without Iced. Tabs are keyed by the real document path, not by filename, so duplicate filenames remain distinct. Each tab stores a `buffer`, `saved_content`, and dirty flag derived by direct comparison between those strings.

Markdown remains the source of truth. Save writes the active tab buffer back to the same Markdown file via a temporary file in the same directory followed by rename. After a successful save, the tab updates `saved_content` and becomes clean; the app then feeds the changed path through the existing workspace update pipeline so frontmatter, Collections, Inspector, Relations, Search, and SQL projection converge from the parser instead of editor-specific shortcuts.

Watcher updates synchronize open tabs from loaded `Document.source_content`. A clean tab adopts external content immediately and updates `saved_content`. A dirty tab never has its buffer overwritten silently; instead it records an external conflict and the UI offers reload-from-disk or keep-local-change actions. Workspace changes and window close requests ask for confirmation when dirty tabs exist.

EDITOR-STABILITY-001 tightens this path so filesystem updates are serialized in the app. Watcher events are appended to a pending event queue; only one `workspace_update_from_events` task mutates the `Document Store` at a time, and queued events are processed after the running update completes. This prevents same-workspace update results from being applied out of order while still preserving events for different files.

`workspace_update_from_events` resolves Markdown paths against the final filesystem state when processing a debounced batch. If a transient remove and upsert both mention the same existing file, the result is a single upsert; only paths missing at processing time become removals. Dirty editor tabs are not closed by removal events. They keep their local buffer and record a structured external conflict (`Modified` or `Deleted`). Choosing Keep Local records the already-seen external state so unrelated watcher updates do not recreate the same conflict banner.

The Iced editor widget state is app-owned per document path. `flokin-app` keeps a `text_editor::Content` map keyed by real document path, while `flokin-core` keeps only GUI-independent buffers and dirty/conflict state. Watcher synchronization updates only the `text_editor::Content` entries for changed paths, so an update for `b.md` does not rebuild the active editor state for `a.md`.

## File Watcher

MDB-007 adds a read-only filesystem watcher in `flokin-app` using `notify`. The app watches the current workspace recursively, debounces noisy native filesystem events, and translates backend-specific events into core-owned `WorkspaceEvent` values:

```text
notify event
WorkspaceEvent
workspace update
Document
Collection
TableModel / Inspector / Explorer
```

The core crate does not depend on `notify` or Iced. It owns the shared Markdown/ignore policy and applies incremental workspace updates by reparsing changed Markdown files, removing missing paths, rebuilding collections, and refreshing explorer projections from in-memory documents. Full workspace scan remains available as a manual fallback for ambiguous directory-level changes.

## Search

MDB-008 adds in-memory workspace search in `flokin-core`. The scanner keeps the parsed Markdown body on each `Document`, and the search service queries the already loaded `Document` values without rereading files per keystroke. Search covers document title, file name, relative path, frontmatter property names, frontmatter string values, and Markdown body text.

The GUI owns only interaction concerns: opening/focusing the toolbar search field, debounce, keyboard navigation, popup rendering, and selecting a result. Search state is centralized as `SearchState` on `ShellModel`; result selection still resolves to the single real selected document path used by Table View and Inspector.

The current backend is a deterministic O(n) in-memory scan with simple scoring and snippets. SQLite FTS is an intended future replacement for the search backend/cache, but MDB-008 does not introduce SQLite, SQL, or a persistent search index.

## SQL Projection And Explorer

MDB-009 adds a disposable SQLite projection in `flokin-core` and a read-only SQL Explorer in `flokin-app`.

```text
Markdown files
Document Store
disposable SQLite projection
SQL Explorer
```

Markdown files remain the source of truth. SQLite is an in-memory derived projection/cache built only from the currently loaded `Document` values; FlokinMD does not create a `.db` file in the workspace or user folders, and MDB-009 never writes query results back to Markdown.

The projection maps each real Collection to a deterministic SQL table, normalizes and safely quotes SQL identifiers, resolves table/column collisions deterministically, adds standard `title`, `_path`, and `_file_name` columns, and preserves scalar types where practical. Arrays and objects are stored as valid JSON text for this milestone.

MDB-009 accepts only read-only single-statement SQL. The core validates execution with SQLite statement read-only checks and fails closed for write attempts or multiple statements. The UI executes queries outside rendering, displays result metadata and friendly SQL errors, and limits rendered rows.

After watcher updates, manual reindex, or workspace changes, the SQL projection is rebuilt from the current in-memory Document Store. A full rebuild is intentionally acceptable in MDB-009 because it prioritizes correctness and keeps the architecture simple; the projection boundary can be optimized incrementally later without changing Markdown as the source of truth.

## SQL Autocomplete

MDB-010 adds contextual SQL autocomplete without introducing an LSP or coupling the core crate to Iced.

```text
SqlCatalog
completion engine
SQL Editor popup
```

`flokin-core` owns the completion model and uses `SqlCatalog` as the source of truth for real SQL table names, normalized column names, and column types. The engine performs lightweight context analysis for keywords, tables, columns, aliases, dotted alias access, and a small SQLite function set. It returns replacement ranges and insertion text so the GUI can replace only the current fragment.

`flokin-app` owns editor interaction state: popup visibility, selected suggestion, keyboard navigation, and insertion into the existing Iced text editor. Because watcher updates already rebuild the disposable projection catalog, autocomplete suggestions update from the latest catalog without opening a separate database or rereading Markdown.

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
