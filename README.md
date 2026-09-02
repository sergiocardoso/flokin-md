<div align="center">

<img src="assets/logo.png" alt="FlokinMD logo" width="320" />

### A local-first database for your Markdown files.

**Explore, query, validate, relate, visualize, and safely evolve Markdown as structured data — without giving up your files.**

[Getting started](#getting-started) · [Features](#features) · [How it works](#how-it-works) · [Roadmap](#roadmap) · [Contributing](#contributing)

</div>

---

## What is FlokinMD?

**FlokinMD** is an open-source desktop application that treats a folder of Markdown files like a local database.

Point FlokinMD at an existing directory containing `.md` or `.markdown` files and it builds a structured, queryable view of your content — while keeping the Markdown files themselves as the **single source of truth**.

There is no import step, no proprietary database format, no cloud account, and no lock-in.

```text
Your Markdown files
        │
        ▼
┌──────────────────────┐
│      FlokinMD        │
├──────────────────────┤
│ Explorer             │
│ Markdown Editor      │
│ Markdown Preview     │
│ Collections          │
│ DataGrid             │
│ Relations            │
│ Graph                │
│ Schema               │
│ Database Health      │
│ SQL Explorer         │
└──────────────────────┘
        │
        ▼
The same Markdown files
```

FlokinMD is designed around a simple principle:

> **Markdown is the database. FlokinMD is the interface.**

---

## Why FlokinMD?

Markdown is excellent for writing and long-term ownership, but as a collection grows, simple folders and text search are often no longer enough.

Questions start to appear:

- Which documents are missing a required property?
- Which projects have `status: active`?
- Which documents reference this person?
- Are there broken or ambiguous relationships?
- Do all documents in this collection follow the same structure?
- Can I run SQL across my Markdown?
- Can I change a property in many documents without manually opening every file?
- Can I inspect the structure without migrating everything into another tool?

FlokinMD aims to answer those questions while preserving what makes Markdown valuable in the first place:

- plain text;
- local files;
- Git-friendly workflows;
- portability;
- editor independence;
- long-term ownership.

The product direction is inspired by the power and density of tools such as database IDEs and developer editors, but designed specifically for structured Markdown.

---

## Core principles

### Markdown is the source of truth

FlokinMD does not replace your Markdown with an internal proprietary format.

Indexes, SQL projections, relation indexes, schemas inferred from files, and other derived structures are considered **rebuildable projections**.

If FlokinMD disappears tomorrow, your files are still ordinary Markdown.

### Local-first

Your workspace lives on your machine.

The first release does not require:

- an account;
- a cloud service;
- a remote database;
- a synchronization provider.

### Safe by default

Reading and understanding data is easy. Writing to many files is where tools can become dangerous.

Operations that modify multiple documents are designed around:

```text
Select
  ↓
Plan
  ↓
Preview
  ↓
Validate
  ↓
Confirm
  ↓
Safe write
```

FlokinMD should never silently rewrite dozens of documents because of a single accidental click.

### Useful without AI

AI may become an optional enhancement in the future, but FlokinMD is intentionally designed to remain useful without it.

The core product is deterministic, local, inspectable, and based on your files.

---

## Features

### 📁 Open any Markdown workspace

Open an existing folder and FlokinMD recursively discovers Markdown documents.

The scanner currently:

- recognizes `.md` and `.markdown`;
- works recursively;
- ignores common non-content directories such as `.git`, `target`, and `node_modules`;
- does not follow symlinks;
- updates through a filesystem watcher.

No import or conversion is required.

### ✍️ Real Markdown editor

Open documents in real editor tabs.

Current editor capabilities include:

- multiple tabs;
- dirty state;
- `Ctrl+S`;
- `Ctrl+W`;
- protection when closing unsaved documents;
- external-change conflict handling;
- live filesystem integration;
- stable per-tab editor state.

FlokinMD does **not** autosave behind your back.

### 👁️ Markdown preview

Documents can be viewed in three modes:

- **Edit**
- **Split**
- **Preview**

Split mode places the Markdown source and rendered document side by side.

The preview is generated from the current editor buffer, so unsaved edits can be previewed without forcing a save.

YAML frontmatter remains part of the source, while the rendered preview focuses on the Markdown body.

### 🗂️ Collections

FlokinMD groups documents into Collections.

A document can explicitly define its type:

```yaml
---
title: CARF
type: project
status: active
---
```

When no explicit `type` exists, folder-based fallback rules can be used.

Collections provide a database-like way to explore groups of Markdown documents.

### 📊 DataGrid

Collections can be viewed as structured tables.

Frontmatter properties become columns and documents become rows.

The DataGrid supports database-oriented rendering such as:

- row numbers;
- sorting;
- typed values;
- boolean visualization;
- missing/null values;
- dense desktop layout;
- multi-selection infrastructure for bulk operations.

Example:

```text
#   title       status      priority   published
1   CARF        active      10         ✓
2   CVM         paused      20         ✕
3   Notes       —           —          —
```

### 🔗 Relations

FlokinMD supports explicit Markdown relationships using wikilink-style values.

Single relation:

```yaml
owner: "[[Sergio]]"
```

Multiple values:

```yaml
participants:
  - "[[Sergio]]"
  - "[[Maria]]"
```

Plain strings remain plain strings:

```yaml
owner: Sergio
```

That distinction is intentional.

Relations are classified as:

- **Resolved**
- **Unresolved**
- **Ambiguous**

FlokinMD never chooses an arbitrary destination when multiple documents match.

### 🕸️ Relation graph

Relationships can be explored visually as a graph.

The graph is derived from the same relation model used throughout the application.

Current graph capabilities include:

- document nodes;
- directed relation edges;
- selection;
- Inspector integration;
- open document from graph;
- zoom;
- pan;
- fit graph;
- center selected node;
- self-relations;
- cycles.

The graph does not invent relationships based on similarity or AI inference.

### 🧬 Inferred schema

FlokinMD can infer the structure of a Collection automatically.

Given documents such as:

```yaml
---
title: CARF
status: active
priority: 10
published: true
---
```

FlokinMD can infer:

```text
FIELD       TYPE       REQUIRED   PRESENT
title       String        ✓         7/7
status      String        ✕         4/7
priority    Integer       ✕         3/7
published   Boolean       ✕         2/7
```

Supported observed types include:

- String
- Integer
- Float
- Boolean
- Array
- Object
- Relation
- Mixed
- Null / Unknown

FlokinMD distinguishes between a property that is **missing** and one that is explicitly `null`.

### 📐 Optional explicit schema

Schema inference requires zero configuration.

For stricter validation, a workspace may optionally contain:

```text
flokin.schema.yaml
```

Example:

```yaml
version: 1

collections:
  projects:
    fields:
      title:
        type: string
        required: true

      status:
        type: string
        required: true

      priority:
        type: integer
        required: false
```

The difference is important:

```text
Inferred schema:
"priority appears to be Integer"

Explicit schema:
"priority must be Integer"
```

The explicit schema is completely optional.

FlokinMD can also generate an initial `flokin.schema.yaml` from the currently inferred structure through the UI.

It is never created silently.

### ❤️ Database Health

Database Health provides a consolidated view of structural problems in the workspace.

It can identify issues related to:

#### Parsing

- invalid YAML frontmatter;
- unreadable files;
- parser diagnostics.

#### Schema

- missing required fields;
- type mismatches;
- invalid explicit schema;
- unsupported schema versions;
- undeclared fields;
- inconsistent observed types.

#### Relations

- unresolved relationships;
- ambiguous relationships;
- broken path relations.

A healthy workspace is not one with a fake score such as `92/100`.

FlokinMD reports actual facts:

```text
Errors:    2
Warnings:  4
Healthy:  37 documents
```

Issues can be traced back to the relevant Markdown document.

### 🧮 SQL Explorer

FlokinMD creates a **disposable in-memory SQLite projection** of the current Markdown workspace.

Architecture:

```text
Markdown
   ↓
Document Store
   ↓
SQLite :memory:
   ↓
SQL Explorer
```

Each Collection becomes a table.

Example:

```sql
SELECT
    title,
    status,
    priority
FROM projects
WHERE status = 'active'
ORDER BY priority;
```

Internal helper columns include values such as:

```text
_path
_file_name
```

Frontmatter arrays and objects can be represented as JSON text when projected to SQLite.

The current SQL Explorer is **read-only**.

The SQLite database is derived state and is never the source of truth.

### 🔍 Search

Use global search to find documents by information such as:

- title;
- path;
- frontmatter;
- content.

`Ctrl+K` opens the search experience.

Search integrates with the current in-memory document model and filesystem watcher.

### 🌓 Light and Dark themes

FlokinMD includes semantic Light and Dark themes backed by centralized design tokens.

The UI is being designed as a dense desktop productivity tool rather than a browser-like SaaS interface.

---

## How it works

The high-level architecture follows one important direction of data flow:

```text
Filesystem
    │
    ▼
Markdown Scanner / Parser
    │
    ▼
Document Store
    │
    ├──────────────► Collections
    │
    ├──────────────► DataGrid
    │
    ├──────────────► SchemaCatalog
    │
    ├──────────────► RelationIndex
    │                   │
    │                   └────► Graph
    │
    ├──────────────► SQLite Projection
    │                   │
    │                   └────► SQL Explorer
    │
    └──────────────► Database Health
```

The filesystem remains authoritative.

Derived state should always be reconstructable.

---

## A Markdown document in FlokinMD

Example project:

```markdown
---
title: CARF
type: project
status: active
priority: 10
owner: "[[Sergio]]"
published: false
---

# CARF

A project represented entirely as Markdown.

## Notes

The file remains readable and editable in any Markdown editor.
```

Another document can reference it:

```markdown
---
title: CARF Daily
type: meeting
project: "[[CARF]]"
owner: "[[Sergio]]"
participants:
  - "[[Sergio]]"
  - visitante
---

# CARF Daily

Daily project meeting.
```

FlokinMD can then derive:

```text
CARF Daily ──project──► CARF
     │
     └────owner───────► Sergio
```

without replacing either file.

---

## Repository structure

FlokinMD is a Rust workspace.

```text
flokin-md/
├── assets/
│   └── logo.png
│
├── crates/
│   ├── flokin-app/
│   │   └── Desktop UI built with Iced
│   │
│   └── flokin-core/
│       └── Domain models and application logic
│
├── docs/
│   ├── PRODUCT.md
│   ├── ARCHITECTURE.md
│   ├── DESIGN_SYSTEM.md
│   ├── ROADMAP.md
│   └── ...
│
├── Cargo.toml
└── README.md
```

A core architectural rule is that `flokin-core` should not depend on Iced UI types.

The domain should remain testable independently of the desktop interface.

---

## Technology

FlokinMD is intentionally a native Rust application.

### Main stack

- **Rust**
- **Iced 0.14** — desktop UI
- **SQLite / rusqlite** — disposable in-memory SQL projection
- **YAML frontmatter**
- filesystem watcher
- native file dialogs

There is no React, Electron, Tauri, Node.js runtime, or browser WebView in the application.

---

# Getting started

> FlokinMD is currently under active development and is not yet a stable release.

## Requirements

Install a recent stable Rust toolchain using `rustup`.

You can confirm your environment with:

```bash
rustc --version
cargo --version
```

Clone the repository:

```bash
git clone <YOUR-REPOSITORY-URL>
cd flokin-md
```

> Replace `<YOUR-REPOSITORY-URL>` with the public GitHub URL after the repository is published.

## Run the application

```bash
cargo run
```

When FlokinMD opens:

1. click **Open Folder**;
2. select a folder containing Markdown files;
3. wait for indexing;
4. explore the workspace using the Activity Bar.

You do not need to prepare the folder specifically for FlokinMD.

An ordinary Markdown directory is enough.

## Development checks

Before submitting changes, run:

```bash
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace
```

All checks should pass.

For development:

```bash
cargo run
```

---

## Development philosophy

When adding a feature, prefer this direction:

```text
Domain model
     ↓
Pure/testable projection or service
     ↓
Application state/message integration
     ↓
Iced view
```

Avoid placing important business rules directly inside rendering code.

Examples:

```text
Documents + Relations
        ↓
RelationIndex
        ↓
GraphProjection
        ↓
Graph View
```

and:

```text
Documents + Schema + Relations + Diagnostics
        ↓
HealthProjection
        ↓
Health View
```

This makes the application easier to test and keeps UI changes from changing domain behavior accidentally.

---

## Safety philosophy for writes

FlokinMD is intentionally conservative about writes.

For a future or current multi-file operation, the preferred model is:

```text
User intent
   ↓
Operation
   ↓
Immutable plan
   ↓
Preview
   ↓
Preflight validation
   ↓
Explicit confirmation
   ↓
Staged safe writes
   ↓
Filesystem watcher
   ↓
Derived state rebuild
```

Important rules:

- never silently overwrite dirty editor buffers;
- never apply a stale preview;
- avoid rewriting unrelated Markdown content;
- preserve document body whenever possible;
- do not choose ambiguous relation targets automatically;
- keep derived indexes disposable.

---

# Roadmap

The first public version is intentionally focused.

## Implemented

- [x] Native Rust/Iced desktop shell
- [x] Open Markdown folder
- [x] Recursive Markdown scanner
- [x] Typed YAML frontmatter parsing
- [x] Documents and Collections
- [x] DataGrid
- [x] Document Inspector
- [x] Filesystem watcher
- [x] Global search
- [x] Read-only SQL Explorer
- [x] Explicit Markdown relations
- [x] Real Markdown editor and tabs
- [x] Markdown Preview / Split View
- [x] Relation Graph
- [x] Inferred Schema
- [x] Optional explicit `flokin.schema.yaml`
- [x] Explicit schema onboarding/generation
- [x] Database Health
- [x] Light/Dark design system

## In progress / before v0.1

- [ ] Bulk Edit with mandatory preview
- [ ] Safe multi-file mutation flow
- [ ] SQL Write Preview
- [ ] History / Undo
- [ ] Final UI/UX polish
- [ ] Linux packaging
- [ ] First public release

## Deliberately deferred

These are interesting directions, but they are **not blockers for v0.1**:

- AI assistance
- MCP integration
- cloud sync
- collaboration
- Git UI
- SQL autocomplete
- plugin system
- Mermaid
- advanced schema constraints
- mobile application

The goal is to finish a coherent, safe first version before expanding the scope.

---

# Contributing

FlokinMD is my **first open-source project**, and one of the goals of this repository is not only to build the application, but also to build a healthy project that people feel comfortable contributing to.

Contributions are welcome.

You do **not** need to be an expert in Rust or Iced to help.

Useful contributions can include:

- bug reports;
- reproduction steps;
- documentation improvements;
- UX feedback;
- accessibility suggestions;
- test cases;
- performance investigations;
- small UI fixes;
- Rust refactors;
- feature discussions.

## Before opening a pull request

Please:

1. create or reference an issue when the change is significant;
2. keep the change focused;
3. avoid combining unrelated refactors and features;
4. preserve Markdown-as-source-of-truth behavior;
5. add tests for domain behavior when practical;
6. run the full quality checks;
7. explain the user-facing impact in the PR description.

Run:

```bash
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace
```

---

## Good first contributions

If this is also one of your first open-source contributions, that is completely fine.

Good starting points include:

- documentation corrections;
- clearer error messages;
- missing tests;
- accessibility improvements;
- small layout inconsistencies;
- reproducible bug fixes;
- empty-state improvements;
- tooltips;
- keyboard navigation improvements.

Issues suitable for new contributors can be labeled:

```text
good first issue
```

and:

```text
help wanted
```

as the project grows.

---

## Reporting a bug

A useful bug report should include:

- operating system;
- FlokinMD version or commit;
- exact steps to reproduce;
- expected behavior;
- actual behavior;
- screenshot or short recording if relevant;
- a minimal Markdown fixture when possible;
- whether the problem happens after restarting the app.

Please remove confidential information before uploading Markdown examples.

---

## Suggesting a feature

Before requesting a large feature, consider the product principles:

1. Does it preserve Markdown as the source of truth?
2. Does it work locally or have a clear local-first story?
3. Can derived state be rebuilt?
4. Does it avoid unnecessary lock-in?
5. Is the behavior safe and understandable?
6. Does it make FlokinMD better at treating Markdown as structured data?

Feature requests that fit those principles are especially welcome.

---

## Pull request scope

Small and focused pull requests are preferred.

Good:

```text
fix: keep active document selected after returning from SQL
```

Good:

```text
feat: add filter for unresolved relations
```

Harder to review:

```text
refactor the UI, rewrite relation handling, add sync and fix 12 bugs
```

Focused changes make review, testing, and future maintenance much easier.

---

# Project status

FlokinMD is currently **pre-release software**.

It is already functional enough for development and experimentation, but APIs, UI details, schema rules, and file-writing behavior may still change before the first public release.

For important data, always keep a backup or use Git while testing pre-release versions.

The long-term goal is for safe file handling to be one of the strongest parts of the application.

---

## What FlokinMD is not

FlokinMD is not trying to become:

- a hosted note-taking service;
- a proprietary Markdown vault;
- a replacement for Git;
- an AI-first editor;
- another Electron wrapper around a text area.

It is trying to become:

> **a serious local desktop data tool for people whose database happens to be Markdown.**

---

# Long-term vision

The long-term direction is to make structured Markdown increasingly powerful without sacrificing portability.

Possible future areas include:

```text
Markdown
   │
   ├── Schema enforcement
   ├── Relations / JOINs
   ├── Graph
   ├── SQL
   ├── Database Health
   ├── Bulk migrations
   ├── Safe SQL writes
   ├── History / Undo
   ├── Git-native workflows
   └── Optional AI / MCP
```

Every one of those features should respect the same rule:

**your Markdown remains yours.**

---

# Support the project

If you find FlokinMD useful, some of the best ways to help an early open-source project are:

- ⭐ star the repository;
- 🐛 report reproducible bugs;
- 💡 share focused feature ideas;
- 🧪 test pre-release builds;
- 📝 improve documentation;
- 🔧 contribute a focused pull request;
- 📣 tell other Markdown-heavy developers and knowledge workers about it.

Early feedback is especially valuable while the architecture and product conventions are still being shaped.

---

# A note from the author

FlokinMD is my first open-source project.

I am building it because I like Markdown for exactly the reasons that often make sophisticated tools difficult: it is simple, portable, inspectable, and belongs to the user.

The experiment behind FlokinMD is:

> What if a folder full of Markdown could gain many of the capabilities of a database IDE without stopping being a folder full of Markdown?

The project is still evolving, and there will be rough edges along the way. Clear bug reports, thoughtful criticism, code reviews, and contributions are genuinely valuable.

If you decide to try the project or contribute to it: thank you.

---

<div align="center">

**FlokinMD**

*Markdown is the database.*

</div>
