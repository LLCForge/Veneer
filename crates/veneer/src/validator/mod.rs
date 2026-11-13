//! Validator extensions: options (disabled rules, custom rules, request headers, location, date),
//! capture/import of profiles from other sources, rule metadata, SARIF and distribution drift.

pub(crate) mod capture;
pub(crate) mod context;
pub(crate) mod custom;
pub(crate) mod drift;
pub(crate) mod headers;
pub(crate) mod plausibility;
pub(crate) mod rules;
pub(crate) mod sarif;

use serde::{Deserialize, Serialize};

use crate::headers::{compare_headers, Header, MAX_HEADERS, MAX_HEADER_NAME, MAX_HEADER_VALUE};
use crate::model::Profile;
use crate::validate::{check_into, Collector, Report};
use crate::Error;

pub use capture::{capture, import_profile, ImportFormat, Imported};
pub use custom::{Condition, CustomRule, Op, Test};
pub use drift::{drift, DriftBucket, DriftDimension, DriftReport};
pub use headers::headers_from_har;
pub use rules::{rule_coverage, Coverage, RuleInfo};
pub use sarif::to_sarif;

/// Version of the built-in rule set. Bumped whenever a rule is added, removed or changes meaning,
/// so reports from different releases can be told apart.
pub const RULES_VERSION: &str = "2.0.0";

const MAX_DISABLE: usize = 200;
const MAX_ID: usize = 64;

pub(crate) use rules::annotate;

/// Everything `validate_with` can take beyond the profile. Every field is optional, and the same
/// JSON shape is accepted by `veneer validate --options <file>` and the `/v1/check` API.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidateOptions {
    /// Rule ids to skip. `fonts.*` skips every rule starting with `fonts.`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disable: Vec<String>,
    /// Extra rules declared in JSON. Ids must start with `custom.`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<CustomRule>,
    /// Request headers the browser actually sent (for example from a HAR file), in sent order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<Header>>,
    /// Location the caller knows the traffic comes from (for example the proxy exit). Never looked up.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geo: Option<Geo>,
    /// Today's date, `YYYY-MM-DD`. Enables the browser release-age rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub today: Option<String>,
    /// Oldest acceptable browser release, in days before `today` (default 180).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_age_days: Option<u32>,
}

/// User-supplied location of the connection.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Geo {
    /// ISO 3166-1 alpha-2 country code, e.g. `TR`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// IANA timezone of the exit location, e.g. `Europe/Istanbul`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

impl ValidateOptions {
    fn check(&self) -> Result<(), Error> {
        let bad = |m: String| Err(Error::InvalidConstraint(m));
        if self.disable.len() > MAX_DISABLE {
            return bad(format!("`disable` has more than {MAX_DISABLE} entries"));
        }
        if self
            .disable
            .iter()
            .any(|d| d.is_empty() || d.len() > MAX_ID)
        {
            return bad(format!("`disable` entries must be 1-{MAX_ID} characters"));
        }
        if let Some(headers) = &self.headers {
            check_headers(headers)?;
        }
        custom::check_rules(&self.rules)?;
        if let Some(today) = &self.today {
            context::parse_date(today)
                .ok_or_else(|| Error::InvalidConstraint("`today` must be YYYY-MM-DD".into()))?;
        }
        Ok(())
    }

    fn disabled(&self, rule: &str) -> bool {
        self.disable.iter().any(|d| match d.strip_suffix('*') {
            Some(prefix) => rule.starts_with(prefix),
            None => d == rule,
        })
    }
}

/// Length and count limits for headers that come from outside.
pub(crate) fn check_headers(headers: &[Header]) -> Result<(), Error> {
    if headers.len() > MAX_HEADERS {
        return Err(Error::InvalidConstraint(format!(
            "more than {MAX_HEADERS} request headers"
        )));
    }
    if headers.iter().any(|h| {
        h.name.is_empty() || h.name.len() > MAX_HEADER_NAME || h.value.len() > MAX_HEADER_VALUE
    }) {
        return Err(Error::InvalidConstraint(format!(
            "header names must be 1-{MAX_HEADER_NAME} bytes and values at most {MAX_HEADER_VALUE} bytes"
        )));
    }
    Ok(())
}

/// Every built-in rule, plus the checks the options enable, minus the disabled rules.
pub fn validate_with(profile: &Profile, options: &ValidateOptions) -> Result<Report, Error> {
    options.check()?;
    let mut c = Collector::default();
    check_into(profile, &mut c);
    if let Some(h) = &options.headers {
        compare_headers(profile, h)
            .issues
            .into_iter()
            .for_each(|i| c.push_issue(i));
    }
    if let Some(g) = options
        .geo
        .as_ref()
        .filter(|g| g.country.is_some() || g.timezone.is_some())
    {
        crate::network::check_location(profile, g.country.as_deref(), g.timezone.as_deref())?
            .report
            .issues
            .into_iter()
            .for_each(|i| c.push_issue(i));
    }
    if let Some(today) = options.today.as_deref().and_then(context::parse_date) {
        context::stale(profile, today, options.max_age_days.unwrap_or(180), &mut c);
    }
    custom::run(profile, &options.rules, &mut c)?;
    c.retain(|i| !options.disabled(&i.rule));
    Ok(c.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate, validate, Constraints, FixAction, Severity};

    fn profile(seed: &str, browser: &str) -> Profile {
        generate(&Constraints {
            browser: Some(browser.into()),
            seed: Some(seed.into()),
            ..Default::default()
        })
        .unwrap()
    }

    #[test]
    fn issues_carry_hint_fix_and_rules_version() {
        let mut p = profile("fix", "chrome");
        p.navigator.platform = "Nope".into();
        p.navigator.device_memory = Some(3.0);
        let r = validate(&p);
        assert_eq!(r.rules_version, RULES_VERSION);
        let platform = r
            .issues
            .iter()
            .find(|i| i.rule == "navigator.platform")
            .unwrap();
        let fix = platform.fix.as_ref().unwrap();
        assert_eq!(fix.path, "/navigator/platform");
        assert_eq!(fix.action, FixAction::Set);
        assert!(fix.expected.as_ref().unwrap().is_string());
        assert!(!platform.hint.is_empty());
        for i in &r.issues {
            assert!(
                !i.hint.is_empty() && i.fix.as_ref().is_some_and(|f| !f.path.is_empty()),
                "{i:?}"
            );
        }
    }

    #[test]
    fn disabled_rules_are_dropped() {
        let mut p = profile("dis", "firefox");
        p.navigator.platform = "Nope".into();
        p.fonts.clear();
        let o = ValidateOptions {
            disable: vec!["navigator.platform".into(), "fonts.*".into()],
            ..Default::default()
        };
        let r = validate_with(&p, &o).unwrap();
        assert!(r.issues.is_empty(), "{:?}", r.issues);
        assert!(r.valid);
    }

    #[test]
    fn options_reject_oversized_input() {
        let o = ValidateOptions {
            disable: vec!["x".repeat(65)],
            ..Default::default()
        };
        assert!(validate_with(&profile("big", "chrome"), &o).is_err());
        let o = ValidateOptions {
            today: Some("2026-13-01".into()),
            ..Default::default()
        };
        assert!(validate_with(&profile("big", "chrome"), &o).is_err());
    }

    #[test]
    fn generated_profiles_pass_with_their_own_headers_and_location() {
        for (i, browser) in ["chrome", "edge", "firefox", "safari", "samsung"]
            .iter()
            .enumerate()
        {
            let p = profile(&format!("ctx-{i}"), browser);
            let region = p.locale.rsplit('-').next().unwrap().to_owned();
            let o = ValidateOptions {
                headers: Some(crate::navigation_headers(&p)),
                geo: Some(Geo {
                    country: Some(region),
                    timezone: Some(p.timezone.clone()),
                }),
                ..Default::default()
            };
            let r = validate_with(&p, &o).unwrap();
            assert!(r.issues.is_empty(), "{browser}: {:?}", r.issues);
        }
    }

    #[test]
    fn chromium_147_memory_buckets() {
        let mut p = profile("mem", "chrome");
        p.os.id = "windows".into();
        p.navigator.device_memory = Some(32.0);
        let rules: Vec<_> = validate(&p).issues.into_iter().map(|i| i.rule).collect();
        assert!(
            !rules.contains(&"navigator.device_memory".to_owned()),
            "{rules:?}"
        );
        p.browser.major = 140;
        p.navigator.user_agent_data = None;
        let rules: Vec<_> = validate(&p).issues.into_iter().map(|i| i.rule).collect();
        assert!(
            rules.contains(&"navigator.device_memory".to_owned()),
            "{rules:?}"
        );
    }

    #[test]
    fn severity_counts_match_score() {
        let mut p = profile("score", "chrome");
        p.screen.pixel_depth = 30;
        let r = validate(&p);
        let errors = r
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count();
        assert!(errors >= 1 && !r.valid);
    }
}
