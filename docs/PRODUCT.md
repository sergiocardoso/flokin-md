# FlokinMD Product

FlokinMD is a local-first desktop application that turns folders containing Markdown files into a visual, structured, queryable database.

The visible product name is **FlokinMD**. The technical project name is **flokin-md**. The bundle identifier is **dev.flokin.md**.

## Principle

Markdown files are the source of truth.

FlokinMD must not convert `.md` files into a proprietary format. Future indexes, caches, schemas, and views exist to help users understand and operate on their Markdown, not to replace it.

## MDB-001R Scope

MDB-001R implements only the native desktop shell:

- Rust stable.
- Iced 0.14 native desktop GUI.
- Cargo workspace with app and core boundaries.
- Dark professional tool UI with menu, toolbar, activity bar, explorer, tabs, editor, bottom panel, inspector, and status bar.

No real filesystem access, Markdown parsing, SQLite, search, graph, watcher, sync, login, AI, MCP, or external API integration is included in this milestone.

## Target Users

People and teams that keep durable knowledge in Markdown folders and want a local visual interface for browsing, organizing, and eventually querying that knowledge.

## Product Promise

FlokinMD should make Markdown folders feel like a database while keeping the files open, readable, portable, and under the user's control.
