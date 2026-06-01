/// Snapshot of a running session — persisted to and read from the IPC file.
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
