use crate::domain::models::SessionInfo;

/// Port: acquire and hold OS-level power management assertions.
pub trait PowerManager: Send {
    /// Acquire assertions. Returns an opaque guard; dropping it releases them.
    fn acquire(&self, prevent_display: bool) -> Result<Box<dyn PowerGuard>, String>;
}

/// Opaque RAII handle. Dropping releases the underlying power assertion.
/// `Send` is required so `CaffeineService` can be moved into the tao event-loop closure.
pub trait PowerGuard: Send {}

/// Port: query how long the system has been idle.
pub trait IdleDetector: Send {
    fn idle_seconds(&self) -> f64;
}

/// Port: simulate user presence by posting a mouse-move event pair.
pub trait Jiggler: Send {
    fn jiggle(&self);
}

/// Port: persist and query the running-instance status file.
pub trait StatusRepository: Send {
    fn write(&self, info: &SessionInfo);
    fn read(&self) -> Option<SessionInfo>;
    fn delete(&self);
    fn is_alive(&self, pid: u32) -> bool;
    fn now_secs(&self) -> u64;
}
