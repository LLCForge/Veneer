//! Navigator and window surface that follows from device, OS, browser and locale: media device
//! kinds, PDF viewer, Network Information, Battery, storage quota, JS heap limit, AudioContext
//! latency, keyboard layout, speech voices, screen orientation, window geometry, notification
//! permission, and the APIs a browser does not have at all.
//!
//! Every field is re-derivable from the profile, except connection, battery and storage quota,
//! which are random within real ranges and validated by range. `None` means "unknown, leave the
//! browser's own value"; APIs that must not exist are listed in `absent`.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use rand_chacha::rand_core::RngCore;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::embedded::EXTRA_DATA_FILES;
use crate::model::{Engine, Kind, Profile};
use crate::validate::{Collector, Severity};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavApi {
    /// `MediaDeviceInfo.kind` of each entry `enumerateDevices()` returns before any permission
    /// (ids and labels are empty strings then).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_devices: Option<Vec<String>>,
    /// `navigator.pdfViewerEnabled`; `true` also means the fixed five-entry `navigator.plugins`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pdf_viewer_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection: Option<Connection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub battery: Option<Battery>,
    /// `navigator.storage.estimate().quota` in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_quota: Option<u64>,
    /// `performance.memory.jsHeapSizeLimit` in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub js_heap_size_limit: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<AudioLatency>,
    /// `navigator.keyboard.getLayoutMap()` entries: `KeyboardEvent.code` to unshifted character.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyboard_layout: Option<BTreeMap<String, String>>,
    /// `speechSynthesis.getVoices()` in order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voices: Option<Vec<Voice>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation: Option<Orientation>,
    /// Maximized browser window geometry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<WindowGeometry>,
    /// `Notification.permission` for a fresh visit (`default`; the Permissions API says `prompt`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notification_permission: Option<String>,
    /// APIs this browser does not expose, e.g. `getBattery`, `hid`, `Notification`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub absent: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub effective_type: String,
    /// Milliseconds; Chromium rounds to 50 ms and caps at 3000.
    pub rtt: u32,
    /// Mbps; Chromium rounds to 0.05 and caps at 10.
    pub downlink: f64,
    pub save_data: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Battery {
    pub charging: bool,
    /// Seconds; `None` is `Infinity`.
    pub charging_time: Option<f64>,
    /// Seconds; `None` is `Infinity`.
    pub discharging_time: Option<f64>,
    pub level: f64,
}

/// `AudioContext.baseLatency`: a fixed value, or a buffer size divided by the context's sample rate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioLatency {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_latency: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_latency_frames: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Voice {
    pub name: String,
    pub lang: String,
    pub local_service: bool,
    pub default: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Orientation {
    /// `screen.orientation.type`.
    #[serde(rename = "type")]
    pub kind: String,
    pub angle: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowGeometry {
    pub outer_width: u32,
    pub outer_height: u32,
    pub screen_x: i32,
    pub screen_y: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct VoiceData {
    windows_local: BTreeMap<String, Vec<String>>,
    chrome_network: Vec<(String, String)>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct KeyboardData {
    windows_locales: BTreeMap<String, String>,
    layouts: BTreeMap<String, BTreeMap<String, String>>,
}

struct Data {
    voices: VoiceData,
    keyboard: KeyboardData,
}

fn data() -> Result<&'static Data, String> {
    static DATA: OnceLock<Result<Data, String>> = OnceLock::new();
    DATA.get_or_init(|| {
        let parse = |stem: &str| -> Result<&'static str, String> {
            EXTRA_DATA_FILES
                .iter()
                .find(|(s, _)| *s == stem)
                .map(|(_, text)| *text)
                .ok_or_else(|| format!("data/extra/{stem}.json is missing"))
        };
        Ok(Data {
            voices: serde_json::from_str(parse("speech-voices")?)
                .map_err(|e| format!("data/extra/speech-voices.json: {e}"))?,
            keyboard: serde_json::from_str(parse("keyboard-layouts")?)
                .map_err(|e| format!("data/extra/keyboard-layouts.json: {e}"))?,
        })
    })
    .as_ref()
    .map_err(Clone::clone)
}

const DESKTOP_OS: [&str; 4] = ["windows", "macos", "linux", "chromeos"];
const GIB: u64 = 1 << 30;
const MIB: u64 = 1 << 20;

/// Everything that follows from the profile without randomness. Random fields stay `None`.
fn fixed(p: &Profile, d: &Data) -> NavApi {
    let engine = p.browser.engine;
    let browser = p.browser.id.as_str();
    let os = p.os.id.as_str();
    let kind = p.device.kind;
    let personal = matches!(
        kind,
        Kind::Desktop | Kind::Laptop | Kind::Phone | Kind::Tablet
    );
    let computer = matches!(kind, Kind::Desktop | Kind::Laptop) && DESKTOP_OS.contains(&os);
    let apple_mobile = matches!(os, "ios" | "ipados");
    let chromium = engine == Engine::Chromium;
    // WebKit gained screen.orientation, userActivation and pdfViewerEnabled in 16.4.
    let webkit_164 = match (engine, os) {
        (Engine::Webkit, "ios" | "ipados") => Some(version_at_least(&p.os.version, 16, 4)),
        (Engine::Webkit, "macos") => Some(version_at_least(&p.browser.version, 16, 4)),
        _ => None,
    };

    let mut absent: Vec<&str> = Vec::new();
    if !chromium {
        absent.extend([
            "connection",
            "getBattery",
            "keyboard",
            "performance.memory",
            "hid",
            "usb",
            "bluetooth",
        ]);
        if engine == Engine::Webkit || !computer || p.browser.major < 151 {
            absent.push("serial");
        }
    } else {
        match (browser, os) {
            ("brave", _) => absent.extend(["connection", "bluetooth", "serial"]),
            ("chrome", "android") if p.browser.major < 138 => absent.push("serial"),
            ("chrome" | "edge", "linux") => absent.push("bluetooth"),
            _ => {}
        }
        if matches!(browser, "chrome" | "brave" | "samsung") && os == "android" {
            absent.push("hid");
        }
    }
    if webkit_164 == Some(false) {
        absent.extend(["userActivation", "screen.orientation"]);
    }
    if engine == Engine::Webkit && apple_mobile {
        absent.push("Notification");
    }
    if computer {
        absent.push("window.orientation");
    }
    absent.sort_unstable();

    let media_devices = match kind {
        Kind::Laptop | Kind::Phone | Kind::Tablet => {
            let mut kinds = vec!["audioinput", "videoinput"];
            if chromium {
                kinds.push("audiooutput");
            }
            Some(kinds)
        }
        // No built-in camera or microphone assumed; Chromium lists the default output.
        Kind::Desktop => Some(if chromium {
            vec!["audiooutput"]
        } else {
            vec![]
        }),
        _ => None,
    };

    let pdf_viewer_enabled = match engine {
        Engine::Chromium if computer => Some(true),
        Engine::Chromium if os == "android" => Some(false),
        Engine::Gecko => Some(true),
        Engine::Webkit if os == "macos" && webkit_164 == Some(true) => Some(true),
        _ => None,
    };

    let js_heap_size_limit = match (chromium, computer, p.navigator.device_memory) {
        (true, true, Some(gb)) if gb > 0.0 => Some(heap_limit(gb)),
        _ => None,
    };

    let audio = match (engine, os) {
        (Engine::Chromium, "windows") => Some(AudioLatency {
            base_latency: Some(0.01),
            base_latency_frames: None,
        }),
        (Engine::Chromium, "macos") => Some(AudioLatency {
            base_latency: None,
            base_latency_frames: Some(256),
        }),
        (Engine::Gecko, _) => Some(AudioLatency {
            base_latency: Some(0.0),
            base_latency_frames: None,
        }),
        _ => None,
    };

    let keyboard_layout = (chromium && computer && os == "windows")
        .then(|| d.keyboard.windows_locales.get(&p.locale))
        .flatten()
        .and_then(|id| d.keyboard.layouts.get(id))
        .cloned();

    let voices = (browser == "chrome" && os == "windows")
        .then(|| d.voices.windows_local.get(&p.locale))
        .flatten()
        .map(|local| {
            let lang = p.locale.as_str();
            local
                .iter()
                .map(|name| (name.as_str(), lang, true))
                .chain(
                    d.voices
                        .chrome_network
                        .iter()
                        .map(|(n, l)| (n.as_str(), l.as_str(), false)),
                )
                .enumerate()
                .map(|(i, (name, lang, local_service))| Voice {
                    name: name.to_owned(),
                    lang: lang.to_owned(),
                    local_service,
                    default: i == 0,
                })
                .collect()
        });

    let orientation = (personal && webkit_164 != Some(false)).then(|| Orientation {
        kind: if p.screen.width >= p.screen.height {
            "landscape-primary".into()
        } else {
            "portrait-primary".into()
        },
        angle: 0,
    });

    let s = &p.screen;
    let top = i32::try_from(s.height.saturating_sub(s.avail_height)).unwrap_or(0);
    let window = match (computer, engine, os) {
        (true, Engine::Gecko, "windows") => Some(WindowGeometry {
            outer_width: s.avail_width + 16,
            outer_height: s.avail_height + 16,
            screen_x: -8,
            screen_y: -8,
        }),
        (true, Engine::Chromium, _) => Some(WindowGeometry {
            outer_width: s.avail_width,
            outer_height: s.avail_height,
            screen_x: 0,
            // The taskbar is at the bottom on Windows; on macOS the inset above is the menu bar.
            screen_y: if os == "macos" { top } else { 0 },
        }),
        _ => None,
    };

    NavApi {
        media_devices: media_devices.map(|v| v.into_iter().map(str::to_owned).collect()),
        pdf_viewer_enabled,
        connection: None,
        battery: None,
        storage_quota: None,
        js_heap_size_limit,
        audio,
        keyboard_layout,
        voices,
        orientation,
        window,
        notification_permission: (personal && !absent.contains(&"Notification"))
            .then(|| "default".to_owned()),
        absent: absent.into_iter().map(str::to_owned).collect(),
    }
}

/// Which random fields a profile must carry.
fn has_connection(p: &Profile) -> bool {
    p.browser.engine == Engine::Chromium && p.browser.id != "brave" && personal(p)
}
fn has_battery(p: &Profile) -> bool {
    has_connection(p)
}
fn personal(p: &Profile) -> bool {
    matches!(
        p.device.kind,
        Kind::Desktop | Kind::Laptop | Kind::Phone | Kind::Tablet
    )
}

fn version_at_least(version: &str, major: u32, minor: u32) -> bool {
    let mut parts = version.split('.').map(|x| x.parse::<u32>().unwrap_or(0));
    let v = (parts.next().unwrap_or(0), parts.next().unwrap_or(0));
    v >= (major, minor)
}

/// V8's default heap limit for `ram` GiB on 64-bit desktop: old space RAM/2 (at most 4 GiB) plus
/// three semi-spaces of RAM/128 rounded up to a power of two within [2, 32] MiB.
fn heap_limit(ram_gib: f64) -> u64 {
    let ram = (ram_gib * GIB as f64) as u64;
    let old = (ram / 2).min(4 * GIB);
    let semi = (ram / 4 / 32).next_power_of_two().clamp(2 * MIB, 32 * MIB);
    old + 3 * semi
}

fn below(rng: &mut ChaCha8Rng, n: u32) -> u32 {
    rng.next_u32() % n.max(1)
}

/// Total disk size in bytes a device of this kind plausibly has (marketed decimal size, minus
/// system partitions).
fn disk_bytes(rng: &mut ChaCha8Rng, kind: Kind) -> u64 {
    let sizes: &[u64] = match kind {
        Kind::Desktop | Kind::Laptop => &[256, 512, 512, 1000],
        _ => &[64, 128, 128, 256, 256, 512],
    };
    let gb = sizes[below(rng, sizes.len() as u32) as usize];
    gb * 1_000_000_000 - u64::from(500 + below(rng, 1500)) * MIB
}

fn quota(engine: Engine, disk: u64) -> u64 {
    match engine {
        // Firefox best-effort: 10 % of the disk, at most 10 GiB per group.
        Engine::Gecko => (disk / 10).min(10 * GIB),
        // Chromium: pool 80 % of the disk, 75 % of it per origin; Safari ~60 %.
        _ => (disk as f64 * 0.8 * 0.75) as u64,
    }
}

pub(crate) fn derive(p: &Profile, rng: &mut ChaCha8Rng) -> Option<NavApi> {
    let d = data().ok()?;
    let mut out = fixed(p, d);
    if has_connection(p) {
        out.connection = Some(Connection {
            effective_type: "4g".into(),
            rtt: 50 * (1 + below(rng, 3)),
            downlink: f64::from(30 + below(rng, 171)) * 0.05,
            save_data: false,
        });
    }
    if has_battery(p) {
        let on_mains = p.device.kind == Kind::Desktop || below(rng, 2) == 0;
        out.battery = Some(if on_mains {
            Battery {
                charging: true,
                charging_time: Some(0.0),
                discharging_time: None,
                level: 1.0,
            }
        } else {
            Battery {
                charging: false,
                charging_time: None,
                discharging_time: None,
                level: f64::from(20 + below(rng, 80)) / 100.0,
            }
        });
    }
    if personal(p) {
        out.storage_quota = Some(quota(p.browser.engine, disk_bytes(rng, p.device.kind)));
    }
    // Downlink must be a whole number of 0.05 steps when serialized.
    if let Some(c) = out.connection.as_mut() {
        c.downlink = (c.downlink * 100.0).round() / 100.0;
    }
    Some(out)
}

pub(crate) fn validate(p: &Profile, c: &mut Collector) {
    let Some(actual) = &p.ext.navapi else {
        return;
    };
    let d = match data() {
        Ok(d) => d,
        Err(e) => return c.push(Severity::Error, "navapi.data", e),
    };
    let mut expected = fixed(p, d);
    c.error_if(
        actual.connection.is_some() != has_connection(p),
        "navapi.connection",
        || {
            format!(
                "{} {} must {}expose navigator.connection",
                p.browser.name,
                p.os.name,
                if has_connection(p) { "" } else { "not " }
            )
        },
    );
    if let Some(n) = &actual.connection {
        let steps = n.downlink / 0.05;
        c.error_if(
            n.effective_type != "4g"
                || n.rtt == 0
                || n.rtt > 3000
                || n.rtt % 50 != 0
                || !(0.0..=10.0).contains(&n.downlink)
                || (steps - steps.round()).abs() > 1e-6,
            "navapi.connection",
            || format!("connection {n:?} is not what Chromium reports (rtt in 50 ms steps, downlink in 0.05 Mbps steps up to 10)"),
        );
    }
    c.error_if(
        actual.battery.is_some() != has_battery(p),
        "navapi.battery",
        || "battery values must be present exactly where the browser exposes getBattery".into(),
    );
    if let Some(b) = &actual.battery {
        let full = b.charging && b.level == 1.0 && b.charging_time == Some(0.0);
        let draining = !b.charging && (0.0..1.0).contains(&b.level) && b.charging_time.is_none();
        c.error_if(
            !(full || draining) || b.discharging_time.is_some(),
            "navapi.battery",
            || format!("battery state {b:?} is inconsistent"),
        );
        c.error_if(
            p.device.kind == Kind::Desktop && !full,
            "navapi.battery",
            || "a desktop has no battery: charging, level 1, chargingTime 0".into(),
        );
    }
    c.error_if(
        actual.storage_quota.is_some() != personal(p),
        "navapi.storageQuota",
        || "storage quota must be set for desktops, laptops, phones and tablets".into(),
    );
    if let Some(q) = actual.storage_quota {
        let (lo, hi) = match p.device.kind {
            Kind::Desktop | Kind::Laptop => (100, 1000),
            _ => (20, 512),
        };
        let disk = |gb: u64| gb * 1_000_000_000;
        let ok = match p.browser.engine {
            Engine::Gecko => q >= disk(lo) / 20 && q <= 10 * GIB,
            _ => q >= quota(p.browser.engine, disk(lo)) && q <= quota(p.browser.engine, disk(hi)),
        };
        c.error_if(!ok, "navapi.storageQuota", || {
            format!("storage quota {q} does not fit a {:?} disk", p.device.kind)
        });
    }
    expected.connection = actual.connection.clone();
    expected.battery = actual.battery.clone();
    expected.storage_quota = actual.storage_quota;
    if *actual != expected {
        let a = serde_json::to_value(actual).unwrap_or(Value::Null);
        let e = serde_json::to_value(&expected).unwrap_or(Value::Null);
        let fields: Vec<String> = e
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(k, v)| a.get(k.as_str()) != Some(*v))
            .map(|(k, _)| k.clone())
            .chain(
                a.as_object()
                    .into_iter()
                    .flatten()
                    .filter(|(k, _)| e.get(k.as_str()).is_none())
                    .map(|(k, _)| k.clone()),
            )
            .collect();
        c.push(
            Severity::Error,
            "navapi.mismatch",
            format!(
                "navigator/window values do not match the device, OS, browser and locale: {}",
                fields.join(", ")
            ),
        );
    }
}

/// (JSON pointer, expected) pairs the collector (`collect/60-navapi.js`) must reproduce.
pub(crate) fn live_checks(p: &Profile) -> Vec<(String, Value)> {
    let Some(n) = &p.ext.navapi else {
        return Vec::new();
    };
    let mut out: Vec<(String, Value)> = Vec::new();
    let mut add = |k: &str, v: Value| out.push((format!("/navapi/{k}"), v));
    if let Some(m) = &n.media_devices {
        add("mediaDevices", json!(m));
    }
    if let Some(pdf) = n.pdf_viewer_enabled {
        add("pdfViewerEnabled", json!(pdf));
        add(
            "plugins",
            json!(if pdf {
                PDF_PLUGINS.to_vec()
            } else {
                Vec::new()
            }),
        );
        add(
            "mimeTypes",
            json!(if pdf {
                vec!["application/pdf", "text/pdf"]
            } else {
                Vec::new()
            }),
        );
    }
    if let Some(c) = &n.connection {
        add("connection", json!(c));
    }
    if let Some(b) = &n.battery {
        add("battery", json!(b));
    }
    if let Some(q) = n.storage_quota {
        add("storageQuota", json!(q));
    }
    if let Some(h) = n.js_heap_size_limit {
        add("jsHeapSizeLimit", json!(h));
    }
    if let Some(a) = &n.audio {
        if let Some(l) = a.base_latency {
            add("audio/baseLatency", json!(l));
        }
        if let Some(f) = a.base_latency_frames {
            add("audio/baseLatencyFrames", json!(f));
        }
    }
    if let Some(k) = &n.keyboard_layout {
        add("keyboardLayout", json!(k));
    }
    if let Some(v) = &n.voices {
        add("voices", json!(v));
    }
    if let Some(o) = &n.orientation {
        add("orientation", json!(o));
    }
    if let Some(w) = &n.window {
        add("window", json!(w));
    }
    if let Some(s) = &n.notification_permission {
        add("notificationPermission", json!(s));
        add(
            "notificationQuery",
            json!(if s == "default" { "prompt" } else { s.as_str() }),
        );
    }
    let legacy = !n.absent.iter().any(|a| a == "window.orientation");
    if n.orientation.is_some() && legacy {
        add("windowOrientation", json!(0));
    }
    for name in &n.absent {
        add(&format!("present/{name}"), json!(false));
    }
    add("timezoneConsistent", json!(true));
    out
}

/// `navigator.plugins` names in the order the HTML standard fixes.
const PDF_PLUGINS: [&str; 5] = [
    "PDF Viewer",
    "Chrome PDF Viewer",
    "Chromium PDF Viewer",
    "Microsoft Edge PDF Viewer",
    "WebKit built-in PDF",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heap_limit_matches_observed_chromium() {
        // Chromium 64-bit on a 16+ GiB Windows machine reported 4395630592.
        assert_eq!(heap_limit(8.0), 4_395_630_592);
        assert_eq!(heap_limit(4.0), 2 * GIB + 96 * MIB);
    }

    #[test]
    fn data_files_parse_and_layouts_exist() {
        let d = data().unwrap();
        for id in d.keyboard.windows_locales.values() {
            assert!(d.keyboard.layouts.contains_key(id), "{id}");
        }
        assert_eq!(d.voices.chrome_network.len(), 19);
    }

    #[test]
    fn versions_compare() {
        assert!(version_at_least("16.6", 16, 4));
        assert!(!version_at_least("15.8", 16, 4));
        assert!(version_at_least("26", 16, 4));
    }

    fn rules(p: &Profile) -> Vec<String> {
        let mut c = Collector::default();
        validate(p, &mut c);
        c.finish().issues.into_iter().map(|i| i.rule).collect()
    }

    #[test]
    fn validator_catches_contradictions() {
        let p = crate::generate(&crate::Constraints {
            os: Some("windows".into()),
            browser: Some("chrome".into()),
            locale: Some("de-DE".into()),
            kind: Some(Kind::Laptop),
            seed: Some("navapi-test".into()),
            ..Default::default()
        })
        .unwrap();
        let n = p.ext.navapi.clone().unwrap();
        assert!(rules(&p).is_empty());
        assert_eq!(n.keyboard_layout.as_ref().unwrap()["KeyY"], "z");
        assert_eq!(
            n.voices.as_ref().unwrap()[0].name,
            "Microsoft Hedda - German (Germany)"
        );

        let mut bad = p.clone();
        bad.ext.navapi.as_mut().unwrap().pdf_viewer_enabled = Some(false);
        assert_eq!(rules(&bad), ["navapi.mismatch"]);

        let mut bad = p.clone();
        bad.ext
            .navapi
            .as_mut()
            .unwrap()
            .connection
            .as_mut()
            .unwrap()
            .rtt = 120;
        assert_eq!(rules(&bad), ["navapi.connection"]);

        let mut bad = p.clone();
        bad.ext.navapi.as_mut().unwrap().battery = None;
        assert_eq!(rules(&bad), ["navapi.battery"]);

        let mut bad = p;
        bad.ext.navapi.as_mut().unwrap().storage_quota = Some(1);
        assert_eq!(rules(&bad), ["navapi.storageQuota"]);
    }

    #[test]
    fn engines_and_platforms() {
        let gen = |os: &str, browser: &str| {
            crate::generate(&crate::Constraints {
                os: Some(os.into()),
                browser: Some(browser.into()),
                seed: Some("navapi-engines".into()),
                ..Default::default()
            })
            .unwrap()
            .ext
            .navapi
            .unwrap()
        };
        let ios = gen("ios", "safari");
        assert!(ios.absent.contains(&"Notification".to_owned()));
        assert!(ios.connection.is_none() && ios.battery.is_none());
        let android = gen("android", "chrome");
        assert_eq!(android.pdf_viewer_enabled, Some(false));
        assert!(android.absent.contains(&"hid".to_owned()));
        let firefox = gen("windows", "firefox");
        assert_eq!(firefox.window.unwrap().screen_x, -8);
        assert!(firefox.storage_quota.unwrap() <= 10 * GIB);
    }
}
