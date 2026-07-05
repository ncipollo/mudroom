# Mudroom

## After Each Change
Run the following commands after every code change and fix any issues before considering the change complete:

1. `cargo fmt` - Format all code
2. `cargo test` - Run all tests
3. `cargo clippy` - Run linter; fix all warnings and errors before completing the change

### Fixing Clippy Complexity Warnings
When clippy reports `cognitive_complexity`, `too_many_lines`, or `too_many_arguments` warnings, fix them by refactoring — never suppress with `#[allow]`:
- Extract logical sub-steps into well-named helper functions.
- When a file accumulates many functions, reorganize into helper files and structs (following the module conventions below).

## Dependencies
Always use exact versions for dependencies in `Cargo.toml` (e.g., `"4.5.60"` not `"4"`). Check `Cargo.lock` for the resolved version when pinning.

## Module Conventions
Never use `mod.rs`. Always use the modern Rust style: create a top-level file (e.g., `foo.rs`) as the module root, and a matching folder (`foo/`) for any submodules.

Top-level module files (e.g. `engagement.rs`, `game_loop.rs`) must contain only globally-scoped, cross-cutting functions. Logic specific to a sub-domain belongs in the matching submodule (e.g. battle logic → `engagement/battle/`, not `engagement.rs`).

## Imports
Always use `use` imports rather than full crate paths at call sites. For example, prefer `use crate::game::engagement;` + `engagement::process(...)` over `crate::game::engagement::process(...)`.

## Architecture

See [`arch.md`](docs/engine/arch.md) for the full architecture overview. New code must be placed within one of the existing domain modules below — do not create new top-level modules unless explicitly instructed. Quick module map:

| Module | Layer | Notes |
|---|---|---|
| `game/` | Domain | Tick loop, ECS, interaction dispatch, engagement, map |
| `game/interaction/` | Domain | Player input → entity mailbox handlers |
| `game/engagement/` | Domain | Turn-based encounter system |
| `network/` | Infrastructure | axum SSE server, HTTP client, UDP discovery |
| `network/session/` | Infrastructure | ClientSession, ServerSession |
| `tui/` | Presentation | ratatui TUI; screens in `tui/screens/` |
| `persistence/` | Infrastructure | SQLite repos (sqlx) |
| `agent/` | Infrastructure | LLM providers + entity AI state |
| `paths.rs` | Infrastructure | Filesystem path helpers |
| `cli.rs` | Entry | clap struct/enum definitions; declares `cli/` submodules; re-exports `router()` |
| `cli/router.rs` | Entry | `CliRouter` struct + `router()` free function; dispatches on top-level commands |
| `cli/server.rs` | Entry | Server command handler and all startup helpers |
| `cli/client.rs` | Entry | Client command handler |
| `cli/players.rs` | Entry | Players subcommand handlers |
| `logging.rs` | Cross-cutting | — |

### CLI organization principle
`cli.rs` holds only clap type definitions. Each top-level command gets its own file under `cli/` (one file per command). The router (`cli/router.rs`) owns dispatch only — no business logic. Entry point in `main.rs` calls `cli::router().route(cli).await`.
