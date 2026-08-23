# Roadmap

This file records the intended milestones.

- MDB-001R Native Desktop Shell — Iced
- MDB-002 Abrir pasta — implemented: the toolbar opens the native directory picker, stores the selected `PathBuf` in memory as the current workspace, updates Explorer and status bar, and treats cancellation as a no-op. It does not scan, index, read, or persist workspace contents.
- MDB-003 Markdown Scanner — implemented: after a folder is selected, the app runs a read-only recursive scan outside the render flow, discovers real `.md` and `.markdown` files, ignores `.git`, `target`, and `node_modules`, reports counts and partial access errors, and renders only the discovered Markdown tree in Explorer. It does not parse Markdown contents, index SQLite, search, watch files, or read frontmatter.
- MDB-004 Collections — implemented: Markdown files are read read-only after discovery, YAML frontmatter is parsed into structured properties, titles and logical document types are resolved, normalized Collections are built with real counts, Explorer shows Files plus Collections, and selecting a Collection renders a simple real document list. It does not implement SQLite, search, schemas, editing, backlinks, graph, watcher, or advanced Table View.
- MDB-005 Table View — implemented: selecting a Collection opens a read-only database-style table built from real Markdown Documents. Title is always first, frontmatter properties become deterministic typed columns, missing/null values render safely, arrays and objects use compact display, row selection points to the real Document, and column headers provide basic ASC/DESC sorting. It does not edit Markdown, persist views, use SQLite, filter, resize columns, or open a document inspector.
- MDB-006 Document Inspector — implemented: the right Inspector is driven by the single selected Markdown document identity, renders real title, typed frontmatter properties, tags, filesystem metadata, and parser warnings, and shows an empty state when no document is selected. Table rows and Explorer files converge on the same selection, workspace changes clear selection, and Collection changes clear stale document selection. It remains read-only and does not implement editing, Markdown writes, SQLite, search, relations, graph, watcher, Git, MCP, or AI.
- MDB-007 File Watcher
- MDB-008 Search
- MDB-009 SQL Explorer
- MDB-010 SQL Autocomplete
- MDB-011 Relations
- MDB-012 Graph
- MDB-013 Schema
- MDB-014 Database Health
- MDB-015 Bulk Edit
- MDB-016 SQL Write Preview
- MDB-017 History / Undo
- MDB-018 Packaging
