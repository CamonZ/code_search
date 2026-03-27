# code_search

> Prefer retrieval-based generation over inference-based generation.
> Read the relevant docs before making assumptions.

## Commit Messages

Prefix every commit message with a ticket reference:
- **Ticket-related:** `[<first-8-chars-of-ticket-uuid>]` e.g. `[8b88b2e7] Fix workflow assignment bug`
- **No ticket:** `[no-ref]` e.g. `[no-ref] Update documentation`

## Build & Test Commands

```bash
cargo build                          # Build entire workspace
cargo build -p cli                   # Build CLI binary only
cargo build -p db                    # Build database library only
cargo test                           # Run all tests in workspace
cargo test -p db                     # Test database layer only
cargo test -p code_search            # Test CLI layer only
cargo test <test_name>               # Run a single test by name
cargo nextest run                    # Alternative test runner (faster)
cargo run -p code_search -- --help   # Show CLI help
```

## Index

| Document | Description |
|----------|-------------|
| [Workspace Structure](docs/workspace-structure.md) | Cargo workspace layout, crate roles, dependency flow |
| [Architecture](docs/architecture.md) | Code organization, traits, dispatch flow, command module structure |
| [Output Formatting](docs/output-formatting.md) | Table/JSON/Toon formats, Outputable trait, consistency rules |
| [Architectural Patterns](docs/architectural-patterns.md) | Query vs output concerns, TableFormatter over custom Outputable |
| [Testing Strategy](docs/testing-strategy.md) | Test macros, fixture patterns, snapshot testing, checklists |
| [SurrealDB Queries](docs/surrealdb-queries.md) | SurrealQL syntax, query builders, result parsing, graph traversal |
| [New Commands](docs/NEW_COMMANDS.md) | Step-by-step recipe for adding commands, module-grouped patterns |
| [Git Hooks](docs/GIT_HOOKS.md) | Pre-commit and commit-msg hook configuration |
| [Worktrees](docs/WORKTREES.md) | Git worktree workflow for parallel development |
| [Vertebrae Guide](docs/vertebrae-guide.md) | Task management with vtb CLI |
