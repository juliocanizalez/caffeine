use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Status {
    pub pid: u32,
    pub started_at: u64,
    pub expiry: Option<u64>,
    pub prevent_display: bool,
}

pub fn lock_file_path() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME not set");
    PathBuf::from(home).join("Library/Application Support/caffeine/status")
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl Status {
    pub fn write(&self) {
        let path = lock_file_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let content = format!(
            "{}\n{}\n{}\n{}\n",
            self.pid,
            self.started_at,
            self.expiry.unwrap_or(0),
            self.prevent_display,
        );
        let _ = fs::write(&path, content);
    }

    pub fn read() -> Option<Self> {
        let content = fs::read_to_string(lock_file_path()).ok()?;
        let mut lines = content.lines();
        let pid: u32 = lines.next()?.parse().ok()?;
        let started_at: u64 = lines.next()?.parse().ok()?;
        let expiry_raw: u64 = lines.next()?.parse().ok()?;
        let prevent_display: bool = lines.next()?.parse().ok()?;
        Some(Self {
            pid,
            started_at,
            expiry: if expiry_raw == 0 {
                None
            } else {
                Some(expiry_raw)
            },
            prevent_display,
        })
    }

    pub fn delete() {
        let _ = fs::remove_file(lock_file_path());
    }

    pub fn is_alive(&self) -> bool {
        unsafe { libc::kill(self.pid as libc::pid_t, 0) == 0 }
    }
}
