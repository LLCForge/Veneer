//! Hardware plausibility (cores and memory against the device model or its GPU) and timezone
//! against the locale's region. Every bound comes from the device and locale data, never from a
//! hard-coded guess.

use serde_json::json;

use crate::data::{BrowserDef, Dataset, DeviceDef};
use crate::generate::device_memory;
use crate::model::{Engine, Profile};
use crate::validate::{Collector, Severity};

/// Cores and memory must be values the device model (or, for unknown devices, any known device
/// with the same GPU on the same OS) ships with.
pub(crate) fn check(
    data: &Dataset,
    p: &Profile,
    device: Option<&DeviceDef>,
    browser: &BrowserDef,
    c: &mut Collector,
) {
    let engine = p.browser.engine;
    let (sources, cores_rule, memory_rule, basis): (Vec<&DeviceDef>, _, _, String) = match device {
        Some(d) => (
            vec![d],
            "hardware.cores_device",
            "hardware.memory_device",
            format!("`{}`", d.id),
        ),
        None => {
            let Some(renderer) = &p.webgl.unmasked_renderer else {
                return;
            };
            let same_gpu: Vec<&DeviceDef> = data
                .devices
                .iter()
                .filter(|d| {
                    d.os == p.os.id
                        && d.webgl
                            .get(&engine)
                            .is_some_and(|g| &g.renderer == renderer)
                })
                .collect();
            (
                same_gpu,
                "hardware.cores_gpu",
                "hardware.memory_gpu",
                format!("devices with `{renderer}`"),
            )
        }
    };
    if sources.is_empty() {
        return;
    }

    let cap = |cores: u32| browser.max_cores.map_or(cores, |max| cores.min(max));
    let mut cores: Vec<u32> = sources
        .iter()
        .flat_map(|d| d.cores.iter().map(|(n, _)| cap(*n)))
        .collect();
    cores.sort_unstable();
    cores.dedup();
    let have = p.navigator.hardware_concurrency;
    c.fix_if(
        Severity::Warning,
        !cores.contains(&have),
        cores_rule,
        || json!(nearest(&cores, have)),
        || format!("{basis} report {cores:?} cores, not {have}"),
    );

    if let (Engine::Chromium, Some(m)) = (engine, p.navigator.device_memory) {
        let mut memory: Vec<f64> = sources
            .iter()
            .flat_map(|d| {
                let android = d.os == "android";
                let chromium = crate::validate::chromium_major(p);
                d.memory
                    .iter()
                    .map(move |(gb, _)| device_memory(*gb, chromium, android))
            })
            .collect();
        if crate::validate::device_memory_values(p).contains(&32.0) {
            // Chromium 147+ outside Android: power of two rounded down, from 2 to 32.
            memory.extend(sources.iter().flat_map(|d| {
                d.memory
                    .iter()
                    .map(|(gb, _)| f64::from(gb.max(&1).ilog2()).exp2().clamp(2.0, 32.0))
            }));
        }
        memory.sort_by(f64::total_cmp);
        memory.dedup();
        c.fix_if(
            Severity::Warning,
            !memory.contains(&m),
            memory_rule,
            || {
                json!(memory
                    .iter()
                    .copied()
                    .min_by(|a, b| (a - m).abs().total_cmp(&(b - m).abs())))
            },
            || format!("{basis} report deviceMemory {memory:?}, not {m}"),
        );
    }
}

fn nearest(values: &[u32], to: u32) -> Option<u32> {
    values.iter().copied().min_by_key(|v| v.abs_diff(to))
}

/// For locales outside the dataset: the timezone must be used by some dataset locale of the same region.
pub(crate) fn timezone_region(data: &Dataset, p: &Profile, c: &mut Collector) {
    let Some(region) = region_of(&p.locale) else {
        return;
    };
    let zones: Vec<&str> = data
        .locales
        .iter()
        .filter(|l| l.region().eq_ignore_ascii_case(region))
        .flat_map(|l| l.timezones.iter().map(|(t, _)| t.as_str()))
        .collect();
    if zones.is_empty() {
        return;
    }
    c.warn_if(
        !zones.contains(&p.timezone.as_str()),
        "locale.timezone_region",
        || format!("`{}` is not a timezone of region {region}", p.timezone),
    );
}

/// Two-letter region subtag of a BCP 47 tag, e.g. `TR` in `tr-TR`.
pub(crate) fn region_of(locale: &str) -> Option<&str> {
    locale
        .split('-')
        .skip(1)
        .find(|s| s.len() == 2 && s.bytes().all(|b| b.is_ascii_alphabetic()))
}

#[cfg(test)]
mod tests {
    use crate::{generate, validate, Constraints};

    #[test]
    fn cores_and_memory_must_fit_the_device() {
        let mut p = generate(&Constraints {
            browser: Some("chrome".into()),
            os: Some("android".into()),
            seed: Some("hw".into()),
            ..Default::default()
        })
        .unwrap();
        p.navigator.hardware_concurrency = 255;
        p.navigator.device_memory = Some(0.25);
        let rules: Vec<_> = validate(&p).issues.into_iter().map(|i| i.rule).collect();
        assert!(rules.contains(&"hardware.cores_device".into()), "{rules:?}");
        assert!(
            rules.contains(&"hardware.memory_device".into()),
            "{rules:?}"
        );
    }

    #[test]
    fn unknown_device_is_checked_against_its_gpu() {
        let mut p = generate(&Constraints {
            os: Some("macos".into()),
            browser: Some("chrome".into()),
            seed: Some("gpu".into()),
            ..Default::default()
        })
        .unwrap();
        p.device.id = "some-mac".into();
        p.navigator.hardware_concurrency = 255;
        let rules: Vec<_> = validate(&p).issues.into_iter().map(|i| i.rule).collect();
        assert!(rules.contains(&"hardware.cores_gpu".into()), "{rules:?}");
    }

    #[test]
    fn unknown_locale_timezone_must_fit_its_region() {
        let mut p = generate(&Constraints {
            locale: Some("tr".into()),
            seed: Some("tz".into()),
            ..Default::default()
        })
        .unwrap();
        // A locale outside the dataset whose region (TR) is known.
        p.locale = "ku-TR".into();
        p.navigator.language = "ku-TR".into();
        p.navigator.languages = vec!["ku-TR".into()];
        p.timezone = "America/New_York".into();
        let rules: Vec<_> = validate(&p).issues.into_iter().map(|i| i.rule).collect();
        assert!(
            rules.contains(&"locale.timezone_region".into()),
            "{rules:?}"
        );
        assert!(!rules.contains(&"locale.timezone_plausible".into()));
    }
}
