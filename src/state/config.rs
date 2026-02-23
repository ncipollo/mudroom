use std::path::PathBuf;

type StateResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

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
}
