//! Rules declared in JSON. A rule fires when `when` holds (or is absent) and `require` does not.
//!
//! ```json
//! { "id": "custom.windows-chrome-only", "severity": "error",
//!   "message": "only Chrome on Windows is allowed",
//!   "when": { "path": "/os/id", "op": "eq", "value": "windows" },
//!   "require": { "path": "/browser/id", "op": "in", "value": ["chrome", "edge"] } }
//! ```
//!
//! Conditions: `{ "all": [..] }`, `{ "any": [..] }`, `{ "not": {..} }` or a test
//! `{ "path": "<JSON pointer into the profile>", "op": "<op>", "value": <JSON> }`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::model::Profile;
use crate::validate::{Collector, Fix, FixAction, Issue, Severity};
use crate::Error;

const MAX_RULES: usize = 100;
const MAX_DEPTH: usize = 8;
const MAX_NODES: usize = 64;
const MAX_TEXT: usize = 512;
const MAX_VALUE: usize = 4096;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CustomRule {
    /// Must start with `custom.`; letters, digits, `.`, `_` and `-`.
    pub id: String,
    pub severity: Severity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<Condition>,
    pub require: Condition,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Condition {
    All(AllOf),
    Any(AnyOf),
    Not(NotOf),
    Test(Test),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllOf {
    pub all: Vec<Condition>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnyOf {
    pub any: Vec<Condition>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotOf {
    pub not: Box<Condition>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Test {
    /// JSON pointer into the profile JSON, e.g. `/navigator/hardwareConcurrency`.
    pub path: String,
    pub op: Op,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub value: Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Op {
    Eq,
    Ne,
    /// `value` is an array; the field equals one of its items.
    In,
    NotIn,
    Lt,
    Lte,
    Gt,
    Gte,
    /// The field exists and is not `null`.
    Exists,
    /// The field is missing or `null`.
    Absent,
    /// Substring of a string field, or item of an array field.
    Contains,
    StartsWith,
    EndsWith,
}

impl CustomRule {
    /// Every path the rule reads.
    pub(crate) fn paths(&self) -> Vec<String> {
        let mut out = Vec::new();
        for c in self.when.iter().chain(std::iter::once(&self.require)) {
            c.walk(&mut |t| {
                if !out.contains(&t.path) {
                    out.push(t.path.clone());
                }
            });
        }
        out
    }
}

impl Condition {
    fn walk(&self, f: &mut impl FnMut(&Test)) {
        match self {
            Condition::All(a) => a.all.iter().for_each(|c| c.walk(f)),
            Condition::Any(a) => a.any.iter().for_each(|c| c.walk(f)),
            Condition::Not(n) => n.not.walk(f),
            Condition::Test(t) => f(t),
        }
    }

    /// (depth, nodes); used to bound rule size.
    fn size(&self) -> (usize, usize) {
        let kids: Vec<&Condition> = match self {
            Condition::All(a) => a.all.iter().collect(),
            Condition::Any(a) => a.any.iter().collect(),
            Condition::Not(n) => vec![&n.not],
            Condition::Test(_) => Vec::new(),
        };
        kids.iter()
            .map(|c| c.size())
            .fold((1, 1), |(d, n), (cd, cn)| (d.max(cd + 1), n + cn))
    }

    fn eval(&self, doc: &Value) -> bool {
        match self {
            Condition::All(a) => a.all.iter().all(|c| c.eval(doc)),
            Condition::Any(a) => a.any.iter().any(|c| c.eval(doc)),
            Condition::Not(n) => !n.not.eval(doc),
            Condition::Test(t) => t.eval(doc),
        }
    }

    fn first_test(&self) -> Option<&Test> {
        match self {
            Condition::Test(t) => Some(t),
            Condition::All(a) => a.all.iter().find_map(Condition::first_test),
            Condition::Any(a) => a.any.iter().find_map(Condition::first_test),
            Condition::Not(n) => n.not.first_test(),
        }
    }
}

impl Test {
    fn eval(&self, doc: &Value) -> bool {
        let actual = doc.pointer(&self.path).filter(|v| !v.is_null());
        let num = |v: &Value| v.as_f64();
        let cmp = |f: fn(f64, f64) -> bool| match (actual.and_then(num), num(&self.value)) {
            (Some(a), Some(b)) => f(a, b),
            _ => false,
        };
        let text =
            |f: fn(&str, &str) -> bool| match (actual.and_then(Value::as_str), self.value.as_str())
            {
                (Some(a), Some(b)) => f(a, b),
                _ => false,
            };
        let one_of = || {
            self.value
                .as_array()
                .is_some_and(|list| actual.is_some_and(|a| list.iter().any(|v| json_eq(a, v))))
        };
        match self.op {
            Op::Eq => actual.is_some_and(|a| json_eq(a, &self.value)),
            Op::Ne => !actual.is_some_and(|a| json_eq(a, &self.value)),
            Op::In => one_of(),
            Op::NotIn => !one_of(),
            Op::Lt => cmp(|a, b| a < b),
            Op::Lte => cmp(|a, b| a <= b),
            Op::Gt => cmp(|a, b| a > b),
            Op::Gte => cmp(|a, b| a >= b),
            Op::Exists => actual.is_some(),
            Op::Absent => actual.is_none(),
            Op::Contains => match actual {
                Some(Value::Array(items)) => items.iter().any(|i| json_eq(i, &self.value)),
                _ => text(|a, b| a.contains(b)),
            },
            Op::StartsWith => text(|a, b| a.starts_with(b)),
            Op::EndsWith => text(|a, b| a.ends_with(b)),
        }
    }
}

/// JSON equality that treats `1` and `1.0` as the same number.
fn json_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| json_eq(a, b))
        }
        _ => a == b,
    }
}

pub(crate) fn check_rules(rules: &[CustomRule]) -> Result<(), Error> {
    let bad = |m: String| Err(Error::InvalidConstraint(m));
    if rules.len() > MAX_RULES {
        return bad(format!("at most {MAX_RULES} custom rules"));
    }
    for r in rules {
        let id_ok = r.id.starts_with("custom.")
            && r.id.len() <= 64
            && r.id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b));
        if !id_ok {
            return bad(format!(
                "custom rule id `{}` must start with `custom.` and use at most 64 of A-Z a-z 0-9 . _ -",
                r.id.chars().take(64).collect::<String>()
            ));
        }
        if r.message.len() > MAX_TEXT || r.hint.as_ref().is_some_and(|h| h.len() > MAX_TEXT) {
            return bad(format!(
                "`{}`: message and hint are limited to {MAX_TEXT} bytes",
                r.id
            ));
        }
        for c in r.when.iter().chain(std::iter::once(&r.require)) {
            let (depth, nodes) = c.size();
            if depth > MAX_DEPTH || nodes > MAX_NODES {
                return bad(format!(
                    "`{}`: conditions are limited to depth {MAX_DEPTH} and {MAX_NODES} nodes",
                    r.id
                ));
            }
            let mut err = None;
            c.walk(&mut |t| {
                let size = serde_json::to_string(&t.value).map_or(usize::MAX, |s| s.len());
                if (!t.path.is_empty() && !t.path.starts_with('/')) || t.path.len() > 256 {
                    err = Some("paths are JSON pointers like /navigator/platform (max 256 bytes)");
                } else if size > MAX_VALUE {
                    err = Some("test values are limited to 4096 bytes of JSON");
                } else if matches!(t.op, Op::In | Op::NotIn) && !t.value.is_array() {
                    err = Some("`in` and `notIn` need an array value");
                }
            });
            if let Some(e) = err {
                return bad(format!("`{}`: {e}", r.id));
            }
        }
    }
    Ok(())
}

pub(crate) fn run(p: &Profile, rules: &[CustomRule], c: &mut Collector) -> Result<(), Error> {
    if rules.is_empty() {
        return Ok(());
    }
    let doc = serde_json::to_value(p)?;
    for r in rules {
        if r.when.as_ref().is_some_and(|w| !w.eval(&doc)) || r.require.eval(&doc) {
            continue;
        }
        let test = r.require.first_test();
        let set = test.filter(|t| t.op == Op::Eq && matches!(r.require, Condition::Test(_)));
        c.push_issue(Issue {
            rule: r.id.clone(),
            severity: r.severity,
            message: r.message.clone(),
            hint: r.hint.clone().unwrap_or_else(|| "Custom rule.".into()),
            fix: Some(Fix {
                path: test.map_or_else(String::new, |t| t.path.clone()),
                action: if set.is_some() {
                    FixAction::Set
                } else {
                    FixAction::Review
                },
                expected: set.map(|t| t.value.clone()),
            }),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate, validate_with, Constraints, ValidateOptions};

    fn rule(json: &str) -> CustomRule {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn json_rules_fire_and_carry_fixes() {
        let p = generate(&Constraints {
            os: Some("windows".into()),
            browser: Some("firefox".into()),
            seed: Some("custom".into()),
            ..Default::default()
        })
        .unwrap();
        let rules = vec![
            rule(
                r#"{"id":"custom.chromium-only","severity":"error","message":"Chromium only",
                "require":{"path":"/browser/engine","op":"eq","value":"chromium"}}"#,
            ),
            rule(
                r#"{"id":"custom.cores","severity":"warning","message":"few cores",
                "when":{"all":[{"path":"/os/id","op":"in","value":["windows","macos"]}]},
                "require":{"not":{"path":"/navigator/hardwareConcurrency","op":"lt","value":0}}}"#,
            ),
            rule(
                r#"{"id":"custom.ua","severity":"warning","message":"ua",
                "require":{"any":[{"path":"/navigator/userAgent","op":"contains","value":"Firefox/"},
                                  {"path":"/navigator/oscpu","op":"absent"}]}}"#,
            ),
        ];
        let r = validate_with(
            &p,
            &ValidateOptions {
                rules,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r.issues.len(), 1, "{:?}", r.issues);
        let i = &r.issues[0];
        assert_eq!(i.rule, "custom.chromium-only");
        let fix = i.fix.as_ref().unwrap();
        assert_eq!(
            (fix.path.as_str(), fix.action),
            ("/browser/engine", FixAction::Set)
        );
        assert!(!r.valid);
    }

    #[test]
    fn bad_rules_are_rejected() {
        let bad_id = rule(
            r#"{"id":"x","severity":"error","message":"m","require":{"path":"/a","op":"exists"}}"#,
        );
        assert!(check_rules(&[bad_id]).is_err());
        let bad_in = rule(
            r#"{"id":"custom.x","severity":"error","message":"m","require":{"path":"/a","op":"in","value":1}}"#,
        );
        assert!(check_rules(&[bad_in]).is_err());
        let mut deep = r#"{"path":"/a","op":"exists"}"#.to_owned();
        for _ in 0..10 {
            deep = format!(r#"{{"not":{deep}}}"#);
        }
        let deep = rule(&format!(
            r#"{{"id":"custom.d","severity":"error","message":"m","require":{deep}}}"#
        ));
        assert!(check_rules(&[deep]).is_err());
        assert!(serde_json::from_str::<CustomRule>(
            r#"{"id":"custom.x","severity":"error","message":"m","require":{"path":"/a","op":"exists","extra":1}}"#
        )
        .is_err());
    }
}
