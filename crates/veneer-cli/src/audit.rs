//! `veneer validate`, `veneer capture`, `veneer drift` and `veneer rules`: auditing profiles from
//! Veneer, real browsers (collector output) and other tools.

use std::process::ExitCode;

use clap::{Args, ValueEnum};
use serde_json::{json, Value};
use veneer::{Header, ImportFormat, Imported, Profile, ValidateOptions};

use crate::{print, read_input, Format};

/// Profiles accepted by one `validate` or `drift` run.
const MAX_PROFILES: usize = 100_000;

#[derive(Clone, Copy, Default, ValueEnum)]
pub(crate) enum FormatArg {
    #[default]
    Auto,
    Veneer,
    Collector,
    FingerprintSuite,
    Flat,
}

impl From<FormatArg> for ImportFormat {
    fn from(f: FormatArg) -> Self {
        match f {
            FormatArg::Auto => ImportFormat::Auto,
            FormatArg::Veneer => ImportFormat::Veneer,
            FormatArg::Collector => ImportFormat::Collector,
            FormatArg::FingerprintSuite => ImportFormat::FingerprintSuite,
            FormatArg::Flat => ImportFormat::Flat,
        }
    }
}

#[derive(Clone, Copy, Default, ValueEnum)]
pub(crate) enum OutputArg {
    /// One report, or an array of reports.
    #[default]
    Report,
    /// Profile, missing fields and report for each input.
    Full,
    /// SARIF 2.1.0 for GitHub code scanning.
    Sarif,
}

#[derive(Args, Default)]
pub(crate) struct CheckArgs {
    /// Validator options JSON: { disable, rules, headers, geo, today, maxAgeDays }.
    #[arg(long)]
    options: Option<String>,
    /// Rule id to skip; `fonts.*` skips a group. Repeatable.
    #[arg(long)]
    disable: Vec<String>,
    /// Request headers the browser sent: a HAR file, a [{name, value}] array or a {name: value} object.
    #[arg(long)]
    headers: Option<String>,
    /// Country of the connection (ISO 3166, e.g. TR), as known from your proxy. Never looked up.
    #[arg(long)]
    country: Option<String>,
    /// Timezone of the connection (IANA, e.g. Europe/Istanbul), as known from your proxy.
    #[arg(long)]
    geo_timezone: Option<String>,
    /// Warn about browser releases older than this many days (uses --today or the system date).
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=3650))]
    max_age_days: Option<u32>,
    /// Date for the release-age rule, YYYY-MM-DD.
    #[arg(long)]
    today: Option<String>,
}

impl CheckArgs {
    fn options(self) -> Result<ValidateOptions, String> {
        let mut o: ValidateOptions = match &self.options {
            Some(file) => serde_json::from_str(&read_input(file)?)
                .map_err(|e| format!("invalid options: {e}"))?,
            None => ValidateOptions::default(),
        };
        o.disable.extend(self.disable);
        if let Some(file) = &self.headers {
            o.headers = Some(read_headers(file)?);
        }
        if self.country.is_some() || self.geo_timezone.is_some() {
            let geo = o.geo.get_or_insert_with(Default::default);
            geo.country = self.country.or(geo.country.take());
            geo.timezone = self.geo_timezone.or(geo.timezone.take());
        }
        if self.max_age_days.is_some() {
            o.max_age_days = self.max_age_days;
        }
        o.today = self.today.or(o.today);
        if o.max_age_days.is_some() && o.today.is_none() {
            o.today = Some(system_date()?);
        }
        Ok(o)
    }
}

/// Headers from a HAR log or any JSON form `headers_from_json` accepts.
fn read_headers(file: &str) -> Result<Vec<Header>, String> {
    let value: Value = serde_json::from_str(&read_input(file)?)
        .map_err(|e| format!("invalid headers JSON: {e}"))?;
    if value.pointer("/log/entries").is_some() {
        veneer::headers_from_har(&value).map_err(|e| e.to_string())
    } else {
        veneer::headers_from_json(&value)
            .map(|(h, _)| h)
            .map_err(|e| e.to_string())
    }
}

/// Today's UTC date from the system clock.
fn system_date() -> Result<String, String> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let days = i64::try_from(secs / 86_400).map_err(|e| e.to_string())?;
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    Ok(format!("{y:04}-{m:02}-{d:02}"))
}

/// One JSON value, an array of them, or JSON Lines.
fn read_items(file: &str) -> Result<Vec<Value>, String> {
    let text = read_input(file)?;
    let items = match serde_json::from_str::<Value>(&text) {
        Ok(Value::Array(items)) => items,
        Ok(one) => vec![one],
        Err(_) => text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).map_err(|e| format!("invalid JSON: {e}")))
            .collect::<Result<_, _>>()?,
    };
    if items.len() > MAX_PROFILES {
        return Err(format!("at most {MAX_PROFILES} profiles per run"));
    }
    Ok(items)
}

fn audit(
    imported: Imported,
    options: &ValidateOptions,
) -> Result<(Imported, veneer::Report), String> {
    let mut o = options.clone();
    if o.headers.is_none() && !imported.headers.is_empty() {
        o.headers = Some(imported.headers.clone());
    }
    let report = veneer::validate_with(&imported.profile, &o).map_err(|e| e.to_string())?;
    Ok((imported, report))
}

fn full(imported: &Imported, report: &veneer::Report) -> Value {
    json!({
        "profile": imported.profile,
        "format": imported.format,
        "missing": imported.missing,
        "report": report,
    })
}

pub(crate) fn validate(
    file: &str,
    format: FormatArg,
    output: OutputArg,
    args: CheckArgs,
) -> Result<ExitCode, String> {
    let options = args.options()?;
    let mut results = Vec::new();
    for item in read_items(file)? {
        let imported =
            veneer::import_profile(&item, format.into(), &[]).map_err(|e| e.to_string())?;
        results.push(audit(imported, &options)?);
    }
    let ok = results.iter().all(|(_, r)| r.valid);
    match output {
        OutputArg::Report if results.len() == 1 => print(&results[0].1, Format::Pretty)?,
        OutputArg::Report => print(
            &results.iter().map(|(_, r)| r).collect::<Vec<_>>(),
            Format::Pretty,
        )?,
        OutputArg::Full => print(
            &results.iter().map(|(i, r)| full(i, r)).collect::<Vec<_>>(),
            Format::Pretty,
        )?,
        OutputArg::Sarif => {
            let label = if file == "-" { "stdin.json" } else { file };
            let reports: Vec<(String, veneer::Report)> = results
                .into_iter()
                .map(|(_, r)| (label.to_owned(), r))
                .collect();
            print(&veneer::to_sarif(&reports, &options.rules), Format::Pretty)?;
        }
    }
    Ok(if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Collector output (from `page.evaluate(collectorScript())`) to a full profile and its report.
pub(crate) fn capture(file: &str, args: CheckArgs) -> Result<ExitCode, String> {
    let options = args.options()?;
    let observed: Value =
        serde_json::from_str(&read_input(file)?).map_err(|e| format!("invalid JSON: {e}"))?;
    let headers = options.headers.clone().unwrap_or_default();
    let imported = veneer::capture(&observed, &headers).map_err(|e| e.to_string())?;
    let (imported, report) = audit(imported, &options)?;
    print(&full(&imported, &report), Format::Pretty)?;
    Ok(ExitCode::SUCCESS)
}

/// Share of OS, browser, engine, device class and locale against the dataset distribution.
pub(crate) fn drift(file: &str, format: FormatArg) -> Result<ExitCode, String> {
    let profiles: Vec<Profile> = read_items(file)?
        .iter()
        .map(|v| {
            veneer::import_profile(v, format.into(), &[])
                .map(|i| i.profile)
                .map_err(|e| e.to_string())
        })
        .collect::<Result<_, _>>()?;
    let report = veneer::drift(&profiles).map_err(|e| e.to_string())?;
    let drifted = report.dimensions.iter().any(|d| d.drifted);
    print(&report, Format::Pretty)?;
    Ok(if drifted {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

/// Rule list with the fields each rule protects, including custom rules from `--options`.
pub(crate) fn rules(options: Option<String>) -> Result<ExitCode, String> {
    let custom = match options {
        Some(file) => {
            serde_json::from_str::<ValidateOptions>(&read_input(&file)?)
                .map_err(|e| format!("invalid options: {e}"))?
                .rules
        }
        None => Vec::new(),
    };
    print(
        &veneer::rule_coverage(&custom).map_err(|e| e.to_string())?,
        Format::Pretty,
    )?;
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    #[test]
    fn system_date_is_well_formed() {
        let d = super::system_date().unwrap();
        assert_eq!(d.len(), 10);
        assert!(d.starts_with("20"));
    }
}
