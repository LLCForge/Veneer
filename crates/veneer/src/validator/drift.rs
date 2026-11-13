//! Distribution drift: how far a batch of profiles is from the distribution the dataset implies.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::data::dataset;
use crate::generate::browser_versions;
use crate::model::{Constraints, Profile};
use crate::Error;

/// Batches smaller than this are too noisy to call drifted.
const MIN_COUNT: usize = 50;
/// Total variation distance above which a dimension is drifted.
const THRESHOLD: f64 = 0.1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftReport {
    pub count: usize,
    pub dimensions: Vec<DriftDimension>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftDimension {
    /// `os`, `browser`, `engine`, `kind` or `locale`.
    pub name: String,
    /// Half the sum of absolute share differences: 0 = identical, 1 = disjoint.
    pub total_variation: f64,
    /// Total variation a batch of this size reaches by chance alone (about twice its expected value).
    pub noise_floor: f64,
    /// `true` when there are at least 50 profiles and the total variation exceeds both 0.1 and the noise floor.
    pub drifted: bool,
    pub buckets: Vec<DriftBucket>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftBucket {
    pub value: String,
    /// Share the generator gives this value without constraints (0-1).
    pub expected: f64,
    /// Share in the batch (0-1).
    pub observed: f64,
    pub delta: f64,
}

/// Compare a batch with the unconstrained generator distribution (device weight × browser share × OS version weight; locale weight).
pub fn drift(profiles: &[Profile]) -> Result<DriftReport, Error> {
    let data = dataset()?;
    let mut expected: [BTreeMap<String, f64>; 5] = Default::default();
    let none = Constraints::default();
    for device in &data.devices {
        let os = &data.os[&device.os];
        for (browser_id, share) in &device.browsers {
            let browser = &data.browsers[browser_id];
            for v in &os.versions {
                if !device.runs(v)
                    || !v.supports(browser_id)
                    || browser_versions(browser, v, browser_id, &none).is_empty()
                {
                    continue;
                }
                let w = f64::from(device.weight) * f64::from(*share) * f64::from(v.weight);
                for (map, key) in expected.iter_mut().zip([
                    device.os.clone(),
                    browser_id.clone(),
                    enum_name(&browser.engine),
                    enum_name(&device.kind),
                ]) {
                    *map.entry(key).or_default() += w;
                }
            }
        }
    }
    for l in &data.locales {
        *expected[4].entry(l.locale.clone()).or_default() += f64::from(l.weight);
    }

    let mut observed: [BTreeMap<String, f64>; 5] = Default::default();
    for p in profiles {
        for (map, key) in observed.iter_mut().zip([
            p.os.id.clone(),
            p.browser.id.clone(),
            enum_name(&p.browser.engine),
            enum_name(&p.device.kind),
            p.locale.clone(),
        ]) {
            *map.entry(key).or_default() += 1.0;
        }
    }

    let dimensions = ["os", "browser", "engine", "kind", "locale"]
        .iter()
        .zip(expected.iter().zip(&observed))
        .map(|(name, (e, o))| dimension(name, e, o, profiles.len()))
        .collect();
    Ok(DriftReport {
        count: profiles.len(),
        dimensions,
    })
}

fn enum_name<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn dimension(
    name: &str,
    expected: &BTreeMap<String, f64>,
    observed: &BTreeMap<String, f64>,
    count: usize,
) -> DriftDimension {
    let share = |m: &BTreeMap<String, f64>, k: &str| {
        let total: f64 = m.values().sum();
        if total > 0.0 {
            m.get(k).copied().unwrap_or(0.0) / total
        } else {
            0.0
        }
    };
    let mut keys: Vec<&String> = expected.keys().chain(observed.keys()).collect();
    keys.sort_unstable();
    keys.dedup();
    let mut buckets: Vec<DriftBucket> = keys
        .into_iter()
        .map(|k| {
            let (e, o) = (share(expected, k), share(observed, k));
            DriftBucket {
                value: k.clone(),
                expected: round(e),
                observed: round(o),
                delta: round(o - e),
            }
        })
        .collect();
    let total_variation = round(
        expected
            .keys()
            .chain(observed.keys())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .map(|k| (share(observed, k) - share(expected, k)).abs())
            .sum::<f64>()
            / 2.0,
    );
    // Expected TVD of a multinomial sample is about 0.4 * sum(sqrt(p(1-p)/n)); allow twice that.
    let noise_floor = if count == 0 {
        1.0
    } else {
        let total: f64 = expected.values().sum();
        round(
            expected
                .values()
                .map(|w| {
                    let p = if total > 0.0 { w / total } else { 0.0 };
                    (p * (1.0 - p) / count as f64).sqrt()
                })
                .sum::<f64>()
                * 0.8,
        )
    };
    buckets.sort_by(|a, b| {
        b.expected
            .total_cmp(&a.expected)
            .then(a.value.cmp(&b.value))
    });
    DriftDimension {
        name: name.to_owned(),
        total_variation,
        noise_floor,
        drifted: count >= MIN_COUNT && total_variation > THRESHOLD.max(noise_floor),
        buckets,
    }
}

fn round(x: f64) -> f64 {
    (x * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate;

    #[test]
    fn generated_batch_does_not_drift_but_skewed_batch_does() {
        let batch: Vec<Profile> = (0..1500)
            .map(|i| {
                generate(&Constraints {
                    seed: Some(format!("drift-{i}")),
                    ..Default::default()
                })
                .unwrap()
            })
            .collect();
        let r = drift(&batch).unwrap();
        assert_eq!(r.count, 1500);
        for d in &r.dimensions {
            assert!(!d.drifted, "{} drifted: {}", d.name, d.total_variation);
        }
        let skewed: Vec<Profile> = (0..100)
            .map(|i| {
                generate(&Constraints {
                    os: Some("linux".into()),
                    seed: Some(format!("skew-{i}")),
                    ..Default::default()
                })
                .unwrap()
            })
            .collect();
        let r = drift(&skewed).unwrap();
        assert!(
            r.dimensions
                .iter()
                .find(|d| d.name == "os")
                .unwrap()
                .drifted
        );
    }
}
