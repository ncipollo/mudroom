use std::path::PathBuf;

type StateResult<T> = Result<T, Box<dyn std::error::Error>>;

pub fn mudroom_dir() -> StateResult<PathBuf> {
    dirs::home_dir()
        .map(|h| h.join(".mudroom"))
        .ok_or_else(|| "Could not determine home directory".into())
}

pub fn session_dir() -> StateResult<PathBuf> {
    Ok(mudroom_dir()?.join("session"))
}

pub fn server_session_dir(server_id: &str) -> StateResult<PathBuf> {
    Ok(session_dir()?.join("server").join(server_id))
}

pub fn client_session_dir(server_id: &str) -> StateResult<PathBuf> {
    Ok(session_dir()?.join("client").join(server_id))
}

pub async fn create_state_dirs(server_id: &str) -> StateResult<()> {
    tokio::fs::create_dir_all(server_session_dir(server_id)?).await?;
    tokio::fs::create_dir_all(client_session_dir(server_id)?).await?;
    Ok(())
}

pub fn server_session_file(name: &str) -> StateResult<PathBuf> {
    Ok(session_dir()?.join("server").join(format!("{name}.json")))
}

pub fn client_session_file(server_id: &str) -> StateResult<PathBuf> {
    Ok(session_dir()?
        .join("client")
        .join(format!("{server_id}.json")))
}

pub fn database_url(server_name: &str) -> StateResult<String> {
    let path = server_session_dir(server_name)?.join("mudroom.db");
    Ok(format!("sqlite:{}?mode=rwc", path.display()))
}

pub async fn create_session_base_dirs() -> StateResult<()> {
    tokio::fs::create_dir_all(session_dir()?.join("server")).await?;
    tokio::fs::create_dir_all(session_dir()?.join("client")).await?;
    Ok(())
}

pub fn find_last_server_key() -> StateResult<Option<String>> {
    let dir = session_dir()?.join("server");
    if !dir.exists() {
        return Ok(None);
    }
    let mut latest: Option<(std::time::SystemTime, String)> = None;
    for entry in std::fs::read_dir(&dir)?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let stem = match path.file_stem().and_then(|s| s.to_str()).map(String::from) {
            Some(s) => s,
            None => continue,
        };
        if let Ok(modified) = path.metadata().and_then(|m| m.modified()) {
            match &latest {
                None => latest = Some((modified, stem)),
                Some((t, _)) if modified > *t => latest = Some((modified, stem)),
                _ => {}
            }
        }
    }
    Ok(latest.map(|(_, key)| key))
}

pub fn find_config_dir() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;

    if cwd.join("mud.toml").exists() {
        return Some(cwd);
    }

    let muds_dir = cwd.join("muds");
    if muds_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&muds_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("mud.toml").exists() {
                return Some(path);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mudroom_dir_ends_with_mudroom() {
        let path = mudroom_dir().expect("home dir should be available");
        assert!(path.ends_with(".mudroom"));
    }

    #[test]
    fn session_dir_ends_with_session() {
        let path = session_dir().expect("home dir should be available");
        assert!(path.ends_with(".mudroom/session"));
    }

    #[test]
    fn server_session_dir_has_correct_path() {
        let path = server_session_dir("abc").expect("home dir should be available");
        assert!(path.ends_with("session/server/abc"));
    }

    #[test]
    fn client_session_dir_has_correct_path() {
        let path = client_session_dir("abc").expect("home dir should be available");
        assert!(path.ends_with("session/client/abc"));
    }

    #[test]
    fn database_url_points_to_server_session_dir() {
        let url = database_url("myserver").expect("home dir should be available");
        assert!(url.contains("session/server/myserver/mudroom.db"));
    }
}
