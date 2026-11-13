//! Network-layer helpers that sit next to a profile: a TLS (JA4) family check, a proxy-location
//! consistency check, and WebRTC IP-handling settings per engine.
//!
//! Veneer runs inside the page and in launch options; it cannot change the TLS ClientHello, the
//! HTTP/2 SETTINGS frame or the egress IP. These helpers only report whether what the caller observed
//! or chose fits the profile. Background and sources: `docs/network/`.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::data::dataset;
use crate::model::{Engine, Kind, Profile};
use crate::validate::{Collector, Report};
use crate::Error;

/// A check result plus notes that explain what was and was not compared.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetworkCheck {
    pub report: Report,
    pub notes: Vec<String>,
}

const TLS_NOTE: &str = "Veneer cannot change the TLS ClientHello: it is produced by the browser's own TLS stack (BoringSSL in Chromium, NSS in Firefox, Apple's stack in Safari and every iOS browser). Launch the engine that matches the profile.";

/// JA4 cipher-suite hash (JA4_b) of Chromium's TCP ClientHello, from FoxIO's labelled Chrome example.
const CHROMIUM_TCP_CIPHER_HASH: &str = "8daaf6152771";
/// JA4 cipher-suite hash of Chromium's QUIC ClientHello, from FoxIO's labelled Chrome example.
const CHROMIUM_QUIC_CIPHER_HASH: &str = "55b375c5d22e";

/// Parsed JA4 TLS client fingerprint (`t13d1517h2_8daaf6152771_cb7bf5808d99`).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Ja4<'a> {
    protocol: char,
    version: &'a str,
    sni: char,
    ciphers: u32,
    extensions: u32,
    alpn: &'a str,
    cipher_hash: &'a str,
}

fn parse_ja4(s: &str) -> Result<Ja4<'_>, Error> {
    let bad = || {
        Error::InvalidConstraint(
            "ja4 must look like t13d1517h2_8daaf6152771_cb7bf5808d99 (JA4_a, 12-hex JA4_b, 12-hex JA4_c)"
                .into(),
        )
    };
    let hex12 = |p: &str| p.len() == 12 && p.bytes().all(|b| b.is_ascii_hexdigit());
    let mut parts = s.split('_');
    let (Some(a), Some(b), Some(c), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(bad());
    };
    if a.len() != 10 || !a.is_ascii() || !hex12(b) || !hex12(c) {
        return Err(bad());
    }
    let protocol = a[0..1].chars().next().ok_or_else(bad)?;
    let sni = a[3..4].chars().next().ok_or_else(bad)?;
    let digits = |p: &str| {
        p.bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| p.parse::<u32>().ok())
            .flatten()
    };
    let alpn = &a[8..10];
    if !matches!(protocol, 't' | 'q' | 'd')
        || !matches!(sni, 'd' | 'i')
        || !alpn.bytes().all(|b| b.is_ascii_alphanumeric())
    {
        return Err(bad());
    }
    Ok(Ja4 {
        protocol,
        version: &a[1..3],
        sni,
        ciphers: digits(&a[4..6]).ok_or_else(bad)?,
        extensions: digits(&a[6..8]).ok_or_else(bad)?,
        alpn,
        cipher_hash: b,
    })
}

/// Compare a JA4 fingerprint the caller captured (for example from a TLS echo service) with the
/// TLS stack the profile's browser uses. Every finding is a `warning`: JA4 values move with browser
/// versions, so this is informational and never invalidates a profile.
pub fn check_ja4(p: &Profile, ja4: &str) -> Result<NetworkCheck, Error> {
    let ja4 = ja4.trim();
    if ja4.len() > 64 {
        return Err(Error::InvalidConstraint(
            "ja4 is at most 64 characters".into(),
        ));
    }
    let f = parse_ja4(ja4)?;
    let mut c = Collector::default();
    let mut notes = vec![TLS_NOTE.to_owned()];
    if matches!(p.device.kind, Kind::Console | Kind::Tv | Kind::Vr) {
        notes.push(format!(
            "No verified ClientHello data for {} on {}; only the JA4 format was checked.",
            p.browser.name, p.os.name
        ));
        return Ok(NetworkCheck {
            report: c.finish(),
            notes,
        });
    }
    c.warn_if(f.protocol == 'd', "tls.ja4.protocol", || {
        "DTLS fingerprint (WebRTC), not a page load; capture an HTTPS request instead".into()
    });
    c.warn_if(f.version != "13", "tls.ja4.version", || {
        format!(
            "highest TLS version `{}`; {} {} offers TLS 1.3 (`13`)",
            f.version, p.browser.name, p.browser.version
        )
    });
    c.warn_if(f.sni == 'i', "tls.ja4.sni", || {
        "no SNI: browsers send SNI for host names (expected for IP-literal URLs only)".into()
    });
    let alpn = if f.protocol == 'q' { "h3" } else { "h2" };
    c.warn_if(f.protocol != 'd' && f.alpn != alpn, "tls.ja4.alpn", || {
        format!(
            "first ALPN `{}`; browsers offer `{alpn}` first (`00` means no ALPN, typical of HTTP libraries)",
            f.alpn
        )
    });
    let _ = f.extensions; // Extension counts change between browser releases; not compared.
    match (p.browser.engine, f.protocol) {
        (Engine::Chromium, 't') => {
            c.warn_if(f.ciphers != 15, "tls.ja4.ciphers", || {
                format!("{} cipher suites; Chromium (BoringSSL) offers 15", f.ciphers)
            });
            c.warn_if(
                f.cipher_hash != CHROMIUM_TCP_CIPHER_HASH,
                "tls.ja4.cipher-hash",
                || {
                    format!(
                        "cipher hash `{}`; Chromium's is `{CHROMIUM_TCP_CIPHER_HASH}`",
                        f.cipher_hash
                    )
                },
            );
        }
        (Engine::Chromium, 'q') => {
            c.warn_if(
                f.cipher_hash != CHROMIUM_QUIC_CIPHER_HASH,
                "tls.ja4.cipher-hash",
                || {
                    format!(
                        "QUIC cipher hash `{}`; Chromium's is `{CHROMIUM_QUIC_CIPHER_HASH}`",
                        f.cipher_hash
                    )
                },
            );
        }
        (Engine::Gecko, 't') => {
            c.warn_if(f.ciphers != 17, "tls.ja4.ciphers", || {
                format!("{} cipher suites; Firefox (NSS) offers 17", f.ciphers)
            });
            c.warn_if(
                f.cipher_hash == CHROMIUM_TCP_CIPHER_HASH,
                "tls.ja4.cipher-hash",
                || "Chromium's cipher hash on a Firefox profile: the page was loaded by a Chromium-based browser".into(),
            );
        }
        (_, 'd') => {}
        _ => notes.push(format!(
            "Public sources disagree on {} JA4 cipher counts; only protocol, TLS version, SNI and ALPN were checked.",
            p.browser.name
        )),
    }
    Ok(NetworkCheck {
        report: c.finish(),
        notes,
    })
}

/// Longest IANA timezone name accepted (the longest real one is 32 characters).
const MAX_TIMEZONE: usize = 64;

/// Compare the egress location the caller knows (from their proxy provider; Veneer never looks up
/// IPs) with the profile's timezone and locale.
///
/// - `location.timezone` (error): the given timezone differs from the profile's.
/// - `location.country` (error): the profile's timezone is not used in the given country.
/// - `location.locale` (warning): the locale's region is another country (common for expatriates).
pub fn check_location(
    p: &Profile,
    country: Option<&str>,
    timezone: Option<&str>,
) -> Result<NetworkCheck, Error> {
    let mut c = Collector::default();
    let mut notes = Vec::new();
    if country.is_none() && timezone.is_none() {
        return Err(Error::InvalidConstraint(
            "give a country, a timezone or both".into(),
        ));
    }
    if let Some(tz) = timezone {
        let valid = !tz.is_empty()
            && tz.len() <= MAX_TIMEZONE
            && tz
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"/_+-".contains(&b));
        if !valid {
            return Err(Error::InvalidConstraint(
                "timezone must be an IANA name such as Europe/Istanbul".into(),
            ));
        }
        c.error_if(tz != p.timezone, "location.timezone", || {
            format!(
                "the connection is in `{tz}`, the profile reports `{}`",
                p.timezone
            )
        });
    }
    if let Some(cc) = country {
        if cc.len() != 2 || !cc.bytes().all(|b| b.is_ascii_alphabetic()) {
            return Err(Error::InvalidConstraint(
                "country must be a two-letter ISO 3166 code such as TR".into(),
            ));
        }
        let cc = cc.to_ascii_uppercase();
        let zones: BTreeSet<&str> = dataset()?
            .locales
            .iter()
            .filter(|l| l.region().eq_ignore_ascii_case(&cc))
            .flat_map(|l| l.timezones.iter().map(|(tz, _)| tz.as_str()))
            .collect();
        if zones.is_empty() {
            notes.push(format!(
                "`{cc}` is not in the locale dataset; its timezones were not compared"
            ));
        } else {
            c.error_if(
                !zones.contains(p.timezone.as_str()),
                "location.country",
                || {
                    format!(
                        "the connection is in {cc}, the profile timezone `{}` is not one of {}",
                        p.timezone,
                        zones.iter().copied().collect::<Vec<_>>().join(", ")
                    )
                },
            );
        }
        let region = p.locale.rsplit('-').next().unwrap_or_default();
        c.warn_if(
            !region.eq_ignore_ascii_case(&cc),
            "location.locale",
            || {
                format!(
                    "locale `{}` belongs to another region than {cc}; generate with --country {cc} to align it",
                    p.locale
                )
            },
        );
    }
    Ok(NetworkCheck {
        report: c.finish(),
        notes,
    })
}

/// How much a page may learn about local network interfaces through WebRTC ICE candidates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WebRtcPolicy {
    /// Only the interface of the default route; no private addresses.
    PublicInterfaceOnly,
    /// As above, and UDP only through a proxy that supports it (otherwise TCP or no WebRTC).
    DisableNonProxiedUdp,
    /// Turn `RTCPeerConnection` off where the engine allows it.
    Disabled,
}

/// Launch settings that apply a [`WebRtcPolicy`] to one engine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebRtcSettings {
    pub engine: Engine,
    pub policy: WebRtcPolicy,
    /// Chromium command-line switches.
    pub args: Vec<String>,
    /// Chromium profile `Preferences` entries (JSON merged into the file), or Firefox prefs.
    pub prefs: Value,
    /// Options object for Playwright `browserType.launch()`.
    pub playwright_launch_options: Value,
    pub notes: Vec<String>,
}

/// Engine-specific settings for a WebRTC IP-handling policy. Values come from Chromium
/// (`force-webrtc-ip-handling-policy`, `webrtc.ip_handling_policy`) and Firefox (`media.peerconnection.*`).
pub fn webrtc_settings(engine: Engine, policy: WebRtcPolicy) -> WebRtcSettings {
    let mut notes = Vec::new();
    let (args, prefs, launch) = match engine {
        Engine::Chromium => {
            let value = match policy {
                WebRtcPolicy::PublicInterfaceOnly => "default_public_interface_only",
                WebRtcPolicy::DisableNonProxiedUdp | WebRtcPolicy::Disabled => {
                    "disable_non_proxied_udp"
                }
            };
            if policy == WebRtcPolicy::Disabled {
                notes.push("Chromium has no switch that removes RTCPeerConnection; the strictest IP-handling policy is used instead.".into());
            }
            let args = vec![format!("--force-webrtc-ip-handling-policy={value}")];
            let launch = json!({ "args": args });
            (
                args,
                json!({ "webrtc": { "ip_handling_policy": value } }),
                launch,
            )
        }
        Engine::Gecko => {
            let prefs = match policy {
                WebRtcPolicy::PublicInterfaceOnly => json!({
                    "media.peerconnection.ice.default_address_only": true,
                    "media.peerconnection.ice.no_host": true,
                }),
                WebRtcPolicy::DisableNonProxiedUdp => json!({
                    "media.peerconnection.ice.default_address_only": true,
                    "media.peerconnection.ice.no_host": true,
                    "media.peerconnection.ice.proxy_only_if_behind_proxy": true,
                }),
                WebRtcPolicy::Disabled => json!({ "media.peerconnection.enabled": false }),
            };
            let launch = json!({ "firefoxUserPrefs": prefs });
            (Vec::new(), prefs, launch)
        }
        Engine::Webkit => {
            notes.push("WebKit exposes no WebRTC IP-handling setting to Playwright; route traffic through the proxy at the OS level or use another engine.".into());
            (Vec::new(), json!({}), json!({}))
        }
    };
    if policy == WebRtcPolicy::Disabled && engine != Engine::Webkit {
        notes.push("A page can notice that WebRTC is missing or restricted; real users rarely change this setting.".into());
    }
    WebRtcSettings {
        engine,
        policy,
        args,
        prefs,
        playwright_launch_options: launch,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Constraints;

    fn profile(os: &str, browser: &str, country: Option<&str>) -> Profile {
        crate::generate(&Constraints {
            os: Some(os.into()),
            browser: Some(browser.into()),
            country: country.map(Into::into),
            seed: Some("network-test".into()),
            ..Default::default()
        })
        .expect("profile")
    }

    fn rules(check: &NetworkCheck) -> Vec<&str> {
        check
            .report
            .issues
            .iter()
            .map(|i| i.rule.as_str())
            .collect()
    }

    #[test]
    fn ja4_chrome_examples_match_chromium_profiles() {
        let chrome = profile("windows", "chrome", None);
        for ja4 in [
            "t13d1517h2_8daaf6152771_cb7bf5808d99",
            "t13d1516h2_8daaf6152771_02713d6af862",
            "q13d0312h3_55b375c5d22e_178839b6cec1",
        ] {
            let check = check_ja4(&chrome, ja4).expect("valid ja4");
            assert!(check.report.issues.is_empty(), "{ja4}: {:?}", check.report);
        }
        let firefox = profile("windows", "firefox", None);
        let check = check_ja4(&firefox, "t13d1517h2_8daaf6152771_cb7bf5808d99").expect("ja4");
        assert_eq!(rules(&check), ["tls.ja4.ciphers", "tls.ja4.cipher-hash"]);
        assert!(check.report.valid, "JA4 findings are warnings only");
        let library = check_ja4(&chrome, "t12i190900_9dc949149365_97f8aa674fd9").expect("ja4");
        assert!(rules(&library).contains(&"tls.ja4.alpn"));
        assert!(rules(&library).contains(&"tls.ja4.version"));
        assert!(rules(&library).contains(&"tls.ja4.sni"));
        for bad in [
            "",
            "t13d1517h2",
            "x13d1517h2_8daaf6152771_cb7bf5808d99",
            "t13d15a7h2_8daaf6152771_cb7bf5808d99",
            "t13d1517h2_8daaf615277_cb7bf5808d99",
        ] {
            assert!(check_ja4(&chrome, bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn location_matches_country_and_timezone() {
        let p = profile("windows", "chrome", Some("TR"));
        let ok = check_location(&p, Some("tr"), Some(&p.timezone.clone())).expect("check");
        assert!(ok.report.issues.is_empty(), "{:?}", ok.report);
        let wrong = check_location(&p, Some("US"), Some("America/New_York")).expect("check");
        assert_eq!(
            rules(&wrong),
            ["location.timezone", "location.country", "location.locale"]
        );
        let unknown = check_location(&p, Some("ZZ"), None).expect("check");
        assert_eq!(unknown.notes.len(), 1);
        assert!(check_location(&p, None, None).is_err());
        assert!(check_location(&p, Some("TUR"), None).is_err());
        assert!(check_location(&p, None, Some("Europe/<script>")).is_err());
    }

    #[test]
    fn webrtc_settings_per_engine() {
        let chromium = webrtc_settings(Engine::Chromium, WebRtcPolicy::PublicInterfaceOnly);
        assert_eq!(
            chromium.args,
            ["--force-webrtc-ip-handling-policy=default_public_interface_only"]
        );
        let firefox = webrtc_settings(Engine::Gecko, WebRtcPolicy::Disabled);
        assert_eq!(
            firefox.playwright_launch_options["firefoxUserPrefs"]["media.peerconnection.enabled"],
            json!(false)
        );
        assert!(!webrtc_settings(Engine::Webkit, WebRtcPolicy::Disabled)
            .notes
            .is_empty());
    }
}
