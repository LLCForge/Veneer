//! Runtime dataset extension. Its own test binary, because extending changes the process-wide dataset.

use serde_json::Value;
use veneer::{
    catalog, extend_dataset, extend_dataset_dir, generate, validate, Constraints, DataFile,
};

/// A copy of a real device under a new id: only the id is new, so every value stays a real one.
fn copied_device(id: &str) -> String {
    let devices: Vec<Value> =
        serde_json::from_str(include_str!("../data/devices/iphone.json")).unwrap();
    let mut device = devices[0].clone();
    device["id"] = Value::String(id.into());
    serde_json::to_string(&vec![device]).unwrap()
}

#[test]
fn extensions_merge_validate_and_are_used() {
    // Rejected input leaves the dataset unchanged.
    let before = catalog().unwrap().devices.len();
    assert!(extend_dataset(&[DataFile {
        path: "devices/a.json".into(),
        text: copied_device("dup").replace("\"ios\"", "\"no-such-os\""),
    }])
    .is_err());
    assert!(extend_dataset(&[
        DataFile {
            path: "devices/a.json".into(),
            text: copied_device("x"),
        },
        DataFile {
            path: "devices/a.json".into(),
            text: copied_device("y"),
        },
    ])
    .is_err());
    assert_eq!(catalog().unwrap().devices.len(), before);

    // In-memory files.
    extend_dataset(&[DataFile {
        path: "devices/extension-test.json".into(),
        text: copied_device("extension-phone-a"),
    }])
    .unwrap();
    assert_eq!(catalog().unwrap().devices.len(), before + 1);

    // A directory; the device with an existing id replaces it instead of duplicating.
    let dir = std::env::temp_dir().join(format!("veneer-extend-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("devices")).unwrap();
    std::fs::write(
        dir.join("devices").join("more.json"),
        copied_device("extension-phone-b"),
    )
    .unwrap();
    std::fs::write(
        dir.join("devices").join("again.json"),
        copied_device("extension-phone-a"),
    )
    .unwrap();
    std::fs::write(dir.join("README.md"), "ignored").unwrap();
    let merged = extend_dataset_dir(&dir);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(merged.unwrap(), 2);
    assert_eq!(catalog().unwrap().devices.len(), before + 2);

    for id in ["extension-phone-a", "extension-phone-b"] {
        for i in 0..30 {
            let p = generate(&Constraints {
                device: Some(id.into()),
                seed: Some(format!("{id}-{i}")),
                ..Default::default()
            })
            .unwrap();
            assert_eq!(p.device.id, id);
            let report = validate(&p);
            assert!(report.issues.is_empty(), "{:#?}", report.issues);
        }
    }
    assert!(extend_dataset_dir(std::path::Path::new("definitely/not/here")).is_err());
}
