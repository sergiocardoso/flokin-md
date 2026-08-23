# Product

<!-- impeccable:product-schema 1 -->

## Platform

native desktop

## Stack

Rust stable, Iced 0.14, and Cargo. FlokinMD does not use Node.js, WebView, Tauri, React, TypeScript, Vite, or pnpm as application stack.

## Users

Primary users are people and teams that keep durable knowledge in local Markdown folders and want a visual interface to browse, organize, and eventually query those files.

## Product Purpose

FlokinMD turns folders containing Markdown files into a visual, structured, queryable database while preserving the files as the source of truth.

## Positioning

Markdown files remain open, readable, portable, and local. Future indexes and caches help inspect the files; they do not replace them.

## Capabilities and Constraints

MDB-001R is limited to the native desktop shell. It must not implement real filesystem access, Markdown parsing, SQLite, search, graph, watcher, sync, login, AI, MCP, or external APIs.

## Brand Commitments

Visible name: FlokinMD. Technical name: flokin-md. Theme: dark native desktop tool with clean technical surfaces and purple accent.

## Product Principles

- Markdown files are the source of truth.
- Local-first behavior is a durable requirement.
- Future Rust core logic must stay independent from GUI concerns.
- Each milestone should produce a demonstrable user-facing result.
