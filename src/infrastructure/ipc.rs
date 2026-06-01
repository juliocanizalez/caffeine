use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::models::SessionInfo;
use crate::domain::ports::StatusRepository;

fn lock_file_path() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME not set");
    PathBuf::from(home).join("Library/Application Support/caffeine/status")
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub struct FileStatusRepository;

impl StatusRepository for FileStatusRepository {
    fn write(&self, info: &SessionInfo) {
        let path = lock_file_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let content = format!(
            "{}\n{}\n{}\n{}\n",
            info.pid,
            info.expiry.unwrap_or(0),
            info.prevent_display,
            info.jiggle,
        );
        let _ = fs::write(&path, content);
    }

    fn read(&self) -> Option<SessionInfo> {
        let content = fs::read_to_string(lock_file_path()).ok()?;
        let mut lines = content.lines();
        let pid: u32 = lines.next()?.parse().ok()?;
        let expiry_raw: u64 = lines.next()?.parse().ok()?;
        let prevent_display: bool = lines.next()?.parse().ok()?;
        let jiggle: bool = lines.next().and_then(|l| l.parse().ok()).unwrap_or(false);
        Some(SessionInfo {
            pid,
            expiry: if expiry_raw == 0 {
                None
            } else {
                Some(expiry_raw)
            },
            prevent_display,
            jiggle,
        })
    }

    fn delete(&self) {
        let _ = fs::remove_file(lock_file_path());
    }

    fn is_alive(&self, pid: u32) -> bool {
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }

    fn now_secs(&self) -> u64 {
        now_secs()
    }
}
