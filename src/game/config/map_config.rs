use serde::Deserialize;
use std::error::Error;
use std::path::Path;

use crate::game::{Description, Dungeon, Navigation, Room, Universe, World};

#[derive(Debug, Deserialize)]
pub struct RoomConfig {
    pub name: Option<String>,
    pub description: Description,
    pub north: Option<Navigation>,
    pub south: Option<Navigation>,
    pub east: Option<Navigation>,
    pub west: Option<Navigation>,
}

#[derive(Debug, Deserialize)]
pub struct WorldConfig {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DungeonConfig {
    pub name: Option<String>,
}

pub fn load_map(config_dir: Option<&Path>) -> Result<Universe, Box<dyn Error>> {
    let config_dir = match config_dir {
        Some(d) => d,
        None => return Ok(Universe::default()),
    };

    let maps_dir = config_dir.join("maps");
    if !maps_dir.exists() {
        return Ok(Universe::default());
    }

    let mut universe = Universe::default();

    let mut world_entries: Vec<_> = std::fs::read_dir(&maps_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    world_entries.sort_by_key(|e| e.file_name());

    for world_entry in world_entries {
        let world_path = world_entry.path();
        let world_folder = world_entry.file_name().to_string_lossy().to_string();

        let world_name = load_world_name(&world_path, &world_folder)?;
        let mut world = World::new(world_name.clone());

        let mut dungeon_entries: Vec<_> = std::fs::read_dir(&world_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();
        dungeon_entries.sort_by_key(|e| e.file_name());

        for dungeon_entry in dungeon_entries {
            let dungeon_path = dungeon_entry.path();
            let dungeon_folder = dungeon_entry.file_name().to_string_lossy().to_string();

            let dungeon_name = load_dungeon_name(&dungeon_path, &dungeon_folder)?;
            let mut dungeon = Dungeon::new(dungeon_name.clone());

            let mut room_entries: Vec<_> = std::fs::read_dir(&dungeon_path)?
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let path = e.path();
                    path.is_file()
                        && path.extension().and_then(|x| x.to_str()) == Some("toml")
                        && path.file_name().and_then(|x| x.to_str()) != Some("dungeon.toml")
                })
                .collect();
            room_entries.sort_by_key(|e| e.file_name());

            for room_entry in room_entries {
                let room_path = room_entry.path();
                let stem = room_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();

                let contents = std::fs::read_to_string(&room_path)?;
                let config: RoomConfig = toml::from_str(&contents)?;
                let room_name = config.name.unwrap_or(stem);
                let room = Room {
                    id: room_name.clone(),
                    description: config.description,
                    north: config.north,
                    south: config.south,
                    east: config.east,
                    west: config.west,
                };
                dungeon.rooms.insert(room_name, room);
            }

            world.dungeons.insert(dungeon_name, dungeon);
        }

        universe.worlds.insert(world_name, world);
    }

    Ok(universe)
}

fn load_world_name(world_path: &Path, folder_name: &str) -> Result<String, Box<dyn Error>> {
    let config_path = world_path.join("world.toml");
    if config_path.exists() {
        let contents = std::fs::read_to_string(&config_path)?;
        let config: WorldConfig = toml::from_str(&contents)?;
        if let Some(name) = config.name {
            return Ok(name);
        }
    }
    Ok(folder_name.to_string())
}

fn load_dungeon_name(dungeon_path: &Path, folder_name: &str) -> Result<String, Box<dyn Error>> {
    let config_path = dungeon_path.join("dungeon.toml");
    if config_path.exists() {
        let contents = std::fs::read_to_string(&config_path)?;
        let config: DungeonConfig = toml::from_str(&contents)?;
        if let Some(name) = config.name {
            return Ok(name);
        }
    }
    Ok(folder_name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_dir(base: &Path, rel: &str) {
        fs::create_dir_all(base.join(rel)).unwrap();
    }

    fn write_file(base: &Path, rel: &str, contents: &str) {
        let path = base.join(rel);
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn load_map_returns_default_when_no_config_dir() {
        let universe = load_map(None).unwrap();
        assert_eq!(universe.id, "universe");
        assert!(universe.worlds.is_empty());
    }

    #[test]
    fn load_map_returns_default_when_no_maps_subdir() {
        let tmp = TempDir::new().unwrap();
        let universe = load_map(Some(tmp.path())).unwrap();
        assert!(universe.worlds.is_empty());
    }

    #[test]
    fn load_map_loads_rooms_from_folder_structure() {
        let tmp = TempDir::new().unwrap();
        make_dir(tmp.path(), "maps/world1/dungeon1");
        write_file(
            tmp.path(),
            "maps/world1/dungeon1/room1.toml",
            r#"
[description]
basic = "A room."
"#,
        );

        let universe = load_map(Some(tmp.path())).unwrap();
        assert!(universe.worlds.contains_key("world1"));
        let world = &universe.worlds["world1"];
        assert!(world.dungeons.contains_key("dungeon1"));
        let dungeon = &world.dungeons["dungeon1"];
        assert!(dungeon.rooms.contains_key("room1"));
        let room = &dungeon.rooms["room1"];
        assert_eq!(room.description.basic.as_deref(), Some("A room."));
    }

    #[test]
    fn load_map_uses_world_toml_name() {
        let tmp = TempDir::new().unwrap();
        make_dir(tmp.path(), "maps/myfolder/d1");
        write_file(
            tmp.path(),
            "maps/myfolder/world.toml",
            r#"name = "OverriddenWorld""#,
        );
        write_file(tmp.path(), "maps/myfolder/d1/room.toml", r#"[description]"#);

        let universe = load_map(Some(tmp.path())).unwrap();
        assert!(universe.worlds.contains_key("OverriddenWorld"));
    }

    #[test]
    fn load_map_uses_dungeon_toml_name() {
        let tmp = TempDir::new().unwrap();
        make_dir(tmp.path(), "maps/w1/myfolder");
        write_file(
            tmp.path(),
            "maps/w1/myfolder/dungeon.toml",
            r#"name = "OverriddenDungeon""#,
        );
        write_file(tmp.path(), "maps/w1/myfolder/room.toml", r#"[description]"#);

        let universe = load_map(Some(tmp.path())).unwrap();
        let world = &universe.worlds["w1"];
        assert!(world.dungeons.contains_key("OverriddenDungeon"));
    }

    #[test]
    fn load_map_uses_room_config_name() {
        let tmp = TempDir::new().unwrap();
        make_dir(tmp.path(), "maps/w1/d1");
        write_file(
            tmp.path(),
            "maps/w1/d1/myfile.toml",
            r#"
name = "Custom Room"
[description]
basic = "A custom room."
"#,
        );

        let universe = load_map(Some(tmp.path())).unwrap();
        let dungeon = &universe.worlds["w1"].dungeons["d1"];
        assert!(dungeon.rooms.contains_key("Custom Room"));
    }

    #[test]
    fn load_map_handles_navigation() {
        let tmp = TempDir::new().unwrap();
        make_dir(tmp.path(), "maps/w1/d1");
        write_file(
            tmp.path(),
            "maps/w1/d1/room.toml",
            r#"
[description]
basic = "Main room."
[north]
room_id = "tavern"
"#,
        );

        let universe = load_map(Some(tmp.path())).unwrap();
        let room = &universe.worlds["w1"].dungeons["d1"].rooms["room"];
        assert!(room.north.is_some());
        assert_eq!(
            room.north.as_ref().unwrap().room_id.as_deref(),
            Some("tavern")
        );
    }
}
