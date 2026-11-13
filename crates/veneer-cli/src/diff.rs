//! `veneer diff <a.json> <b.json>`: field-level difference between two profiles (or any JSON).

use std::collections::BTreeMap;
use std::process::ExitCode;

use serde::Serialize;
use serde_json::Value;

use crate::read_input;

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase", tag = "op")]
pub(crate) enum Change {
    Added {
        path: String,
        value: Value,
    },
    Removed {
        path: String,
        value: Value,
    },
    Changed {
        path: String,
        from: Value,
        to: Value,
    },
}

/// Leaf values keyed by JSON pointer (RFC 6901). Empty objects and arrays count as leaves.
fn flatten(value: &Value, path: String, out: &mut BTreeMap<String, Value>) {
    let escape = |k: &str| k.replace('~', "~0").replace('/', "~1");
    match value {
        Value::Object(map) if !map.is_empty() => {
            for (k, v) in map {
                flatten(v, format!("{path}/{}", escape(k)), out);
            }
        }
        Value::Array(items) if !items.is_empty() => {
            for (i, v) in items.iter().enumerate() {
                flatten(v, format!("{path}/{i}"), out);
            }
        }
        leaf => {
            out.insert(path, leaf.clone());
        }
    }
}

pub(crate) fn diff(a: &Value, b: &Value) -> Vec<Change> {
    let (mut left, mut right) = (BTreeMap::new(), BTreeMap::new());
    flatten(a, String::new(), &mut left);
    flatten(b, String::new(), &mut right);
    let mut changes = Vec::new();
    for (path, from) in &left {
        match right.remove(path) {
            Some(to) if &to == from => {}
            Some(to) => changes.push(Change::Changed {
                path: path.clone(),
                from: from.clone(),
                to,
            }),
            None => changes.push(Change::Removed {
                path: path.clone(),
                value: from.clone(),
            }),
        }
    }
    changes.extend(
        right
            .into_iter()
            .map(|(path, value)| Change::Added { path, value }),
    );
    changes.sort_by(|x, y| path_of(x).cmp(path_of(y)));
    changes
}

fn path_of(c: &Change) -> &str {
    match c {
        Change::Added { path, .. }
        | Change::Removed { path, .. }
        | Change::Changed { path, .. } => path,
    }
}

fn read_json(file: &str) -> Result<Value, String> {
    serde_json::from_str(&read_input(file)?).map_err(|e| format!("`{file}` is not JSON: {e}"))
}

/// Exit code 0 when the documents are equal, 1 when they differ (like `diff`).
pub(crate) fn run(a: &str, b: &str, json: bool) -> Result<ExitCode, String> {
    if a == "-" && b == "-" {
        return Err("only one side can be `-` (stdin)".into());
    }
    let changes = diff(&read_json(a)?, &read_json(b)?);
    if json {
        crate::print(&changes, crate::Format::Pretty)?;
    } else if changes.is_empty() {
        println!("no differences");
    } else {
        for c in &changes {
            match c {
                Change::Added { path, value } => println!("+ {path}: {value}"),
                Change::Removed { path, value } => println!("- {path}: {value}"),
                Change::Changed { path, from, to } => println!("~ {path}: {from} -> {to}"),
            }
        }
        println!("{} field(s) differ", changes.len());
    }
    Ok(if changes.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn field_level_changes() {
        let a = json!({ "navigator": { "languages": ["tr-TR", "tr"], "a/b": 1 }, "seed": "x", "gone": true });
        let b =
            json!({ "navigator": { "languages": ["tr-TR"], "a/b": 2 }, "seed": "x", "new": [] });
        let changes = diff(&a, &b);
        assert_eq!(
            changes,
            vec![
                Change::Removed {
                    path: "/gone".into(),
                    value: json!(true)
                },
                Change::Changed {
                    path: "/navigator/a~1b".into(),
                    from: json!(1),
                    to: json!(2)
                },
                Change::Removed {
                    path: "/navigator/languages/1".into(),
                    value: json!("tr")
                },
                Change::Added {
                    path: "/new".into(),
                    value: json!([])
                },
            ]
        );
        assert!(diff(&a, &a).is_empty());
    }
}
