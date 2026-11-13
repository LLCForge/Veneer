//! What the embedded dataset can produce: valid ids for every constraint.

use serde::Serialize;

use crate::data::{dataset, Dataset};
use crate::generate::browser_versions;
use crate::model::Constraints;
use crate::model::{Engine, Kind};
use crate::Error;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    /// Distinct consistent configurations the dataset can produce (before per-seed canvas/audio noise).
    pub combinations: u64,
    pub os: Vec<CatalogOs>,
    pub browsers: Vec<CatalogBrowser>,
    pub devices: Vec<CatalogDevice>,
    pub locales: Vec<CatalogLocale>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogOs {
    pub id: String,
    pub name: String,
    pub versions: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogBrowser {
    pub id: String,
    pub name: String,
    pub engine: Engine,
    pub versions: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogDevice {
    pub id: String,
    pub os: String,
    pub kind: Kind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub os_versions: Vec<String>,
    pub browsers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogLocale {
    pub locale: String,
    pub timezones: Vec<String>,
}

/// Every OS, browser, device and locale the generator knows.
pub fn catalog() -> Result<Catalog, Error> {
    let data = dataset()?;
    Ok(Catalog {
        combinations: combinations(data),
        os: data
            .os
            .iter()
            .map(|(id, os)| CatalogOs {
                id: id.clone(),
                name: os.name.clone(),
                versions: os.versions.iter().map(|v| v.name.clone()).collect(),
            })
            .collect(),
        browsers: data
            .browsers
            .iter()
            .map(|(id, b)| CatalogBrowser {
                id: id.clone(),
                name: b.name.clone(),
                engine: b.engine,
                versions: b.versions.iter().map(|v| v.full.clone()).collect(),
            })
            .collect(),
        devices: data
            .devices
            .iter()
            .map(|d| CatalogDevice {
                id: d.id.clone(),
                os: d.os.clone(),
                kind: d.kind,
                model: d.model.clone(),
                os_versions: data.os[&d.os]
                    .versions
                    .iter()
                    .filter(|v| d.runs(v))
                    .map(|v| v.name.clone())
                    .collect(),
                browsers: d.browsers.keys().cloned().collect(),
            })
            .collect(),
        locales: data
            .locales
            .iter()
            .map(|l| CatalogLocale {
                locale: l.locale.clone(),
                timezones: l.timezones.iter().map(|(t, _)| t.clone()).collect(),
            })
            .collect(),
    })
}

/// Every device x browser x OS version x browser version x screen x cores x memory x (locale, timezone).
fn combinations(data: &Dataset) -> u64 {
    let any = Constraints::default();
    let locale_timezones: u64 = data.locales.iter().map(|l| l.timezones.len() as u64).sum();
    let mut total = 0u64;
    for d in &data.devices {
        let hardware = (d.screens.len() * d.cores.len() * d.memory.len()) as u64;
        let os = &data.os[&d.os];
        for browser_id in d.browsers.keys() {
            let browser = &data.browsers[browser_id];
            for v in os
                .versions
                .iter()
                .filter(|v| d.runs(v) && v.supports(browser_id))
            {
                let versions = browser_versions(browser, v, browser_id, &any).len() as u64;
                total = total.saturating_add(versions * hardware * locale_timezones);
            }
        }
    }
    total
}
