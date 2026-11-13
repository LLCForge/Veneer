//! C ABI for Veneer: JSON in, JSON out.
//!
//! Every function returns a NUL-terminated UTF-8 JSON envelope, either `{"ok": <value>}` or
//! `{"error": "<message>"}`, allocated by this library. The caller must release it with
//! [`veneer_free`] exactly once. `NULL` is returned only if the envelope itself could not be
//! allocated. Panics never cross the boundary: they are caught and reported as an error envelope.
//!
//! A C ABI cannot avoid `unsafe`: it is confined to reading argument pointers (`read_arg`) and
//! reclaiming returned strings (`veneer_free`); everything else is safe Rust.

use std::ffi::{c_char, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};

use serde::Serialize;
use serde_json::{json, Value};

/// Largest JSON argument accepted (a profile is ~10 KB; collector output is smaller).
pub const MAX_INPUT_BYTES: usize = 4 * 1024 * 1024;

type Outcome = Result<Value, String>;

fn to_value<T: Serialize>(value: &T) -> Outcome {
    serde_json::to_value(value).map_err(|e| format!("serialization failed: {e}"))
}

fn parse<T: for<'de> serde::Deserialize<'de>>(text: &str, what: &str) -> Result<T, String> {
    serde_json::from_str(text).map_err(|e| format!("invalid {what}: {e}"))
}

/// Borrow a C string argument as validated UTF-8. `None` means the caller passed `NULL`.
///
/// # Safety
/// `ptr` must be `NULL` or point to a NUL-terminated buffer that stays valid for the call.
unsafe fn read_arg<'a>(ptr: *const c_char, what: &str) -> Result<Option<&'a str>, String> {
    if ptr.is_null() {
        return Ok(None);
    }
    // SAFETY: non-null and NUL-terminated per this function's contract.
    let bytes = unsafe { CStr::from_ptr(ptr) }.to_bytes();
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(format!("{what} exceeds {MAX_INPUT_BYTES} bytes"));
    }
    std::str::from_utf8(bytes)
        .map(Some)
        .map_err(|_| format!("{what} is not valid UTF-8"))
}

fn required<'a>(arg: Option<&'a str>, what: &str) -> Result<&'a str, String> {
    arg.ok_or_else(|| format!("{what} must not be NULL"))
}

fn envelope(outcome: Outcome) -> *mut c_char {
    let body = match outcome {
        Ok(value) => json!({ "ok": value }),
        Err(message) => json!({ "error": message }),
    };
    // serde_json escapes U+0000, so the serialized text never contains an interior NUL.
    match CString::new(body.to_string()) {
        Ok(text) => text.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn guarded(f: impl FnOnce() -> Outcome) -> *mut c_char {
    let outcome = catch_unwind(AssertUnwindSafe(f))
        .unwrap_or_else(|_| Err("internal error: veneer panicked".to_owned()));
    envelope(outcome)
}

fn profile_from(arg: Option<&str>) -> Result<veneer::Profile, String> {
    parse(required(arg, "profile")?, "profile")
}

/// Generate a profile. `constraints_json` may be `NULL` (no constraints).
///
/// # Safety
/// `constraints_json` must be `NULL` or a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn veneer_generate(constraints_json: *const c_char) -> *mut c_char {
    guarded(|| {
        // SAFETY: forwarded from this function's contract.
        let constraints: veneer::Constraints =
            match unsafe { read_arg(constraints_json, "constraints")? } {
                Some(text) => parse(text, "constraints")?,
                None => veneer::Constraints::default(),
            };
        to_value(&veneer::generate(&constraints).map_err(|e| e.to_string())?)
    })
}

/// Check a profile against every consistency rule; `ok` is a report `{valid, score, issues}`.
///
/// # Safety
/// `profile_json` must be a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn veneer_validate(profile_json: *const c_char) -> *mut c_char {
    guarded(|| {
        // SAFETY: forwarded from this function's contract.
        let profile = profile_from(unsafe { read_arg(profile_json, "profile")? })?;
        to_value(&veneer::validate(&profile))
    })
}

/// Init script for the browser (`ok` is a string).
///
/// # Safety
/// `profile_json` must be a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn veneer_init_script(profile_json: *const c_char) -> *mut c_char {
    guarded(|| {
        // SAFETY: forwarded from this function's contract.
        let profile = profile_from(unsafe { read_arg(profile_json, "profile")? })?;
        Ok(Value::String(
            veneer::to_init_script(&profile).map_err(|e| e.to_string())?,
        ))
    })
}

/// Navigation request headers in browser order (`ok` is `[{name, value}]`).
///
/// # Safety
/// `profile_json` must be a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn veneer_navigation_headers(profile_json: *const c_char) -> *mut c_char {
    guarded(|| {
        // SAFETY: forwarded from this function's contract.
        let profile = profile_from(unsafe { read_arg(profile_json, "profile")? })?;
        to_value(&veneer::navigation_headers(&profile))
    })
}

/// Every OS, browser, device and locale the generator knows.
#[no_mangle]
pub extern "C" fn veneer_catalog() -> *mut c_char {
    guarded(|| to_value(&veneer::catalog().map_err(|e| e.to_string())?))
}

/// Expression for a page `evaluate` call that resolves to what the page can observe (`ok` is a string).
#[no_mangle]
pub extern "C" fn veneer_collector_script() -> *mut c_char {
    guarded(|| Ok(Value::String(veneer::collector_script())))
}

/// Compare collector output from a real browser against the profile; `ok` is a report.
///
/// # Safety
/// Both arguments must be valid NUL-terminated strings.
#[no_mangle]
pub unsafe extern "C" fn veneer_compare_observed(
    profile_json: *const c_char,
    observed_json: *const c_char,
) -> *mut c_char {
    guarded(|| {
        // SAFETY: forwarded from this function's contract.
        let profile = profile_from(unsafe { read_arg(profile_json, "profile")? })?;
        // SAFETY: forwarded from this function's contract.
        let observed: Value = parse(
            required(
                unsafe { read_arg(observed_json, "observed values")? },
                "observed values",
            )?,
            "observed values",
        )?;
        to_value(&veneer::compare_observed(&profile, &observed))
    })
}

/// Release a string returned by any `veneer_*` function. `NULL` is ignored.
///
/// # Safety
/// `ptr` must be `NULL` or a pointer returned by this library that has not been freed yet.
#[no_mangle]
pub unsafe extern "C" fn veneer_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        // SAFETY: the pointer came from `CString::into_raw` in `envelope` and is freed once.
        drop(unsafe { CString::from_raw(ptr) });
    }
}
