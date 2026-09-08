<div align="center">

<img src="assets/logo.png" alt="FlokinMD logo" width="320" />

### A local-first database for your Markdown files.

Open a Markdown folder and explore it as documents, data, relations, and project context — without importing anything.

**Markdown is the database. Your files stay your files.**

[Website](https://md.flokin.com.br) · [Releases](https://github.com/sergiocardoso/flokin-md/releases) · [License](LICENSE)

</div>

---

## What is FlokinMD?

**FlokinMD** is an open-source desktop app for working with Markdown as structured data.

Point it at an existing folder containing `.md` or `.markdown` files and FlokinMD helps you understand, query, edit, and organize your content while keeping the Markdown files themselves as the **source of truth**.

No import step. No proprietary vault. No mandatory cloud.

---

## Features

- **Markdown Editor** — edit real files with tabs, Split view, Preview, frontmatter inspection, and safe unsaved-change handling.
- **Collections & DataGrid** — view frontmatter as structured tables and compare many documents at once.
- **AI Context** — recognize and organize Agents, Skills, Specs / SDD, Rules, Prompts, Memory, and other project context files.
- **Relations & Graph** — explore explicit links between documents and understand how files connect.
- **SQL Explorer** — query Markdown through a disposable local SQLite projection.
- **Database Health** — find invalid YAML, inconsistent properties, schema issues, and broken relations.
- **Bulk Edit** — change properties across multiple files with review before writing.
- **History & Safe Undo** — inspect supported structured changes and safely undo them when possible.
- **Local-first** — your files stay on your machine and remain compatible with Git, VS Code, Obsidian, Claude, Codex, and other Markdown tools.

> FlokinMD recognizes AI-related context files, but it does **not** execute AI agents.

---

## Example

A normal Markdown file:

```yaml
---
title: CARF
type: project
status: active
priority: 10
owner: "[[Sergio]]"
---
```

can become a row in a Collection, a SQL record, a node in the relation graph, and part of workspace health checks — while remaining the same Markdown file on disk.

---

## Download

Pre-release builds are available on the [GitHub Releases](https://github.com/sergiocardoso/flokin-md/releases) page.

Release packaging targets:

- Linux
- Windows
- macOS Apple Silicon
- macOS Intel

> FlokinMD is currently pre-release software. Keep important work backed up or under version control while testing.

---

## Build from source

FlokinMD is a native Rust application built with [Iced](https://iced.rs/).

Clone the repository:

```bash
git clone https://github.com/sergiocardoso/flokin-md.git
cd flokin-md
```

Run:

```bash
cargo run
```

Quality checks:

```bash
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

---

## Principles

FlokinMD follows a few simple rules:

1. **Markdown is the source of truth.**
2. Derived indexes and SQL projections must be rebuildable.
3. Multi-file changes must be reviewed before writing.
4. Dirty files and external conflicts must never be silently overwritten.
5. The core product must remain useful without cloud services or AI.

---

## Contributing

Contributions are welcome.

Good ways to help include:

- bug reports;
- documentation improvements;
- UX and accessibility feedback;
- tests;
- small fixes;
- focused pull requests.

Please keep changes focused and run the quality checks before opening a PR.

---

## License

FlokinMD is licensed under the **Apache License 2.0**.

See [LICENSE](LICENSE) for the full license text.

---

## About

FlokinMD is created by **Sérgio Cardoso** as a Flokin project.

[Website](https://sergiocardoso.dev) · [LinkedIn](https://www.linkedin.com/in/sergiocardososp/) · [FlokinMD](https://md.flokin.com.br)

---

<div align="center">

**FlokinMD**

*Markdown is the database.*

</div>
