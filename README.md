# FlokinMD

Local-first desktop app that turns Markdown folders into a visual, structured, queryable database.

MDB-001 implements only the desktop shell. Markdown scanning, SQLite, search, graph, relations, filesystem watching, and other future features are intentionally not implemented yet.

## Development

Install dependencies:

```sh
pnpm install
```

Run the desktop app:

```sh
pnpm tauri dev
```

Run checks:

```sh
pnpm lint
pnpm test
pnpm build
cargo check --manifest-path src-tauri/Cargo.toml
```

## Documentation

- `AGENTS.md`
- `docs/PRODUCT.md`
- `docs/ARCHITECTURE.md`
- `docs/DESIGN_SYSTEM.md`
- `docs/ROADMAP.md`
