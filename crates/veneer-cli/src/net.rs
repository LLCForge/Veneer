//! `veneer headers`, `veneer check-headers` and `veneer net`: request headers for non-browser HTTP
//! clients, captured-header verification and network-layer consistency helpers.

use std::process::ExitCode;

use clap::{Subcommand, ValueEnum};
use veneer::{Engine, Header, Profile, WebRtcPolicy};

use crate::{print, read_input, Format};

/// Longest URL accepted for `veneer headers --url`.
const MAX_URL: usize = 2048;

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum HeaderFormat {
    /// A curl command line (POSIX shell quoting).
    Curl,
    /// `[{name, value}]` in browser order.
    Json,
    /// A Python dict for requests / httpx.
    Python,
    /// A JavaScript object for fetch / undici.
    Node,
}

#[derive(Subcommand)]
pub(crate) enum NetCommand {
    /// Compare a JA4 TLS fingerprint you captured with the profile's browser family (informational).
    Ja4 {
        /// Profile JSON file, or `-` for stdin.
        #[arg(long)]
        profile: String,
        /// JA4 fingerprint, e.g. t13d1517h2_8daaf6152771_cb7bf5808d99.
        ja4: String,
    },
    /// Check that the proxy's country and timezone (which you supply) fit the profile.
    Location {
        #[arg(long)]
        profile: String,
        /// ISO 3166 country code of the egress IP, e.g. TR.
        #[arg(long)]
        country: Option<String>,
        /// IANA timezone of the egress IP, e.g. Europe/Istanbul.
        #[arg(long)]
        timezone: Option<String>,
    },
    /// Launch settings that stop WebRTC from exposing local IP addresses.
    Webrtc {
        #[arg(long, value_parser = crate::parse_engine)]
        engine: Engine,
        #[arg(long, value_enum, default_value_t = PolicyArg::PublicInterfaceOnly)]
        policy: PolicyArg,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum PolicyArg {
    PublicInterfaceOnly,
    DisableNonProxiedUdp,
    Disabled,
}

impl From<PolicyArg> for WebRtcPolicy {
    fn from(p: PolicyArg) -> Self {
        match p {
            PolicyArg::PublicInterfaceOnly => WebRtcPolicy::PublicInterfaceOnly,
            PolicyArg::DisableNonProxiedUdp => WebRtcPolicy::DisableNonProxiedUdp,
            PolicyArg::Disabled => WebRtcPolicy::Disabled,
        }
    }
}

pub(crate) fn read_profile(file: &str) -> Result<Profile, String> {
    serde_json::from_str(&read_input(file)?).map_err(|e| format!("not a Veneer profile: {e}"))
}

/// Single-quoted POSIX shell word; embedded quotes become `'\''`.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

fn json_str(s: &str) -> String {
    serde_json::Value::String(s.to_owned()).to_string()
}

pub(crate) fn render_headers(
    headers: &[Header],
    format: HeaderFormat,
    url: &str,
) -> Result<String, String> {
    if url.len() > MAX_URL
        || !(url.starts_with("https://") || url.starts_with("http://"))
        || url.chars().any(char::is_control)
    {
        return Err(format!(
            "--url must be an http(s) URL of at most {MAX_URL} characters"
        ));
    }
    let entries = |indent: &str| {
        headers
            .iter()
            .map(|h| format!("{indent}{}: {},", json_str(&h.name), json_str(&h.value)))
            .collect::<Vec<_>>()
            .join("\n")
    };
    Ok(match format {
        HeaderFormat::Json => serde_json::to_string_pretty(headers).map_err(|e| e.to_string())?,
        // --compressed lets curl decode the encodings the browser advertises (if curl supports them).
        HeaderFormat::Curl => {
            let mut lines = vec!["curl --compressed".to_owned()];
            lines.extend(
                headers
                    .iter()
                    .map(|h| format!("  -H {}", shell_quote(&format!("{}: {}", h.name, h.value)))),
            );
            lines.push(format!("  {}", shell_quote(url)));
            lines.join(" \\\n")
        }
        HeaderFormat::Python => format!(
            "# Headers in browser order. The TLS and HTTP/2 fingerprints still come from your HTTP library.\nHEADERS = {{\n{}\n}}\n\n# import requests\n# response = requests.get({}, headers=HEADERS)",
            entries("    "),
            json_str(url)
        ),
        HeaderFormat::Node => format!(
            "// Headers in browser order. The TLS and HTTP/2 fingerprints still come from Node's HTTP stack.\nconst headers = {{\n{}\n}};\n\n// const response = await fetch({}, {{ headers }});",
            entries("  "),
            json_str(url)
        ),
    })
}

pub(crate) fn check_headers(profile: &str, headers: &str) -> Result<ExitCode, String> {
    if profile == "-" && headers == "-" {
        return Err("only one of --profile and the headers file can be `-`".into());
    }
    let profile = read_profile(profile)?;
    let observed: serde_json::Value =
        serde_json::from_str(&read_input(headers)?).map_err(|e| format!("invalid JSON: {e}"))?;
    let report = veneer::compare_headers_json(&profile, &observed).map_err(|e| e.to_string())?;
    print(&report, Format::Pretty)?;
    Ok(if report.valid {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

pub(crate) fn run(command: NetCommand) -> Result<ExitCode, String> {
    let check = match command {
        NetCommand::Ja4 { profile, ja4 } => {
            veneer::check_ja4(&read_profile(&profile)?, &ja4).map_err(|e| e.to_string())?
        }
        NetCommand::Location {
            profile,
            country,
            timezone,
        } => veneer::check_location(
            &read_profile(&profile)?,
            country.as_deref(),
            timezone.as_deref(),
        )
        .map_err(|e| e.to_string())?,
        NetCommand::Webrtc { engine, policy } => {
            print(
                &veneer::webrtc_settings(engine, policy.into()),
                Format::Pretty,
            )?;
            return Ok(ExitCode::SUCCESS);
        }
    };
    print(&check, Format::Pretty)?;
    Ok(if check.report.valid {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Header> {
        vec![
            Header {
                name: "user-agent".into(),
                value: "Mozilla/5.0 (it's)".into(),
            },
            Header {
                name: "accept-language".into(),
                value: "tr-TR,tr;q=0.9".into(),
            },
        ]
    }

    #[test]
    fn curl_quotes_safely() {
        let out =
            render_headers(&sample(), HeaderFormat::Curl, "https://example.com/").expect("curl");
        assert!(out.contains(r"-H 'user-agent: Mozilla/5.0 (it'\''s)'"));
        assert!(out.ends_with("'https://example.com/'"));
        assert!(render_headers(&sample(), HeaderFormat::Curl, "file:///etc/passwd").is_err());
        assert!(render_headers(&sample(), HeaderFormat::Curl, "https://a/\n").is_err());
    }

    #[test]
    fn python_and_node_keep_order() {
        for format in [HeaderFormat::Python, HeaderFormat::Node] {
            let out = render_headers(&sample(), format, "https://example.com/").expect("render");
            let ua = out.find("\"user-agent\"").expect("ua");
            let lang = out.find("\"accept-language\"").expect("lang");
            assert!(ua < lang);
        }
    }
}
