//! Browser release age against a caller-supplied date.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::model::Profile;
use crate::validate::Collector;

/// Days since 1970-01-01 for a strict `YYYY-MM-DD` date.
pub(crate) fn parse_date(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let num = |r: std::ops::Range<usize>| s.get(r)?.parse::<i64>().ok();
    let (y, m, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days_in = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=12).contains(&m) || d < 1 || d > days_in[(m - 1) as usize] {
        return None;
    }
    // Howard Hinnant's days_from_civil.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Releases {
    /// Browser id → release family whose major numbers its user agent reports.
    families: BTreeMap<String, String>,
    /// Family → major → first stable release date.
    releases: BTreeMap<String, BTreeMap<u32, String>>,
}

fn releases() -> Option<&'static Releases> {
    static TABLE: OnceLock<Option<Releases>> = OnceLock::new();
    TABLE
        .get_or_init(|| {
            crate::embedded::EXTRA_DATA_FILES
                .iter()
                .find(|(stem, _)| *stem == "browser-releases")
                .and_then(|(_, text)| serde_json::from_str(text).ok())
        })
        .as_ref()
}

/// Release date of a browser major, or of the oldest known major when this one is older still.
pub(crate) fn release_date(browser_id: &str, major: u32) -> Option<&'static str> {
    let t = releases()?;
    let family = t.releases.get(t.families.get(browser_id)?)?;
    match family.get(&major) {
        Some(d) => Some(d),
        None => family
            .first_key_value()
            .filter(|(oldest, _)| major < **oldest)
            .map(|(_, d)| d.as_str()),
    }
}

pub(crate) fn stale(p: &Profile, today: i64, max_age_days: u32, c: &mut Collector) {
    let Some(date) = release_date(&p.browser.id, p.browser.major) else {
        return;
    };
    let Some(released) = parse_date(date) else {
        return;
    };
    let age = today - released;
    c.warn_if(
        age > i64::from(max_age_days),
        "browser.stale_version",
        || {
            format!(
                "{} {} was released on or before {date}, {age} days ago (limit {max_age_days})",
                p.browser.name, p.browser.major
            )
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate, Constraints};

    #[test]
    fn dates() {
        assert_eq!(parse_date("1970-01-01"), Some(0));
        assert_eq!(parse_date("2024-02-29"), Some(19_782));
        assert_eq!(parse_date("2023-02-29"), None);
        assert_eq!(parse_date("2024-1-01"), None);
    }

    #[test]
    fn release_table_parses_and_old_versions_warn() {
        assert_eq!(release_date("chrome", 120), Some("2023-12-05"));
        assert_eq!(release_date("firefox", 128), Some("2024-07-09"));
        assert_eq!(release_date("chrome", 60), Some("2022-03-29"));
        let mut p = generate(&Constraints {
            browser: Some("chrome".into()),
            seed: Some("stale".into()),
            ..Default::default()
        })
        .unwrap();
        p.browser.major = 120;
        let mut c = Collector::default();
        stale(&p, parse_date("2026-11-13").unwrap(), 180, &mut c);
        assert_eq!(c.finish().issues[0].rule, "browser.stale_version");
    }
}
