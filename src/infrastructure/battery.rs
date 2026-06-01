use std::ffi::{CString, c_void};

use crate::domain::ports::BatteryMonitor;

const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;
const K_CF_NUMBER_S_INT32_TYPE: i32 = 3;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPSCopyPowerSourcesInfo() -> *const c_void;
    fn IOPSCopyPowerSourcesList(blob: *const c_void) -> *const c_void;
    fn IOPSGetPowerSourceDescription(blob: *const c_void, ps: *const c_void) -> *const c_void;
}

// Same signatures as in power.rs and jiggle.rs — identical signatures across modules are fine.
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(cf: *const c_void);
    fn CFStringCreateWithCString(
        alloc: *const c_void,
        c_str: *const std::ffi::c_char,
        encoding: u32,
    ) -> *const c_void;
    fn CFArrayGetCount(arr: *const c_void) -> isize;
    fn CFArrayGetValueAtIndex(arr: *const c_void, idx: isize) -> *const c_void;
    fn CFDictionaryGetValue(dict: *const c_void, key: *const c_void) -> *const c_void;
    fn CFNumberGetValue(number: *const c_void, the_type: i32, value_ptr: *mut c_void) -> bool;
    fn CFStringCompare(s1: *const c_void, s2: *const c_void, compare_options: u64) -> i32;
}

struct OwnedCf(*const c_void);

impl Drop for OwnedCf {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0) };
        }
    }
}

fn cf_string(s: &str) -> Option<OwnedCf> {
    let c = CString::new(s).ok()?;
    let ptr = unsafe {
        CFStringCreateWithCString(std::ptr::null(), c.as_ptr(), K_CF_STRING_ENCODING_UTF8)
    };
    if ptr.is_null() {
        None
    } else {
        Some(OwnedCf(ptr))
    }
}

/// Returns `(battery_percent, is_on_ac)` for the first power source found.
/// Returns `None` on desktops with no battery.
fn query_battery() -> Option<(u8, bool)> {
    let blob = OwnedCf(unsafe { IOPSCopyPowerSourcesInfo() });
    if blob.0.is_null() {
        return None;
    }

    let list = OwnedCf(unsafe { IOPSCopyPowerSourcesList(blob.0) });
    if list.0.is_null() {
        return None;
    }

    let count = unsafe { CFArrayGetCount(list.0) };
    if count == 0 {
        return None;
    }

    let ps = unsafe { CFArrayGetValueAtIndex(list.0, 0) };
    if ps.is_null() {
        return None;
    }

    // desc is owned by blob — do NOT release it separately
    let desc = unsafe { IOPSGetPowerSourceDescription(blob.0, ps) };
    if desc.is_null() {
        return None;
    }

    let key_current = cf_string("Current Capacity")?;
    let current_val = unsafe { CFDictionaryGetValue(desc, key_current.0) };
    if current_val.is_null() {
        return None;
    }
    let mut current: i32 = 0;
    unsafe {
        CFNumberGetValue(
            current_val,
            K_CF_NUMBER_S_INT32_TYPE,
            &raw mut current as *mut c_void,
        )
    };

    let key_max = cf_string("Max Capacity")?;
    let max_val = unsafe { CFDictionaryGetValue(desc, key_max.0) };
    if max_val.is_null() {
        return None;
    }
    let mut max: i32 = 0;
    unsafe {
        CFNumberGetValue(
            max_val,
            K_CF_NUMBER_S_INT32_TYPE,
            &raw mut max as *mut c_void,
        )
    };

    if max == 0 {
        return None;
    }
    let percent = ((current as f64 / max as f64) * 100.0).round() as u8;

    let key_state = cf_string("Power Source State")?;
    let state_val = unsafe { CFDictionaryGetValue(desc, key_state.0) };

    let is_ac = if state_val.is_null() {
        true
    } else {
        let ac_str = cf_string("AC Power")?;
        let cmp = unsafe { CFStringCompare(state_val, ac_str.0, 0) };
        cmp == 0
    };

    Some((percent, is_ac))
}

pub struct IokitBatteryMonitor;

impl BatteryMonitor for IokitBatteryMonitor {
    fn battery_percent(&self) -> Option<u8> {
        query_battery().map(|(p, _)| p)
    }

    fn is_on_ac(&self) -> bool {
        query_battery().map(|(_, ac)| ac).unwrap_or(true)
    }
}
