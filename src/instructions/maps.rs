pub fn render() -> String {
    let mut lines = vec![
        "mudroom instructions maps — map and room config reference".to_string(),
        String::new(),
        "Maps define the navigable world: rooms, descriptions, exits, and entity".to_string(),
        "placements. They live under the mud root in a nested directory hierarchy:".to_string(),
        String::new(),
        "  maps/<world_id>/<dungeon_id>/<room_id>.toml".to_string(),
        String::new(),
        "Each <world_id> directory holds one or more <dungeon_id> directories, and each"
            .to_string(),
        "<dungeon_id> directory holds one .toml file per room. Directory names become the"
            .to_string(),
        "world_id and dungeon_id; the room .toml filename (without the .toml extension)"
            .to_string(),
        "becomes the room_id.".to_string(),
    ];
    lines.extend(naming_overrides());
    lines.extend(room_fields());
    lines.extend(examples());
    lines.extend(spawn_point());
    lines.join("\n")
}

fn naming_overrides() -> Vec<String> {
    vec![
        String::new(),
        "Overriding names:".to_string(),
        "  - Add a world.toml inside a <world_id> directory with a `name` key to override"
            .to_string(),
        "    the world's id (otherwise it defaults to the directory name).".to_string(),
        "  - Add a dungeon.toml inside a <dungeon_id> directory with a `name` key to".to_string(),
        "    override the dungeon's id (otherwise it defaults to the directory name).".to_string(),
        "  - Add a `name` key at the top of a room .toml file to override that room's id"
            .to_string(),
        "    (otherwise it defaults to the filename stem).".to_string(),
    ]
}

fn room_fields() -> Vec<String> {
    vec![
        String::new(),
        "Room fields:".to_string(),
        "  - entities   — optional array of entity reference strings, e.g.".to_string(),
        "                 [\"entities/innkeeper\"]. Paths are relative to the mud root and"
            .to_string(),
        "                 must match a corresponding file under entities/ (without the".to_string(),
        "                 .toml extension).".to_string(),
        "  - [description]".to_string(),
        "      standard  — the room's prose description, shown to players.".to_string(),
        "      checked   — optional list of conditional descriptions, each gated by an".to_string(),
        "                  attribute check (attribute_id + expected_value). Rarely needed"
            .to_string(),
        "                  for basic authoring; defaults to an empty list.".to_string(),
        "  - [north] / [south] / [east] / [west]".to_string(),
        "      room_id   — the id of the room this exit leads to, another room .toml".to_string(),
        "                  filename (or its `name` override) in the same".to_string(),
        "                  <world_id>/<dungeon_id>/ directory. Omit the sub-table entirely"
            .to_string(),
        "                  to block movement in that direction. Cross-dungeon exits are"
            .to_string(),
        "                  not yet documented — keep exits within the same dungeon.".to_string(),
    ]
}

fn examples() -> Vec<String> {
    vec![
        String::new(),
        "Example room (muds/basic/maps/default/default/tavern.toml):".to_string(),
        String::new(),
        "  entities = [\"entities/innkeeper\"]".to_string(),
        String::new(),
        "  [description]".to_string(),
        "  standard = \"A warm tavern with a crackling fireplace. The town square is to"
            .to_string(),
        "  the south. A dim corner lies to the east.\"".to_string(),
        String::new(),
        "  [south]".to_string(),
        "  room_id = \"default\"".to_string(),
        String::new(),
        "  [east]".to_string(),
        "  room_id = \"back_corner\"".to_string(),
        String::new(),
        "Example room — crossroads (muds/basic/maps/default/default/default.toml):".to_string(),
        String::new(),
        "  [description]".to_string(),
        "  standard = \"You stand at the crossroads of a small town. A tavern lies to the"
            .to_string(),
        "  north. A dark corridor leads to the east.\"".to_string(),
        String::new(),
        "  [north]".to_string(),
        "  room_id = \"tavern\"".to_string(),
        String::new(),
        "  [east]".to_string(),
        "  room_id = \"dark_corridor\"".to_string(),
    ]
}

fn spawn_point() -> Vec<String> {
    vec![
        String::new(),
        "Spawn point:".to_string(),
        "  mud.toml's [spawn] table references a room by world_id, dungeon_id, and".to_string(),
        "  room_id, which must correspond to an existing".to_string(),
        "  maps/<world_id>/<dungeon_id>/<room_id>.toml file:".to_string(),
        String::new(),
        "  [spawn]".to_string(),
        "  world_id = \"default\"".to_string(),
        "  dungeon_id = \"default\"".to_string(),
        "  room_id = \"default\"".to_string(),
    ]
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
        assert!(text.contains("standard"));
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
