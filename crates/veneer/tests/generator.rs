//! Screen constraints, unique batches, profile ids and sampling determinism.

use std::collections::HashSet;

use veneer::{
    catalog, generate, generate_batch, generate_from_id, profile_id, validate, Constraints, Error,
    Kind, Profile, MAX_BATCH,
};

fn clean(p: &Profile) {
    let report = validate(p);
    assert!(
        report.issues.is_empty(),
        "{:#?}\n{}",
        report.issues,
        serde_json::to_string_pretty(p).unwrap()
    );
}

fn key(p: &Profile) -> (String, String, String, u32, u32, u64, String) {
    (
        p.device.id.clone(),
        p.browser.id.clone(),
        p.browser.version.clone(),
        p.screen.width,
        p.screen.height,
        p.screen.device_pixel_ratio.to_bits(),
        p.locale.clone(),
    )
}

#[test]
fn screen_and_dpr_ranges_are_honoured() {
    let cases = [
        Constraints {
            min_width: Some(1900),
            ..Default::default()
        },
        Constraints {
            max_width: Some(400),
            min_dpr: Some(3.0),
            ..Default::default()
        },
        Constraints {
            kind: Some(Kind::Desktop),
            min_height: Some(800),
            max_height: Some(1100),
            max_dpr: Some(1.0),
            ..Default::default()
        },
        Constraints {
            min_dpr: Some(2.0),
            max_dpr: Some(2.0),
            ..Default::default()
        },
    ];
    for c in cases {
        for i in 0..150 {
            let c = Constraints {
                seed: Some(format!("screen-{i}")),
                ..c.clone()
            };
            let p = generate(&c).unwrap();
            clean(&p);
            let s = &p.screen;
            assert!(c.min_width.is_none_or(|v| s.width >= v), "{c:?} {s:?}");
            assert!(c.max_width.is_none_or(|v| s.width <= v), "{c:?} {s:?}");
            assert!(c.min_height.is_none_or(|v| s.height >= v), "{c:?} {s:?}");
            assert!(c.max_height.is_none_or(|v| s.height <= v), "{c:?} {s:?}");
            assert!(
                c.min_dpr.is_none_or(|v| s.device_pixel_ratio >= v),
                "{c:?} {s:?}"
            );
            assert!(
                c.max_dpr.is_none_or(|v| s.device_pixel_ratio <= v),
                "{c:?} {s:?}"
            );
        }
    }
}

#[test]
fn bad_screen_ranges_are_rejected() {
    let invalid = [
        Constraints {
            min_width: Some(500),
            max_width: Some(400),
            ..Default::default()
        },
        Constraints {
            min_dpr: Some(3.0),
            max_dpr: Some(2.0),
            ..Default::default()
        },
        Constraints {
            min_dpr: Some(f64::NAN),
            ..Default::default()
        },
        Constraints {
            max_dpr: Some(0.0),
            ..Default::default()
        },
        Constraints {
            max_dpr: Some(11.0),
            ..Default::default()
        },
    ];
    for c in invalid {
        assert!(
            matches!(generate(&c), Err(Error::InvalidConstraint(_))),
            "{c:?}"
        );
    }
    let impossible = Constraints {
        min_width: Some(100_000),
        ..Default::default()
    };
    assert!(matches!(
        generate(&impossible),
        Err(Error::Unsatisfiable(_))
    ));
}

#[test]
fn batches_are_reproducible_and_ids_regenerate_each_profile() {
    for unique in [false, true] {
        let c = Constraints {
            os: Some("android".into()),
            seed: Some(format!("batch-{unique}")),
            ..Default::default()
        };
        let a = generate_batch(&c, 40, unique).unwrap();
        let b = generate_batch(&c, 40, unique).unwrap();
        assert_eq!(a, b);
        for (i, g) in a.iter().enumerate() {
            clean(&g.profile);
            assert_eq!(g.profile.seed, format!("batch-{unique}-{i}"));
            assert_eq!(generate_from_id(&g.id).unwrap(), g.profile, "{unique} {i}");
        }
    }
    // A single profile's id, from its seed.
    let p = generate(&Constraints {
        browser: Some("firefox".into()),
        min_width: Some(1280),
        ..Default::default()
    })
    .unwrap();
    let id = profile_id(&Constraints {
        browser: Some("firefox".into()),
        min_width: Some(1280),
        seed: Some(p.seed.clone()),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(generate_from_id(&id).unwrap(), p);
    assert!(profile_id(&Constraints::default()).is_err());
    assert!(generate_from_id("not-an-id").is_err());
}

#[test]
fn unique_batches_never_repeat_a_configuration() {
    let c = Constraints {
        os: Some("windows".into()),
        seed: Some("uniq".into()),
        ..Default::default()
    };
    let batch = generate_batch(&c, 500, true).unwrap();
    let keys: HashSet<_> = batch.iter().map(|g| key(&g.profile)).collect();
    assert_eq!(keys.len(), batch.len());
    for g in &batch {
        clean(&g.profile);
    }
}

#[test]
fn unique_batches_can_exhaust_a_small_space_exactly() {
    // One device, one browser, one locale: the space is versions x screens.
    let c = Constraints {
        device: Some("iphone-se-3".into()),
        browser: Some("safari".into()),
        locale: Some("tr-TR".into()),
        seed: Some("small".into()),
        ..Default::default()
    };
    let err = generate_batch(&c, MAX_BATCH, true).unwrap_err();
    let Error::Unsatisfiable(message) = err else {
        panic!("expected Unsatisfiable, got {err:?}")
    };
    let space: u32 = message
        .split("only ")
        .nth(1)
        .and_then(|rest| rest.split(' ').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("no count in: {message}"));
    assert!(space >= 2, "{message}");
    let batch = generate_batch(&c, space, true).unwrap();
    let keys: HashSet<_> = batch.iter().map(|g| key(&g.profile)).collect();
    assert_eq!(keys.len(), space as usize);
    let last = batch.last().unwrap();
    assert_eq!(generate_from_id(&last.id).unwrap(), last.profile);
    assert!(generate_batch(&c, space + 1, true).is_err());
}

#[test]
fn batch_count_is_bounded() {
    assert!(generate_batch(&Constraints::default(), 0, false).is_err());
    assert!(generate_batch(&Constraints::default(), MAX_BATCH + 1, false).is_err());
}

/// Sampling must keep following the weights after the alias-table change.
#[test]
fn weighted_sampling_tracks_weights() {
    let mut windows = 0;
    let n = 4000;
    for i in 0..n {
        let p = generate(&Constraints {
            seed: Some(format!("dist-{i}")),
            kind: Some(Kind::Desktop),
            ..Default::default()
        })
        .unwrap();
        if p.os.id == "windows" {
            windows += 1;
        }
    }
    // Windows dominates desktop weights in the dataset; a broken sampler would drift far from that.
    assert!(windows * 2 > n, "windows {windows} of {n}");
}

/// Split builds (`--no-default-features --features data-mobile`, ...) embed only the selected kinds.
#[test]
fn embedded_device_kinds_follow_features() {
    let devices = catalog().unwrap().devices;
    assert!(!devices.is_empty());
    for d in &devices {
        let allowed = match d.kind {
            Kind::Desktop | Kind::Laptop => cfg!(feature = "data-desktop"),
            Kind::Phone | Kind::Tablet => cfg!(feature = "data-mobile"),
            _ => cfg!(feature = "data-other"),
        };
        assert!(allowed, "{} ({:?}) should not be embedded", d.id, d.kind);
    }
    let p = generate(&Constraints {
        seed: Some("split".into()),
        ..Default::default()
    })
    .unwrap();
    clean(&p);
}
