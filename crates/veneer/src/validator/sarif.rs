//! SARIF 2.1.0 output, for GitHub code scanning and other SARIF viewers.

use serde_json::{json, Value};

use super::custom::CustomRule;
use crate::validate::{Report, Severity};

fn level(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

/// One SARIF run for any number of reports. `artifact` is the file (or label) each profile came
/// from; `custom` adds descriptions for custom rules.
pub fn to_sarif(reports: &[(String, Report)], custom: &[CustomRule]) -> Value {
    let mut rules: Vec<Value> = super::rules::builtin()
        .map(|(id, severity, hint)| rule(id, severity, hint))
        .chain(
            custom
                .iter()
                .map(|r| rule(&r.id, r.severity, r.hint.as_deref().unwrap_or(&r.message))),
        )
        .collect();
    let mut results = Vec::new();
    for (index, (artifact, report)) in reports.iter().enumerate() {
        for issue in &report.issues {
            if !rules.iter().any(|r| r["id"] == issue.rule.as_str()) {
                rules.push(rule(&issue.rule, issue.severity, &issue.hint));
            }
            let path = issue.fix.as_ref().map_or("", |f| f.path.as_str());
            results.push(json!({
                "ruleId": issue.rule,
                "level": level(issue.severity),
                "message": { "text": format!("{} {}", issue.message, issue.hint).trim().to_owned() },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": artifact },
                        "region": { "startLine": 1 }
                    },
                    "logicalLocations": [{ "fullyQualifiedName": format!("profile[{index}]{path}"), "kind": "member" }]
                }],
                "properties": { "fix": issue.fix }
            }));
        }
    }
    json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": { "driver": {
                "name": "veneer",
                "version": env!("CARGO_PKG_VERSION"),
                "informationUri": "https://github.com/LLCForge/Veneer",
                "rules": rules,
                "properties": { "rulesVersion": super::RULES_VERSION }
            }},
            "results": results
        }]
    })
}

fn rule(id: &str, severity: Severity, hint: &str) -> Value {
    json!({
        "id": id,
        "shortDescription": { "text": id },
        "fullDescription": { "text": hint },
        "defaultConfiguration": { "level": level(severity) }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate, validate, Constraints};

    #[test]
    fn sarif_lists_every_issue() {
        let mut p = generate(&Constraints {
            seed: Some("sarif".into()),
            ..Default::default()
        })
        .unwrap();
        p.navigator.vendor = "x".into();
        let r = validate(&p);
        let s = to_sarif(&[("profiles.json".into(), r.clone())], &[]);
        assert_eq!(s["version"], "2.1.0");
        let results = s["runs"][0]["results"].as_array().unwrap();
        assert_eq!(results.len(), r.issues.len());
        assert_eq!(results[0]["ruleId"], "navigator.vendor");
        assert_eq!(results[0]["level"], "error");
        let rules = s["runs"][0]["tool"]["driver"]["rules"].as_array().unwrap();
        assert!(rules.iter().any(|r| r["id"] == "navigator.vendor"));
    }
}
