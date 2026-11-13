//! `veneer pack <dir>`: validate a custom dataset directory (same layout as `crates/veneer/data`)
//! and bundle it into one JSON file.
//!
//! Layout: `locales.json`, `os/<id>.json`, `browsers/<id>.json`, `devices/<family>.json` (array) and
//! optional `extra/<topic>.json`. This checks structure and cross-file references; the full
//! semantic check runs when the library loads the data.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde_json::{json, Map, Value};

const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
const MAX_FILES: usize = 2000;

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

struct Reader {
    total: u64,
    files: usize,
}

impl Reader {
    fn read(&mut self, path: &Path) -> Result<Value, String> {
        let meta = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if !meta.is_file() {
            return Err(format!("{}: not a regular file", path.display()));
        }
        if meta.len() > MAX_FILE_BYTES {
            return Err(format!(
                "{}: larger than {MAX_FILE_BYTES} bytes",
                path.display()
            ));
        }
        self.total += meta.len();
        self.files += 1;
        if self.total > MAX_TOTAL_BYTES || self.files > MAX_FILES {
            return Err(format!(
                "dataset exceeds {MAX_FILES} files or {MAX_TOTAL_BYTES} bytes"
            ));
        }
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// `<dir>/<sub>/*.json` keyed by file stem; missing optional directories give an empty map.
    fn group(
        &mut self,
        dir: &Path,
        sub: &str,
        required: bool,
    ) -> Result<Map<String, Value>, String> {
        let path = dir.join(sub);
        if !path.is_dir() {
            return if required {
                Err(format!("missing directory {}", path.display()))
            } else {
                Ok(Map::new())
            };
        }
        let mut files: Vec<_> = fs::read_dir(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        files.sort();
        let mut out = Map::new();
        for file in files {
            let stem = file
                .file_stem()
                .and_then(|s| s.to_str())
                .filter(|s| valid_id(s))
                .ok_or_else(|| {
                    format!(
                        "{}: file names must be lowercase a-z, 0-9 and '-'",
                        file.display()
                    )
                })?
                .to_owned();
            out.insert(stem, self.read(&file)?);
        }
        if required && out.is_empty() {
            return Err(format!("{} has no .json files", path.display()));
        }
        Ok(out)
    }
}

/// Read, check and bundle a dataset directory.
pub(crate) fn pack(dir: &Path) -> Result<Value, String> {
    let mut r = Reader { total: 0, files: 0 };
    let locales = r.read(&dir.join("locales.json"))?;
    let os = r.group(dir, "os", true)?;
    let browsers = r.group(dir, "browsers", true)?;
    let devices = r.group(dir, "devices", true)?;
    let extra = r.group(dir, "extra", false)?;

    let mut errors = Vec::new();
    match locales.as_array() {
        Some(list) if !list.is_empty() => {
            for (i, l) in list.iter().enumerate() {
                if l.get("locale").and_then(Value::as_str).is_none() {
                    errors.push(format!("locales.json[{i}]: missing \"locale\""));
                }
            }
        }
        _ => errors.push("locales.json: expected a non-empty array".into()),
    }
    for (id, def) in &os {
        if !def.is_object() {
            errors.push(format!("os/{id}.json: expected an object"));
        }
    }
    for (id, def) in &browsers {
        match def.get("ua").and_then(Value::as_object) {
            Some(ua) => errors.extend(
                ua.keys()
                    .filter(|o| !os.contains_key(*o))
                    .map(|o| format!("browsers/{id}.json: ua template for unknown os `{o}`")),
            ),
            None => errors.push(format!("browsers/{id}.json: missing \"ua\" object")),
        }
    }
    let mut ids = BTreeSet::new();
    for (family, list) in &devices {
        let Some(list) = list.as_array() else {
            errors.push(format!("devices/{family}.json: expected an array"));
            continue;
        };
        for (i, d) in list.iter().enumerate() {
            let at = format!("devices/{family}.json[{i}]");
            match d.get("id").and_then(Value::as_str) {
                Some(id) if !ids.insert(id.to_owned()) => {
                    errors.push(format!("{at}: duplicate device id `{id}`"));
                }
                Some(_) => {}
                None => errors.push(format!("{at}: missing \"id\"")),
            }
            match d.get("os").and_then(Value::as_str) {
                Some(o) if !os.contains_key(o) => errors.push(format!("{at}: unknown os `{o}`")),
                Some(_) => {}
                None => errors.push(format!("{at}: missing \"os\"")),
            }
            if let Some(b) = d.get("browsers").and_then(Value::as_object) {
                errors.extend(
                    b.keys()
                        .filter(|b| !browsers.contains_key(*b))
                        .map(|b| format!("{at}: unknown browser `{b}`")),
                );
            }
        }
    }
    if !errors.is_empty() {
        return Err(format!(
            "{} problem(s):\n  {}",
            errors.len(),
            errors.join("\n  ")
        ));
    }
    let counts: BTreeMap<&str, usize> = [
        ("os", os.len()),
        ("browsers", browsers.len()),
        ("devices", ids.len()),
        ("locales", locales.as_array().map_or(0, Vec::len)),
        ("extra", extra.len()),
    ]
    .into();
    Ok(json!({
        "format": "veneer-dataset",
        "version": 1,
        "counts": counts,
        "locales": locales,
        "os": os,
        "browsers": browsers,
        "devices": devices,
        "extra": extra,
    }))
}

pub(crate) fn run(dir: &str, out: Option<&str>) -> Result<(), String> {
    let bundle = pack(Path::new(dir))?;
    let text = serde_json::to_string(&bundle).map_err(|e| e.to_string())?;
    match out {
        Some(file) => {
            fs::write(file, text).map_err(|e| format!("cannot write `{file}`: {e}"))?;
            eprintln!("wrote {file}: {}", bundle["counts"]);
        }
        None => println!("{text}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_dataset_packs_cleanly() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../veneer/data");
        let bundle = pack(&dir).expect("embedded dataset is valid");
        assert!(bundle["counts"]["devices"].as_u64().unwrap_or(0) > 50);
    }

    #[test]
    fn broken_references_are_reported() {
        let dir = std::env::temp_dir().join(format!("veneer-pack-test-{}", std::process::id()));
        for sub in ["os", "browsers", "devices"] {
            fs::create_dir_all(dir.join(sub)).expect("dir");
        }
        fs::write(dir.join("locales.json"), r#"[{"locale":"en-US"}]"#).expect("write");
        fs::write(dir.join("os/windows.json"), "{}").expect("write");
        fs::write(dir.join("browsers/chrome.json"), r#"{"ua":{"macos":"x"}}"#).expect("write");
        fs::write(
            dir.join("devices/pc.json"),
            r#"[{"id":"a","os":"windows","browsers":{"edge":1}},{"id":"a","os":"linux"}]"#,
        )
        .expect("write");
        let err = pack(&dir).expect_err("invalid");
        for needle in [
            "unknown os `macos`",
            "unknown browser `edge`",
            "duplicate device id",
            "unknown os `linux`",
        ] {
            assert!(err.contains(needle), "{needle} in {err}");
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
