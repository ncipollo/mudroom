pub fn render() -> String {
    let sections = [
        header(),
        test_mode_section(),
        tick_endpoint_section(),
        interactions_section(),
        events_section(),
        scenario_section(),
    ];
    sections.join("\n\n")
}

fn header() -> String {
    r#"mudroom instructions testing — driving/observing the server for tests

This topic is for agents and scripts driving mudroom over HTTP to write
automated test scenarios. The real-time game loop advances on a fixed
interval (see `mudroom instructions mud-config` for tick_rate_ms), which
makes scripted interactions slow and racy — a script can't know how long
to wait for a tick to land. The debug tick-advance endpoint below lets a
test script force exactly one tick on demand instead."#
        .to_string()
}

fn test_mode_section() -> String {
    r#"Enabling test mode:
  mudroom server --enable-test-api

  Off by default — never enable this flag in normal play. It's what
  unlocks the /debug/tick route below; without it, that route doesn't
  exist (requests to it get a 404, not a "disabled" response)."#
        .to_string()
}

fn tick_endpoint_section() -> String {
    r#"POST /debug/tick:
  Only reachable when the server was started with --enable-test-api.

  Request:  no body
  Response: { "tick": <u64> }  — the tick number just processed

  Runs exactly one tick of the real game loop's processing sequence out
  of band from the real-time ticker (activations, interactions,
  engagement, effects, attributes, world update — the same order the
  real loop runs). Results are observable immediately after the response
  returns; there is no need to wait on tick_rate_ms."#
        .to_string()
}

fn interactions_section() -> String {
    r#"POST /interactions:
  Request: { "client_id": "...", "interaction": <Interaction> }

  <Interaction> is the same externally-tagged JSON shape sent over
  /events — a unit variant like "look" serializes as the bare string
  "look"; a variant with fields, e.g. LookAt { target }, serializes as
  { "look_at": { "target": "..." } }.

  Queues an interaction for the player's entity. Queued interactions sit
  in the entity's mailbox until a tick — real or debug-triggered — drains
  and processes them. A test script queues an interaction, then calls
  POST /debug/tick to force it through deterministically rather than
  waiting for the real loop to pick it up."#
        .to_string()
}

fn events_section() -> String {
    r#"GET /events?client_id=...:
  Server-sent events stream of NetworkEvent JSON for one client. This is
  how a test script observes the effect of a tick — subscribe before
  queuing an interaction, then watch for the events a forced tick
  produces. A reconnect under the same client_id supersedes the previous
  connection."#
        .to_string()
}

fn scenario_section() -> String {
    r#"Worked example — a full scripted test loop:

  1. POST /session/start                      → get a client_id
  2. POST /players/select                     → select a character
  3. GET  /events?client_id=<id>              → subscribe to this client's
                                                 events (keep the
                                                 connection open)
  4. POST /interactions                        → queue an interaction,
     { "client_id": "<id>",                     e.g. Look
       "interaction": "look" }
  5. POST /debug/tick                          → force the queued
                                                 interaction through
                                                 immediately
  6. Read from the still-open /events stream   → assert on the resulting
                                                 NetworkEvent(s)

  Repeating steps 4–6 drives a deterministic, step-by-step test scenario
  without ever waiting on the real tick interval."#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_documents_test_mode_flag() {
        let text = render();
        assert!(text.contains("--enable-test-api"));
        assert!(text.contains("Off by default"));
    }

    #[test]
    fn render_documents_tick_endpoint_contract() {
        let text = render();
        assert!(text.contains("POST /debug/tick"));
        assert!(text.contains("\"tick\""));
        assert!(text.contains("404"));
    }

    #[test]
    fn render_documents_interactions_endpoint() {
        let text = render();
        assert!(text.contains("POST /interactions"));
        assert!(text.contains("mailbox"));
    }

    #[test]
    fn render_documents_events_endpoint() {
        let text = render();
        assert!(text.contains("GET /events"));
        assert!(text.contains("NetworkEvent"));
    }

    #[test]
    fn render_includes_worked_scenario() {
        let text = render();
        assert!(text.contains("Worked example"));
        assert!(text.contains("/session/start"));
        assert!(text.contains("/debug/tick"));
    }
}
