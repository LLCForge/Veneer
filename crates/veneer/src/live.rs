//! Compare what a real browser reports (via `collector_script()`) against the profile it should present.

use serde_json::{json, Value};

use crate::model::Profile;
use crate::validate::{Collector, Fix, FixAction, Issue, Report, Severity};

/// Every observed value that differs from the profile is an `error` with rule id `live.<field>`.
pub fn compare_observed(profile: &Profile, observed: &Value) -> Report {
    let nav = &profile.navigator;
    let s = &profile.screen;
    let mut checks: Vec<(&str, Value)> = vec![
        ("/userAgent", json!(nav.user_agent)),
        ("/appVersion", json!(nav.app_version)),
        ("/platform", json!(nav.platform)),
        ("/vendor", json!(nav.vendor)),
        ("/language", json!(nav.language)),
        ("/languages", json!(nav.languages)),
        ("/hardwareConcurrency", json!(nav.hardware_concurrency)),
        ("/deviceMemory", json!(nav.device_memory)),
        ("/maxTouchPoints", json!(nav.max_touch_points)),
        ("/oscpu", json!(nav.oscpu)),
        ("/screen/width", json!(s.width)),
        ("/screen/height", json!(s.height)),
        ("/screen/availWidth", json!(s.avail_width)),
        ("/screen/availHeight", json!(s.avail_height)),
        ("/screen/colorDepth", json!(s.color_depth)),
        ("/screen/pixelDepth", json!(s.pixel_depth)),
        ("/devicePixelRatio", json!(s.device_pixel_ratio)),
        ("/timezone", json!(profile.timezone)),
        ("/webgl/vendor", json!(profile.webgl.vendor)),
        ("/webgl/renderer", json!(profile.webgl.renderer)),
    ];
    if profile.webgl.unmasked_vendor.is_some() {
        checks.push((
            "/webgl/unmaskedVendor",
            json!(profile.webgl.unmasked_vendor),
        ));
        checks.push((
            "/webgl/unmaskedRenderer",
            json!(profile.webgl.unmasked_renderer),
        ));
    }
    match &nav.user_agent_data {
        Some(u) => checks.extend([
            ("/userAgentData/brands", json!(u.brands)),
            ("/userAgentData/mobile", json!(u.mobile)),
            ("/userAgentData/platform", json!(u.platform)),
            ("/userAgentData/platformVersion", json!(u.platform_version)),
            ("/userAgentData/architecture", json!(u.architecture)),
            ("/userAgentData/bitness", json!(u.bitness)),
            ("/userAgentData/model", json!(u.model)),
            ("/userAgentData/fullVersionList", json!(u.full_version_list)),
        ]),
        None => checks.push(("/userAgentData", Value::Null)),
    }

    let mut c = Collector::default();
    // Extension fields come from collector modules an older or partial collector may lack: a
    // missing top-level key is reported once as not collected instead of one error per field.
    let mut not_collected: Vec<&str> = Vec::new();
    let extra = crate::extras::live_checks(profile);
    let extra = extra.iter().filter(|(p, _)| {
        let key = p
            .trim_start_matches('/')
            .split('/')
            .next()
            .unwrap_or_default();
        if observed.get(key).is_some() {
            return true;
        }
        if !not_collected.contains(&key) {
            not_collected.push(key);
        }
        false
    });
    let all = checks
        .into_iter()
        .chain(extra.map(|(p, v)| (p.as_str(), v.clone())))
        .collect::<Vec<_>>();
    for key in &not_collected {
        c.push_issue(Issue {
            rule: format!("live.not_collected.{key}"),
            severity: Severity::Warning,
            message: format!("the collector output has no `{key}`; its checks were skipped"),
            hint: "Use the collector of this Veneer version (`collector_script()`) so these fields are observed.".into(),
            fix: Some(Fix {
                path: format!("/{key}"),
                action: FixAction::Review,
                expected: None,
            }),
        });
    }
    for (pointer, expected) in all {
        let actual = observed.pointer(pointer).cloned().unwrap_or(Value::Null);
        if !same(&actual, &expected) {
            let rule = format!("live.{}", pointer.trim_start_matches('/').replace('/', "."));
            c.push_issue(Issue {
                rule,
                severity: Severity::Error,
                message: format!("expected {expected}, observed {actual}"),
                hint: "The browser reports a different value than the profile; apply it with the init script and context options.".into(),
                // The path points into the collector output; the browser, not the profile, must change.
                fix: Some(Fix {
                    path: pointer.to_owned(),
                    action: FixAction::Review,
                    expected: Some(expected),
                }),
            });
        }
    }
    c.warn_if(
        observed.pointer("/webdriver") == Some(&Value::Bool(true)),
        "live.webdriver",
        || "navigator.webdriver is true; the page can see automation".into(),
    );
    c.finish()
}

/// JSON equality that treats `1` and `1.0` as the same number.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| same(a, b))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

/// Where the collector ran.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LiveContext {
    /// A browser driven by Playwright/Puppeteer with the profile's context options applied.
    #[default]
    Automation,
    /// A regular browser tab with only the init script (for example the `/probe` page).
    Page,
}

/// Rule prefixes that only browser emulation can satisfy: a script cannot change CSS media features.
const EMULATION_ONLY: [&str; 1] = ["live.graphics.media."];

/// Like [`compare_observed`]; in [`LiveContext::Page`] emulation-only mismatches become warnings.
pub fn compare_observed_in(profile: &Profile, observed: &Value, context: LiveContext) -> Report {
    let mut report = compare_observed(profile, observed);
    if context == LiveContext::Page {
        for issue in &mut report.issues {
            if issue.severity == Severity::Error
                && EMULATION_ONLY.iter().any(|p| issue.rule.starts_with(p))
            {
                issue.severity = Severity::Warning;
                issue.message = format!(
                    "{} (needs browser emulation; a plain page cannot change CSS media features)",
                    issue.message
                );
            }
        }
        let errors = report
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count() as u32;
        let warnings = report.issues.len() as u32 - errors;
        report.valid = errors == 0;
        report.score = 100u32.saturating_sub(errors * 25 + warnings * 5);
    }
    report
}
