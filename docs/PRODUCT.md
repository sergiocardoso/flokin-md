# FlokinMD Product

FlokinMD is a local-first desktop application that turns folders containing Markdown files into a visual, structured, queryable database.

The visible product name is **FlokinMD**. The technical project name is **flokin-md**. The bundle identifier is **dev.flokin.md**.

## Principle

Markdown files are the source of truth.

FlokinMD must not convert `.md` files into a proprietary format. Future indexes, caches, schemas, and views exist to help users understand and operate on their Markdown, not to replace it.

## MDB-001 Scope

MDB-001 implements only the desktop shell:

- Tauri 2 desktop app.
- React, TypeScript, Vite, and pnpm frontend.
- Light theme with a premium desktop-app look.
- Static visual controls for search, filters, settings, navigation, recent folders, CTAs, and status.

No real filesystem access, Markdown parsing, SQLite, search, graph, watcher, sync, login, AI, MCP, or external API integration is included in this milestone.

## Target Users

People and teams that keep durable knowledge in Markdown folders and want a local visual interface for browsing, organizing, and eventually querying that knowledge.

## Product Promise

FlokinMD should make Markdown folders feel like a database while keeping the files open, readable, portable, and under the user's control.
