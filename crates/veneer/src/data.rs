//! Embedded dataset: parsed once, checked for integrity, then shared read-only.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Mutex, OnceLock, RwLock};

use serde::Deserialize;

use crate::model::{Engine, Kind};
use crate::Error;

#[derive(Debug)]
pub(crate) struct Dataset {
    pub browsers: BTreeMap<String, BrowserDef>,
    pub os: BTreeMap<String, OsDef>,
    pub devices: Vec<DeviceDef>,
    pub locales: Vec<LocaleDef>,
}

impl Dataset {
    pub fn device(&self, id: &str) -> Option<&DeviceDef> {
        self.devices.iter().find(|d| d.id == id)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BrowserDef {
    pub name: String,
    pub engine: Engine,
    #[serde(default)]
    pub client_hints: bool,
    #[serde(default)]
    pub ch_brand: Option<String>,
    pub toolbar: Toolbar,
    #[serde(default)]
    pub max_cores: Option<u32>,
    pub versions: Vec<VersionDef>,
    /// User-agent template per OS id this browser ships on.
    pub ua: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Toolbar {
    pub desktop: u32,
    pub mobile: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct VersionDef {
    pub major: u32,
    pub full: String,
    #[serde(default)]
    pub chromium: Option<u32>,
    #[serde(default)]
    pub chromium_full: Option<String>,
    /// App build number used by in-app browser user agents.
    #[serde(default)]
    pub build: Option<String>,
    /// Second per-version app number some in-app user agents carry (Facebook `FBRV`).
    #[serde(default)]
    pub revision: Option<String>,
    pub weight: u32,
}

impl VersionDef {
    pub fn chromium_major(&self) -> u32 {
        self.chromium.unwrap_or(self.major)
    }

    pub fn chromium_full(&self) -> &str {
        self.chromium_full.as_deref().unwrap_or(&self.full)
    }

    /// A version fixed by the OS (Safari on iOS), built from its full version string.
    pub fn pinned(full: &str) -> Self {
        let major = full
            .split('.')
            .next()
            .and_then(|m| m.parse().ok())
            .unwrap_or(0);
        VersionDef {
            major,
            full: full.to_owned(),
            chromium: None,
            chromium_full: None,
            build: None,
            revision: None,
            weight: 1,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OsDef {
    pub name: String,
    pub platform: String,
    #[serde(default)]
    pub ch_platform: Option<String>,
    #[serde(default)]
    pub architecture: String,
    #[serde(default)]
    pub bitness: String,
    /// `navigator.platform` in Firefox when it differs from other engines.
    #[serde(default)]
    pub gecko_platform: Option<String>,
    #[serde(default)]
    pub gecko_oscpu: Option<String>,
    #[serde(default)]
    pub gecko_app_version: Option<String>,
    pub insets: Insets,
    pub versions: Vec<OsVersionDef>,
    pub fonts: Vec<String>,
}

impl OsDef {
    pub fn version(&self, name: &str) -> Option<&OsVersionDef> {
        self.versions.iter().find(|v| v.name == name)
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Insets {
    pub top: u32,
    pub bottom: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OsVersionDef {
    pub name: String,
    pub platform_version: String,
    pub weight: u32,
    /// Value of `{osUa}` in user-agent templates (frozen on recent iOS).
    #[serde(default)]
    pub ua_token: Option<String>,
    /// OS build number (`23F77`), the `{osBuild}` token some in-app user agents send.
    #[serde(default)]
    pub build: Option<String>,
    /// Browser versions fixed by this OS version, e.g. Safari on iOS.
    #[serde(default)]
    pub pins: BTreeMap<String, String>,
    #[serde(default)]
    pub exclude_browsers: Vec<String>,
    #[serde(default)]
    pub insets: Option<Insets>,
    #[serde(default)]
    pub extra_fonts: Vec<String>,
}

impl OsVersionDef {
    pub fn supports(&self, browser: &str) -> bool {
        !self.exclude_browsers.iter().any(|b| b == browser)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DeviceDef {
    pub id: String,
    pub os: String,
    pub kind: Kind,
    #[serde(default)]
    pub model: Option<String>,
    /// Apple hardware identifier such as `iPhone17,1`, used by in-app user agents.
    #[serde(default)]
    pub hw_model: Option<String>,
    pub weight: u32,
    /// OS version names this device can run; all versions when absent.
    #[serde(default)]
    pub os_versions: Option<Vec<String>>,
    /// Overrides the OS architecture (Windows on ARM, Intel Macs).
    #[serde(default)]
    pub architecture: Option<String>,
    pub browsers: BTreeMap<String, u32>,
    pub cores: Vec<(u32, u32)>,
    pub memory: Vec<(u32, u32)>,
    pub touch_points: u32,
    #[serde(default)]
    pub insets: Option<Insets>,
    #[serde(default)]
    pub chromium_color_depth: Option<u32>,
    pub screens: Vec<ScreenDef>,
    pub webgl: BTreeMap<Engine, GpuStrings>,
}

impl DeviceDef {
    pub fn runs(&self, version: &OsVersionDef) -> bool {
        self.os_versions
            .as_ref()
            .is_none_or(|names| names.contains(&version.name))
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScreenDef {
    pub width: u32,
    pub height: u32,
    pub dpr: f64,
    pub weight: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GpuStrings {
    pub vendor: String,
    pub renderer: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LocaleDef {
    pub locale: String,
    pub languages: Vec<String>,
    pub weight: u32,
    pub timezones: Vec<(String, u32)>,
}

impl LocaleDef {
    pub fn region(&self) -> &str {
        self.locale.rsplit('-').next().unwrap_or_default()
    }
}

static DATASET: OnceLock<Result<Dataset, String>> = OnceLock::new();

pub(crate) fn dataset() -> Result<&'static Dataset, Error> {
    if let Some(extended) = EXTENDED.read().ok().and_then(|guard| *guard) {
        return Ok(extended);
    }
    DATASET
        .get_or_init(load)
        .as_ref()
        .map_err(|e| Error::Dataset(e.clone()))
}

fn parse<T: for<'de> Deserialize<'de>>(name: &str, text: &str) -> Result<T, String> {
    serde_json::from_str(text).map_err(|e| format!("{name}: {e}"))
}

fn load() -> Result<Dataset, String> {
    let data = parse_embedded()?;
    check(&data)?;
    Ok(data)
}

fn parse_embedded() -> Result<Dataset, String> {
    use crate::embedded::{BROWSER_FILES, DEVICE_FILES, OS_FILES};
    let mut data = Dataset {
        browsers: BTreeMap::new(),
        os: BTreeMap::new(),
        devices: Vec::new(),
        locales: parse("locales.json", include_str!("../data/locales.json"))?,
    };
    for (id, text) in OS_FILES {
        data.os
            .insert((*id).to_owned(), parse(&format!("os/{id}.json"), text)?);
    }
    for (id, text) in BROWSER_FILES {
        data.browsers.insert(
            (*id).to_owned(),
            parse(&format!("browsers/{id}.json"), text)?,
        );
    }
    for (family, text) in DEVICE_FILES {
        let devices: Vec<DeviceDef> = parse(&format!("devices/{family}.json"), text)?;
        data.devices.extend(devices);
    }
    Ok(data)
}

/// Most extension files accepted over the life of the process.
pub const MAX_DATA_FILES: usize = 256;
/// Largest extension file accepted.
pub const MAX_DATA_FILE_BYTES: usize = 4 * 1024 * 1024;

/// One extra dataset file for [`extend_dataset`]. `path` is `os/<id>.json`, `browsers/<id>.json`,
/// `devices/<family>.json` or `locales.json`; `text` is its JSON, in the same format as the embedded files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataFile {
    pub path: String,
    pub text: String,
}

/// The merged dataset once extensions are loaded. Leaked on purpose: extending happens a few times per
/// process at most, and generated profiles borrow from the dataset for `'static`.
static EXTENDED: RwLock<Option<&'static Dataset>> = RwLock::new(None);
/// Every extension file accepted so far, in merge order.
static EXTENSIONS: Mutex<Vec<DataFile>> = Mutex::new(Vec::new());

/// Merge extra OS, browser, device or locale files into the dataset used by every later call.
///
/// Files of one call are merged in path order, after the files of earlier calls, so the result is the
/// same however the files were listed. `os/<id>.json` and `browsers/<id>.json` add or replace that id;
/// devices and locales add entries or replace the entry with the same `id` / `locale`. The merged
/// dataset must pass the same integrity checks as the embedded one; if it does not, nothing changes.
pub fn extend_dataset(files: &[DataFile]) -> Result<(), Error> {
    let mut accepted = EXTENSIONS
        .lock()
        .map_err(|_| Error::Dataset("extension state is unavailable".into()))?;
    if accepted.len().saturating_add(files.len()) > MAX_DATA_FILES {
        return Err(Error::Dataset(format!(
            "at most {MAX_DATA_FILES} extension files can be loaded"
        )));
    }
    let mut batch: Vec<&DataFile> = files.iter().collect();
    batch.sort_by(|a, b| a.path.cmp(&b.path));
    if batch.windows(2).any(|w| w[0].path == w[1].path) {
        return Err(Error::Dataset("extension file paths must be unique".into()));
    }
    let mut data = parse_embedded().map_err(Error::Dataset)?;
    for file in accepted.iter().chain(batch.iter().copied()) {
        merge(&mut data, file).map_err(|e| Error::Dataset(format!("extension {e}")))?;
    }
    check(&data).map_err(|e| Error::Dataset(format!("after extensions: {e}")))?;
    let mut slot = EXTENDED
        .write()
        .map_err(|_| Error::Dataset("extension state is unavailable".into()))?;
    *slot = Some(Box::leak(Box::new(data)));
    accepted.extend(batch.into_iter().cloned());
    Ok(())
}

/// Read `os/*.json`, `browsers/*.json`, `devices/*.json` and `locales.json` under `dir` (other files are
/// ignored) and merge them with [`extend_dataset`]. Returns how many files were merged.
pub fn extend_dataset_dir(dir: &Path) -> Result<usize, Error> {
    let fail = |what: &str, path: &Path, e: std::io::Error| {
        Error::Dataset(format!("cannot {what} {}: {e}", path.display()))
    };
    if !dir.is_dir() {
        return Err(Error::Dataset(format!(
            "{} is not a directory",
            dir.display()
        )));
    }
    let mut paths = Vec::new();
    for group in ["os", "browsers", "devices"] {
        let sub = dir.join(group);
        if !sub.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(&sub).map_err(|e| fail("read", &sub, e))? {
            let path = entry.map_err(|e| fail("read", &sub, e))?.path();
            if regular_file(&path) && path.extension().is_some_and(|x| x == "json") {
                let name = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
                    Error::Dataset(format!("non-UTF-8 file name in {}", sub.display()))
                })?;
                paths.push((format!("{group}/{name}"), path.clone()));
            }
        }
    }
    let locales = dir.join("locales.json");
    if regular_file(&locales) {
        paths.push(("locales.json".to_owned(), locales));
    }
    if paths.len() > MAX_DATA_FILES {
        return Err(Error::Dataset(format!(
            "at most {MAX_DATA_FILES} extension files can be loaded"
        )));
    }
    let mut files = Vec::with_capacity(paths.len());
    for (name, path) in paths {
        let size = std::fs::metadata(&path)
            .map_err(|e| fail("read", &path, e))?
            .len();
        if size > MAX_DATA_FILE_BYTES as u64 {
            return Err(Error::Dataset(format!(
                "{name} is larger than {MAX_DATA_FILE_BYTES} bytes"
            )));
        }
        let text = std::fs::read_to_string(&path).map_err(|e| fail("read", &path, e))?;
        files.push(DataFile { path: name, text });
    }
    extend_dataset(&files)?;
    Ok(files.len())
}

/// Symbolic links are not followed, so a data directory cannot pull in files from elsewhere.
fn regular_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file())
}

fn merge(data: &mut Dataset, file: &DataFile) -> Result<(), String> {
    let path = file.path.as_str();
    if file.text.len() > MAX_DATA_FILE_BYTES {
        return Err(format!("{path}: larger than {MAX_DATA_FILE_BYTES} bytes"));
    }
    if path == "locales.json" {
        for locale in parse::<Vec<LocaleDef>>(path, &file.text)? {
            match data.locales.iter_mut().find(|l| l.locale == locale.locale) {
                Some(slot) => *slot = locale,
                None => data.locales.push(locale),
            }
        }
        return Ok(());
    }
    let (group, stem) = path
        .split_once('/')
        .and_then(|(g, name)| Some((g, name.strip_suffix(".json")?)))
        .ok_or_else(|| {
            format!("{path}: expected os/<id>.json, browsers/<id>.json, devices/<family>.json or locales.json")
        })?;
    let valid_stem = (1..=64).contains(&stem.len())
        && stem
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_');
    if !valid_stem {
        return Err(format!(
            "{path}: file name must be 1-64 characters of a-z, 0-9, '-' or '_'"
        ));
    }
    match group {
        "os" => {
            data.os.insert(stem.to_owned(), parse(path, &file.text)?);
        }
        "browsers" => {
            data.browsers
                .insert(stem.to_owned(), parse(path, &file.text)?);
        }
        "devices" => {
            for device in parse::<Vec<DeviceDef>>(path, &file.text)? {
                match data.devices.iter_mut().find(|d| d.id == device.id) {
                    Some(slot) => *slot = device,
                    None => data.devices.push(device),
                }
            }
        }
        _ => return Err(format!("{path}: unknown group `{group}`")),
    }
    Ok(())
}

/// Cross-file invariants the generator relies on. Breaking one is a data bug, reported at first use.
fn check(d: &Dataset) -> Result<(), String> {
    fn nonzero<T>(what: &str, items: &[T], weight: impl Fn(&T) -> u32) -> Result<(), String> {
        if items.iter().any(|i| weight(i) > 0) {
            Ok(())
        } else {
            Err(format!(
                "{what}: needs at least one entry with a non-zero weight"
            ))
        }
    }

    for (id, b) in &d.browsers {
        nonzero(&format!("browser `{id}` versions"), &b.versions, |v| {
            v.weight
        })?;
        if b.client_hints && (b.engine != Engine::Chromium || b.ch_brand.is_none()) {
            return Err(format!(
                "browser `{id}`: clientHints needs engine chromium and a chBrand"
            ));
        }
        for os_id in b.ua.keys() {
            let os = d
                .os
                .get(os_id)
                .ok_or_else(|| format!("browser `{id}`: ua template for unknown os `{os_id}`"))?;
            if b.engine == Engine::Gecko
                && (os.gecko_oscpu.is_none() || os.gecko_app_version.is_none())
            {
                return Err(format!(
                    "browser `{id}` on `{os_id}`: gecko needs geckoOscpu and geckoAppVersion"
                ));
            }
        }
    }
    for (id, os) in &d.os {
        nonzero(&format!("os `{id}` versions"), &os.versions, |v| v.weight)?;
        let names: BTreeSet<&str> = os.versions.iter().map(|v| v.name.as_str()).collect();
        if names.len() != os.versions.len() {
            return Err(format!("os `{id}`: version names must be unique"));
        }
        for v in &os.versions {
            for browser in v.pins.keys().chain(&v.exclude_browsers) {
                if !d.browsers.contains_key(browser) {
                    return Err(format!("os `{id}` {}: unknown browser `{browser}`", v.name));
                }
            }
        }
    }
    let mut seen = BTreeSet::new();
    for dev in &d.devices {
        if !seen.insert(dev.id.as_str()) {
            return Err(format!("device `{}`: duplicate id", dev.id));
        }
        let os =
            d.os.get(&dev.os)
                .ok_or_else(|| format!("device `{}`: unknown os `{}`", dev.id, dev.os))?;
        for name in dev.os_versions.iter().flatten() {
            if os.version(name).is_none() {
                return Err(format!(
                    "device `{}`: unknown {} version `{name}`",
                    dev.id, dev.os
                ));
            }
        }
        for browser in dev.browsers.keys() {
            let template = d
                .browsers
                .get(browser)
                .and_then(|b| b.ua.get(&dev.os))
                .ok_or_else(|| {
                    format!(
                        "device `{}`: os `{}` has no ua template for `{browser}`",
                        dev.id, dev.os
                    )
                })?;
            if template.contains("{hwModel}") && dev.hw_model.is_none() {
                return Err(format!("device `{}`: `{browser}` needs hwModel", dev.id));
            }
            if !os
                .versions
                .iter()
                .any(|v| dev.runs(v) && v.supports(browser))
            {
                return Err(format!(
                    "device `{}`: no OS version runs `{browser}`",
                    dev.id
                ));
            }
        }
        let browser_weights: Vec<u32> = dev.browsers.values().copied().collect();
        nonzero(
            &format!("device `{}` browsers", dev.id),
            &browser_weights,
            |w| *w,
        )?;
        nonzero(&format!("device `{}` screens", dev.id), &dev.screens, |s| {
            s.weight
        })?;
        nonzero(&format!("device `{}` cores", dev.id), &dev.cores, |c| c.1)?;
        nonzero(&format!("device `{}` memory", dev.id), &dev.memory, |m| m.1)?;
        if dev.kind.is_handheld() && dev.touch_points == 0 {
            return Err(format!(
                "device `{}`: phones and tablets need touchPoints",
                dev.id
            ));
        }
        if dev.screens.iter().any(|s| !(0.5..=5.0).contains(&s.dpr)) {
            return Err(format!("device `{}`: dpr out of range", dev.id));
        }
    }
    nonzero("locales", &d.locales, |l| l.weight)?;
    for l in &d.locales {
        nonzero(
            &format!("locale `{}` timezones", l.locale),
            &l.timezones,
            |t| t.1,
        )?;
        if l.languages.first() != Some(&l.locale) {
            return Err(format!(
                "locale `{}`: languages must start with the locale",
                l.locale
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod extension_tests {
    use super::*;

    #[test]
    fn bad_extensions_are_rejected_and_change_nothing() {
        let before = dataset().map(|d| d.devices.len()).unwrap();
        for (path, text) in [
            ("devices/x.json", "not json"),
            ("devices/../x.json", "[]"),
            ("readme.md", "[]"),
            ("os/Windows.json", "{}"),
            // A device for an OS that does not exist fails the integrity check.
            (
                "devices/ghost.json",
                r#"[{"id":"ghost","os":"nope","kind":"phone","weight":1,"browsers":{"chrome":1},
                "cores":[[8,1]],"memory":[[8,1]],"touchPoints":5,
                "screens":[{"width":400,"height":800,"dpr":2,"weight":1}],"webgl":{}}]"#,
            ),
        ] {
            let err = extend_dataset(&[DataFile {
                path: path.into(),
                text: text.into(),
            }]);
            assert!(err.is_err(), "{path} was accepted");
        }
        assert_eq!(dataset().map(|d| d.devices.len()).unwrap(), before);
    }
}
