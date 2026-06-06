use std::ffi::c_void;

use crate::domain::ports::{IdleDetector, Jiggler};

const CG_EVENT_SOURCE_STATE_COMBINED_SESSION: i32 = 1;
const CG_ANY_INPUT_EVENT_TYPE: u32 = 0xFFFF_FFFF;
const CG_EVENT_FLAGS_CHANGED: u32 = 12;
const CG_HID_EVENT_TAP: i32 = 0;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state_id: i32, event_type: u32) -> f64;
    fn CGEventCreate(source: *mut c_void) -> *mut c_void;
    fn CGEventSetType(event: *mut c_void, event_type: u32);
    fn CGEventSetFlags(event: *mut c_void, flags: u64);
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
        let ev = CGEventCreate(std::ptr::null_mut());
        if ev.is_null() {
            return;
        }
        // Post a null flags-changed event to reset the HID idle timer.
        // Unlike mouse-move events, this does not cause browsers to show media controls.
        CGEventSetType(ev, CG_EVENT_FLAGS_CHANGED);
        CGEventSetFlags(ev, 0);
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
