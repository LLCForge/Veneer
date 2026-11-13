//! `veneer bench`: generator throughput and dataset load time, without a benchmarking framework.

use std::time::Instant;

use serde::Serialize;
use veneer::{Constraints, MAX_BATCH};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Measurement {
    profiles: u32,
    seconds: f64,
    profiles_per_second: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BenchReport {
    /// First dataset use in this process: parse, integrity check and catalog.
    first_catalog_ms: f64,
    /// A later catalog call with the dataset already parsed.
    warm_catalog_ms: f64,
    /// `generate` called once per profile (prepares the sampler every time).
    single: Measurement,
    /// `generate_batch`, which prepares the sampler once per batch.
    batch: Measurement,
    /// One `generate_batch` with `unique` (up to 1000 profiles); absent when the constraints allow fewer.
    #[serde(skip_serializing_if = "Option::is_none")]
    unique_batch: Option<Measurement>,
    /// `validate` on the generated profiles.
    validate: Measurement,
}

fn measure(profiles: u32, start: Instant) -> Measurement {
    let seconds = start.elapsed().as_secs_f64();
    Measurement {
        profiles,
        seconds,
        profiles_per_second: if seconds > 0.0 {
            f64::from(profiles) / seconds
        } else {
            0.0
        },
    }
}

/// Batches of up to `MAX_BATCH`, each with its own base seed so no work is repeated.
fn batches(c: &Constraints, count: u32) -> Result<Vec<veneer::Profile>, String> {
    let mut out = Vec::with_capacity(count as usize);
    let mut round = 0u32;
    while out.len() < count as usize {
        let size = (count - out.len() as u32).min(MAX_BATCH);
        let c = Constraints {
            seed: Some(format!("bench-{round}")),
            ..c.clone()
        };
        let batch = veneer::generate_batch(&c, size, false).map_err(|e| e.to_string())?;
        out.extend(batch.into_iter().map(|g| g.profile));
        round += 1;
    }
    Ok(out)
}

pub(crate) fn run(constraints: &Constraints, count: u32) -> Result<BenchReport, String> {
    let start = Instant::now();
    veneer::catalog().map_err(|e| e.to_string())?;
    let first_catalog_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    veneer::catalog().map_err(|e| e.to_string())?;
    let warm_catalog_ms = start.elapsed().as_secs_f64() * 1000.0;

    let start = Instant::now();
    for i in 0..count {
        let c = Constraints {
            seed: Some(format!("bench-single-{i}")),
            ..constraints.clone()
        };
        veneer::generate(&c).map_err(|e| e.to_string())?;
    }
    let single = measure(count, start);

    let start = Instant::now();
    let profiles = batches(constraints, count)?;
    let batch = measure(count, start);

    let size = count.min(1000);
    let start = Instant::now();
    let unique_batch = match veneer::generate_batch(constraints, size, true) {
        Ok(_) => Some(measure(size, start)),
        Err(veneer::Error::Unsatisfiable(_)) => None,
        Err(e) => return Err(e.to_string()),
    };

    let start = Instant::now();
    let invalid = profiles
        .iter()
        .filter(|p| !veneer::validate(p).valid)
        .count();
    let validate = measure(count, start);
    if invalid > 0 {
        return Err(format!("{invalid} generated profiles failed validation"));
    }

    Ok(BenchReport {
        first_catalog_ms,
        warm_catalog_ms,
        single,
        batch,
        unique_batch,
        validate,
    })
}
