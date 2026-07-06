use std::fmt;

pub struct ConnectionKey {
    pub machine_id: String,
    pub pid: u32,
}

impl ConnectionKey {
    pub fn new(machine_id: impl Into<String>) -> Self {
        Self {
            machine_id: machine_id.into(),
            pid: std::process::id(),
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let mut parts = s.splitn(2, ':');
        let machine_id = parts.next()?.to_string();
        let pid = parts.next()?.parse().ok()?;
        Some(Self { machine_id, pid })
    }
}

impl fmt::Display for ConnectionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.machine_id, self.pid)
    }
}
