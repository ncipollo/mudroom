# network module

## Structure

- `network.rs`              — module root; re-exports `NetworkEvent`
- `event.rs`                — `NetworkEvent` enum, `SessionStartResponse` struct
- `client.rs`               — public API: `connect_sse`, `start_session`, `run_ping_loop`, `end_session`
  - `client/sse.rs`         — SSE subscriber (`connect_sse`)
  - `client/ping.rs`        — client ping loop (`run_ping_loop`)
  - `client/session.rs`     — session lifecycle (`start_session`, `end_session`)
- `server.rs`               — public API: `start()`
  - `server/state.rs`       — `AppState`, `ConnectedClient`, `SseCleanupGuard`, `GuardedStream`, request body structs
  - `server/handlers.rs`    — axum HTTP handlers
  - `server/router.rs`      — builds axum `Router` (`build_router`)
  - `server/ping_reaper.rs` — background reaper for stale connections
- `discovery.rs`            — re-exports `DiscoveredServer`, `discover`, `DiscoveryServer`
  - `discovery/client.rs`   — UDP broadcaster + collector
  - `discovery/server.rs`   — UDP responder
