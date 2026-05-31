use std::ffi::c_void;

use crate::domain::ports::{IdleDetector, Jiggler};

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

const CG_EVENT_SOURCE_STATE_COMBINED_SESSION: i32 = 1;
const CG_ANY_INPUT_EVENT_TYPE: u32 = 0xFFFF_FFFF;
const CG_EVENT_MOUSE_MOVED: u32 = 5;
const CG_HID_EVENT_TAP: i32 = 0;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state_id: i32, event_type: u32) -> f64;
    fn CGEventCreate(source: *mut c_void) -> *mut c_void;
    fn CGEventGetLocation(event: *mut c_void) -> CGPoint;
    fn CGEventCreateMouseEvent(
        source: *mut c_void,
        mouse_type: u32,
        cursor_position: CGPoint,
        mouse_button: i32,
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
        let ev = CGEventCreate(std::ptr::null_mut());
        if ev.is_null() {
            return;
        }
        let pos = CGEventGetLocation(ev);
        CFRelease(ev.cast_const());

        let ev1 = CGEventCreateMouseEvent(
            std::ptr::null_mut(),
            CG_EVENT_MOUSE_MOVED,
            CGPoint {
                x: pos.x + 1.0,
                y: pos.y,
            },
            0,
        );
        if !ev1.is_null() {
            CGEventPost(CG_HID_EVENT_TAP, ev1);
            CFRelease(ev1.cast_const());
        }

        let ev2 = CGEventCreateMouseEvent(std::ptr::null_mut(), CG_EVENT_MOUSE_MOVED, pos, 0);
        if !ev2.is_null() {
            CGEventPost(CG_HID_EVENT_TAP, ev2);
            CFRelease(ev2.cast_const());
        }
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
