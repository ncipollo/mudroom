# Mudroom

## After Each Change
Run the following commands after every code change and fix any issues before considering the change complete:

1. `cargo fmt` - Format all code
2. `cargo test` - Run all tests
3. `cargo clippy` - Run linter; fix all warnings and errors before completing the change
4. `(git diff --name-only HEAD; git ls-files --others --exclude-standard) | smell -q -` - Run smell against the changed/new files only; fix all reported limit failures before completing the change

Smell is scoped to the diff (not the whole repo) because `smell.toml`'s limits are tighter than some pre-existing files. Only files you touch need to satisfy them — see `smell --info` for details on diff piping and the config file.

### Fixing Clippy Complexity Warnings
When clippy reports `cognitive_complexity`, `too_many_lines`, or `too_many_arguments` warnings, fix them by refactoring — never suppress with `#[allow]`:
- Extract logical sub-steps into well-named helper functions.
- When a file accumulates many functions, reorganize into helper files and structs (following the module conventions below).

### Fixing Smell Warnings
When `smell` reports a limit failure, fix it by refactoring — never suppress by raising the limit in `smell.toml`:
- **Too complex** (`max_complexity`) - break the function up into sub-functions.
- **Too complex from a big switch/match** (`max_complexity`) - convert it into a map of lambdas instead.
- **Too many methods** (`max_methods`) - the type probably has too many responsibilities; enumerate its responsibilities and split each into a helper type/file.
- **Too many declarations** (`max_declarations`) - the file itself is too big, usually from mixing responsibilities and tests; split it into separate files along responsibility lines.
- **Too many lines** (`max_lines`) - same fix as too many declarations.
- **Too many/long comments** (`max_comments`, `max_comment_lines`) - delete comments that restate the code; keep only WHY comments, and keep those short.

## Manual Server Testing
To manually verify a change by running a real server instance, use a disposable
name/config so it never collides with a real play session, and clean up afterward.

### Start
There is no wrapper script — invoke the server directly with `cargo run`, backgrounded
(the game loop runs forever and never returns), with output redirected to a log file:

```
cargo run -- server --name test --config muds/basic > /tmp/mudroom-test.log 2>&1 &
```

`--name test` keeps session/database state isolated under a scratch key (see Teardown).
`--config muds/basic` points at the repo's own minimal throwaway mud config — no need
to author one. `muds/basic/mud.toml` resolves `$MUDROOM_OLLAMA_BASE_URL` and
`$MUDROOM_OLLAMA_MODEL` from the environment; set them (any value is fine if no agent
LLM calls are exercised) or startup fails with a TOML parse error.

### Confirm it came up
The server binds an OS-assigned ephemeral port (`0.0.0.0:0`), so the real port is only
known from the startup log. Poll the log for the listening line, then hit
`GET /server/info`:

```
grep "Server listening on" /tmp/mudroom-test.log
curl http://127.0.0.1:<port>/server/info
```

Retry briefly if the log line or endpoint isn't there yet — startup isn't instant.

### Teardown
Kill the backgrounded process, then remove the scratch server's state so repeated runs
don't accumulate stale session/database files:

```
kill <pid>
rm -rf ~/.mudroom/session/server/test.json ~/.mudroom/session/server/test
```

For driving the server programmatically over HTTP (queuing interactions, forcing
ticks, observing events) rather than just confirming it's alive, see
`mudroom instructions testing`.

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
