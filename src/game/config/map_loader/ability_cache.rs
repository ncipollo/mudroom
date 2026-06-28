use crate::game::component::Ability;
use crate::game::config::ability_config::load_ability;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

pub fn build_ability_cache(config_dir: &Path) -> Result<HashMap<String, Ability>, Box<dyn Error>> {
    let mut cache = HashMap::new();
    let abilities_dir = config_dir.join("abilities");
    if !abilities_dir.exists() {
        return Ok(cache);
    }
    for entry in walkdir::WalkDir::new(&abilities_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("toml"))
    {
        let path = entry.path();
        let mut ability = load_ability(path)?;
        let rel = path.strip_prefix(&abilities_dir)?.with_extension("");
        ability.id = rel.to_string_lossy().to_string();
        cache.insert(ability.id.clone(), ability);
    }
    Ok(cache)
}
