//! `veneer devices [--os <id>]`: the device catalog as a readable table.

use veneer::CatalogDevice;

fn version_span(versions: &[String]) -> String {
    match versions {
        [] => "-".into(),
        [only] => only.clone(),
        [first, .., last] => format!("{first} .. {last} ({})", versions.len()),
    }
}

pub(crate) fn table(devices: &[&CatalogDevice]) -> String {
    let rows: Vec<[String; 6]> = devices
        .iter()
        .map(|d| {
            [
                d.id.clone(),
                serde_json::to_value(d.kind)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default(),
                d.os.clone(),
                d.model.clone().unwrap_or_else(|| "-".into()),
                version_span(&d.os_versions),
                d.browsers.join(", "),
            ]
        })
        .collect();
    let head = ["DEVICE", "KIND", "OS", "MODEL", "OS VERSIONS", "BROWSERS"].map(String::from);
    let mut widths = head.clone().map(|h| h.chars().count());
    for row in &rows {
        for (w, cell) in widths.iter_mut().zip(row) {
            *w = (*w).max(cell.chars().count());
        }
    }
    std::iter::once(&head)
        .chain(&rows)
        .map(|row| {
            row.iter()
                .zip(widths)
                .map(|(cell, w)| format!("{cell:<w$}"))
                .collect::<Vec<_>>()
                .join("  ")
                .trim_end()
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn run(os: Option<&str>) -> Result<(), String> {
    let catalog = veneer::catalog().map_err(|e| e.to_string())?;
    if let Some(os) = os {
        if !catalog.os.iter().any(|o| o.id == os) {
            let ids: Vec<&str> = catalog.os.iter().map(|o| o.id.as_str()).collect();
            return Err(format!(
                "unknown os `{os}`; expected one of {}",
                ids.join(", ")
            ));
        }
    }
    let devices: Vec<&CatalogDevice> = catalog
        .devices
        .iter()
        .filter(|d| os.is_none_or(|o| d.os == o))
        .collect();
    println!("{}", table(&devices));
    println!("\n{} device(s)", devices.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_lists_every_device() {
        let catalog = veneer::catalog().expect("catalog");
        let devices: Vec<&CatalogDevice> = catalog.devices.iter().collect();
        let out = table(&devices);
        assert_eq!(out.lines().count(), devices.len() + 1);
        assert!(out.starts_with("DEVICE"));
        assert!(out.contains("iphone"));
    }
}
