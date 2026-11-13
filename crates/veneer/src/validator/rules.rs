//! Rule metadata: which profile fields each rule protects, the default repair and a human hint.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::custom::CustomRule;
use crate::model::{Constraints, Engine, Profile};
use crate::validate::{Fix, FixAction, Issue, Severity};
use crate::Error;

use FixAction::{Merge, Review, Set};
use Severity::{Error as E, Warning as W};

/// Where a rule gets its evidence from.
#[derive(Clone, Copy)]
enum Needs {
    Profile,
    Dataset,
    Headers,
    Geo,
    Today,
    Observed,
    Network,
}

impl Needs {
    fn name(self) -> &'static str {
        match self {
            Needs::Profile => "profile",
            Needs::Dataset => "dataset",
            Needs::Headers => "headers",
            Needs::Geo => "geo",
            Needs::Today => "today",
            Needs::Observed => "observed",
            Needs::Network => "network",
        }
    }
}

struct Meta {
    id: &'static str,
    severity: Severity,
    action: FixAction,
    /// The first field is where the fix applies.
    fields: &'static [&'static str],
    needs: Needs,
    hint: &'static str,
}

const fn m(
    id: &'static str,
    severity: Severity,
    action: FixAction,
    fields: &'static [&'static str],
    needs: Needs,
    hint: &'static str,
) -> Meta {
    Meta {
        id,
        severity,
        action,
        fields,
        needs,
        hint,
    }
}

use Needs::{
    Dataset as D, Geo as G, Headers as H, Network as N, Observed as O, Profile as P, Today as T,
};

const UACH: &str = "/navigator/userAgentData";

static RULES: &[Meta] = &[
    m("dataset.load", E, Review, &[""], D, "The embedded dataset failed to load; rebuild Veneer."),
    m("profile.known", W, Review, &["/os/id", "/browser/id"], D, "Use OS and browser ids from `veneer catalog` so the dataset rules can run."),
    m("browser.engine", E, Set, &["/browser/engine"], D, "Set the engine this browser really uses."),
    m("browser.available_on_os", E, Review, &["/browser/id", "/os/id"], D, "This browser does not ship on this OS; pick another browser or regenerate."),
    m("os.known_version", W, Review, &["/os/version"], D, "Use an OS version listed in the catalog."),
    m("device.known", W, Review, &["/device/id"], D, "Use a device id from the catalog to enable the device rules."),
    m("device.os", E, Review, &["/device/id", "/os/id"], D, "The device runs a different OS; regenerate with matching device and OS."),
    m("device.kind", E, Review, &["/device/kind"], D, "Set the form factor of this device."),
    m("device.os_version", E, Review, &["/os/version", "/device/id"], D, "This device cannot run this OS version; pick a supported version."),
    m("device.browser", W, Review, &["/browser/id"], D, "This browser is not known on this device."),
    m("device.touch_points", E, Set, &["/navigator/maxTouchPoints"], D, "Report the touch points this device really has."),
    m("browser.os_version_support", E, Review, &["/browser/id", "/os/version"], D, "This browser does not run on this OS version."),
    m("browser.os_version_pin", E, Set, &["/browser/version"], D, "This OS version ships a fixed browser version; use it."),
    m("browser.known_version", W, Review, &["/browser/version"], D, "Use a browser version listed in the catalog."),
    m("browser.stale_version", W, Review, &["/browser/version"], T, "This browser version is older than the allowed release age; update it."),
    m("ua.matches_template", E, Set, &["/navigator/userAgent"], D, "Use the user agent this browser sends on this device and OS version."),
    m("navigator.app_version", E, Set, &["/navigator/appVersion"], D, "appVersion is derived from the user agent; use the expected value."),
    m("navigator.oscpu", E, Set, &["/navigator/oscpu"], D, "oscpu exists only in Firefox; use the expected value."),
    m("navigator.platform", E, Set, &["/navigator/platform"], D, "Use the navigator.platform this engine reports on this OS."),
    m("navigator.hardware_concurrency_cap", E, Set, &["/navigator/hardwareConcurrency"], D, "This browser caps hardwareConcurrency; lower it."),
    m("uach.presence", E, Review, &[UACH], D, "Only Chromium browsers with client hints expose navigator.userAgentData."),
    m("uach.brands", E, Set, &["/navigator/userAgentData/brands"], D, "Use the GREASE brand list Chromium builds for this version."),
    m("uach.full_version_list", E, Set, &["/navigator/userAgentData/fullVersionList"], D, "Use the full version list for this browser version."),
    m("uach.mobile", E, Set, &["/navigator/userAgentData/mobile"], D, "mobile is true on phones only."),
    m("uach.platform", E, Set, &["/navigator/userAgentData/platform"], D, "Use the client-hint platform of this OS."),
    m("uach.architecture", E, Merge, &[UACH, "/navigator/userAgentData/architecture", "/navigator/userAgentData/bitness"], D, "Use the architecture and bitness of this device."),
    m("uach.platform_version", E, Set, &["/navigator/userAgentData/platformVersion"], D, "Use the platform version of this OS version."),
    m("uach.model", E, Set, &["/navigator/userAgentData/model"], D, "Use the model string of this device (empty on desktops)."),
    m("webgl.known_renderer", W, Review, &["/webgl/unmaskedRenderer"], D, "Use a GPU that exists on this OS for this engine."),
    m("fonts.os_base", W, Review, &["/fonts"], D, "Include every font the OS ships by default."),
    m("fonts.foreign", W, Review, &["/fonts"], D, "Remove fonts this OS does not ship."),
    m("locale.known", W, Review, &["/locale"], D, "Use a locale from the catalog."),
    m("locale.timezone_plausible", W, Review, &["/timezone", "/locale"], D, "Pick a timezone where this locale is common."),
    m("locale.timezone_region", W, Review, &["/timezone", "/locale"], D, "The timezone is not in the locale's region; pick one that is."),
    m("navigator.languages_typical", W, Set, &["/navigator/languages"], D, "Use the language list this locale usually has."),
    m("hardware.cores_device", W, Set, &["/navigator/hardwareConcurrency"], D, "Use a core count this device model ships with."),
    m("hardware.memory_device", W, Set, &["/navigator/deviceMemory"], D, "Use a memory size this device model ships with."),
    m("hardware.cores_gpu", W, Set, &["/navigator/hardwareConcurrency"], D, "No known device with this GPU has this many cores."),
    m("hardware.memory_gpu", W, Set, &["/navigator/deviceMemory"], D, "No known device with this GPU reports this memory size."),
    m("navigator.vendor", E, Set, &["/navigator/vendor"], P, "Use the navigator.vendor this engine reports."),
    m("navigator.device_memory", E, Review, &["/navigator/deviceMemory"], P, "Chromium reports deviceMemory as 0.25-8 (power of two); other engines do not have it."),
    m("navigator.hardware_concurrency", E, Review, &["/navigator/hardwareConcurrency"], P, "hardwareConcurrency must be between 1 and 256."),
    m("navigator.touch", E, Review, &["/navigator/maxTouchPoints"], P, "Phones and tablets report at least one touch point."),
    m("navigator.touch_desktop", W, Review, &["/navigator/maxTouchPoints"], P, "Desktop browsers rarely report touch points; use 0."),
    m("navigator.languages", E, Set, &["/navigator/language", "/navigator/languages"], P, "navigator.language must be the first entry of navigator.languages."),
    m("navigator.language_locale", W, Set, &["/locale", "/navigator/language"], P, "The profile locale usually equals navigator.language."),
    m("screen.avail_within_screen", E, Review, &["/screen/availWidth", "/screen/availHeight"], P, "The available screen area cannot exceed the screen."),
    m("screen.viewport_within_avail", E, Review, &["/viewport/width", "/viewport/height"], P, "The viewport cannot exceed the available screen area."),
    m("screen.device_pixel_ratio", E, Review, &["/screen/devicePixelRatio"], P, "devicePixelRatio must be between 0.5 and 5."),
    m("screen.pixel_depth", E, Set, &["/screen/pixelDepth", "/screen/colorDepth"], P, "pixelDepth always equals colorDepth."),
    m("screen.color_depth", W, Review, &["/screen/colorDepth"], P, "Real screens report a colorDepth of 24, 30 or 32."),
    m("screen.phone_width", W, Review, &["/screen/width"], P, "Phone screens are narrower than 600 CSS pixels."),
    m("screen.phone_portrait", W, Review, &["/screen/width", "/screen/height"], P, "Describe phones in portrait orientation."),
    m("screen.desktop_width", W, Review, &["/screen/width"], P, "Desktop screens are at least 1024 CSS pixels wide."),
    m("webgl.masked", E, Merge, &["/webgl", "/webgl/vendor", "/webgl/renderer"], P, "Use the masked WebGL vendor and renderer this engine reports."),
    m("webgl.unmasked_pair", E, Review, &["/webgl/unmaskedVendor", "/webgl/unmaskedRenderer"], P, "Set both unmasked WebGL strings or neither."),
    m("headers.user-agent", E, Review, &["/headers/user-agent", "/navigator/userAgent"], H, "The User-Agent header must equal navigator.userAgent."),
    m("headers.accept-language", E, Review, &["/headers/accept-language", "/navigator/languages"], H, "Accept-Language must list navigator.languages with this engine's q-values."),
    m("headers.sec-ch-ua", E, Review, &["/headers/sec-ch-ua", "/navigator/userAgentData/brands"], H, "Sec-CH-UA must list navigator.userAgentData.brands; browsers without client hints send none."),
    m("headers.sec-ch-ua-mobile", E, Review, &["/headers/sec-ch-ua-mobile", "/navigator/userAgentData/mobile"], H, "Sec-CH-UA-Mobile must match navigator.userAgentData.mobile."),
    m("headers.sec-ch-ua-platform", E, Review, &["/headers/sec-ch-ua-platform", "/navigator/userAgentData/platform"], H, "Sec-CH-UA-Platform must match navigator.userAgentData.platform."),
    m("headers.order", W, Review, &["/headers"], H, "This engine sends its navigation headers in a fixed order."),
    m("location.timezone", E, Review, &["/timezone"], G, "The profile timezone should be the timezone of the connection's location."),
    m("location.country", E, Review, &["/timezone"], G, "Pick a timezone used in the connection's country."),
    m("location.locale", W, Review, &["/locale"], G, "The locale belongs to another country than the connection; generate with --country to align it."),
    m("tls.ja4.protocol", W, Review, &["/ja4"], N, "Veneer cannot change TLS; launch the engine that matches the profile."),
    m("tls.ja4.version", W, Review, &["/ja4"], N, "Veneer cannot change TLS; launch the engine that matches the profile."),
    m("tls.ja4.sni", W, Review, &["/ja4"], N, "Veneer cannot change TLS; launch the engine that matches the profile."),
    m("tls.ja4.alpn", W, Review, &["/ja4"], N, "Veneer cannot change TLS; launch the engine that matches the profile."),
    m("tls.ja4.ciphers", W, Review, &["/ja4"], N, "Veneer cannot change TLS; launch the engine that matches the profile."),
    m("tls.ja4.cipher-hash", W, Review, &["/ja4"], N, "Veneer cannot change TLS; launch the engine that matches the profile."),
    m("live.webdriver", W, Review, &["/webdriver"], O, "navigator.webdriver is visible. Veneer reports automation flags; it does not hide them."),
];

fn find(id: &str) -> Option<&'static Meta> {
    RULES.iter().find(|r| r.id == id)
}

/// Fill in the hint and fix of an issue from the rule table.
pub(crate) fn annotate(issue: &mut Issue) {
    if issue.fix.as_ref().is_some_and(|f| !f.path.is_empty()) {
        if issue.hint.is_empty() {
            issue.hint = "See the message.".into();
        }
        return;
    }
    let expected = issue.fix.take().and_then(|f| f.expected);
    let meta = find(&issue.rule);
    let live = issue.rule.strip_prefix("live.");
    if issue.hint.is_empty() {
        issue.hint = match (meta, live) {
            (Some(m), _) => m.hint.to_owned(),
            (None, Some(_)) => "The browser reports a different value than the profile; apply it with the init script and context options.".into(),
            (None, None) => "See the message.".into(),
        };
    }
    let path = match (meta, live) {
        (Some(m), _) => m.fields.first().copied().unwrap_or_default().to_owned(),
        // live.screen.width checks the collector field /screen/width.
        (None, Some(rest)) => format!("/{}", rest.replace('.', "/")),
        (None, None) => format!("/{}", issue.rule.replace('.', "/")),
    };
    let default_action = meta.map_or(Review, |m| m.action);
    let action = match &expected {
        Some(Value::Null) => FixAction::Remove,
        Some(_) if default_action == Merge => Merge,
        Some(_) => Set,
        None => Review,
    };
    issue.fix = Some(Fix {
        path,
        action,
        expected: expected.filter(|v| !v.is_null()),
    });
}

/// One rule as reported by `rule_coverage`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleInfo {
    pub id: String,
    pub severity: Severity,
    /// JSON pointers into the profile (or `/headers/<name>`, `/webdriver`) the rule protects.
    pub fields: Vec<String>,
    /// `profile`, `dataset`, `headers`, `geo`, `today`, `observed`, `network` or `custom`.
    pub needs: String,
    pub hint: String,
}

/// Which profile field each rule protects, and which fields no rule protects.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Coverage {
    pub rules_version: String,
    pub rules: Vec<RuleInfo>,
    /// Profile field → rule ids that check it.
    pub fields: BTreeMap<String, Vec<String>>,
    /// Profile fields that no rule checks.
    pub unprotected: Vec<String>,
}

/// Built-in rules, plus `custom` ones, mapped onto the fields of generated profiles.
pub fn rule_coverage(custom: &[CustomRule]) -> Result<Coverage, Error> {
    super::custom::check_rules(custom)?;
    let mut rules: Vec<RuleInfo> = RULES
        .iter()
        .map(|r| RuleInfo {
            id: r.id.to_owned(),
            severity: r.severity,
            fields: r
                .fields
                .iter()
                .filter(|f| !f.is_empty())
                .map(|f| (*f).to_owned())
                .collect(),
            needs: r.needs.name().to_owned(),
            hint: r.hint.to_owned(),
        })
        .collect();
    rules.extend(custom.iter().map(|r| RuleInfo {
        id: r.id.clone(),
        severity: r.severity,
        fields: r.paths(),
        needs: "custom".into(),
        hint: r.hint.clone().unwrap_or_default(),
    }));

    let mut leaves = std::collections::BTreeSet::new();
    for engine in [Engine::Chromium, Engine::Gecko, Engine::Webkit] {
        let p: Profile = crate::generate(&Constraints {
            engine: Some(engine),
            seed: Some("coverage".into()),
            ..Default::default()
        })?;
        collect_leaves(&serde_json::to_value(&p)?, String::new(), &mut leaves);
    }
    let mut fields: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut unprotected = Vec::new();
    for leaf in leaves {
        let ids: Vec<String> = rules
            .iter()
            .filter(|r| r.fields.contains(&leaf))
            .map(|r| r.id.clone())
            .collect();
        if ids.is_empty() {
            unprotected.push(leaf.clone());
        }
        fields.insert(leaf, ids);
    }
    Ok(Coverage {
        rules_version: super::RULES_VERSION.to_owned(),
        rules,
        fields,
        unprotected,
    })
}

/// Object members become separate fields; arrays count as one field.
fn collect_leaves(v: &Value, at: String, out: &mut std::collections::BTreeSet<String>) {
    match v {
        Value::Object(map) => {
            for (k, child) in map {
                let key = k.replace('~', "~0").replace('/', "~1");
                collect_leaves(child, format!("{at}/{key}"), out);
            }
        }
        _ => {
            out.insert(at);
        }
    }
}

/// Metadata for SARIF: (id, severity, hint) of every built-in rule.
pub(crate) fn builtin() -> impl Iterator<Item = (&'static str, Severity, &'static str)> {
    RULES.iter().map(|r| (r.id, r.severity, r.hint))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rule_id_is_unique() {
        let mut ids: Vec<_> = RULES.iter().map(|r| r.id).collect();
        ids.sort_unstable();
        let n = ids.len();
        ids.dedup();
        assert_eq!(n, ids.len());
    }

    #[test]
    fn every_rule_in_the_source_has_metadata() {
        let sources = [
            include_str!("../validate.rs"),
            include_str!("plausibility.rs"),
            include_str!("context.rs"),
            include_str!("../network.rs"),
        ];
        for src in sources {
            for part in src.split('"').skip(1).step_by(2) {
                let looks_like_rule = part.contains('.')
                    && !part.contains(' ')
                    && part.split('.').next().is_some_and(|p| {
                        [
                            "dataset",
                            "profile",
                            "browser",
                            "os",
                            "device",
                            "ua",
                            "navigator",
                            "uach",
                            "webgl",
                            "fonts",
                            "locale",
                            "screen",
                            "hardware",
                            "location",
                            "tls",
                        ]
                        .contains(&p)
                    });
                if looks_like_rule && !part.ends_with(".rs") && !part.ends_with(".json") {
                    assert!(find(part).is_some(), "no metadata for rule `{part}`");
                }
            }
        }
    }

    #[test]
    fn coverage_maps_fields_and_lists_gaps() {
        let c = rule_coverage(&[]).unwrap();
        assert!(c.fields["/navigator/userAgent"].contains(&"ua.matches_template".to_owned()));
        assert!(c.unprotected.contains(&"/canvas/seed".to_owned()));
        assert!(!c.unprotected.contains(&"/navigator/platform".to_owned()));
    }
}
