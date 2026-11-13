//! Joint weighted sampling of (device, browser, OS version), then values derived from them.
//!
//! The (device, browser, OS version) draw uses an alias table (O(1) per draw). Releases before the
//! alias table used a linear cumulative scan, so the same seed now gives a different (still valid)
//! profile than it did in those releases.

mod alias;
mod id;

use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::Serialize;

use crate::data::{
    dataset, BrowserDef, Dataset, DeviceDef, LocaleDef, OsDef, OsVersionDef, ScreenDef, VersionDef,
};
use crate::model::*;
use crate::Error;
use alias::Alias;
use id::BatchRef;

const MAX_FIELD: usize = 128;
const MAX_SEED: usize = 256;
const MAX_DPR: f64 = 10.0;
/// Most profiles one [`generate_batch`] call returns.
pub const MAX_BATCH: u32 = 10_000;

/// Generate one coherent profile that satisfies `constraints`.
pub fn generate(constraints: &Constraints) -> Result<Profile, Error> {
    let data = dataset()?;
    check_input(data, constraints)?;
    let seed = constraints.seed.clone().unwrap_or_else(random_seed);
    cached_sampler(data, constraints)?
        .profile(seed, None)
        .map(|(profile, _)| profile)
}

/// The sampler for the most recent constraints, so loops over seeds skip re-enumerating the dataset.
fn cached_sampler(
    data: &'static Dataset,
    constraints: &Constraints,
) -> Result<Arc<Sampler<'static>>, Error> {
    static LAST: Mutex<Option<Arc<Sampler<'static>>>> = Mutex::new(None);
    let key = Constraints {
        seed: None,
        ..constraints.clone()
    };
    if let Ok(last) = LAST.lock() {
        if let Some(s) = last.as_ref() {
            if std::ptr::eq(s.data, data) && s.constraints == key {
                return Ok(Arc::clone(s));
            }
        }
    }
    let sampler = Arc::new(Sampler::new(data, key)?);
    if let Ok(mut last) = LAST.lock() {
        *last = Some(Arc::clone(&sampler));
    }
    Ok(sampler)
}

/// A generated profile and the short id that regenerates it with [`generate_from_id`].
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Generated {
    pub id: String,
    pub profile: Profile,
}

/// Generate `count` profiles (1 to [`MAX_BATCH`]) for the same constraints.
///
/// With more than one profile, profile `i` gets the seed `<seed>-<i>` (a random base seed when none is
/// given), so the whole batch is reproducible. With `unique`, no two profiles share the same
/// (device, browser, version, screen, locale); if fewer such configurations match the constraints than
/// `count`, this fails with [`Error::Unsatisfiable`] naming how many exist.
pub fn generate_batch(
    constraints: &Constraints,
    count: u32,
    unique: bool,
) -> Result<Vec<Generated>, Error> {
    let data = dataset()?;
    check_input(data, constraints)?;
    if !(1..=MAX_BATCH).contains(&count) {
        return Err(Error::InvalidConstraint(format!(
            "count must be between 1 and {MAX_BATCH}"
        )));
    }
    let base = constraints.seed.clone().unwrap_or_else(random_seed);
    let sampler = cached_sampler(data, constraints)?;
    let mut used = (unique && count > 1).then(Used::default);
    if used.is_some() {
        let space = sampler.space();
        if u64::from(count) > space {
            return Err(Error::Unsatisfiable(format!(
                "unique batch of {count} requested, but only {space} distinct (device, browser, version, screen, locale) configurations match {}",
                serde_json::to_string(constraints).unwrap_or_default()
            )));
        }
    }
    let id_constraints = Constraints {
        seed: Some(base.clone()),
        ..constraints.clone()
    };
    let mut out = Vec::with_capacity(count as usize);
    for index in 0..count {
        let seed = if count == 1 {
            base.clone()
        } else {
            format!("{base}-{index}")
        };
        let (profile, key) = sampler.profile(seed, used.as_ref())?;
        if let Some(used) = &mut used {
            used.insert(key);
        }
        let batch = (count > 1).then_some(BatchRef { index, unique });
        out.push(Generated {
            id: id::encode(&id_constraints, batch)?,
            profile,
        });
    }
    Ok(out)
}

/// Short shareable id for one profile. `constraints` must carry the profile's seed (`profile.seed`).
pub fn profile_id(constraints: &Constraints) -> Result<String, Error> {
    check_input(dataset()?, constraints)?;
    if constraints.seed.is_none() {
        return Err(Error::InvalidConstraint(
            "a profile id needs the profile's seed".into(),
        ));
    }
    id::encode(constraints, None)
}

/// Regenerate exactly the profile behind an id from [`profile_id`] or [`generate_batch`].
/// Ids are stable for a given dataset; after a dataset update the same id can give a different profile.
pub fn generate_from_id(id: &str) -> Result<Profile, Error> {
    let (constraints, batch) = id::decode(id)?;
    let data = dataset()?;
    check_input(data, &constraints)?;
    let base = constraints
        .seed
        .clone()
        .ok_or_else(|| Error::InvalidConstraint("profile id has no seed".into()))?;
    match batch {
        None => cached_sampler(data, &constraints)?
            .profile(base, None)
            .map(|(p, _)| p),
        // Profile 0 of a unique batch had nothing to avoid yet.
        Some(BatchRef { index, unique }) if !unique || index == 0 => {
            cached_sampler(data, &constraints)?
                .profile(format!("{base}-{index}"), None)
                .map(|(p, _)| p)
        }
        Some(BatchRef { index, .. }) => {
            let count = index
                .checked_add(1)
                .filter(|n| *n <= MAX_BATCH)
                .ok_or_else(|| {
                    Error::InvalidConstraint("profile id batch index is too large".into())
                })?;
            generate_batch(&constraints, count, true)?
                .pop()
                .map(|g| g.profile)
                .ok_or_else(|| Error::InvalidConstraint("profile id batch is empty".into()))
        }
    }
}

type ScreenKey = (u32, u32, u64);
/// What `unique` batches never repeat: device, browser, browser version, screen, locale.
type Key<'s> = (&'s str, &'s str, &'s str, ScreenKey, &'s str);

fn screen_key(s: &ScreenDef) -> ScreenKey {
    (s.width, s.height, s.dpr.to_bits())
}

/// Configurations already used in a unique batch, with counts that make "is anything left?" cheap.
#[derive(Default)]
struct Used<'s> {
    keys: HashSet<Key<'s>>,
    per_version: HashMap<(&'s str, &'s str, &'s str), u64>,
    per_screen: HashMap<(&'s str, &'s str, &'s str, ScreenKey), u64>,
}

impl<'s> Used<'s> {
    fn insert(&mut self, key: Key<'s>) {
        if self.keys.insert(key) {
            let (d, b, v, s, _) = key;
            *self.per_version.entry((d, b, v)).or_default() += 1;
            *self.per_screen.entry((d, b, v, s)).or_default() += 1;
        }
    }

    fn free_version(&self, d: &str, b: &str, v: &str, screens: u64, locales: u64) -> u64 {
        let used = self.per_version.get(&(d, b, v)).copied().unwrap_or(0);
        screens.saturating_mul(locales).saturating_sub(used)
    }

    fn free_screen(&self, d: &str, b: &str, v: &str, s: ScreenKey, locales: u64) -> u64 {
        let used = self.per_screen.get(&(d, b, v, s)).copied().unwrap_or(0);
        locales.saturating_sub(used)
    }
}

/// Everything that depends only on the constraints, prepared once and reused for every seed.
struct Sampler<'a> {
    data: &'a Dataset,
    /// Without the seed.
    constraints: Constraints,
    combos: Vec<Combo<'a>>,
    table: Alias,
    locales: Vec<&'a LocaleDef>,
}

impl<'a> Sampler<'a> {
    fn new(data: &'a Dataset, constraints: Constraints) -> Result<Self, Error> {
        let combos = combinations(data, &constraints);
        let table = Alias::new(combos.iter().map(|c| c.weight)).ok_or_else(|| {
            Error::Unsatisfiable(describe(&constraints, "device/browser/version"))
        })?;
        let locales: Vec<&LocaleDef> = data
            .locales
            .iter()
            .filter(|l| l.weight > 0 && locale_allowed(l, &constraints))
            .collect();
        if locales.is_empty() {
            return Err(Error::Unsatisfiable(describe(
                &constraints,
                "locale/country/timezone",
            )));
        }
        Ok(Sampler {
            data,
            constraints,
            combos,
            table,
            locales,
        })
    }

    fn screens(&self, device: &'a DeviceDef) -> impl Iterator<Item = &'a ScreenDef> + '_ {
        device
            .screens
            .iter()
            .filter(|s| s.weight > 0 && screen_allowed(s, &self.constraints))
    }

    fn free_combo(&self, used: &Used, c: &Combo) -> u64 {
        let screens = self.screens(c.device).count() as u64;
        let locales = self.locales.len() as u64;
        c.versions
            .iter()
            .map(|v| used.free_version(&c.device.id, c.browser, &v.full, screens, locales))
            .sum()
    }

    /// Number of distinct (device, browser, version, screen, locale) configurations.
    fn space(&self) -> u64 {
        let mut seen = HashSet::new();
        let locales = self.locales.len() as u64;
        self.combos
            .iter()
            .flat_map(|c| c.versions.iter().map(move |v| (c, v)))
            .filter(|(c, v)| seen.insert((c.device.id.as_str(), c.browser, v.full.as_str())))
            .map(|(c, _)| (self.screens(c.device).count() as u64).saturating_mul(locales))
            .fold(0u64, u64::saturating_add)
    }

    /// One profile for `seed`. With `used`, only configurations not in it are drawn.
    fn profile<'s>(
        &'s self,
        seed: String,
        used: Option<&Used<'s>>,
    ) -> Result<(Profile, Key<'s>), Error> {
        let data = self.data;
        let constraints = &self.constraints;
        let exhausted = || Error::Unsatisfiable(describe(constraints, "unused configuration"));
        let mut rng = ChaCha8Rng::seed_from_u64(fnv1a(seed.as_bytes()));

        let combo = match used {
            None => &self.combos[self.table.sample(&mut rng)],
            Some(u) => {
                let free: Vec<&Combo> = self
                    .combos
                    .iter()
                    .filter(|c| self.free_combo(u, c) > 0)
                    .collect();
                if free.len() == self.combos.len() {
                    &self.combos[self.table.sample(&mut rng)]
                } else {
                    let table = Alias::new(free.iter().map(|c| c.weight)).ok_or_else(exhausted)?;
                    free[table.sample(&mut rng)]
                }
            }
        };
        let (device, browser_id, os_version) = (combo.device, combo.browser, combo.os_version);
        let os = &data.os[&device.os];
        let browser = &data.browsers[browser_id];
        let screens: Vec<&ScreenDef> = self.screens(device).collect();
        let (screen_count, locale_count) = (screens.len() as u64, self.locales.len() as u64);

        let versions: Vec<&VersionDef> = combo
            .versions
            .iter()
            .filter(|v| {
                used.is_none_or(|u| {
                    u.free_version(&device.id, browser_id, &v.full, screen_count, locale_count) > 0
                })
            })
            .collect();
        let version = *pick(&mut rng, &versions, |v| u64::from(v.weight)).ok_or_else(exhausted)?;
        let screens: Vec<&ScreenDef> = screens
            .into_iter()
            .filter(|s| {
                used.is_none_or(|u| {
                    let key = screen_key(s);
                    u.free_screen(&device.id, browser_id, &version.full, key, locale_count) > 0
                })
            })
            .collect();
        let screen = *pick(&mut rng, &screens, |s| u64::from(s.weight)).ok_or_else(exhausted)?;
        let cores = required(pick(&mut rng, &device.cores, |c| u64::from(c.1)), "cores")?.0;
        let memory = required(pick(&mut rng, &device.memory, |m| u64::from(m.1)), "memory")?.0;
        let key_for = |l: &'s LocaleDef| -> Key<'s> {
            (
                device.id.as_str(),
                browser_id,
                version.full.as_str(),
                screen_key(screen),
                l.locale.as_str(),
            )
        };
        let locales: Vec<&LocaleDef> = self
            .locales
            .iter()
            .copied()
            .filter(|l| used.is_none_or(|u| !u.keys.contains(&key_for(l))))
            .collect();
        let locale = *pick(&mut rng, &locales, |l| u64::from(l.weight)).ok_or_else(exhausted)?;
        let key = key_for(locale);
        let timezones: Vec<_> = locale
            .timezones
            .iter()
            .filter(|(t, _)| constraints.timezone.as_ref().is_none_or(|want| t == want))
            .collect();
        let timezone = required(pick(&mut rng, &timezones, |t| u64::from(t.1)), "timezones")?
            .0
            .clone();

        let kind = device.kind;
        let engine = browser.engine;
        let ctx = UaContext {
            version,
            kind,
            os_version,
            hw_model: device.hw_model.as_deref().unwrap_or_default(),
            locale: &locale.locale,
            screen: (screen.width, screen.height, screen.dpr),
        };
        let user_agent = ctx.render(&browser.ua[&device.os]);
        let languages = languages_for(engine, &locale.languages);

        let insets = device.insets.or(os_version.insets).unwrap_or(os.insets);
        let avail_height = screen.height.saturating_sub(insets.top + insets.bottom);
        let toolbar = if kind.is_handheld() {
            browser.toolbar.mobile
        } else {
            browser.toolbar.desktop
        };
        let color_depth = match engine {
            Engine::Chromium => device.chromium_color_depth.unwrap_or(24),
            _ => 24,
        };
        let (masked_vendor, masked_renderer) = masked_webgl(engine);
        let gpu = device.webgl.get(&engine);

        let mut fonts: Vec<String> = os
            .fonts
            .iter()
            .chain(&os_version.extra_fonts)
            .cloned()
            .collect();
        fonts.sort_unstable();
        fonts.dedup();

        let mut profile = Profile {
            device: DeviceInfo {
                id: device.id.clone(),
                kind,
                model: device.model.clone(),
            },
            os: OsInfo {
                id: device.os.clone(),
                name: os.name.clone(),
                version: os_version.name.clone(),
            },
            browser: BrowserInfo {
                id: browser_id.to_owned(),
                name: browser.name.clone(),
                engine,
                version: version.full.clone(),
                major: version.major,
            },
            navigator: Navigator {
                app_version: app_version(engine, os, &ctx, &user_agent),
                user_agent,
                user_agent_data: user_agent_data(browser, version, os, os_version, device),
                platform: platform(engine, os).to_owned(),
                vendor: vendor(engine).to_owned(),
                language: languages[0].clone(),
                languages,
                hardware_concurrency: browser.max_cores.map_or(cores, |max| cores.min(max)),
                device_memory: (engine == Engine::Chromium).then(|| {
                    device_memory(memory, version.chromium_major(), device.os == "android")
                }),
                max_touch_points: device.touch_points,
                oscpu: oscpu(engine, os, &ctx),
            },
            screen: Screen {
                width: screen.width,
                height: screen.height,
                avail_width: screen.width,
                avail_height,
                color_depth,
                pixel_depth: color_depth,
                device_pixel_ratio: screen.dpr,
            },
            viewport: Viewport {
                width: screen.width,
                height: avail_height.saturating_sub(toolbar),
            },
            webgl: WebGl {
                vendor: masked_vendor.to_owned(),
                renderer: masked_renderer.to_owned(),
                unmasked_vendor: gpu.map(|g| g.vendor.clone()),
                unmasked_renderer: gpu.map(|g| g.renderer.clone()),
            },
            canvas: Noise {
                seed: fnv1a(format!("{seed}/canvas").as_bytes()) as u32,
            },
            audio: Noise {
                seed: fnv1a(format!("{seed}/audio").as_bytes()) as u32,
            },
            fonts,
            locale: locale.locale.clone(),
            timezone,
            seed,
            ext: Default::default(),
        };
        crate::extras::derive(
            &mut profile,
            &crate::extras::ExtraContext {
                data,
                device,
                os,
                os_version,
                browser,
                browser_id,
                version,
                locale,
            },
        );
        Ok((profile, key))
    }
}

struct Combo<'a> {
    device: &'a DeviceDef,
    browser: &'a str,
    os_version: &'a OsVersionDef,
    /// Browser versions possible on this OS version within the requested range (never empty).
    versions: Vec<VersionDef>,
    weight: u64,
}

/// Every (device, browser, OS version) the dataset allows under the constraints, weighted.
fn combinations<'a>(data: &'a Dataset, c: &Constraints) -> Vec<Combo<'a>> {
    let mut out = Vec::new();
    for device in &data.devices {
        if c.os.as_ref().is_some_and(|os| &device.os != os)
            || c.kind.is_some_and(|k| device.kind != k)
            || c.device.as_ref().is_some_and(|id| &device.id != id)
            || !device
                .screens
                .iter()
                .any(|s| s.weight > 0 && screen_allowed(s, c))
        {
            continue;
        }
        let os = &data.os[&device.os];
        for (browser_id, share) in &device.browsers {
            let browser = &data.browsers[browser_id];
            if c.browser.as_ref().is_some_and(|b| b != browser_id)
                || c.engine.is_some_and(|e| browser.engine != e)
                || !gpu_matches(device, browser.engine, c.gpu_vendor.as_deref())
            {
                continue;
            }
            for os_version in &os.versions {
                if !device.runs(os_version) || !os_version.supports(browser_id) {
                    continue;
                }
                let versions = browser_versions(browser, os_version, browser_id, c);
                if versions.is_empty() {
                    continue;
                }
                out.push(Combo {
                    device,
                    browser: browser_id,
                    os_version,
                    versions,
                    weight: u64::from(device.weight)
                        .saturating_mul(u64::from(*share))
                        .saturating_mul(u64::from(os_version.weight)),
                });
            }
        }
    }
    out
}

fn screen_allowed(s: &ScreenDef, c: &Constraints) -> bool {
    c.min_width.is_none_or(|v| s.width >= v)
        && c.max_width.is_none_or(|v| s.width <= v)
        && c.min_height.is_none_or(|v| s.height >= v)
        && c.max_height.is_none_or(|v| s.height <= v)
        && c.min_dpr.is_none_or(|v| s.dpr >= v)
        && c.max_dpr.is_none_or(|v| s.dpr <= v)
}

fn gpu_matches(device: &DeviceDef, engine: Engine, wanted: Option<&str>) -> bool {
    let Some(wanted) = wanted else { return true };
    let wanted = wanted.to_ascii_lowercase();
    device.webgl.get(&engine).is_some_and(|g| {
        g.vendor.to_ascii_lowercase().contains(&wanted)
            || g.renderer.to_ascii_lowercase().contains(&wanted)
    })
}

/// Versions this browser can have on this OS version, within the requested range.
pub(crate) fn browser_versions(
    browser: &BrowserDef,
    os_version: &OsVersionDef,
    browser_id: &str,
    c: &Constraints,
) -> Vec<VersionDef> {
    let in_range = |v: &VersionDef| {
        c.min_version.is_none_or(|min| v.major >= min)
            && c.max_version.is_none_or(|max| v.major <= max)
    };
    match os_version.pins.get(browser_id) {
        Some(full) => Some(VersionDef::pinned(full))
            .filter(in_range)
            .into_iter()
            .collect(),
        None => browser
            .versions
            .iter()
            .filter(|v| v.weight > 0 && in_range(v))
            .cloned()
            .collect(),
    }
}

fn locale_allowed(l: &LocaleDef, c: &Constraints) -> bool {
    c.locale.as_ref().is_none_or(|list| {
        list.split(',')
            .any(|want| locale_matches(&l.locale, want.trim()))
    }) && c
        .country
        .as_ref()
        .is_none_or(|country| l.region().eq_ignore_ascii_case(country))
        && c.timezone
            .as_ref()
            .is_none_or(|tz| l.timezones.iter().any(|(t, _)| t == tz))
}

fn check_input(data: &Dataset, c: &Constraints) -> Result<(), Error> {
    let fields = [
        ("os", &c.os),
        ("browser", &c.browser),
        ("device", &c.device),
        ("gpuVendor", &c.gpu_vendor),
        ("locale", &c.locale),
        ("country", &c.country),
        ("timezone", &c.timezone),
    ];
    for (name, value) in fields {
        if value.as_ref().is_some_and(|v| v.len() > MAX_FIELD) {
            return Err(Error::InvalidConstraint(format!(
                "`{name}` is longer than {MAX_FIELD} characters"
            )));
        }
    }
    if c.seed.as_ref().is_some_and(|s| s.len() > MAX_SEED) {
        return Err(Error::InvalidConstraint(format!(
            "`seed` is longer than {MAX_SEED} characters"
        )));
    }
    if let (Some(min), Some(max)) = (c.min_version, c.max_version) {
        if min > max {
            return Err(Error::InvalidConstraint(
                "`minVersion` is greater than `maxVersion`".into(),
            ));
        }
    }
    for (name, value) in [("minDpr", c.min_dpr), ("maxDpr", c.max_dpr)] {
        if value.is_some_and(|d| !(d.is_finite() && d > 0.0 && d <= MAX_DPR)) {
            return Err(Error::InvalidConstraint(format!(
                "`{name}` must be greater than 0 and at most {MAX_DPR}"
            )));
        }
    }
    let ranges = [
        (
            "Width",
            c.min_width.map(f64::from),
            c.max_width.map(f64::from),
        ),
        (
            "Height",
            c.min_height.map(f64::from),
            c.max_height.map(f64::from),
        ),
        ("Dpr", c.min_dpr, c.max_dpr),
    ];
    for (name, min, max) in ranges {
        if let (Some(min), Some(max)) = (min, max) {
            if min > max {
                return Err(Error::InvalidConstraint(format!(
                    "`min{name}` is greater than `max{name}`"
                )));
            }
        }
    }

    fn known<'a>(
        what: &str,
        value: &Option<String>,
        ids: impl Iterator<Item = &'a String> + Clone,
    ) -> Result<(), Error> {
        match value {
            Some(v) if !ids.clone().any(|id| id == v) => {
                let list: Vec<&str> = ids.map(String::as_str).collect();
                Err(Error::InvalidConstraint(format!(
                    "unknown {what} `{v}`; expected one of: {}",
                    list.join(", ")
                )))
            }
            _ => Ok(()),
        }
    }
    known("os", &c.os, data.os.keys())?;
    known("browser", &c.browser, data.browsers.keys())?;
    known("device", &c.device, data.devices.iter().map(|d| &d.id))?;
    Ok(())
}

fn describe(c: &Constraints, part: &str) -> String {
    let json = serde_json::to_string(c).unwrap_or_default();
    format!("no {part} combination in the dataset matches {json}")
}

fn required<T>(value: Option<T>, what: &str) -> Result<T, Error> {
    value.ok_or_else(|| Error::Dataset(format!("empty {what}")))
}

fn locale_matches(locale: &str, wanted: &str) -> bool {
    locale.eq_ignore_ascii_case(wanted)
        || (!wanted.contains('-')
            && locale
                .split('-')
                .next()
                .is_some_and(|lang| lang.eq_ignore_ascii_case(wanted)))
}

/// Weighted choice. Returns `None` when every weight is zero.
fn pick<'a, T>(rng: &mut ChaCha8Rng, items: &'a [T], weight: impl Fn(&T) -> u64) -> Option<&'a T> {
    let total: u64 = items.iter().map(&weight).sum();
    if total == 0 {
        return None;
    }
    // shortcut: modulo bias is total / 2^64, invisible at dataset scale; switch to rejection sampling if weights ever approach 2^32
    let mut roll = rng.next_u64() % total;
    for item in items {
        let w = weight(item);
        if roll < w {
            return Some(item);
        }
        roll -= w;
    }
    None
}

pub(crate) fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Unpredictable seed for callers that did not pass one. Not a security value.
fn random_seed() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        "{:016x}",
        std::collections::hash_map::RandomState::new().hash_one(n)
    )
}

/// Everything a user-agent template can refer to.
pub(crate) struct UaContext<'a> {
    pub version: &'a VersionDef,
    pub kind: Kind,
    pub os_version: &'a OsVersionDef,
    pub hw_model: &'a str,
    pub locale: &'a str,
    /// CSS width, CSS height, device pixel ratio.
    pub screen: (u32, u32, f64),
}

impl UaContext<'_> {
    pub fn render(&self, template: &str) -> String {
        let v = self.version;
        let (w, h, dpr) = self.screen;
        let physical = |css: u32| (f64::from(css) * dpr).round() as u64;
        let lang = self.locale.split('-').next().unwrap_or_default();
        template
            .replace("{major}", &v.major.to_string())
            .replace("{full}", &v.full)
            .replace("{build}", v.build.as_deref().unwrap_or_default())
            .replace("{chromiumFull}", v.chromium_full())
            .replace("{chromium}", &v.chromium_major().to_string())
            .replace(
                "{mobile}",
                if self.kind == Kind::Phone {
                    "Mobile "
                } else {
                    ""
                },
            )
            .replace(
                "{formFx}",
                if self.kind == Kind::Tablet {
                    "Tablet"
                } else {
                    "Mobile"
                },
            )
            .replace(
                "{osUa}",
                self.os_version.ua_token.as_deref().unwrap_or_default(),
            )
            .replace("{osVersion_}", &self.os_version.name.replace('.', "_"))
            .replace("{osVersion}", &self.os_version.name)
            // Three components (`18_5_0`), as Chrome for iOS, Edge for iOS and the Google app send it.
            .replace(
                "{osVersionFull_}",
                &self.os_version.platform_version.replace('.', "_"),
            )
            .replace(
                "{osMajor}",
                self.os_version.name.split('.').next().unwrap_or_default(),
            )
            // `15E148` is WebKit's fixed `Mobile/` token, the value used when the OS build is unknown.
            .replace(
                "{osBuild}",
                self.os_version.build.as_deref().unwrap_or("15E148"),
            )
            .replace("{revision}", v.revision.as_deref().unwrap_or_default())
            .replace("{hwModel}", self.hw_model)
            .replace("{locale_}", &self.locale.replace('-', "_"))
            .replace("{lang}", lang)
            .replace("{screenPx}", &format!("{}x{}", physical(w), physical(h)))
            .replace("{scaleInt}", &format!("{}", dpr.round() as u32))
            .replace("{scale}", &format!("{dpr:.2}"))
    }
}

pub(crate) fn languages_for(engine: Engine, languages: &[String]) -> Vec<String> {
    match engine {
        // Safari and WKWebView expose only the primary language.
        Engine::Webkit => languages.iter().take(1).cloned().collect(),
        _ => languages.to_vec(),
    }
}

pub(crate) fn platform(engine: Engine, os: &OsDef) -> &str {
    match (engine, &os.gecko_platform) {
        (Engine::Gecko, Some(p)) => p,
        _ => &os.platform,
    }
}

pub(crate) fn oscpu(engine: Engine, os: &OsDef, ctx: &UaContext) -> Option<String> {
    (engine == Engine::Gecko)
        .then(|| os.gecko_oscpu.as_deref().map(|t| ctx.render(t)))
        .flatten()
}

pub(crate) fn app_version(engine: Engine, os: &OsDef, ctx: &UaContext, user_agent: &str) -> String {
    match (engine, &os.gecko_app_version) {
        (Engine::Gecko, Some(template)) => ctx.render(template),
        _ => user_agent
            .strip_prefix("Mozilla/")
            .unwrap_or(user_agent)
            .to_owned(),
    }
}

pub(crate) fn vendor(engine: Engine) -> &'static str {
    match engine {
        Engine::Chromium => "Google Inc.",
        Engine::Webkit => "Apple Computer, Inc.",
        Engine::Gecko => "",
    }
}

pub(crate) fn masked_webgl(engine: Engine) -> (&'static str, &'static str) {
    match engine {
        Engine::Gecko => ("Mozilla", "Mozilla"),
        _ => ("WebKit", "WebKit WebGL"),
    }
}

/// Chromium reports RAM rounded down to a power of two. Before 147 the range was capped at 8 GB;
/// from 147 it is 2-32 GB on desktop and 2-8 GB on Android (chromestatus 6330376953921536).
pub(crate) fn device_memory(gb: u32, chromium_major: u32, android: bool) -> f64 {
    let floor = if gb == 0 {
        0
    } else {
        1u32 << (31 - gb.leading_zeros())
    };
    let (min, max) = match (chromium_major >= 147, android) {
        (true, false) => (2, 32),
        (true, true) => (2, 8),
        (false, _) => (1, 8),
    };
    f64::from(floor.clamp(min, max))
}

pub(crate) fn user_agent_data(
    browser: &BrowserDef,
    v: &VersionDef,
    os: &OsDef,
    os_version: &OsVersionDef,
    device: &DeviceDef,
) -> Option<UserAgentData> {
    if !browser.client_hints {
        return None;
    }
    let brand = browser.ch_brand.as_deref()?;
    let chromium = v.chromium_major();
    Some(UserAgentData {
        brands: brand_list(
            chromium,
            brand,
            &v.major.to_string(),
            &chromium.to_string(),
            false,
        ),
        full_version_list: brand_list(chromium, brand, &v.full, v.chromium_full(), true),
        mobile: device.kind == Kind::Phone,
        platform: os.ch_platform.clone().unwrap_or_default(),
        platform_version: os_version.platform_version.clone(),
        architecture: device
            .architecture
            .clone()
            .unwrap_or_else(|| os.architecture.clone()),
        bitness: os.bitness.clone(),
        model: device.model.clone().unwrap_or_default(),
        ua_full_version: v.full.clone(),
        form_factors: vec![form_factor(device.kind).to_owned()],
        wow64: false,
    })
}

fn form_factor(kind: Kind) -> &'static str {
    match kind {
        Kind::Phone => "Mobile",
        Kind::Tablet => "Tablet",
        Kind::Vr => "XR",
        _ => "Desktop",
    }
}

/// Chromium's GREASE brand list (components/embedder_support/user_agent_utils.cc), seeded by the Chromium major.
pub(crate) fn brand_list(
    seed: u32,
    brand: &str,
    brand_version: &str,
    chromium_version: &str,
    full: bool,
) -> Vec<Brand> {
    const CHARS: [&str; 11] = [" ", "(", ":", "-", ".", "/", ")", ";", "=", "?", "_"];
    const VERSIONS: [&str; 3] = ["8", "99", "24"];
    const ORDERS: [[usize; 3]; 6] = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let s = seed as usize;
    let grease_version = if full {
        format!("{}.0.0.0", VERSIONS[s % 3])
    } else {
        VERSIONS[s % 3].to_owned()
    };
    let grease = Brand {
        brand: format!("Not{}A{}Brand", CHARS[s % 11], CHARS[(s + 1) % 11]),
        version: grease_version,
    };
    let chromium = Brand {
        brand: "Chromium".into(),
        version: chromium_version.into(),
    };
    let named = Brand {
        brand: brand.into(),
        version: brand_version.into(),
    };
    let order = ORDERS[s % 6];
    let mut out = vec![Brand::default(); 3];
    out[order[0]] = grease;
    out[order[1]] = chromium;
    out[order[2]] = named;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[Brand]) -> Vec<(&str, &str)> {
        list.iter()
            .map(|b| (b.brand.as_str(), b.version.as_str()))
            .collect()
    }

    #[test]
    fn grease_matches_real_chrome_releases() {
        // Observed Sec-CH-UA headers from Chrome 120, 131 and 132.
        assert_eq!(
            names(&brand_list(120, "Google Chrome", "120", "120", false)),
            [
                ("Not_A Brand", "8"),
                ("Chromium", "120"),
                ("Google Chrome", "120")
            ]
        );
        assert_eq!(
            names(&brand_list(131, "Google Chrome", "131", "131", false)),
            [
                ("Google Chrome", "131"),
                ("Chromium", "131"),
                ("Not_A Brand", "24")
            ]
        );
        assert_eq!(
            names(&brand_list(132, "Google Chrome", "132", "132", false)),
            [
                ("Not A(Brand", "8"),
                ("Chromium", "132"),
                ("Google Chrome", "132")
            ]
        );
    }

    #[test]
    fn device_memory_rounds_down_and_caps() {
        assert_eq!(device_memory(6, 136, false), 4.0);
        assert_eq!(device_memory(36, 136, false), 8.0);
        assert_eq!(device_memory(16, 158, false), 16.0);
        assert_eq!(device_memory(36, 158, false), 32.0);
        assert_eq!(device_memory(12, 158, true), 8.0);
        assert_eq!(device_memory(1, 158, true), 2.0);
    }

    #[test]
    fn locale_matching() {
        assert!(locale_matches("tr-TR", "tr"));
        assert!(locale_matches("tr-TR", "TR-tr"));
        assert!(!locale_matches("en-GB", "en-US"));
    }
}
