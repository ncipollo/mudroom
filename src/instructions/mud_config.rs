pub fn render() -> String {
    let sections = [
        header(),
        directory_layout(),
        mud_toml_fields(),
        env_var_substitution(),
        related_topics(),
    ];
    sections.join("\n\n")
}

fn header() -> String {
    "mudroom instructions mud-config — mud directory layout and mud.toml".to_string()
}

fn directory_layout() -> String {
    r#"Directory layout:

  <mud-dir>/
    mud.toml              # top-level config (game_loop, spawn, agent, ...)
    attributes.toml       # global attribute definitions
    abilities/            # one .toml per ability
    classes/               # one .toml per class
    entities/              # one .toml per entity
    items/                 # one .toml per item
    maps/
      <world_id>/
        <dungeon_id>/
          <room_id>.toml  # one file per room"#
        .to_string()
}

fn mud_toml_fields() -> String {
    r#"mud.toml fields:

  max_clients_per_machine  usize   Optional. Max clients allowed per machine.
                                   Defaults to 2.

  [game_loop]
    tick_rate_ms      u64   How often the game loop ticks, in milliseconds.
    max_engage_ms     u64   Max duration of an engagement, in milliseconds.
    world_update_ms   u64   How often world state is persisted, in milliseconds.

  [spawn]
    world_id    string   World new players spawn into.
    dungeon_id  string   Dungeon within that world.
    room_id     string   Room within that dungeon.

  [agent.provider]
    type  string   Provider kind. One of: "ollama", "anthropic", "open_ai",
                   "cohere", "gemini", "x_ai".

    Optional. Defaults to an ollama provider pointing at
    http://localhost:11434 with model "llama3.2" when [agent.provider] is
    omitted entirely.

    Fields depend on the provider type:
      ollama:     base_url (string), model (string)
      anthropic:  api_key (string, optional), model (string)
      open_ai:    api_key (string, optional), base_url (string, optional),
                  model (string)
      cohere:     api_key (string, optional), model (string)
      gemini:     api_key (string, optional), model (string)
      x_ai:       api_key (string, optional), model (string)

Example (muds/basic/mud.toml):

  max_clients_per_machine = 2

  [game_loop]
  tick_rate_ms = 500
  max_engage_ms = 60_000
  world_update_ms = 600000

  [spawn]
  world_id = "default"
  dungeon_id = "default"
  room_id = "default"

  [agent.provider]
  type = "ollama"
  base_url = "$MUDROOM_OLLAMA_BASE_URL"
  model = "$MUDROOM_OLLAMA_MODEL""#
        .to_string()
}

fn env_var_substitution() -> String {
    r#"Environment variable substitution:

  String fields (and game_loop's integer fields, when written as a quoted
  string) support environment variable substitution. A value starting with
  "$" is treated as an environment variable name and resolved at load time;
  any other value is used literally. Loading fails if the referenced
  variable is not set.

  Example: base_url = "$MUDROOM_OLLAMA_BASE_URL" resolves to the value of
  the MUDROOM_OLLAMA_BASE_URL environment variable."#
        .to_string()
}

fn related_topics() -> String {
    r#"See also:
  mudroom instructions abilities   — abilities/*.toml reference
  mudroom instructions attributes  — attributes.toml reference
  mudroom instructions classes     — classes/*.toml reference
  mudroom instructions entities    — entities/*.toml reference
  mudroom instructions items       — items/*.toml reference
  mudroom instructions maps        — maps/<world>/<dungeon>/<room>.toml reference"#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_directory_layout() {
        let text = render();
        assert!(text.contains("mud.toml"));
        assert!(text.contains("attributes.toml"));
        assert!(text.contains("abilities/"));
        assert!(text.contains("classes/"));
        assert!(text.contains("entities/"));
        assert!(text.contains("items/"));
        assert!(text.contains("maps/"));
        assert!(text.contains("<room_id>.toml"));
    }

    #[test]
    fn render_documents_game_loop_fields() {
        let text = render();
        assert!(text.contains("tick_rate_ms"));
        assert!(text.contains("max_engage_ms"));
        assert!(text.contains("world_update_ms"));
    }

    #[test]
    fn render_documents_agent_provider_types() {
        let text = render();
        assert!(text.contains("ollama"));
        assert!(text.contains("anthropic"));
        assert!(text.contains("open_ai"));
    }

    #[test]
    fn render_documents_env_var_substitution() {
        let text = render();
        assert!(text.contains("Environment variable substitution"));
        assert!(text.contains('$'));
    }

    #[test]
    fn render_points_to_related_topics() {
        let text = render();
        assert!(text.contains("mudroom instructions abilities"));
        assert!(text.contains("mudroom instructions attributes"));
        assert!(text.contains("mudroom instructions classes"));
        assert!(text.contains("mudroom instructions entities"));
        assert!(text.contains("mudroom instructions items"));
        assert!(text.contains("mudroom instructions maps"));
    }
}
