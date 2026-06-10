use std::ffi::c_void;

use crate::domain::ports::{IdleDetector, Jiggler};

const CG_EVENT_SOURCE_STATE_COMBINED_SESSION: i32 = 1;
const CG_ANY_INPUT_EVENT_TYPE: u32 = 0xFFFF_FFFF;
const CG_HID_EVENT_TAP: i32 = 0;
/// kVK_F16 — unmapped on default Apple keyboards, so the event has no visible effect.
const VK_F16: u16 = 0x6A;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state_id: i32, event_type: u32) -> f64;
    fn CGEventCreateKeyboardEvent(
        source: *mut c_void,
        virtual_key: u16,
        key_down: bool,
    ) -> *mut c_void;
    fn CGEventPost(tap: i32, event: *mut c_void);
}

// Same signature as in infrastructure/power.rs — identical signatures across modules are fine.
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(cf: *const c_void);
}

fn idle_seconds() -> f64 {
    unsafe {
        CGEventSourceSecondsSinceLastEventType(
            CG_EVENT_SOURCE_STATE_COMBINED_SESSION,
            CG_ANY_INPUT_EVENT_TYPE,
        )
    }
}

fn jiggle() {
    unsafe {
        // Post a lone F16 key-UP. The event must register as real user input —
        // both our IdleDetector and Electron apps (Slack) read
        // CGEventSourceSecondsSinceLastEventType, and no-op events (e.g.
        // flags-changed with flags=0) are not counted by the WindowServer.
        // Key-up alone cannot trigger app shortcuts (those fire on key-down)
        // and, unlike mouse moves, does not surface browser media controls.
        let ev = CGEventCreateKeyboardEvent(std::ptr::null_mut(), VK_F16, false);
        if ev.is_null() {
            return;
        }
        CGEventPost(CG_HID_EVENT_TAP, ev);
        CFRelease(ev.cast_const());
    }
}

// ── Public infrastructure types ───────────────────────────────────────────────

pub struct CoreGraphicsIdleDetector;

impl IdleDetector for CoreGraphicsIdleDetector {
    fn idle_seconds(&self) -> f64 {
        idle_seconds()
    }
}

pub struct CoreGraphicsJiggler;

impl Jiggler for CoreGraphicsJiggler {
    fn jiggle(&self) {
        jiggle();
    }
}
