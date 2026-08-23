# AGENTS.md

Guidance for agents working on FlokinMD.

1. Markdown files are the source of truth.
2. SQLite will be only an index/cache in the future. It must be disposable and rebuildable.
3. The future Rust core must not be coupled to the GUI.
4. Do not implement features outside the current task.
5. Each task should produce a user-demonstrable delivery whenever technically possible.
6. Never consider a task finished with a broken build.
7. Do not perform unrelated refactors.
8. Do not arbitrarily change the design system.
9. New screens must reuse existing tokens and components.
10. Future operations that modify Markdown must be safe, predictable, and allow preview when appropriate.
