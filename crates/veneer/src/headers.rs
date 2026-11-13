//! HTTP request headers a real browser with this profile sends for a top-level navigation.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::model::{Brand, Engine, Profile};
use crate::validate::{Collector, Report, Severity};
use crate::Error;

/// Most headers one captured request may carry; real navigations send about 15.
pub const MAX_HEADERS: usize = 256;
/// Longest header name accepted in a comparison.
pub const MAX_HEADER_NAME: usize = 256;
/// Longest header value accepted in a comparison.
pub const MAX_HEADER_VALUE: usize = 8 * 1024;

/// Headers that identify the browser; their values must match the profile exactly.
const IDENTITY: [&str; 5] = [
    "user-agent",
    "accept-language",
    "sec-ch-ua",
    "sec-ch-ua-mobile",
    "sec-ch-ua-platform",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    pub name: String,
    pub value: String,
}

const ACCEPT_CHROMIUM: &str = "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7";
const ACCEPT_OTHER: &str = "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8";

/// Navigation request headers in the order the browser sends them.
pub fn navigation_headers(p: &Profile) -> Vec<Header> {
    let nav = &p.navigator;
    let ua = nav.user_agent.as_str();
    let lang = accept_language(p.browser.engine, &nav.languages);
    let mut h: Vec<(&str, String)> = Vec::new();
    match p.browser.engine {
        Engine::Chromium => {
            if let Some(u) = &nav.user_agent_data {
                h.push(("sec-ch-ua", sec_ch_ua(&u.brands)));
                h.push((
                    "sec-ch-ua-mobile",
                    if u.mobile { "?1" } else { "?0" }.into(),
                ));
                h.push(("sec-ch-ua-platform", format!("\"{}\"", u.platform)));
            }
            h.push(("upgrade-insecure-requests", "1".into()));
            h.push(("user-agent", ua.into()));
            h.push(("accept", ACCEPT_CHROMIUM.into()));
            h.push(("sec-fetch-site", "none".into()));
            h.push(("sec-fetch-mode", "navigate".into()));
            h.push(("sec-fetch-user", "?1".into()));
            h.push(("sec-fetch-dest", "document".into()));
            // zstd shipped in Chromium 123; browsers without client hints here run older Chromium.
            let encoding = if nav.user_agent_data.is_some() {
                "gzip, deflate, br, zstd"
            } else {
                "gzip, deflate, br"
            };
            h.push(("accept-encoding", encoding.into()));
            h.push(("accept-language", lang));
            h.push(("priority", "u=0, i".into()));
        }
        Engine::Gecko => {
            h.push(("user-agent", ua.into()));
            h.push(("accept", ACCEPT_OTHER.into()));
            h.push(("accept-language", lang));
            h.push(("accept-encoding", "gzip, deflate, br, zstd".into()));
            h.push(("upgrade-insecure-requests", "1".into()));
            h.push(("sec-fetch-dest", "document".into()));
            h.push(("sec-fetch-mode", "navigate".into()));
            h.push(("sec-fetch-site", "none".into()));
            h.push(("sec-fetch-user", "?1".into()));
            h.push(("priority", "u=0, i".into()));
        }
        Engine::Webkit => {
            h.push(("accept", ACCEPT_OTHER.into()));
            h.push(("sec-fetch-site", "none".into()));
            h.push(("accept-encoding", "gzip, deflate, br".into()));
            h.push(("sec-fetch-mode", "navigate".into()));
            h.push(("user-agent", ua.into()));
            h.push(("accept-language", lang));
            h.push(("sec-fetch-dest", "document".into()));
            h.push(("priority", "u=0, i".into()));
        }
    }
    h.into_iter()
        .map(|(name, value)| Header {
            name: name.to_owned(),
            value,
        })
        .collect()
}

/// `Accept-Language` with each engine's q-value scheme.
pub fn accept_language(engine: Engine, languages: &[String]) -> String {
    let n = languages.len();
    match engine {
        Engine::Webkit => match languages.first() {
            // Safari exposes one language but also sends its base language.
            Some(first) => match first.split_once('-') {
                Some((base, _)) => format!("{first},{base};q=0.9"),
                None => first.clone(),
            },
            None => String::new(),
        },
        _ => languages
            .iter()
            .enumerate()
            .map(|(i, l)| {
                if i == 0 {
                    return l.clone();
                }
                let q = match engine {
                    // Firefox spreads q-values evenly: 1 - i/n, rounded to one decimal.
                    Engine::Gecko => ((1.0 - i as f64 / n as f64) * 10.0).round() / 10.0,
                    _ => (1.0 - 0.1 * i as f64).max(0.1),
                };
                format!("{l};q={q:.1}")
            })
            .collect::<Vec<_>>()
            .join(","),
    }
}

fn sec_ch_ua(brands: &[Brand]) -> String {
    brands
        .iter()
        .map(|b| format!("\"{}\";v=\"{}\"", b.brand, b.version))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Parse captured request headers: an array of `{name, value}` objects or `[name, value]` pairs
/// (order kept), or a `{name: value}` object (order lost). Names are lower-cased and sizes bounded,
/// so untrusted captures cannot exhaust memory. Returns the headers and whether their order is known.
pub fn headers_from_json(value: &Value) -> Result<(Vec<Header>, bool), Error> {
    let bad = |m: &str| Error::InvalidConstraint(format!("headers: {m}"));
    let (pairs, ordered): (Vec<(&str, &str)>, bool) = match value {
        Value::Object(map) => (
            map.iter()
                .map(|(k, v)| {
                    v.as_str()
                        .map(|v| (k.as_str(), v))
                        .ok_or_else(|| bad("every value must be a string"))
                })
                .collect::<Result<_, _>>()?,
            false,
        ),
        Value::Array(items) => (
            items
                .iter()
                .map(|item| {
                    let pair = match item {
                        Value::Object(o) => (o.get("name"), o.get("value")),
                        Value::Array(a) if a.len() == 2 => (a.first(), a.get(1)),
                        _ => (None, None),
                    };
                    match pair {
                        (Some(Value::String(n)), Some(Value::String(v))) => {
                            Ok((n.as_str(), v.as_str()))
                        }
                        _ => Err(bad(
                            "each item must be {name, value} or [name, value] strings",
                        )),
                    }
                })
                .collect::<Result<_, _>>()?,
            true,
        ),
        _ => return Err(bad("expected an array or an object")),
    };
    if pairs.len() > MAX_HEADERS {
        return Err(bad(&format!("at most {MAX_HEADERS} headers")));
    }
    let headers = pairs
        .into_iter()
        .map(|(name, value)| {
            // HTTP/2 pseudo-headers (`:authority`) start with a colon; otherwise RFC 9110 token chars.
            let token = name.strip_prefix(':').unwrap_or(name);
            let valid = !token.is_empty()
                && name.len() <= MAX_HEADER_NAME
                && token
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b));
            if !valid {
                return Err(bad("invalid header name"));
            }
            if value.len() > MAX_HEADER_VALUE {
                return Err(bad(&format!(
                    "header values are limited to {MAX_HEADER_VALUE} bytes"
                )));
            }
            Ok(Header {
                name: name.to_ascii_lowercase(),
                value: value.to_owned(),
            })
        })
        .collect::<Result<_, _>>()?;
    Ok((headers, ordered))
}

/// Compare request headers captured from a real navigation (for example a server-side
/// `req.rawHeaders` capture or a HAR) with what a browser with this profile sends.
///
/// Identity headers (`user-agent`, `accept-language`, `sec-ch-ua*`) must match exactly: a mismatch
/// is an `error` named `headers.<name>`, and client hints sent for a profile without them are an
/// `error` too. A different relative order of the expected headers is a `warning` (`headers.order`).
pub fn compare_headers(p: &Profile, observed: &[Header]) -> Report {
    compare(p, observed, true)
}

/// [`compare_headers`] for JSON input in any form [`headers_from_json`] accepts; the order check
/// is skipped when the input is an object, whose order is not preserved.
pub fn compare_headers_json(p: &Profile, observed: &Value) -> Result<Report, Error> {
    let (headers, ordered) = headers_from_json(observed)?;
    Ok(compare(p, &headers, ordered))
}

fn compare(p: &Profile, observed: &[Header], check_order: bool) -> Report {
    let mut c = Collector::default();
    let expected = navigation_headers(p);
    let position = |name: &str| {
        observed
            .iter()
            .position(|h| h.name.eq_ignore_ascii_case(name))
    };
    for name in IDENTITY {
        let want = expected
            .iter()
            .find(|h| h.name == name)
            .map(|h| h.value.as_str());
        let sent = position(name).map(|i| observed[i].value.as_str());
        let rule = format!("headers.{name}");
        let problem = match (want, sent) {
            (Some(w), Some(s)) if w == s => None,
            (Some(w), Some(s)) => Some(format!("sent `{s}`, the profile expects `{w}`")),
            (Some(w), None) => Some(format!("not sent, the profile expects `{w}`")),
            (None, Some(s)) => Some(format!(
                "sent `{s}`, but {} {} does not send client hints",
                p.browser.name, p.browser.version
            )),
            (None, None) => None,
        };
        if let Some(message) = problem {
            c.push(Severity::Error, &rule, message);
        }
    }
    if check_order {
        let order: Vec<&str> = expected
            .iter()
            .map(|h| h.name.as_str())
            .filter(|n| position(n).is_some())
            .collect();
        let mut actual = order.clone();
        actual.sort_by_key(|n| position(n));
        c.warn_if(order != actual, "headers.order", || {
            format!(
                "sent in the order {}; {} sends {}",
                actual.join(", "),
                p.browser.name,
                order.join(", ")
            )
        });
    }
    c.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(os: &str, browser: &str) -> Profile {
        crate::generate(&crate::Constraints {
            os: Some(os.into()),
            browser: Some(browser.into()),
            seed: Some("headers-test".into()),
            ..Default::default()
        })
        .expect("profile")
    }

    #[test]
    fn own_headers_compare_clean() {
        for (os, browser) in [
            ("windows", "chrome"),
            ("macos", "safari"),
            ("linux", "firefox"),
        ] {
            let p = profile(os, browser);
            let report = compare_headers(&p, &navigation_headers(&p));
            assert!(report.issues.is_empty(), "{browser}: {:?}", report.issues);
        }
    }

    #[test]
    fn mismatches_are_reported() {
        let p = profile("windows", "chrome");
        let mut sent = navigation_headers(&p);
        sent.retain(|h| h.name != "sec-ch-ua-platform");
        for h in &mut sent {
            if h.name == "user-agent" {
                h.value = "curl/8.0".into();
            }
        }
        sent.swap(0, 1);
        let report = compare_headers(&p, &sent);
        let rules: Vec<&str> = report.issues.iter().map(|i| i.rule.as_str()).collect();
        assert!(!report.valid);
        assert!(rules.contains(&"headers.user-agent"));
        assert!(rules.contains(&"headers.sec-ch-ua-platform"));
        assert!(rules.contains(&"headers.order"));

        let firefox = profile("linux", "firefox");
        let mut leaked = navigation_headers(&firefox);
        leaked.push(Header {
            name: "sec-ch-ua".into(),
            value: "\"Chromium\";v=\"140\"".into(),
        });
        let report = compare_headers(&firefox, &leaked);
        assert_eq!(report.issues.len(), 1);
        assert_eq!(report.issues[0].rule, "headers.sec-ch-ua");
    }

    #[test]
    fn header_json_forms() {
        let p = profile("linux", "firefox");
        let object: serde_json::Map<String, Value> = navigation_headers(&p)
            .into_iter()
            .map(|h| (h.name.to_ascii_uppercase(), Value::String(h.value)))
            .collect();
        let report = compare_headers_json(&p, &Value::Object(object)).expect("object");
        assert!(report.issues.is_empty(), "{:?}", report.issues);
        let list = serde_json::json!([{ "name": "user-agent", "value": "x" }, ["accept", "*/*"]]);
        let (headers, ordered) = headers_from_json(&list).expect("list");
        assert!(ordered);
        assert_eq!(headers[1].name, "accept");
        assert!(headers_from_json(&serde_json::json!([{ "name": "a b", "value": "x" }])).is_err());
        assert!(headers_from_json(&serde_json::json!(":")).is_err());
        assert!(headers_from_json(&serde_json::json!({ "x": 1 })).is_err());
    }

    fn langs(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn accept_language_matches_real_browsers() {
        let de = langs(&["de-DE", "de", "en-US", "en"]);
        assert_eq!(
            accept_language(Engine::Chromium, &de),
            "de-DE,de;q=0.9,en-US;q=0.8,en;q=0.7"
        );
        assert_eq!(
            accept_language(Engine::Gecko, &de),
            "de-DE,de;q=0.8,en-US;q=0.5,en;q=0.3"
        );
        assert_eq!(
            accept_language(Engine::Gecko, &langs(&["en-US", "en"])),
            "en-US,en;q=0.5"
        );
        assert_eq!(
            accept_language(Engine::Webkit, &langs(&["en-US"])),
            "en-US,en;q=0.9"
        );
    }
}
