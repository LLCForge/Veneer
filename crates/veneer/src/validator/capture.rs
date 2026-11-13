//! Build a Veneer `Profile` from what a real page reports, or from another tool's profile JSON.
//!
//! Supported input formats:
//! - `veneer`: a Veneer profile (taken as is).
//! - `collector`: the output of Veneer's in-page collector (`collector_script()`), as returned by
//!   `page.evaluate`. This is what `capture` reads.
//! - `fingerprint-suite`: the object returned by Apify fingerprint-suite's
//!   `FingerprintGenerator.getFingerprint()` (`{ fingerprint: { navigator, screen, videoCard, fonts }, headers }`).
//! - `flat`: one flat object with common field names (`userAgent`/`user_agent`/`ua`, `platform`,
//!   `language`, `languages`, `hardwareConcurrency`/`cores`, `deviceMemory`/`memory`,
//!   `maxTouchPoints`/`touchPoints`, `screenWidth`, `screenHeight`, `availWidth`, `availHeight`,
//!   `colorDepth`, `devicePixelRatio`/`dpr`, `viewportWidth`/`innerWidth`, `timezone`/`timeZone`,
//!   `webglVendor`, `webglRenderer`, `gpuVendor`/`unmaskedVendor`, `gpuRenderer`/`unmaskedRenderer`, `fonts`).
//!
//! The device, OS version and browser version are found by matching the user agent against every
//! user-agent template in the dataset; GPU, screen, cores, memory and client hints break ties.
//! Values the input does not contain are derived from the dataset where Veneer would derive them
//! anyway, and every such field is listed in `Imported::missing`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::data::{dataset, BrowserDef, Dataset, DeviceDef, OsDef, OsVersionDef, VersionDef};
use crate::generate::{
    app_version, browser_versions, device_memory, fnv1a, masked_webgl, oscpu, platform,
    user_agent_data, vendor, UaContext,
};
use crate::headers::Header;
use crate::model::*;
use crate::Error;

const MAX_TEXT: usize = 1024;
const MAX_LIST: usize = 1000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImportFormat {
    /// Detect from the shape of the input.
    #[default]
    Auto,
    Veneer,
    Collector,
    FingerprintSuite,
    Flat,
}

/// A profile built from outside data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Imported {
    pub profile: Profile,
    /// The format that was read (never `auto`).
    pub format: ImportFormat,
    /// JSON pointers of profile fields the input did not contain; they were derived or left empty.
    pub missing: Vec<String>,
    /// Request headers that came with the input or were passed in, for header checks.
    pub headers: Vec<Header>,
}

/// Profile from Veneer collector output, plus optional request headers of the same page load.
pub fn capture(observed: &Value, headers: &[Header]) -> Result<Imported, Error> {
    import_profile(observed, ImportFormat::Collector, headers)
}

/// Profile from any supported format (see the module documentation).
pub fn import_profile(
    input: &Value,
    format: ImportFormat,
    headers: &[Header],
) -> Result<Imported, Error> {
    super::check_headers(headers)?;
    let format = match format {
        ImportFormat::Auto => detect(input),
        f => f,
    };
    if format == ImportFormat::Veneer {
        let profile: Profile = serde_json::from_value(input.clone())
            .map_err(|e| Error::InvalidConstraint(format!("not a Veneer profile: {e}")))?;
        return Ok(Imported {
            profile,
            format,
            missing: Vec::new(),
            headers: headers.to_vec(),
        });
    }
    let (root, mut all_headers) = match format {
        ImportFormat::FingerprintSuite => (
            input.get("fingerprint").ok_or_else(|| {
                Error::InvalidConstraint(
                    "fingerprint-suite input needs a `fingerprint` object".into(),
                )
            })?,
            header_map(input.get("headers"))?,
        ),
        _ => (input, Vec::new()),
    };
    for h in headers {
        all_headers.retain(|x| !x.name.eq_ignore_ascii_case(&h.name));
        all_headers.push(h.clone());
    }
    super::check_headers(&all_headers)?;
    let profile_and_missing = build(dataset()?, &Src { root }, &all_headers)?;
    Ok(Imported {
        profile: profile_and_missing.0,
        format,
        missing: profile_and_missing.1,
        headers: all_headers,
    })
}

fn detect(v: &Value) -> ImportFormat {
    if v.pointer("/fingerprint/navigator").is_some() {
        ImportFormat::FingerprintSuite
    } else if v.pointer("/device/id").is_some() && v.pointer("/browser/engine").is_some() {
        ImportFormat::Veneer
    } else if v.get("userAgent").is_some_and(Value::is_string)
        && v.get("screen").is_some_and(Value::is_object)
    {
        ImportFormat::Collector
    } else {
        ImportFormat::Flat
    }
}

fn header_map(v: Option<&Value>) -> Result<Vec<Header>, Error> {
    let Some(map) = v.and_then(Value::as_object) else {
        return Ok(Vec::new());
    };
    Ok(map
        .iter()
        .filter_map(|(k, v)| {
            v.as_str().map(|v| Header {
                name: k.to_ascii_lowercase(),
                value: v.to_owned(),
            })
        })
        .collect())
}

fn invalid(m: String) -> Error {
    Error::InvalidConstraint(m)
}

/// Field lookup over alternative JSON pointers.
struct Src<'a> {
    root: &'a Value,
}

impl Src<'_> {
    fn raw(&self, paths: &[&str]) -> Option<&Value> {
        paths.iter().find_map(|p| self.root.pointer(p))
    }

    fn text(&self, paths: &[&str]) -> Result<Option<String>, Error> {
        match self.raw(paths) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) if s.len() <= MAX_TEXT && !s.chars().any(char::is_control) => {
                Ok(Some(s.clone()))
            }
            Some(_) => Err(invalid(format!(
                "`{}` must be a string of at most {MAX_TEXT} bytes without control characters",
                paths[0]
            ))),
        }
    }

    fn num(&self, paths: &[&str]) -> Result<Option<f64>, Error> {
        match self.raw(paths) {
            None | Some(Value::Null) => Ok(None),
            Some(v) => v
                .as_f64()
                .filter(|n| n.is_finite() && *n >= 0.0 && *n <= 100_000.0)
                .map(Some)
                .ok_or_else(|| {
                    invalid(format!("`{}` must be a number from 0 to 100000", paths[0]))
                }),
        }
    }

    fn int(&self, paths: &[&str]) -> Result<Option<u32>, Error> {
        Ok(self.num(paths)?.map(|n| n.round() as u32))
    }

    fn list(&self, paths: &[&str]) -> Result<Option<Vec<String>>, Error> {
        match self.raw(paths) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Array(items)) if items.len() <= MAX_LIST => items
                .iter()
                .map(|i| match i.as_str() {
                    Some(s) if s.len() <= MAX_TEXT => Ok(s.to_owned()),
                    _ => Err(invalid(format!("`{}` must hold short strings", paths[0]))),
                })
                .collect::<Result<Vec<_>, _>>()
                .map(Some),
            Some(_) => Err(invalid(format!(
                "`{}` must be an array of at most {MAX_LIST} strings",
                paths[0]
            ))),
        }
    }
}

/// Dataset identity of the captured browser.
struct Identity<'a> {
    device: Option<&'a DeviceDef>,
    os_id: String,
    os: Option<&'a OsDef>,
    os_version: Option<&'a OsVersionDef>,
    browser_id: String,
    browser: Option<&'a BrowserDef>,
    version: VersionDef,
    kind: Kind,
    engine: Engine,
}

/// What the page reported, used to break ties between devices with the same user agent.
struct Evidence<'a> {
    ua: &'a str,
    locale: &'a str,
    screen: (u32, u32, f64),
    renderer: Option<&'a str>,
    cores: Option<u32>,
    memory: Option<f64>,
    touch: Option<u32>,
    uach: Option<&'a Value>,
}

const MARKERS: [(&str, u8); 6] = [
    ("{chromiumFull}", 4),
    ("{chromium}", 3),
    ("{major}", 1),
    ("{full}", 2),
    ("{build}", 5),
    ("{revision}", 6),
];

type Caps = [Option<(usize, usize)>; 7];

/// Match `s` against a pattern whose marker bytes 1-6 stand for runs of digits and dots. The same
/// marker must capture the same text each time it appears.
fn wildcard(p: &[u8], s: &[u8], at: usize, caps: &mut Caps) -> bool {
    let mut i = 0;
    while i < p.len() && !(1..=6).contains(&p[i]) {
        if s.get(at + i) != Some(&p[i]) {
            return false;
        }
        i += 1;
    }
    let pos = at + i;
    if i == p.len() {
        return pos == s.len();
    }
    let m = p[i] as usize;
    let run = s[pos.min(s.len())..]
        .iter()
        .take_while(|b| b.is_ascii_digit() || **b == b'.')
        .count();
    if let Some((a, b)) = caps[m] {
        let len = b - a;
        return len <= run
            && s[pos..pos + len] == s[a..b]
            && wildcard(&p[i + 1..], s, pos + len, caps);
    }
    for len in (1..=run).rev() {
        caps[m] = Some((pos, pos + len));
        if wildcard(&p[i + 1..], s, pos + len, caps) {
            return true;
        }
    }
    caps[m] = None;
    false
}

fn identify<'a>(data: &'a Dataset, e: &Evidence) -> Option<Identity<'a>> {
    let placeholder = VersionDef::pinned("0");
    let mut cache: HashMap<(&str, &str, &str, Kind, &str), Option<Caps>> = HashMap::new();
    let mut best: Option<(i32, u64, Identity<'a>)> = None;
    for device in &data.devices {
        let os = &data.os[&device.os];
        for (browser_id, share) in &device.browsers {
            let browser = &data.browsers[browser_id];
            let Some(template) = browser.ua.get(&device.os) else {
                continue;
            };
            for v in &os.versions {
                if !device.runs(v) || !v.supports(browser_id) {
                    continue;
                }
                let hw = device.hw_model.as_deref().unwrap_or_default();
                let key = (
                    browser_id.as_str(),
                    device.os.as_str(),
                    v.name.as_str(),
                    device.kind,
                    hw,
                );
                let caps = *cache.entry(key).or_insert_with(|| {
                    let mut t = template.clone();
                    for (name, byte) in MARKERS {
                        t = t.replace(name, &char::from(byte).to_string());
                    }
                    let ctx = UaContext {
                        version: &placeholder,
                        kind: device.kind,
                        os_version: v,
                        hw_model: hw,
                        locale: e.locale,
                        screen: e.screen,
                    };
                    let pattern = ctx.render(&t);
                    let mut caps: Caps = [None; 7];
                    wildcard(pattern.as_bytes(), e.ua.as_bytes(), 0, &mut caps).then_some(caps)
                });
                let Some(caps) = caps else { continue };
                let (version, known) = resolve_version(browser, v, browser_id, &caps, e);
                let score = score(device, browser, v, known, e);
                let weight = u64::from(device.weight) * u64::from(*share) * u64::from(v.weight);
                if best
                    .as_ref()
                    .is_none_or(|(s, w, _)| (score, weight) > (*s, *w))
                {
                    best = Some((
                        score,
                        weight,
                        Identity {
                            device: Some(device),
                            os_id: device.os.clone(),
                            os: Some(os),
                            os_version: Some(v),
                            browser_id: browser_id.clone(),
                            browser: Some(browser),
                            version,
                            kind: device.kind,
                            engine: browser.engine,
                        },
                    ));
                }
            }
        }
    }
    best.map(|(_, _, id)| id)
}

/// Dataset version consistent with what the user agent shows, else one built from it.
fn resolve_version(
    browser: &BrowserDef,
    v: &OsVersionDef,
    browser_id: &str,
    caps: &Caps,
    e: &Evidence,
) -> (VersionDef, bool) {
    let cap = |i: usize| caps[i].and_then(|(a, b)| e.ua.get(a..b));
    let (major, full, chromium, chromium_full, build, revision) =
        (cap(1), cap(2), cap(3), cap(4), cap(5), cap(6));
    let fits = |d: &VersionDef| {
        major.is_none_or(|m| m == d.major.to_string())
            && full.is_none_or(|f| f == d.full)
            && chromium.is_none_or(|c| c == d.chromium_major().to_string())
            && chromium_full.is_none_or(|c| c == d.chromium_full())
            && build.is_none_or(|b| Some(b) == d.build.as_deref())
            && revision.is_none_or(|r| Some(r) == d.revision.as_deref())
    };
    let defaults = Constraints::default();
    if let Some(d) = browser_versions(browser, v, browser_id, &defaults)
        .into_iter()
        .find(|d| fits(d))
    {
        return (d, true);
    }
    let parse = |s: Option<&str>| s.and_then(|s| s.split('.').next()?.parse::<u32>().ok());
    let brand_full = browser.ch_brand.as_deref().and_then(|brand| {
        e.uach?
            .get("fullVersionList")?
            .as_array()?
            .iter()
            .find(|b| b.get("brand").and_then(Value::as_str) == Some(brand))?
            .get("version")?
            .as_str()
            .map(str::to_owned)
    });
    let major_n = parse(major)
        .or(parse(full))
        .or(parse(chromium))
        .unwrap_or(0);
    let full = full
        .map(str::to_owned)
        .or(brand_full.filter(|f| parse(Some(f)) == Some(major_n)))
        .unwrap_or_else(|| format!("{major_n}.0.0.0"));
    (
        VersionDef {
            major: major_n,
            chromium: parse(chromium).or(parse(chromium_full)),
            chromium_full: chromium_full.map(str::to_owned),
            build: build.map(str::to_owned),
            revision: revision.map(str::to_owned),
            ..VersionDef::pinned(&full)
        },
        false,
    )
}

fn score(
    device: &DeviceDef,
    browser: &BrowserDef,
    v: &OsVersionDef,
    known: bool,
    e: &Evidence,
) -> i32 {
    let mut s = 0;
    if known {
        s += 2;
    }
    if let (Some(r), Some(g)) = (e.renderer, device.webgl.get(&browser.engine)) {
        if g.renderer == r {
            s += 8;
        }
    }
    let (w, h, dpr) = e.screen;
    if device
        .screens
        .iter()
        .any(|x| x.width == w && x.height == h && (x.dpr - dpr).abs() < 0.01)
    {
        s += 3;
    }
    let cap = |c: u32| browser.max_cores.map_or(c, |m| c.min(m));
    if e.cores
        .is_some_and(|c| device.cores.iter().any(|(n, _)| cap(*n) == c))
    {
        s += 1;
    }
    if e.memory.is_some_and(|m| {
        let android = device.os == "android";
        // Either bucket scheme: before and after Chromium 147.
        device.memory.iter().any(|(gb, _)| {
            device_memory(*gb, 0, android) == m || device_memory(*gb, 147, android) == m
        })
    }) {
        s += 1;
    }
    if e.touch == Some(device.touch_points) {
        s += 1;
    }
    let ch = |k: &str| e.uach.and_then(|u| u.get(k)).and_then(Value::as_str);
    match (e.uach, browser.client_hints) {
        (Some(_), true) => {
            let brands = e
                .uach
                .and_then(|u| u.get("brands"))
                .and_then(Value::as_array);
            let brand = browser.ch_brand.as_deref().unwrap_or_default();
            if brands.is_some_and(|b| {
                b.iter()
                    .any(|x| x.get("brand").and_then(Value::as_str) == Some(brand))
            }) {
                s += 4;
            }
            if ch("platformVersion") == Some(v.platform_version.as_str()) {
                s += 2;
            }
            if ch("model") == Some(device.model.as_deref().unwrap_or_default()) {
                s += 2;
            }
        }
        (None, false) => {}
        _ => s -= 4,
    }
    s
}

/// Last resort when no dataset template produces this user agent.
fn guess(ua: &str) -> Identity<'static> {
    let has = |t: &str| ua.contains(t);
    let (os_id, kind) = if has("iPhone") {
        ("ios", Kind::Phone)
    } else if has("iPad") {
        ("ipados", Kind::Tablet)
    } else if has("Android") {
        (
            "android",
            if has("Mobile") {
                Kind::Phone
            } else {
                Kind::Tablet
            },
        )
    } else if has("Windows") {
        ("windows", Kind::Desktop)
    } else if has("Macintosh") {
        ("macos", Kind::Desktop)
    } else if has("CrOS") {
        ("chromeos", Kind::Laptop)
    } else if has("Linux") {
        ("linux", Kind::Desktop)
    } else {
        ("unknown", Kind::Desktop)
    };
    let ios = matches!(os_id, "ios" | "ipados");
    let (browser_id, token, engine) = if has("Edg/") {
        ("edge", "Edg/", Engine::Chromium)
    } else if has("CriOS/") {
        ("chrome-ios", "CriOS/", Engine::Webkit)
    } else if has("FxiOS/") {
        ("firefox-ios", "FxiOS/", Engine::Webkit)
    } else if has("Firefox/") {
        ("firefox", "Firefox/", Engine::Gecko)
    } else if has("SamsungBrowser/") {
        ("samsung", "SamsungBrowser/", Engine::Chromium)
    } else if has("Chrome/") && !ios {
        ("chrome", "Chrome/", Engine::Chromium)
    } else if has("Version/") {
        ("safari", "Version/", Engine::Webkit)
    } else {
        (
            "unknown",
            "",
            if ios {
                Engine::Webkit
            } else {
                Engine::Chromium
            },
        )
    };
    let full: String = ua
        .split_once(token)
        .filter(|_| !token.is_empty())
        .map(|(_, rest)| {
            rest.chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect()
        })
        .unwrap_or_default();
    Identity {
        device: None,
        os_id: os_id.to_owned(),
        os: None,
        os_version: None,
        browser_id: browser_id.to_owned(),
        browser: None,
        version: VersionDef::pinned(&full),
        kind,
        engine,
    }
}

fn form_factor(kind: Kind) -> &'static str {
    match kind {
        Kind::Phone => "Mobile",
        Kind::Tablet => "Tablet",
        Kind::Vr => "XR",
        _ => "Desktop",
    }
}

fn brands(v: Option<&Value>) -> Vec<Brand> {
    v.and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .take(16)
                .filter_map(|b| {
                    Some(Brand {
                        brand: b.get("brand")?.as_str()?.chars().take(128).collect(),
                        version: b.get("version")?.as_str()?.chars().take(64).collect(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// First language of an Accept-Language header, then the rest without q-values.
fn languages_from_header(value: &str) -> Vec<String> {
    value
        .split(',')
        .filter_map(|l| l.split(';').next())
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty() && l.len() <= 35)
        .take(32)
        .collect()
}

fn build(data: &Dataset, src: &Src, headers: &[Header]) -> Result<(Profile, Vec<String>), Error> {
    let mut missing: Vec<String> = Vec::new();
    let mut miss = |field: &str| missing.push(field.to_owned());
    let header = |n: &str| {
        headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case(n))
            .map(|h| h.value.as_str())
    };

    let ua = match src.text(&["/userAgent", "/navigator/userAgent", "/user_agent", "/ua"])? {
        Some(ua) => ua,
        None => {
            miss("/navigator/userAgent");
            header("user-agent")
                .map(str::to_owned)
                .ok_or_else(|| invalid("no user agent in the input or headers".into()))?
        }
    };
    let language_field = src.text(&["/language", "/navigator/language", "/locale"])?;
    let languages = match src.list(&["/languages", "/navigator/languages"])? {
        Some(l) if !l.is_empty() => l,
        _ => {
            miss("/navigator/languages");
            let from_header = header("accept-language")
                .map(languages_from_header)
                .unwrap_or_default();
            if from_header.is_empty() {
                language_field.iter().cloned().collect()
            } else {
                from_header
            }
        }
    };
    let language = match language_field {
        Some(l) => l,
        None => {
            miss("/navigator/language");
            languages.first().cloned().ok_or_else(|| {
                invalid("no language, languages or Accept-Language in the input".into())
            })?
        }
    };
    let width = src.int(&["/screen/width", "/screenWidth", "/screen_width"])?;
    let height = src.int(&["/screen/height", "/screenHeight", "/screen_height"])?;
    let (Some(width), Some(height)) = (width, height) else {
        return Err(invalid(
            "the input needs the screen width and height".into(),
        ));
    };
    let dpr = match src.num(&[
        "/devicePixelRatio",
        "/screen/devicePixelRatio",
        "/dpr",
        "/pixelRatio",
        "/device_pixel_ratio",
    ])? {
        Some(d) => d,
        None => {
            miss("/screen/devicePixelRatio");
            1.0
        }
    };
    let uach = src
        .raw(&[
            "/userAgentData",
            "/navigator/userAgentData",
            "/user_agent_data",
        ])
        .filter(|v| v.is_object());
    let renderer = src.text(&[
        "/webgl/unmaskedRenderer",
        "/videoCard/renderer",
        "/unmaskedRenderer",
        "/gpuRenderer",
        "/gpu/renderer",
        "/webglRenderer",
    ])?;
    let cores = src.int(&[
        "/hardwareConcurrency",
        "/navigator/hardwareConcurrency",
        "/hardware_concurrency",
        "/cores",
        "/cpuCores",
    ])?;
    let memory = src.num(&[
        "/deviceMemory",
        "/navigator/deviceMemory",
        "/device_memory",
        "/memory",
    ])?;
    let touch = src.int(&[
        "/maxTouchPoints",
        "/navigator/maxTouchPoints",
        "/max_touch_points",
        "/touchPoints",
    ])?;

    let evidence = Evidence {
        ua: &ua,
        locale: &language,
        screen: (width, height, dpr),
        renderer: renderer.as_deref(),
        cores,
        memory,
        touch,
        uach,
    };
    let id = identify(data, &evidence).unwrap_or_else(|| guess(&ua));
    let engine = id.engine;
    let ctx = id.os_version.map(|v| UaContext {
        version: &id.version,
        kind: id.kind,
        os_version: v,
        hw_model: id
            .device
            .and_then(|d| d.hw_model.as_deref())
            .unwrap_or_default(),
        locale: &language,
        screen: (width, height, dpr),
    });

    let hardware_concurrency = match cores {
        Some(c) => c,
        None => {
            miss("/navigator/hardwareConcurrency");
            let top = id
                .device
                .and_then(|d| d.cores.iter().max_by_key(|c| c.1))
                .map(|c| c.0);
            let cap = id.browser.and_then(|b| b.max_cores);
            top.map(|c| cap.map_or(c, |m| c.min(m))).ok_or_else(|| {
                invalid(
                    "the input needs hardwareConcurrency (the device is not in the dataset)".into(),
                )
            })?
        }
    };
    let device_memory_value = match src.raw(&[
        "/deviceMemory",
        "/navigator/deviceMemory",
        "/device_memory",
        "/memory",
    ]) {
        Some(_) => memory,
        None if engine == Engine::Chromium => {
            miss("/navigator/deviceMemory");
            id.device
                .and_then(|d| d.memory.iter().max_by_key(|m| m.1))
                .map(|m| {
                    let android = id.device.is_some_and(|d| d.os == "android");
                    device_memory(m.0, id.version.chromium_major(), android)
                })
        }
        None => None,
    };
    let max_touch_points = match touch {
        Some(t) => t,
        None => {
            miss("/navigator/maxTouchPoints");
            id.device.map_or(0, |d| d.touch_points)
        }
    };
    let app_version_value =
        match src.text(&["/appVersion", "/navigator/appVersion", "/app_version"])? {
            Some(a) => a,
            None => {
                miss("/navigator/appVersion");
                match (id.os, &ctx) {
                    (Some(os), Some(ctx)) => app_version(engine, os, ctx, &ua),
                    _ => ua.strip_prefix("Mozilla/").unwrap_or(&ua).to_owned(),
                }
            }
        };
    let platform_value = match src.text(&["/platform", "/navigator/platform"])? {
        Some(p) => p,
        None => {
            miss("/navigator/platform");
            id.os
                .map(|os| platform(engine, os).to_owned())
                .unwrap_or_default()
        }
    };
    let vendor_value = match src.text(&["/vendor", "/navigator/vendor"])? {
        Some(v) => v,
        None => {
            miss("/navigator/vendor");
            vendor(engine).to_owned()
        }
    };
    let oscpu_value = match src.raw(&["/oscpu", "/navigator/oscpu"]) {
        Some(_) => src.text(&["/oscpu", "/navigator/oscpu"])?,
        None => {
            miss("/navigator/oscpu");
            match (id.os, &ctx) {
                (Some(os), Some(ctx)) => oscpu(engine, os, ctx),
                _ => None,
            }
        }
    };
    let ch_key_present = src
        .raw(&[
            "/userAgentData",
            "/navigator/userAgentData",
            "/user_agent_data",
        ])
        .is_some();
    let user_agent_data_value = match uach {
        Some(u) => {
            let s = |k: &str| {
                u.get(k)
                    .and_then(Value::as_str)
                    .map(|s| s.chars().take(MAX_TEXT).collect::<String>())
            };
            let full_version_list = brands(u.get("fullVersionList"));
            let brand = id.browser.and_then(|b| b.ch_brand.as_deref());
            Some(UserAgentData {
                brands: brands(u.get("brands")),
                ua_full_version: s("uaFullVersion")
                    .or_else(|| {
                        full_version_list
                            .iter()
                            .find(|b| Some(b.brand.as_str()) == brand)
                            .map(|b| b.version.clone())
                    })
                    .unwrap_or_else(|| id.version.full.clone()),
                full_version_list,
                mobile: u
                    .get("mobile")
                    .and_then(Value::as_bool)
                    .unwrap_or(id.kind == Kind::Phone),
                platform: s("platform").unwrap_or_default(),
                platform_version: s("platformVersion").unwrap_or_default(),
                architecture: s("architecture").unwrap_or_default(),
                bitness: s("bitness").unwrap_or_default(),
                model: s("model").unwrap_or_default(),
                form_factors: u
                    .get("formFactors")
                    .and_then(Value::as_array)
                    .map(|l| {
                        l.iter()
                            .filter_map(Value::as_str)
                            .take(8)
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_else(|| vec![form_factor(id.kind).to_owned()]),
                wow64: u.get("wow64").and_then(Value::as_bool).unwrap_or(false),
            })
        }
        None if !ch_key_present => match (id.browser, id.os, id.os_version, id.device) {
            (Some(b), Some(os), Some(v), Some(d)) if b.client_hints => {
                miss("/navigator/userAgentData");
                user_agent_data(b, &id.version, os, v, d)
            }
            _ => None,
        },
        None => None,
    };

    let avail_width = match src.int(&[
        "/screen/availWidth",
        "/availWidth",
        "/screenAvailWidth",
        "/avail_width",
    ])? {
        Some(w) => w,
        None => {
            miss("/screen/availWidth");
            width
        }
    };
    let avail_height = match src.int(&[
        "/screen/availHeight",
        "/availHeight",
        "/screenAvailHeight",
        "/avail_height",
    ])? {
        Some(h) => h,
        None => {
            miss("/screen/availHeight");
            height
        }
    };
    let color_depth = match src.int(&["/screen/colorDepth", "/colorDepth", "/color_depth"])? {
        Some(c) => c,
        None => {
            miss("/screen/colorDepth");
            24
        }
    };
    let pixel_depth = match src.int(&["/screen/pixelDepth", "/pixelDepth", "/pixel_depth"])? {
        Some(c) => c,
        None => {
            miss("/screen/pixelDepth");
            color_depth
        }
    };
    let vw = src.int(&[
        "/viewport/width",
        "/innerWidth",
        "/screen/innerWidth",
        "/viewportWidth",
        "/viewport_width",
    ])?;
    let vh = src.int(&[
        "/viewport/height",
        "/innerHeight",
        "/screen/innerHeight",
        "/viewportHeight",
        "/viewport_height",
    ])?;
    let viewport = match (vw, vh) {
        (Some(width), Some(height)) => Viewport { width, height },
        _ => {
            miss("/viewport");
            let toolbar = id.browser.map_or(0, |b| {
                if id.kind.is_handheld() {
                    b.toolbar.mobile
                } else {
                    b.toolbar.desktop
                }
            });
            Viewport {
                width: avail_width,
                height: avail_height.saturating_sub(toolbar),
            }
        }
    };

    let (mv, mr) = masked_webgl(engine);
    let masked_vendor = src.text(&["/webgl/vendor", "/webglVendor"])?;
    let masked_renderer = src.text(&["/webgl/renderer", "/webglRenderer"])?;
    if masked_vendor.is_none() || masked_renderer.is_none() {
        miss("/webgl/vendor");
    }
    let unmasked_vendor = src.text(&[
        "/webgl/unmaskedVendor",
        "/videoCard/vendor",
        "/unmaskedVendor",
        "/gpuVendor",
        "/gpu/vendor",
    ])?;
    let gpu = id.device.and_then(|d| d.webgl.get(&engine));
    let (unmasked_vendor, unmasked_renderer) = match (unmasked_vendor, renderer) {
        (Some(v), Some(r)) => (Some(v), Some(r)),
        (v, r) => {
            miss("/webgl/unmaskedRenderer");
            (
                v.or_else(|| gpu.map(|g| g.vendor.clone())),
                r.or_else(|| gpu.map(|g| g.renderer.clone())),
            )
        }
    };

    let fonts = match src.list(&["/fonts"])? {
        Some(f) => f,
        None => {
            miss("/fonts");
            let mut f: Vec<String> = id
                .os
                .map(|os| {
                    os.fonts
                        .iter()
                        .chain(id.os_version.iter().flat_map(|v| v.extra_fonts.iter()))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            f.sort_unstable();
            f.dedup();
            f
        }
    };
    let timezone = match src.text(&["/timezone", "/timeZone", "/timezoneId", "/tz"])? {
        Some(t) => t,
        None => {
            miss("/timezone");
            String::new()
        }
    };

    let seed = format!(
        "capture-{:016x}",
        fnv1a(format!("{ua}|{language}|{timezone}|{width}x{height}").as_bytes())
    );
    let profile = Profile {
        device: DeviceInfo {
            id: id
                .device
                .map_or_else(|| "unknown".to_owned(), |d| d.id.clone()),
            kind: id.kind,
            model: id.device.and_then(|d| d.model.clone()),
        },
        os: OsInfo {
            name: id.os.map_or_else(|| id.os_id.clone(), |o| o.name.clone()),
            id: id.os_id,
            version: id
                .os_version
                .map_or_else(|| "unknown".to_owned(), |v| v.name.clone()),
        },
        browser: BrowserInfo {
            name: id
                .browser
                .map_or_else(|| id.browser_id.clone(), |b| b.name.clone()),
            id: id.browser_id,
            engine,
            version: id.version.full.clone(),
            major: id.version.major,
        },
        navigator: Navigator {
            user_agent: ua,
            app_version: app_version_value,
            user_agent_data: user_agent_data_value,
            platform: platform_value,
            vendor: vendor_value,
            language: language.clone(),
            languages,
            hardware_concurrency,
            device_memory: device_memory_value,
            max_touch_points,
            oscpu: oscpu_value,
        },
        screen: Screen {
            width,
            height,
            avail_width,
            avail_height,
            color_depth,
            pixel_depth,
            device_pixel_ratio: dpr,
        },
        viewport,
        webgl: WebGl {
            vendor: masked_vendor.unwrap_or_else(|| mv.to_owned()),
            renderer: masked_renderer.unwrap_or_else(|| mr.to_owned()),
            unmasked_vendor,
            unmasked_renderer,
        },
        canvas: Noise {
            seed: fnv1a(format!("{seed}/canvas").as_bytes()) as u32,
        },
        audio: Noise {
            seed: fnv1a(format!("{seed}/audio").as_bytes()) as u32,
        },
        fonts,
        locale: language,
        timezone,
        seed,
        ext: Default::default(),
    };
    missing.dedup();
    Ok((profile, missing))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compare_observed, generate, navigation_headers, validate, Constraints};
    use serde_json::json;

    /// What the collector reports for a browser that presents `p` perfectly.
    fn observed(p: &Profile) -> Value {
        let n = &p.navigator;
        json!({
            "userAgent": n.user_agent, "appVersion": n.app_version, "platform": n.platform,
            "vendor": n.vendor, "language": n.language, "languages": n.languages,
            "hardwareConcurrency": n.hardware_concurrency, "deviceMemory": n.device_memory,
            "maxTouchPoints": n.max_touch_points, "oscpu": n.oscpu, "webdriver": false,
            "screen": { "width": p.screen.width, "height": p.screen.height,
                "availWidth": p.screen.avail_width, "availHeight": p.screen.avail_height,
                "colorDepth": p.screen.color_depth, "pixelDepth": p.screen.pixel_depth },
            "devicePixelRatio": p.screen.device_pixel_ratio, "timezone": p.timezone,
            "webgl": { "vendor": p.webgl.vendor, "renderer": p.webgl.renderer,
                "unmaskedVendor": p.webgl.unmasked_vendor, "unmaskedRenderer": p.webgl.unmasked_renderer },
            "userAgentData": n.user_agent_data.as_ref().map(|u| json!({
                "brands": u.brands, "mobile": u.mobile, "platform": u.platform,
                "architecture": u.architecture, "bitness": u.bitness, "model": u.model,
                "platformVersion": u.platform_version, "fullVersionList": u.full_version_list })),
        })
    }

    #[test]
    fn captured_profiles_of_generated_browsers_are_clean() {
        let cat = crate::catalog().unwrap();
        for (i, b) in cat.browsers.iter().enumerate() {
            for j in 0..6 {
                let p = generate(&Constraints {
                    browser: Some(b.id.clone()),
                    seed: Some(format!("cap-{i}-{j}")),
                    ..Default::default()
                })
                .unwrap();
                let got = capture(&observed(&p), &navigation_headers(&p)).unwrap();
                let c = &got.profile;
                // Same identity facts the page could reveal; fonts and viewport are not observed.
                assert_eq!(
                    (&c.os.id, &c.browser.id, c.browser.engine),
                    (&p.os.id, &p.browser.id, p.browser.engine),
                    "{}",
                    p.navigator.user_agent
                );
                assert_eq!(
                    c.browser.version, p.browser.version,
                    "{}",
                    p.navigator.user_agent
                );
                let report = validate(c);
                assert!(
                    report.issues.is_empty(),
                    "{} -> {:?}",
                    p.navigator.user_agent,
                    report.issues
                );
                assert!(compare_observed(c, &observed(&p)).issues.is_empty());
                assert!(got.missing.contains(&"/fonts".to_owned()));
            }
        }
    }

    #[test]
    fn unknown_user_agent_falls_back_to_a_guess() {
        let input = json!({
            "userAgent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/99.0.4844.51 Safari/537.36",
            "language": "en-US", "hardwareConcurrency": 8, "screenWidth": 1920, "screenHeight": 1080
        });
        let got = import_profile(&input, ImportFormat::Auto, &[]).unwrap();
        assert_eq!(got.format, ImportFormat::Flat);
        assert_eq!(got.profile.browser.id, "chrome");
        assert_eq!(got.profile.os.id, "windows");
        assert_eq!(got.profile.browser.major, 99);
        assert!(got.missing.contains(&"/timezone".to_owned()));
        let rules: Vec<_> = validate(&got.profile)
            .issues
            .into_iter()
            .map(|i| i.rule)
            .collect();
        assert!(
            rules.contains(&"device.known".to_owned())
                || rules.contains(&"browser.known_version".to_owned()),
            "{rules:?}"
        );
    }

    #[test]
    fn fingerprint_suite_shape_is_read() {
        let p = generate(&Constraints {
            browser: Some("firefox".into()),
            os: Some("windows".into()),
            seed: Some("fps".into()),
            ..Default::default()
        })
        .unwrap();
        let n = &p.navigator;
        let input = json!({
            "fingerprint": {
                "navigator": { "userAgent": n.user_agent, "language": n.language, "languages": n.languages,
                    "platform": n.platform, "hardwareConcurrency": n.hardware_concurrency,
                    "maxTouchPoints": 0, "oscpu": n.oscpu, "appVersion": n.app_version, "vendor": "" },
                "screen": { "width": p.screen.width, "height": p.screen.height, "availWidth": p.screen.avail_width,
                    "availHeight": p.screen.avail_height, "colorDepth": 24, "pixelDepth": 24,
                    "devicePixelRatio": p.screen.device_pixel_ratio, "innerWidth": p.viewport.width, "innerHeight": p.viewport.height },
                "videoCard": { "vendor": p.webgl.unmasked_vendor, "renderer": p.webgl.unmasked_renderer },
                "fonts": p.fonts
            },
            "headers": { "Accept-Language": "fr-FR" }
        });
        let got = import_profile(&input, ImportFormat::Auto, &[]).unwrap();
        assert_eq!(got.format, ImportFormat::FingerprintSuite);
        assert_eq!(got.profile.browser.id, "firefox");
        assert_eq!(got.headers[0].value, "fr-FR");
        assert!(got.missing.contains(&"/timezone".to_owned()));
    }

    #[test]
    fn bad_input_is_rejected() {
        assert!(import_profile(&json!({ "language": "en" }), ImportFormat::Flat, &[]).is_err());
        let long = "x".repeat(MAX_TEXT + 1);
        assert!(import_profile(
            &json!({ "ua": long, "language": "en", "screenWidth": 1, "screenHeight": 1 }),
            ImportFormat::Flat,
            &[]
        )
        .is_err());
        assert!(import_profile(&json!({ "device": {} }), ImportFormat::Veneer, &[]).is_err());
    }

    #[test]
    fn wildcard_backtracks_and_requires_equal_repeats() {
        let mut caps: Caps = [None; 7];
        assert!(wildcard(
            b"Chrome/\x01.0.0.0 x",
            b"Chrome/120.0.0.0 x",
            0,
            &mut caps
        ));
        assert_eq!(caps[1], Some((7, 10)));
        let mut caps: Caps = [None; 7];
        assert!(wildcard(
            b"rv:\x01.0) Firefox/\x01.0",
            b"rv:128.0) Firefox/128.0",
            0,
            &mut caps
        ));
        let mut caps: Caps = [None; 7];
        assert!(!wildcard(
            b"rv:\x01.0) Firefox/\x01.0",
            b"rv:128.0) Firefox/129.0",
            0,
            &mut caps
        ));
    }
}
