use serde::{Deserialize, Serialize};

/// Snapshot of a running session — persisted to and read from the IPC file.
#[derive(Serialize)]
pub struct SessionInfo {
    pub pid: u32,
    pub expiry: Option<u64>,
    pub prevent_display: bool,
    pub jiggle: bool,
}

/// Jiggle behaviour parameters.
pub struct JiggleConfig {
    pub idle_threshold_secs: f64,
    pub interval_secs: u64,
}

impl Default for JiggleConfig {
    fn default() -> Self {
        Self {
            idle_threshold_secs: 300.0,
            interval_secs: 60,
        }
    }
}

/// Persistent user preferences loaded from `~/.config/caffeine/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaffeineConfig {
    /// Prevent display sleep in addition to system sleep (default: true).
    #[serde(default = "default_true")]
    pub prevent_display: bool,
    /// Enable jiggle mode by default (default: false).
    #[serde(default)]
    pub keep_status_active: bool,
    /// Auto-deactivate when battery drops below this percentage; 0 = disabled.
    #[serde(default)]
    pub battery_threshold: u8,
    /// Check GitHub for a newer release on startup (default: true).
    #[serde(default = "default_true")]
    pub check_for_updates: bool,
    /// Timed duration presets shown in the menu (e.g. ["15m", "30m", "1h"]).
    /// Omit to use the built-in defaults.
    #[serde(default = "default_menu_durations")]
    pub menu_durations: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn default_menu_durations() -> Vec<String> {
    ["15m", "30m", "1h", "2h", "4h"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

impl Default for CaffeineConfig {
    fn default() -> Self {
        Self {
            prevent_display: true,
            keep_status_active: false,
            battery_threshold: 0,
            check_for_updates: true,
            menu_durations: default_menu_durations(),
        }
    }
}
