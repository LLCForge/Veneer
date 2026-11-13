//! Embeds every file under data/{os,browsers,devices} and src/{inject,collect}, sorted by name,
//! so adding a device family, browser or browser-API patch is just adding a file.
//!
//! The `data-desktop`, `data-mobile` and `data-other` features select which device kinds are embedded
//! (all by default). With a subset, device files are filtered and minified into OUT_DIR.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

const GROUPS: [(&str, &str, &str); 6] = [
    ("EXTRA_DATA_FILES", "data/extra", "json"),
    ("OS_FILES", "data/os", "json"),
    ("BROWSER_FILES", "data/browsers", "json"),
    ("DEVICE_FILES", "data/devices", "json"),
    ("INJECT_PARTS", "src/inject", "js"),
    ("COLLECT_PARTS", "src/collect", "js"),
];

fn files(dir: &Path, ext: &str) -> Result<Vec<PathBuf>, String> {
    let mut list: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| format!("cannot read {}: {e}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == ext))
        .collect();
    list.sort();
    Ok(list)
}

fn main() -> Result<(), String> {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").map_err(|e| e.to_string())?);
    let out_dir = PathBuf::from(env::var("OUT_DIR").map_err(|e| e.to_string())?);
    let out = out_dir.join("embedded.rs");
    let kinds = device_kinds()?;
    let mut code = String::new();
    for (name, dir, ext) in GROUPS {
        let path = manifest.join(dir);
        println!("cargo:rerun-if-changed={}", path.display());
        let _ = writeln!(code, "pub(crate) const {name}: &[(&str, &str)] = &[");
        for file in files(&path, ext)? {
            let stem = file
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| format!("non-UTF-8 file name {}", file.display()))?;
            let embedded = match (&kinds, name) {
                (Some(kinds), "DEVICE_FILES") => match filter_devices(&file, kinds, &out_dir)? {
                    Some(filtered) => filtered,
                    None => continue,
                },
                _ => file.clone(),
            };
            let _ = writeln!(
                code,
                "    ({stem:?}, include_str!({:?})),",
                embedded.display().to_string()
            );
        }
        code.push_str("];\n");
    }
    fs::write(&out, code).map_err(|e| format!("cannot write {}: {e}", out.display()))
}

/// Device kinds to embed, or `None` for all of them.
fn device_kinds() -> Result<Option<Vec<&'static str>>, String> {
    const GROUPS: [(&str, &[&str]); 2] = [
        ("CARGO_FEATURE_DATA_DESKTOP", &["desktop", "laptop"]),
        ("CARGO_FEATURE_DATA_MOBILE", &["phone", "tablet"]),
    ];
    let other = env::var_os("CARGO_FEATURE_DATA_OTHER").is_some();
    let enabled: Vec<&[&str]> = GROUPS
        .iter()
        .filter(|(feature, _)| env::var_os(feature).is_some())
        .map(|(_, kinds)| *kinds)
        .collect();
    if enabled.len() == GROUPS.len() && other {
        return Ok(None);
    }
    if enabled.is_empty() && !other {
        return Err(
            "enable at least one of the features data-desktop, data-mobile, data-other".into(),
        );
    }
    let mut kinds: Vec<&str> = enabled.concat();
    if other {
        // `other` means every kind not named by another group, so new kinds are never silently dropped.
        kinds.push("*other");
    }
    Ok(Some(kinds))
}

/// Keeps devices whose kind is selected; returns the minified file, or `None` if nothing is left.
fn filter_devices(file: &Path, kinds: &[&str], out_dir: &Path) -> Result<Option<PathBuf>, String> {
    const NAMED: [&str; 4] = ["desktop", "laptop", "phone", "tablet"];
    let text =
        fs::read_to_string(file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let devices: Vec<serde_json::Value> =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", file.display()))?;
    let kept: Vec<&serde_json::Value> = devices
        .iter()
        .filter(|d| {
            let kind = d.get("kind").and_then(|k| k.as_str()).unwrap_or_default();
            kinds.contains(&kind) || (kinds.contains(&"*other") && !NAMED.contains(&kind))
        })
        .collect();
    if kept.is_empty() {
        return Ok(None);
    }
    let dir = out_dir.join("devices");
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let name = file
        .file_name()
        .ok_or_else(|| format!("no file name in {}", file.display()))?;
    let target = dir.join(name);
    let json = serde_json::to_string(&kept).map_err(|e| e.to_string())?;
    fs::write(&target, json).map_err(|e| format!("cannot write {}: {e}", target.display()))?;
    Ok(Some(target))
}
