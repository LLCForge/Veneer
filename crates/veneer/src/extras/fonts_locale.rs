//! Locale-specific system fonts. Windows installs a script's supplemental font package (Japanese,
//! Korean, Chinese, Arabic, Hebrew, Thai, ...) when the matching language is added, so a profile's
//! locale implies those families. Data: `data/extra/fonts-locale.json`, `{os id: {locale or
//! language: [family]}}`; an exact locale key wins over its language subtag (`zh-TW` vs `zh-CN`).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::embedded::EXTRA_DATA_FILES;
use crate::model::Profile;
use crate::validate::{Collector, Severity};

const FILE: &str = "fonts-locale";
const MAX_FAMILY: usize = 64;

/// OS id -> locale or language key -> font families.
type Table = BTreeMap<String, BTreeMap<String, Vec<String>>>;

fn table() -> Result<&'static Table, String> {
    static TABLE: OnceLock<Result<Table, String>> = OnceLock::new();
    TABLE
        .get_or_init(|| {
            let text = EXTRA_DATA_FILES
                .iter()
                .find(|(stem, _)| *stem == FILE)
                .map(|(_, text)| *text)
                .ok_or_else(|| format!("data/extra/{FILE}.json is missing"))?;
            let table: Table =
                serde_json::from_str(text).map_err(|e| format!("data/extra/{FILE}.json: {e}"))?;
            let bad = table
                .values()
                .flat_map(BTreeMap::values)
                .flatten()
                .find(|f| f.is_empty() || f.len() > MAX_FAMILY);
            match bad {
                Some(f) => Err(format!("data/extra/{FILE}.json: invalid family `{f}`")),
                None => Ok(table),
            }
        })
        .as_ref()
        .map_err(Clone::clone)
}

/// Families `os` adds for `locale`; empty when the pair has none or the data cannot be read.
pub(crate) fn fonts_for(os: &str, locale: &str) -> &'static [String] {
    let Ok(table) = table() else { return &[] };
    let Some(by_locale) = table.get(os) else {
        return &[];
    };
    let language = locale.split('-').next().unwrap_or_default();
    by_locale
        .get(locale)
        .or_else(|| by_locale.get(language))
        .map_or(&[], Vec::as_slice)
}

/// Add the locale's fonts to the profile's font list (kept sorted and unique).
pub(crate) fn derive(profile: &mut Profile) {
    let extra = fonts_for(&profile.os.id, &profile.locale);
    if extra.is_empty() {
        return;
    }
    profile.fonts.extend(extra.iter().cloned());
    profile.fonts.sort_unstable();
    profile.fonts.dedup();
}

/// Reports unreadable locale-font data. Missing locale fonts are not flagged: a captured profile
/// cannot observe fonts, and a user may remove the supplemental package.
pub(crate) fn validate(c: &mut Collector) {
    if let Err(e) = table() {
        c.push(Severity::Error, "fonts.locale_data", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_parses_and_resolves_keys() {
        let table = table().unwrap();
        assert!(table.contains_key("windows"));
        assert!(fonts_for("windows", "ja-JP").iter().any(|f| f == "Meiryo"));
        assert!(fonts_for("windows", "ja").iter().any(|f| f == "Meiryo"));
        assert!(fonts_for("windows", "zh-TW")
            .iter()
            .any(|f| f == "PMingLiU"));
        assert!(fonts_for("windows", "zh-CN").iter().any(|f| f == "SimHei"));
        assert!(fonts_for("windows", "en-US").is_empty());
        assert!(fonts_for("macos", "ja-JP").is_empty());
    }
}
