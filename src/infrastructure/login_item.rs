use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::domain::ports::LoginItemManager;

fn plist_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join("Library/LaunchAgents/com.juliocanizalez.caffeine.plist")
}

fn current_exe() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(String::from))
        .unwrap_or_else(|| "caffeine".to_string())
}

fn plist_content(exe_path: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.juliocanizalez.caffeine</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe_path}</string>
    </array>
    <key>RunAtLoad</key>
    <false/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
"#
    )
}

pub struct LaunchdLoginItemManager;

impl LoginItemManager for LaunchdLoginItemManager {
    fn is_enabled(&self) -> bool {
        plist_path().exists()
    }

    fn enable(&self) -> Result<(), String> {
        let path = plist_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&path, plist_content(&current_exe())).map_err(|e| e.to_string())?;
        Command::new("launchctl")
            .args(["load", path.to_str().unwrap_or("")])
            .status()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn disable(&self) -> Result<(), String> {
        let path = plist_path();
        if path.exists() {
            Command::new("launchctl")
                .args(["unload", path.to_str().unwrap_or("")])
                .status()
                .map_err(|e| e.to_string())?;
            fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
