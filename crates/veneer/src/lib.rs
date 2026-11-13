//! Coherent browser fingerprint profiles for Playwright and Puppeteer.
//!
//! ```
//! let profile = veneer::generate(&veneer::Constraints {
//!     os: Some("macos".into()),
//!     seed: Some("session-42".into()),
//!     ..Default::default()
//! })?;
//! assert!(veneer::validate(&profile).valid);
//! let script = veneer::to_init_script(&profile)?;
//! assert!(script.contains(&profile.navigator.user_agent));
//! # Ok::<(), veneer::Error>(())
//! ```
#![forbid(unsafe_code)]

mod catalog;
mod data;
// Some groups are only read once extension modules use them.
#[allow(dead_code)]
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}
mod extras;
mod generate;
mod headers;
mod inject;
mod live;
mod model;
mod network;
mod validate;
mod validator;

pub use catalog::{catalog, Catalog, CatalogBrowser, CatalogDevice, CatalogLocale, CatalogOs};
pub use data::{extend_dataset, extend_dataset_dir, DataFile, MAX_DATA_FILES, MAX_DATA_FILE_BYTES};
pub use generate::generate;
pub use generate::{generate_batch, generate_from_id, profile_id, Generated, MAX_BATCH};
pub use headers::{accept_language, navigation_headers, Header};
pub use headers::{
    compare_headers, compare_headers_json, headers_from_json, MAX_HEADERS, MAX_HEADER_NAME,
    MAX_HEADER_VALUE,
};
pub use inject::{collector_script, to_init_script};
pub use live::{compare_observed, compare_observed_in, LiveContext};
pub use model::*;
pub use network::{
    check_ja4, check_location, webrtc_settings, NetworkCheck, WebRtcPolicy, WebRtcSettings,
};
pub use validate::{validate, Issue, Report, Severity};
pub use validate::{Fix, FixAction};
pub use validator::{
    capture, drift, headers_from_har, import_profile, rule_coverage, to_sarif, validate_with,
    Condition, Coverage, CustomRule, DriftBucket, DriftDimension, DriftReport, Geo, ImportFormat,
    Imported, Op, RuleInfo, Test, ValidateOptions, RULES_VERSION,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unsatisfiable constraints: {0}")]
    Unsatisfiable(String),
    #[error("invalid constraint: {0}")]
    InvalidConstraint(String),
    #[error("embedded dataset is invalid: {0}")]
    Dataset(String),
    #[error("serialization failed: {0}")]
    Serialize(#[from] serde_json::Error),
}
