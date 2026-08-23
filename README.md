# FlokinMD

Local-first desktop app that turns Markdown folders into a visual, structured, queryable database.

FlokinMD is a native Rust desktop application built with Iced.

MDB-001R implements only the native desktop shell. Markdown scanning, SQLite, search, graph, relations, filesystem watching, and other future features are intentionally not implemented yet.

## Development

Run the desktop app:

```sh
cargo run
```

Run checks:

```sh
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace
```

## Documentation

- `AGENTS.md`
- `docs/PRODUCT.md`
- `docs/ARCHITECTURE.md`
- `docs/DESIGN_SYSTEM.md`
- `docs/ROADMAP.md`
