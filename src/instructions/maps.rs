pub fn render() -> String {
    let sections = [
        header(),
        naming_overrides(),
        room_fields(),
        examples(),
        spawn_point(),
    ];
    sections.join("\n\n")
}

fn header() -> String {
    r#"mudroom instructions maps — map and room config reference

Maps define the navigable world: rooms, descriptions, exits, and entity
placements. They live under the mud root in a nested directory hierarchy:

  maps/<world_id>/<dungeon_id>/<room_id>.toml

Each <world_id> directory holds one or more <dungeon_id> directories, and each
<dungeon_id> directory holds one .toml file per room. Directory names become the
world_id and dungeon_id; the room .toml filename (without the .toml extension)
becomes the room_id."#
        .to_string()
}

fn naming_overrides() -> String {
    r#"Overriding names:
  - Add a world.toml inside a <world_id> directory with a `name` key to override
    the world's id (otherwise it defaults to the directory name).
  - Add a dungeon.toml inside a <dungeon_id> directory with a `name` key to
    override the dungeon's id (otherwise it defaults to the directory name).
  - Add a `name` key at the top of a room .toml file to override that room's id
    (otherwise it defaults to the filename stem)."#
        .to_string()
}

fn room_fields() -> String {
    r#"Room fields:
  - entities   — optional array of entity reference strings, e.g.
                 ["entities/innkeeper"]. Paths are relative to the mud root and
                 must match a corresponding file under entities/ (without the
                 .toml extension).
  - [description]
      text   — the room's prose description, shown to players.
      theme  — optional theme id used by the theming pipeline to style the
               narration. Rarely needed for basic authoring; defaults to the
               standard theme when omitted.
  - [north] / [south] / [east] / [west]
      room_id   — the id of the room this exit leads to, another room .toml
                  filename (or its `name` override) in the same
                  <world_id>/<dungeon_id>/ directory. Omit the sub-table entirely
                  to block movement in that direction. Cross-dungeon exits are
                  not yet documented — keep exits within the same dungeon."#
        .to_string()
}

fn examples() -> String {
    r#"Example room (muds/basic/maps/default/default/tavern.toml):

  entities = ["entities/innkeeper"]

  [description]
  text = "A warm tavern with a crackling fireplace. The town square is to
  the south. A dim corner lies to the east."

  [south]
  room_id = "default"

  [east]
  room_id = "back_corner"

Example room — crossroads (muds/basic/maps/default/default/default.toml):

  [description]
  text = "You stand at the crossroads of a small town. A tavern lies to the
  north. A dark corridor leads to the east."

  [north]
  room_id = "tavern"

  [east]
  room_id = "dark_corridor""#
        .to_string()
}

fn spawn_point() -> String {
    r#"Spawn point:
  mud.toml's [spawn] table references a room by world_id, dungeon_id, and
  room_id, which must correspond to an existing
  maps/<world_id>/<dungeon_id>/<room_id>.toml file:

  [spawn]
  world_id = "default"
  dungeon_id = "default"
  room_id = "default""#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_documents_directory_layout() {
        let text = render();
        assert!(text.contains("maps/<world_id>/<dungeon_id>/<room_id>.toml"));
    }

    #[test]
    fn render_documents_room_fields() {
        let text = render();
        assert!(text.contains("entities"));
        assert!(text.contains("[description]"));
        assert!(text.contains("text"));
        assert!(text.contains("[north]"));
        assert!(text.contains("[south]"));
        assert!(text.contains("[east]"));
        assert!(text.contains("[west]"));
        assert!(text.contains("room_id"));
    }

    #[test]
    fn render_documents_spawn_relationship() {
        let text = render();
        assert!(text.contains("[spawn]"));
        assert!(text.contains("mud.toml"));
    }
}
