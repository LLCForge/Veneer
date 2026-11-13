//! `veneer` command-line tool: generate, validate, export scripts, list the catalog, or serve the HTTP API.

mod audit;
mod bench;
mod devices;
mod diff;
mod net;
mod pack;
mod pool;
mod server;

use std::io::Read;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Serialize;
use veneer::{Constraints, Engine, Kind, Profile};

#[derive(Parser)]
#[command(
    name = "veneer",
    version,
    about = "Coherent browser fingerprints for Playwright and Puppeteer"
)]
struct Cli {
    /// Extra dataset directory (os/, browsers/, devices/, locales.json) merged into the embedded data.
    #[arg(long, global = true, env = "VENEER_DATA")]
    data: Option<std::path::PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate fingerprints as JSON.
    Generate {
        #[command(flatten)]
        constraints: ConstraintArgs,
        /// Number of fingerprints (1-1000). With a seed, seeds become `<seed>-<n>`.
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..=1000))]
        count: u32,
        /// Also include navigation headers, the init script and the profile id.
        #[arg(long)]
        full: bool,
        #[arg(long, value_enum, default_value_t = Format::Pretty)]
        format: Format,
        /// Never repeat a (device, browser, version, screen, locale) within the batch.
        #[arg(long)]
        unique: bool,
        /// Regenerate the exact profile behind a profile id (from `--full` output or the API).
        #[arg(long, conflicts_with_all = ["unique", "count"])]
        id: Option<String>,
    },
    /// Measure generator throughput (profiles per second) and dataset load time on this machine.
    Bench {
        #[command(flatten)]
        constraints: ConstraintArgs,
        /// Profiles per measurement.
        #[arg(long, default_value_t = 20_000, value_parser = clap::value_parser!(u32).range(1..=1_000_000))]
        count: u32,
    },
    /// Validate profiles (one object, an array or JSON Lines) from a file or `-` for stdin: Veneer
    /// profiles, collector output, fingerprint-suite output or flat JSON. Exits 1 if any is invalid.
    Validate {
        #[arg(default_value = "-")]
        file: String,
        #[arg(long, value_enum, default_value_t = audit::FormatArg::Auto)]
        format: audit::FormatArg,
        #[arg(long, value_enum, default_value_t = audit::OutputArg::Report)]
        output: audit::OutputArg,
        #[command(flatten)]
        check: audit::CheckArgs,
    },
    /// Build a full profile from collector output of a real browser (`veneer collector` in the page)
    /// and audit it.
    Capture {
        #[arg(default_value = "-")]
        file: String,
        #[command(flatten)]
        check: audit::CheckArgs,
    },
    /// Compare a batch of profiles with the dataset's distribution. Exits 1 when a dimension drifted.
    Drift {
        #[arg(default_value = "-")]
        file: String,
        #[arg(long, value_enum, default_value_t = audit::FormatArg::Auto)]
        format: audit::FormatArg,
    },
    /// List every validator rule and the profile fields it protects.
    Rules {
        /// Validator options JSON whose custom rules are included.
        #[arg(long)]
        options: Option<String>,
    },
    /// Print the init script for a new profile, or for `--profile <file>`.
    Script {
        #[command(flatten)]
        constraints: ConstraintArgs,
        #[arg(long)]
        profile: Option<String>,
    },
    /// List every OS, browser, device and locale the generator knows.
    Catalog,
    /// Print the in-page collector expression (for page.evaluate in any automation tool).
    Collector,
    /// Run the local HTTP API and web UI.
    Serve {
        #[arg(long, default_value = "127.0.0.1", env = "VENEER_HOST")]
        host: String,
        #[arg(long, default_value_t = 8787, env = "VENEER_PORT")]
        port: u16,
        /// Bearer token required for /v1 endpoints. Mandatory when binding to a non-loopback address.
        #[arg(long, env = "VENEER_TOKEN", hide_env_values = true)]
        token: Option<String>,
    },
    /// Print navigation request headers in browser order for non-browser HTTP clients.
    Headers {
        #[command(flatten)]
        constraints: ConstraintArgs,
        /// Use this profile JSON file (or `-` for stdin) instead of generating one.
        #[arg(long)]
        profile: Option<String>,
        #[arg(long, value_enum, default_value_t = net::HeaderFormat::Json)]
        format: net::HeaderFormat,
        /// Target URL for the curl, Python and Node snippets.
        #[arg(long, default_value = "https://example.com/")]
        url: String,
    },
    /// Compare captured request headers (JSON array or object) with a profile. Exits 1 on mismatch.
    CheckHeaders {
        /// Profile JSON file, or `-` for stdin.
        #[arg(long)]
        profile: String,
        /// Captured headers JSON file, or `-` for stdin.
        headers: String,
    },
    /// Network-layer checks and settings: JA4, proxy location, WebRTC.
    #[command(subcommand)]
    Net(net::NetCommand),
    /// Field-level difference between two profile JSON files. Exits 1 when they differ.
    Diff {
        a: String,
        b: String,
        /// Print the changes as JSON.
        #[arg(long)]
        json: bool,
    },
    /// List known devices as a table.
    Devices {
        /// Only devices of this OS id, e.g. ios.
        #[arg(long)]
        os: Option<String>,
    },
    /// File-backed profile pool with usage counts, safe for concurrent processes.
    Pool {
        /// Pool file.
        #[arg(long, default_value = "veneer-pool.json", env = "VENEER_POOL")]
        file: std::path::PathBuf,
        #[command(subcommand)]
        command: pool::PoolCommand,
    },
    /// Validate a custom dataset directory and bundle it into one JSON file.
    Pack {
        dir: String,
        /// Output file (stdout if omitted).
        #[arg(long)]
        out: Option<String>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Pretty,
    Json,
    /// One JSON document per line.
    Jsonl,
}

#[derive(Args, Default)]
pub(crate) struct ConstraintArgs {
    #[arg(long)]
    os: Option<String>,
    #[arg(long)]
    browser: Option<String>,
    #[arg(long, value_parser = parse_kind)]
    kind: Option<Kind>,
    #[arg(long)]
    device: Option<String>,
    #[arg(long, value_parser = parse_engine)]
    engine: Option<Engine>,
    #[arg(long)]
    gpu_vendor: Option<String>,
    #[arg(long)]
    min_version: Option<u32>,
    #[arg(long)]
    max_version: Option<u32>,
    #[arg(long)]
    locale: Option<String>,
    #[arg(long)]
    country: Option<String>,
    #[arg(long)]
    timezone: Option<String>,
    #[arg(long)]
    seed: Option<String>,
    /// Smallest screen width in CSS pixels.
    #[arg(long)]
    min_width: Option<u32>,
    /// Largest screen width in CSS pixels.
    #[arg(long)]
    max_width: Option<u32>,
    /// Smallest screen height in CSS pixels.
    #[arg(long)]
    min_height: Option<u32>,
    /// Largest screen height in CSS pixels.
    #[arg(long)]
    max_height: Option<u32>,
    /// Smallest devicePixelRatio.
    #[arg(long)]
    min_dpr: Option<f64>,
    /// Largest devicePixelRatio.
    #[arg(long)]
    max_dpr: Option<f64>,
}

impl From<ConstraintArgs> for Constraints {
    fn from(a: ConstraintArgs) -> Self {
        Constraints {
            os: a.os,
            browser: a.browser,
            kind: a.kind,
            device: a.device,
            engine: a.engine,
            gpu_vendor: a.gpu_vendor,
            min_version: a.min_version,
            max_version: a.max_version,
            locale: a.locale,
            country: a.country,
            timezone: a.timezone,
            seed: a.seed,
            min_width: a.min_width,
            max_width: a.max_width,
            min_height: a.min_height,
            max_height: a.max_height,
            min_dpr: a.min_dpr,
            max_dpr: a.max_dpr,
        }
    }
}

fn parse_kind(s: &str) -> Result<Kind, String> {
    serde_json::from_value(serde_json::Value::String(s.to_owned()))
        .map_err(|_| "expected desktop, laptop, phone, tablet, console, tv or vr".to_owned())
}

fn parse_engine(s: &str) -> Result<Engine, String> {
    serde_json::from_value(serde_json::Value::String(s.to_owned()))
        .map_err(|_| "expected chromium, gecko or webkit".to_owned())
}

/// A profile plus everything needed to use it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Fingerprint {
    /// Short id that regenerates this profile (`veneer generate --id`, API `id`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub profile: Profile,
    pub report: veneer::Report,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<veneer::Header>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init_script: Option<String>,
}

/// Generate `count` profiles; each must pass validation before it is returned.
pub(crate) fn fingerprints(
    constraints: &Constraints,
    count: u32,
    headers: bool,
    script: bool,
) -> Result<Vec<Fingerprint>, veneer::Error> {
    fingerprint_batch(constraints, count, false, headers, script)
}

/// Like [`fingerprints`]; with `unique`, no two profiles share a (device, browser, version, screen, locale).
pub(crate) fn fingerprint_batch(
    constraints: &Constraints,
    count: u32,
    unique: bool,
    headers: bool,
    script: bool,
) -> Result<Vec<Fingerprint>, veneer::Error> {
    veneer::generate_batch(constraints, count, unique)?
        .into_iter()
        .map(|g| envelope(g.profile, Some(g.id), headers, script))
        .collect()
}

/// Validate a profile and wrap it with its report, id and, on request, headers and init script.
pub(crate) fn envelope(
    profile: Profile,
    id: Option<String>,
    headers: bool,
    script: bool,
) -> Result<Fingerprint, veneer::Error> {
    let report = veneer::validate(&profile);
    if !report.valid {
        return Err(veneer::Error::Dataset(format!(
            "generated profile failed validation: {:?}",
            report.issues
        )));
    }
    Ok(Fingerprint {
        id,
        headers: headers.then(|| veneer::navigation_headers(&profile)),
        init_script: if script {
            Some(veneer::to_init_script(&profile)?)
        } else {
            None
        },
        report,
        profile,
    })
}

fn print<T: Serialize>(value: &T, format: Format) -> Result<(), String> {
    let text = match format {
        Format::Pretty => serde_json::to_string_pretty(value),
        Format::Json | Format::Jsonl => serde_json::to_string(value),
    }
    .map_err(|e| e.to_string())?;
    println!("{text}");
    Ok(())
}

fn read_input(file: &str) -> Result<String, String> {
    if file == "-" {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|e| format!("cannot read stdin: {e}"))?;
        Ok(text)
    } else {
        std::fs::read_to_string(file).map_err(|e| format!("cannot read `{file}`: {e}"))
    }
}

fn run(cli: Cli) -> Result<ExitCode, String> {
    if let Some(dir) = &cli.data {
        veneer::extend_dataset_dir(dir).map_err(|e| e.to_string())?;
    }
    match cli.command {
        Command::Generate {
            constraints,
            count,
            full,
            format,
            unique,
            id,
        } => {
            let constraints = Constraints::from(constraints);
            let list = match id {
                Some(id) => {
                    if constraints != Constraints::default() {
                        return Err(
                            "--id cannot be combined with constraint flags or --seed".into()
                        );
                    }
                    let profile = veneer::generate_from_id(&id).map_err(|e| e.to_string())?;
                    vec![envelope(profile, Some(id), full, full).map_err(|e| e.to_string())?]
                }
                None => fingerprint_batch(&constraints, count, unique, full, full)
                    .map_err(|e| e.to_string())?,
            };
            match format {
                Format::Jsonl => {
                    for fp in &list {
                        print(&output(fp, full), format)?;
                    }
                }
                _ if list.len() == 1 => print(&output(&list[0], full), format)?,
                _ => print(
                    &list.iter().map(|fp| output(fp, full)).collect::<Vec<_>>(),
                    format,
                )?,
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Validate {
            file,
            format,
            output,
            check,
        } => audit::validate(&file, format, output, check),
        Command::Capture { file, check } => audit::capture(&file, check),
        Command::Drift { file, format } => audit::drift(&file, format),
        Command::Rules { options } => audit::rules(options),
        Command::Script {
            constraints,
            profile,
        } => {
            let profile: Profile = match profile {
                Some(file) => serde_json::from_str(&read_input(&file)?)
                    .map_err(|e| format!("not a Veneer profile: {e}"))?,
                None => veneer::generate(&constraints.into()).map_err(|e| e.to_string())?,
            };
            println!(
                "{}",
                veneer::to_init_script(&profile).map_err(|e| e.to_string())?
            );
            Ok(ExitCode::SUCCESS)
        }
        Command::Bench { constraints, count } => {
            print(&bench::run(&constraints.into(), count)?, Format::Pretty)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Collector => {
            println!("{}", veneer::collector_script());
            Ok(ExitCode::SUCCESS)
        }
        Command::Catalog => {
            print(
                &veneer::catalog().map_err(|e| e.to_string())?,
                Format::Pretty,
            )?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Serve { host, port, token } => {
            let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
            runtime.block_on(server::serve(&host, port, token))?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Headers {
            constraints,
            profile,
            format,
            url,
        } => {
            let profile = match profile {
                Some(file) => net::read_profile(&file)?,
                None => veneer::generate(&constraints.into()).map_err(|e| e.to_string())?,
            };
            let headers = veneer::navigation_headers(&profile);
            println!("{}", net::render_headers(&headers, format, &url)?);
            Ok(ExitCode::SUCCESS)
        }
        Command::CheckHeaders { profile, headers } => net::check_headers(&profile, &headers),
        Command::Net(command) => net::run(command),
        Command::Diff { a, b, json } => diff::run(&a, &b, json),
        Command::Devices { os } => {
            devices::run(os.as_deref())?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Pool { file, command } => pool::run(&file, command),
        Command::Pack { dir, out } => {
            pack::run(&dir, out.as_deref())?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

/// Plain profile unless `--full` asked for the envelope with headers and script.
fn output(fp: &Fingerprint, full: bool) -> serde_json::Value {
    let value = if full {
        serde_json::to_value(fp)
    } else {
        serde_json::to_value(&fp.profile)
    };
    value.unwrap_or(serde_json::Value::Null)
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}
