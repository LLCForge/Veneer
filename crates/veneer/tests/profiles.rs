use serde_json::json;
use veneer::{
    catalog, compare_observed, generate, navigation_headers, to_init_script, validate, Constraints,
    Engine, Error, Kind,
};

fn with_seed(seed: &str) -> Constraints {
    Constraints {
        seed: Some(seed.into()),
        ..Default::default()
    }
}

fn assert_clean(c: &Constraints) -> veneer::Profile {
    let profile = generate(c).unwrap();
    let report = validate(&profile);
    assert!(
        report.issues.is_empty() && report.score == 100,
        "{c:?} -> {:#?}\n{}",
        report.issues,
        serde_json::to_string_pretty(&profile).unwrap()
    );
    profile
}

/// Core invariant: nothing the generator emits may trip any rule, not even a warning.
#[test]
fn generated_profiles_pass_every_rule() {
    let cat = catalog().unwrap();
    for i in 0..2000 {
        assert_clean(&with_seed(&format!("any-{i}")));
    }
    for os in &cat.os {
        for i in 0..100 {
            assert_clean(&Constraints {
                os: Some(os.id.clone()),
                ..with_seed(&format!("{}-{i}", os.id))
            });
        }
    }
    for browser in &cat.browsers {
        for i in 0..60 {
            assert_clean(&Constraints {
                browser: Some(browser.id.clone()),
                ..with_seed(&format!("{}-{i}", browser.id))
            });
        }
    }
    for device in &cat.devices {
        for i in 0..20 {
            assert_clean(&Constraints {
                device: Some(device.id.clone()),
                ..with_seed(&format!("{}-{i}", device.id))
            });
        }
    }
}

#[test]
fn same_seed_same_profile() {
    let a = generate(&with_seed("session-42")).unwrap();
    let b = generate(&with_seed("session-42")).unwrap();
    let c = generate(&with_seed("session-43")).unwrap();
    assert_eq!(a, b);
    assert_ne!(a.canvas, c.canvas);
}

#[test]
fn unseeded_profiles_differ_and_report_their_seed() {
    let a = generate(&Constraints::default()).unwrap();
    let b = generate(&Constraints::default()).unwrap();
    assert_ne!(a.seed, b.seed);
    assert_eq!(generate(&with_seed(&a.seed)).unwrap(), a);
}

#[test]
fn constraints_are_honoured() {
    let p = generate(&Constraints {
        os: Some("macos".into()),
        browser: Some("safari".into()),
        locale: Some("tr".into()),
        ..with_seed("x")
    })
    .unwrap();
    assert_eq!(p.os.id, "macos");
    assert_eq!(p.browser.engine, Engine::Webkit);
    assert_eq!(p.locale, "tr-TR");
    assert_eq!(p.timezone, "Europe/Istanbul");
    assert_eq!(p.navigator.languages, ["tr-TR"]);
    assert!(p.navigator.user_agent_data.is_none());
    assert_eq!(p.webgl.unmasked_renderer.as_deref(), Some("Apple GPU"));

    let phone = generate(&Constraints {
        kind: Some(Kind::Phone),
        browser: Some("chrome".into()),
        ..with_seed("y")
    })
    .unwrap();
    assert!(phone.navigator.user_agent.contains("Mobile Safari/537.36"));
    assert!(phone.navigator.user_agent_data.unwrap().mobile);

    let ps5 = generate(&Constraints {
        device: Some("playstation-5".into()),
        ..with_seed("z")
    })
    .unwrap();
    assert!(ps5
        .navigator
        .user_agent
        .starts_with("Mozilla/5.0 (PlayStation; PlayStation 5/"));

    for i in 0..50 {
        let p = generate(&Constraints {
            engine: Some(Engine::Gecko),
            gpu_vendor: Some("nvidia".into()),
            country: Some("DE".into()),
            ..with_seed(&format!("combo-{i}"))
        })
        .unwrap();
        assert_eq!(p.browser.engine, Engine::Gecko);
        assert!(p.webgl.unmasked_vendor.unwrap().contains("NVIDIA"));
        assert_eq!(p.locale, "de-DE");
    }

    let p = generate(&Constraints {
        browser: Some("chrome".into()),
        min_version: Some(157),
        max_version: Some(157),
        locale: Some("ja,ko".into()),
        ..with_seed("range")
    })
    .unwrap();
    assert_eq!(p.browser.major, 157);
    assert!(p.locale == "ja-JP" || p.locale == "ko-KR");
}

#[test]
fn old_iphones_get_the_safari_their_ios_ships() {
    let p = assert_clean(&Constraints {
        device: Some("iphone-8".into()),
        ..with_seed("old")
    });
    assert_eq!(p.os.version, "16.7");
    assert_eq!(p.browser.version, "16.6");
    assert!(p
        .navigator
        .user_agent
        .contains("CPU iPhone OS 16_7 like Mac OS X"));
    assert!(p.navigator.user_agent.contains("Version/16.6 "));

    let new = assert_clean(&Constraints {
        device: Some("iphone-18-pro-max".into()),
        browser: Some("safari".into()),
        ..with_seed("new")
    });
    assert!(new.os.version.starts_with("27."), "{}", new.os.version);
    assert!(new
        .navigator
        .user_agent
        .contains("CPU iPhone OS 18_7 like Mac OS X"));
    assert_eq!((new.screen.width, new.screen.height), (440, 956));
}

#[test]
fn in_app_browsers_carry_device_details() {
    let p = assert_clean(&Constraints {
        device: Some("iphone-16-pro".into()),
        browser: Some("instagram".into()),
        locale: Some("tr-TR".into()),
        ..with_seed("insta")
    });
    let ua = &p.navigator.user_agent;
    assert!(ua.contains("Instagram "), "{ua}");
    assert!(ua.contains("(iPhone17,1; iOS "), "{ua}");
    assert!(ua.contains("; tr_TR; tr; scale=3.00; 1206x2622; "), "{ua}");
    assert!(!ua.contains('{'), "unrendered placeholder in {ua}");

    let fb = assert_clean(&Constraints {
        device: Some("iphone-15".into()),
        browser: Some("facebook".into()),
        ..with_seed("fb")
    });
    assert!(fb.navigator.user_agent.contains("FBDV/iPhone15,4;"));

    // Real reference: "...CPU iPhone OS 18_5_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko)
    // GSA/370.0.762543316 Mobile/15E148 Safari/604.1"
    for i in 0..20 {
        let gsa = assert_clean(&Constraints {
            device: Some("iphone-16".into()),
            browser: Some("gsa".into()),
            ..with_seed(&format!("gsa-{i}"))
        });
        let ua = gsa.navigator.user_agent;
        let os = ua
            .strip_prefix("Mozilla/5.0 (iPhone; CPU iPhone OS ")
            .and_then(|rest| {
                rest.split_once(" like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) GSA/")
            })
            .unwrap_or_else(|| panic!("unexpected GSA prefix: {ua}"));
        assert_eq!(os.0.split('_').count(), 3, "three-part OS version in {ua}");
        assert!(os.1.ends_with(" Mobile/15E148 Safari/604.1"), "{ua}");
        assert_eq!(
            os.1.split(' ').next().map(|v| v.split('.').count()),
            Some(3),
            "{ua}"
        );
    }
}

#[test]
fn bad_constraints_are_rejected_clearly() {
    let err = generate(&Constraints {
        os: Some("beos".into()),
        ..Default::default()
    })
    .unwrap_err();
    assert!(
        matches!(&err, Error::InvalidConstraint(m) if m.contains("windows")),
        "{err}"
    );

    let err = generate(&Constraints {
        os: Some("ios".into()),
        browser: Some("firefox".into()),
        ..Default::default()
    })
    .unwrap_err();
    assert!(matches!(err, Error::Unsatisfiable(_)));

    let err = generate(&Constraints {
        device: Some("iphone-7".into()),
        browser: Some("chrome-ios".into()),
        ..Default::default()
    })
    .unwrap_err();
    assert!(matches!(err, Error::Unsatisfiable(_)));

    let err = generate(&Constraints {
        locale: Some("tr-TR".into()),
        timezone: Some("Asia/Tokyo".into()),
        ..Default::default()
    })
    .unwrap_err();
    assert!(matches!(err, Error::Unsatisfiable(_)));

    let err = generate(&Constraints {
        os: Some("x".repeat(500)),
        ..Default::default()
    })
    .unwrap_err();
    assert!(matches!(err, Error::InvalidConstraint(_)));

    let err = generate(&Constraints {
        min_version: Some(200),
        max_version: Some(100),
        ..Default::default()
    })
    .unwrap_err();
    assert!(matches!(err, Error::InvalidConstraint(_)));
}

#[test]
fn validator_catches_contradictions() {
    let mut p = generate(&Constraints {
        os: Some("windows".into()),
        browser: Some("chrome".into()),
        ..with_seed("v")
    })
    .unwrap();
    p.navigator.platform = "MacIntel".into();
    p.webgl.unmasked_renderer = Some("Apple GPU".into());
    p.navigator.device_memory = Some(3.0);
    p.navigator.max_touch_points = 0;
    p.device.kind = Kind::Phone;
    let report = validate(&p);
    assert!(!report.valid);
    assert!(report.score < 50);
    let rules: Vec<&str> = report.issues.iter().map(|i| i.rule.as_str()).collect();
    for expected in [
        "navigator.platform",
        "navigator.device_memory",
        "navigator.touch",
        "uach.mobile",
        "device.kind",
        "screen.phone_width",
        "webgl.known_renderer",
    ] {
        assert!(rules.contains(&expected), "missing {expected} in {rules:?}");
    }

    let mut old = generate(&Constraints {
        device: Some("iphone-8".into()),
        ..with_seed("pin")
    })
    .unwrap();
    old.browser.version = "27.0".into();
    let rules: Vec<String> = validate(&old).issues.into_iter().map(|i| i.rule).collect();
    assert!(
        rules.contains(&"browser.os_version_pin".to_owned()),
        "{rules:?}"
    );
}

#[test]
fn headers_follow_the_browser() {
    let chrome = generate(&Constraints {
        os: Some("windows".into()),
        browser: Some("chrome".into()),
        locale: Some("de-DE".into()),
        ..with_seed("h")
    })
    .unwrap();
    let h = navigation_headers(&chrome);
    assert_eq!(h[0].name, "sec-ch-ua");
    assert!(h[0].value.contains("\"Google Chrome\";v=\"15"));
    let lang = h.iter().find(|x| x.name == "accept-language").unwrap();
    assert_eq!(lang.value, "de-DE,de;q=0.9,en-US;q=0.8,en;q=0.7");

    let firefox = generate(&Constraints {
        browser: Some("firefox".into()),
        ..with_seed("h")
    })
    .unwrap();
    let h = navigation_headers(&firefox);
    assert_eq!(h[0].name, "user-agent");
    assert!(!h.iter().any(|x| x.name.starts_with("sec-ch-")));
}

#[test]
fn live_comparison_reports_differences() {
    let p = generate(&Constraints {
        os: Some("macos".into()),
        browser: Some("chrome".into()),
        ..with_seed("live")
    })
    .unwrap();
    let u = p.navigator.user_agent_data.as_ref().unwrap();
    let mut observed = json!({
        "userAgent": p.navigator.user_agent,
        "appVersion": p.navigator.app_version,
        "platform": p.navigator.platform,
        "vendor": p.navigator.vendor,
        "language": p.navigator.language,
        "languages": p.navigator.languages,
        "hardwareConcurrency": p.navigator.hardware_concurrency,
        "deviceMemory": p.navigator.device_memory,
        "maxTouchPoints": 0,
        "oscpu": null,
        "webdriver": false,
        "screen": {
            "width": p.screen.width, "height": p.screen.height,
            "availWidth": p.screen.avail_width, "availHeight": p.screen.avail_height,
            "colorDepth": p.screen.color_depth, "pixelDepth": p.screen.pixel_depth
        },
        "devicePixelRatio": p.screen.device_pixel_ratio,
        "timezone": p.timezone,
        "webgl": {
            "vendor": p.webgl.vendor, "renderer": p.webgl.renderer,
            "unmaskedVendor": p.webgl.unmasked_vendor, "unmaskedRenderer": p.webgl.unmasked_renderer
        },
        "userAgentData": {
            "brands": u.brands, "mobile": u.mobile, "platform": u.platform,
            "platformVersion": u.platform_version, "architecture": u.architecture,
            "bitness": u.bitness, "model": u.model, "fullVersionList": u.full_version_list
        }
    });
    // Only core fields were observed: extension checks are skipped with one warning per missing key.
    let clean = compare_observed(&p, &observed);
    let skipped: Vec<&str> = clean
        .issues
        .iter()
        .map(|i| {
            i.rule
                .strip_prefix("live.not_collected.")
                .unwrap_or_else(|| panic!("{i:?}"))
        })
        .collect();
    assert!(clean.valid, "{:#?}", clean.issues);
    let mut unique = skipped.clone();
    unique.dedup();
    assert_eq!(unique, skipped);

    observed["timezone"] = json!("UTC");
    observed["webdriver"] = json!(true);
    let report = compare_observed(&p, &observed);
    let rules: Vec<&str> = report
        .issues
        .iter()
        .map(|i| i.rule.as_str())
        .filter(|r| !r.starts_with("live.not_collected."))
        .collect();
    assert_eq!(rules, ["live.timezone", "live.webdriver"]);

    // A collected key with wrong values is still an error.
    if let Some(key) = skipped.first() {
        observed[*key] = json!({});
        let report = compare_observed(&p, &observed);
        assert!(report
            .issues
            .iter()
            .any(|i| i.rule.starts_with(&format!("live.{key}."))
                && i.severity == veneer::Severity::Error));
    }
}

#[test]
fn init_script_cannot_be_escaped_by_profile_values() {
    let mut p = generate(&with_seed("inject")).unwrap();
    p.navigator.platform = "\"});alert(1);//</script>\u{2028}".into();
    let script = to_init_script(&p).unwrap();
    assert!(script.contains(r#""platform":"\"});alert(1);//</script>"#));
    assert!(!script.contains("__VENEER_PROFILE__"));
}
