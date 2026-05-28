use std::ffi::{CString, c_void};

// ── IOKit / CoreFoundation types ──────────────────────────────────────────────

type IOReturn = i32;
type IOPMAssertionID = u32;
const K_IO_RETURN_SUCCESS: IOReturn = 0;
const K_IOPM_ASSERTION_LEVEL_ON: u32 = 255;
const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

// ── Framework bindings ────────────────────────────────────────────────────────

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPMAssertionCreateWithName(
        assertion_type: *const c_void,
        assertion_level: u32,
        assertion_name: *const c_void,
        assertion_id: *mut IOPMAssertionID,
    ) -> IOReturn;

    fn IOPMAssertionRelease(assertion_id: IOPMAssertionID) -> IOReturn;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithCString(
        alloc: *const c_void,
        c_str: *const std::ffi::c_char,
        encoding: u32,
    ) -> *const c_void;

    fn CFRelease(cf: *const c_void);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

struct CfString(*const c_void);

impl CfString {
    fn new(s: &str) -> Option<Self> {
        let c = CString::new(s).ok()?;
        let ptr = unsafe {
            CFStringCreateWithCString(std::ptr::null(), c.as_ptr(), K_CF_STRING_ENCODING_UTF8)
        };
        if ptr.is_null() { None } else { Some(Self(ptr)) }
    }

    fn as_ptr(&self) -> *const c_void {
        self.0
    }
}

impl Drop for CfString {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0) };
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// RAII guard that holds IOKit power management assertions.
/// Releasing it (or dropping it) lets the system sleep again.
pub struct AssertionGuard {
    display_id: Option<IOPMAssertionID>,
    idle_id: Option<IOPMAssertionID>,
}

impl AssertionGuard {
    /// Acquire power assertions.
    /// `prevent_display = true` also prevents the screen from turning off.
    pub fn acquire(prevent_display: bool) -> Result<Self, String> {
        let app_name = CfString::new("caffeine").ok_or("CFStringCreateWithCString failed")?;

        let display_id = if prevent_display {
            let ty = CfString::new("PreventUserIdleDisplaySleep")
                .ok_or("CFStringCreateWithCString failed")?;
            let mut id: IOPMAssertionID = 0;
            let ret = unsafe {
                IOPMAssertionCreateWithName(
                    ty.as_ptr(),
                    K_IOPM_ASSERTION_LEVEL_ON,
                    app_name.as_ptr(),
                    &mut id,
                )
            };
            if ret != K_IO_RETURN_SUCCESS {
                return Err(format!(
                    "IOPMAssertionCreateWithName(display) error: {ret:#x}"
                ));
            }
            Some(id)
        } else {
            None
        };

        let ty = CfString::new("NoIdleSleepAssertion").ok_or("CFStringCreateWithCString failed")?;
        let mut idle: IOPMAssertionID = 0;
        let ret = unsafe {
            IOPMAssertionCreateWithName(
                ty.as_ptr(),
                K_IOPM_ASSERTION_LEVEL_ON,
                app_name.as_ptr(),
                &mut idle,
            )
        };
        if ret != K_IO_RETURN_SUCCESS {
            if let Some(did) = display_id {
                unsafe { IOPMAssertionRelease(did) };
            }
            return Err(format!("IOPMAssertionCreateWithName(idle) error: {ret:#x}"));
        }

        Ok(Self {
            display_id,
            idle_id: Some(idle),
        })
    }
}

impl Drop for AssertionGuard {
    fn drop(&mut self) {
        if let Some(id) = self.display_id.take() {
            unsafe { IOPMAssertionRelease(id) };
        }
        if let Some(id) = self.idle_id.take() {
            unsafe { IOPMAssertionRelease(id) };
        }
    }
}
