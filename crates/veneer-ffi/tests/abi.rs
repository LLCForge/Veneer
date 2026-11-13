use std::ffi::{c_char, CStr, CString};

use serde_json::Value;
use veneer_ffi::*;

fn take(ptr: *mut c_char) -> Value {
    assert!(!ptr.is_null());
    let text = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap().to_owned();
    unsafe { veneer_free(ptr) };
    serde_json::from_str(&text).unwrap()
}

fn ok(ptr: *mut c_char) -> Value {
    let v = take(ptr);
    assert!(v.get("error").is_none(), "unexpected error: {v}");
    v["ok"].clone()
}

fn err(ptr: *mut c_char) -> String {
    take(ptr)["error"].as_str().unwrap().to_owned()
}

fn c(s: &str) -> CString {
    CString::new(s).unwrap()
}

#[test]
fn full_round_trip() {
    let constraints = c(r#"{"os":"windows","browser":"chrome","seed":"ffi"}"#);
    let profile = ok(unsafe { veneer_generate(constraints.as_ptr()) });
    assert_eq!(profile["seed"], "ffi");
    let profile_c = c(&profile.to_string());

    let report = ok(unsafe { veneer_validate(profile_c.as_ptr()) });
    assert_eq!(report["valid"], true);
    assert_eq!(report["issues"].as_array().unwrap().len(), 0);

    let script = ok(unsafe { veneer_init_script(profile_c.as_ptr()) });
    assert!(script
        .as_str()
        .unwrap()
        .contains(profile["navigator"]["userAgent"].as_str().unwrap()));

    let headers = ok(unsafe { veneer_navigation_headers(profile_c.as_ptr()) });
    assert!(headers
        .as_array()
        .unwrap()
        .iter()
        .any(|h| h["name"] == "user-agent"));

    assert!(ok(veneer_catalog())["os"].is_array());
    assert!(ok(veneer_collector_script()).is_string());

    let observed = c(r#"{"userAgent":"x"}"#);
    let cmp = ok(unsafe { veneer_compare_observed(profile_c.as_ptr(), observed.as_ptr()) });
    assert_eq!(cmp["valid"], false);
    assert!(
        err(unsafe { veneer_compare_observed(profile_c.as_ptr(), std::ptr::null()) })
            .contains("must not be NULL")
    );
}

#[test]
fn same_seed_same_profile() {
    let constraints = c(r#"{"seed":"repeat"}"#);
    let a = ok(unsafe { veneer_generate(constraints.as_ptr()) });
    let b = ok(unsafe { veneer_generate(constraints.as_ptr()) });
    assert_eq!(a, b);
    // NULL constraints are allowed and pick a random seed.
    assert!(ok(unsafe { veneer_generate(std::ptr::null()) })["seed"].is_string());
}

#[test]
fn bad_input_is_an_error_envelope() {
    assert!(err(unsafe { veneer_validate(std::ptr::null()) }).contains("must not be NULL"));
    let junk = c("{not json");
    assert!(err(unsafe { veneer_validate(junk.as_ptr()) }).starts_with("invalid profile"));
    let unknown = c(r#"{"os":"amiga"}"#);
    assert!(!err(unsafe { veneer_generate(unknown.as_ptr()) }).is_empty());
    let typo = c(r#"{"oss":"windows"}"#);
    assert!(err(unsafe { veneer_generate(typo.as_ptr()) }).starts_with("invalid constraints"));
    let bad_utf8: [u8; 3] = [0xff, 0xfe, 0];
    let msg = err(unsafe { veneer_generate(bad_utf8.as_ptr().cast()) });
    assert!(msg.contains("UTF-8"));
    let huge = c(&" ".repeat(MAX_INPUT_BYTES + 1));
    assert!(err(unsafe { veneer_validate(huge.as_ptr()) }).contains("exceeds"));
    unsafe { veneer_free(std::ptr::null_mut()) };
}
