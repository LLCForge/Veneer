//! wasm-bindgen surface: plain JS objects in, plain JS objects out.
#![forbid(unsafe_code)]

use serde::Serialize;
use wasm_bindgen::prelude::*;

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsError::new(&e.to_string()))
}

fn from_js<T: for<'de> serde::Deserialize<'de>>(value: JsValue, what: &str) -> Result<T, JsError> {
    serde_wasm_bindgen::from_value(value).map_err(|e| JsError::new(&format!("invalid {what}: {e}")))
}

/// Generate a profile. `constraints` may be `undefined`.
#[wasm_bindgen]
pub fn generate(constraints: JsValue) -> Result<JsValue, JsError> {
    let constraints: veneer::Constraints = if constraints.is_undefined() || constraints.is_null() {
        veneer::Constraints::default()
    } else {
        from_js(constraints, "constraints")?
    };
    let profile = veneer::generate(&constraints).map_err(|e| JsError::new(&e.to_string()))?;
    to_js(&profile)
}

/// Check a profile against every consistency rule.
#[wasm_bindgen]
pub fn validate(profile: JsValue) -> Result<JsValue, JsError> {
    let profile: veneer::Profile = from_js(profile, "profile")?;
    to_js(&veneer::validate(&profile))
}

/// Init script for `addInitScript` / `evaluateOnNewDocument`.
#[wasm_bindgen(js_name = toInitScript)]
pub fn to_init_script(profile: JsValue) -> Result<String, JsError> {
    let profile: veneer::Profile = from_js(profile, "profile")?;
    veneer::to_init_script(&profile).map_err(|e| JsError::new(&e.to_string()))
}

/// Every OS, browser, device and locale the generator knows.
#[wasm_bindgen]
pub fn catalog() -> Result<JsValue, JsError> {
    to_js(&veneer::catalog().map_err(|e| JsError::new(&e.to_string()))?)
}

/// Top-level navigation request headers, in browser order.
#[wasm_bindgen(js_name = navigationHeaders)]
pub fn navigation_headers(profile: JsValue) -> Result<JsValue, JsError> {
    let profile: veneer::Profile = from_js(profile, "profile")?;
    to_js(&veneer::navigation_headers(&profile))
}

/// Expression for `page.evaluate` that resolves to what the page can observe.
#[wasm_bindgen(js_name = collectorScript)]
pub fn collector_script() -> String {
    veneer::collector_script()
}

/// Compare collector output from a real browser against the profile.
#[wasm_bindgen(js_name = compareObserved)]
pub fn compare_observed(profile: JsValue, observed: JsValue) -> Result<JsValue, JsError> {
    let profile: veneer::Profile = from_js(profile, "profile")?;
    let observed: serde_json::Value = from_js(observed, "observed values")?;
    to_js(&veneer::compare_observed(&profile, &observed))
}

/// Compare captured request headers (`[{name, value}]`, `[[name, value]]` or `{name: value}`) with the profile.
#[wasm_bindgen(js_name = compareHeaders)]
pub fn compare_headers(profile: JsValue, headers: JsValue) -> Result<JsValue, JsError> {
    let profile: veneer::Profile = from_js(profile, "profile")?;
    let headers: serde_json::Value = from_js(headers, "headers")?;
    to_js(
        &veneer::compare_headers_json(&profile, &headers)
            .map_err(|e| JsError::new(&e.to_string()))?,
    )
}

/// Informational check of a captured JA4 TLS fingerprint against the profile's browser family.
#[wasm_bindgen(js_name = checkJa4)]
pub fn check_ja4(profile: JsValue, ja4: String) -> Result<JsValue, JsError> {
    let profile: veneer::Profile = from_js(profile, "profile")?;
    to_js(&veneer::check_ja4(&profile, &ja4).map_err(|e| JsError::new(&e.to_string()))?)
}

/// Check a proxy's country and/or timezone (supplied by the caller) against the profile.
#[wasm_bindgen(js_name = checkLocation)]
pub fn check_location(
    profile: JsValue,
    country: Option<String>,
    timezone: Option<String>,
) -> Result<JsValue, JsError> {
    let profile: veneer::Profile = from_js(profile, "profile")?;
    to_js(
        &veneer::check_location(&profile, country.as_deref(), timezone.as_deref())
            .map_err(|e| JsError::new(&e.to_string()))?,
    )
}

/// Launch settings for a WebRTC IP-handling policy on one engine.
#[wasm_bindgen(js_name = webrtcSettings)]
pub fn webrtc_settings(engine: JsValue, policy: JsValue) -> Result<JsValue, JsError> {
    let engine: veneer::Engine = from_js(engine, "engine")?;
    let policy: veneer::WebRtcPolicy = if policy.is_undefined() || policy.is_null() {
        veneer::WebRtcPolicy::PublicInterfaceOnly
    } else {
        from_js(policy, "policy")?
    };
    to_js(&veneer::webrtc_settings(engine, policy))
}

fn constraints_or_default(constraints: JsValue) -> Result<veneer::Constraints, JsError> {
    if constraints.is_undefined() || constraints.is_null() {
        Ok(veneer::Constraints::default())
    } else {
        from_js(constraints, "constraints")
    }
}

/// `count` profiles as `[{ id, profile }]`; with `unique`, no repeated (device, browser, version, screen, locale).
#[wasm_bindgen(js_name = generateBatch)]
pub fn generate_batch(constraints: JsValue, count: u32, unique: bool) -> Result<JsValue, JsError> {
    let constraints = constraints_or_default(constraints)?;
    let batch = veneer::generate_batch(&constraints, count, unique)
        .map_err(|e| JsError::new(&e.to_string()))?;
    to_js(&batch)
}

/// Regenerate the exact profile behind a profile id.
#[wasm_bindgen(js_name = generateFromId)]
pub fn generate_from_id(id: String) -> Result<JsValue, JsError> {
    to_js(&veneer::generate_from_id(&id).map_err(|e| JsError::new(&e.to_string()))?)
}

/// Short shareable id for constraints that include the profile's seed.
#[wasm_bindgen(js_name = profileId)]
pub fn profile_id(constraints: JsValue) -> Result<String, JsError> {
    let constraints = constraints_or_default(constraints)?;
    veneer::profile_id(&constraints).map_err(|e| JsError::new(&e.to_string()))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct JsDataFile {
    path: String,
    text: String,
}

/// Merge extra dataset files (`[{ path, text }]`, paths like `devices/x.json`) into the dataset.
#[wasm_bindgen(js_name = extendDataset)]
pub fn extend_dataset(files: JsValue) -> Result<(), JsError> {
    let files: Vec<JsDataFile> = from_js(files, "dataset files")?;
    let files: Vec<veneer::DataFile> = files
        .into_iter()
        .map(|f| veneer::DataFile {
            path: f.path,
            text: f.text,
        })
        .collect();
    veneer::extend_dataset(&files).map_err(|e| JsError::new(&e.to_string()))
}

fn options_or_default(options: JsValue) -> Result<veneer::ValidateOptions, JsError> {
    if options.is_undefined() || options.is_null() {
        Ok(veneer::ValidateOptions::default())
    } else {
        from_js(options, "validator options")
    }
}

fn import_format(format: Option<String>) -> Result<veneer::ImportFormat, JsError> {
    match format {
        None => Ok(veneer::ImportFormat::Auto),
        Some(f) => serde_json::from_value(serde_json::Value::String(f)).map_err(|_| {
            JsError::new("format must be auto, veneer, collector, fingerprint-suite or flat")
        }),
    }
}

/// Validate with options: disabled rules, custom JSON rules, request headers, location, date.
#[wasm_bindgen(js_name = validateWith)]
pub fn validate_with(profile: JsValue, options: JsValue) -> Result<JsValue, JsError> {
    let profile: veneer::Profile = from_js(profile, "profile")?;
    let options = options_or_default(options)?;
    to_js(&veneer::validate_with(&profile, &options).map_err(|e| JsError::new(&e.to_string()))?)
}

/// Full profile from collector output of a real browser, plus optional request headers.
#[wasm_bindgen]
pub fn capture(observed: JsValue, headers: JsValue) -> Result<JsValue, JsError> {
    let observed: serde_json::Value = from_js(observed, "observed values")?;
    let headers: Vec<veneer::Header> = if headers.is_undefined() || headers.is_null() {
        Vec::new()
    } else {
        from_js(headers, "headers")?
    };
    to_js(&veneer::capture(&observed, &headers).map_err(|e| JsError::new(&e.to_string()))?)
}

/// Profile from another format: `auto`, `veneer`, `collector`, `fingerprint-suite` or `flat`.
#[wasm_bindgen(js_name = importProfile)]
pub fn import_profile(input: JsValue, format: Option<String>) -> Result<JsValue, JsError> {
    let input: serde_json::Value = from_js(input, "input")?;
    let format = import_format(format)?;
    to_js(&veneer::import_profile(&input, format, &[]).map_err(|e| JsError::new(&e.to_string()))?)
}

/// SARIF 2.1.0 log for `[{ artifact, report }]`.
#[wasm_bindgen(js_name = toSarif)]
pub fn to_sarif(reports: JsValue, options: JsValue) -> Result<JsValue, JsError> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Item {
        artifact: String,
        report: veneer::Report,
    }
    let items: Vec<Item> = from_js(reports, "reports")?;
    let options = options_or_default(options)?;
    let pairs: Vec<(String, veneer::Report)> =
        items.into_iter().map(|i| (i.artifact, i.report)).collect();
    to_js(&veneer::to_sarif(&pairs, &options.rules))
}

/// Share of OS, browser, engine, device class and locale against the dataset distribution.
#[wasm_bindgen]
pub fn drift(profiles: JsValue) -> Result<JsValue, JsError> {
    let profiles: Vec<veneer::Profile> = from_js(profiles, "profiles")?;
    to_js(&veneer::drift(&profiles).map_err(|e| JsError::new(&e.to_string()))?)
}

/// Every rule, the profile fields it protects and the fields no rule protects.
#[wasm_bindgen(js_name = ruleCoverage)]
pub fn rule_coverage(options: JsValue) -> Result<JsValue, JsError> {
    let options = options_or_default(options)?;
    to_js(&veneer::rule_coverage(&options.rules).map_err(|e| JsError::new(&e.to_string()))?)
}

/// Request headers of the document request in a HAR log.
#[wasm_bindgen(js_name = headersFromHar)]
pub fn headers_from_har(har: JsValue) -> Result<JsValue, JsError> {
    let har: serde_json::Value = from_js(har, "HAR")?;
    to_js(&veneer::headers_from_har(&har).map_err(|e| JsError::new(&e.to_string()))?)
}
