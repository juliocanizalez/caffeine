use std::time::{Duration, Instant};

use crate::domain::models::{JiggleConfig, SessionInfo};
use crate::domain::ports::{IdleDetector, Jiggler, PowerGuard, PowerManager, StatusRepository};
use crate::duration;

pub struct CaffeineService {
    power: Box<dyn PowerManager>,
    idle: Box<dyn IdleDetector>,
    jiggler: Box<dyn Jiggler>,
    repo: Box<dyn StatusRepository>,

    guard: Option<Box<dyn PowerGuard>>,
    expiry: Option<Instant>,
    prevent_display: bool,
    pub jiggle_enabled: bool,
    jiggle_config: JiggleConfig,
    last_jiggle: Option<Instant>,

    pid: u32,
}

impl CaffeineService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        power: Box<dyn PowerManager>,
        idle: Box<dyn IdleDetector>,
        jiggler: Box<dyn Jiggler>,
        repo: Box<dyn StatusRepository>,
        prevent_display: bool,
        jiggle_enabled: bool,
        pid: u32,
    ) -> Self {
        Self {
            power,
            idle,
            jiggler,
            repo,
            guard: None,
            expiry: None,
            prevent_display,
            jiggle_enabled,
            jiggle_config: JiggleConfig::default(),
            last_jiggle: None,
            pid,
        }
    }

    pub fn is_active(&self) -> bool {
        self.guard.is_some()
    }

    pub fn remaining(&self) -> Option<Duration> {
        self.expiry.map(|e| {
            e.checked_duration_since(Instant::now())
                .unwrap_or(Duration::ZERO)
        })
    }

    pub fn status_text(&self) -> String {
        if !self.is_active() {
            return "Inactive".into();
        }
        match self.expiry {
            None => "Active · indefinite".into(),
            Some(e) => {
                let rem = e
                    .checked_duration_since(Instant::now())
                    .unwrap_or(Duration::ZERO);
                format!("Active · {} remaining", duration::fmt(rem))
            }
        }
    }

    pub fn activate(&mut self, dur: Option<Duration>) {
        self.guard = self
            .power
            .acquire(self.prevent_display)
            .map_err(|e| eprintln!("caffeine: {e}"))
            .ok();
        self.expiry = dur.map(|d| Instant::now() + d);
    }

    pub fn deactivate(&mut self) {
        self.guard = None;
        self.expiry = None;
        self.last_jiggle = None;
    }

    /// Check expiry every tick; expire the session and clean up the status file.
    pub fn tick(&mut self) {
        if let Some(rem) = self.remaining()
            && rem.is_zero()
        {
            self.deactivate();
            self.repo.delete();
        }
    }

    pub fn set_jiggle_enabled(&mut self, enabled: bool) {
        self.jiggle_enabled = enabled;
        if !enabled {
            self.last_jiggle = None;
        }
    }

    /// Run jiggle logic if conditions are met (idle long enough, interval elapsed).
    pub fn maybe_jiggle(&mut self) {
        if !self.jiggle_enabled || !self.is_active() {
            return;
        }
        let idle = self.idle.idle_seconds();
        let due = self
            .last_jiggle
            .map(|t| t.elapsed().as_secs() >= self.jiggle_config.interval_secs)
            .unwrap_or(true);
        if idle >= self.jiggle_config.idle_threshold_secs && due {
            self.jiggler.jiggle();
            self.last_jiggle = Some(Instant::now());
        }
    }

    /// Write the current state to the IPC status file.
    pub fn sync_status(&self) {
        if !self.is_active() {
            self.repo.delete();
            return;
        }
        let now = self.repo.now_secs();
        let expiry = self.expiry.map(|e| {
            let rem = e.checked_duration_since(Instant::now()).unwrap_or_default();
            now + rem.as_secs()
        });
        self.repo.write(&SessionInfo {
            pid: self.pid,
            expiry,
            prevent_display: self.prevent_display,
            jiggle: self.jiggle_enabled,
        });
    }

    /// Deactivate and remove the status file — call before process exit.
    pub fn shutdown(&mut self) {
        self.deactivate();
        self.repo.delete();
    }
}
