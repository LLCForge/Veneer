//! Extension points. A module here can add profile fields, derive them, validate them and compare
//! them live, without editing the core generator, validator or live checker.
//!
//! To add a module `foo`:
//! 1. create `src/extras/foo.rs` with its types and `derive`, `validate`, `live_checks` functions;
//! 2. add `pub mod foo;` below and one call in each of the three functions in this file;
//! 3. add one `Option<foo::Foo>` field to [`Extensions`] (always `#[serde(default, skip_serializing_if = "Option::is_none")]`);
//! 4. apply it in a new `src/inject/NN-foo.js` and observe it in `src/collect/NN-foo.js`.
//!
//! Each module draws randomness from [`ExtraContext::rng`] with its own name, so modules never
//! change each other's values or the core profile for a given seed.

use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::data::{BrowserDef, Dataset, DeviceDef, LocaleDef, OsDef, OsVersionDef, VersionDef};
use crate::generate::fnv1a;
use crate::model::Profile;
use crate::validate::Collector;

pub mod fonts_locale;
pub mod graphics;
pub mod navapi;

/// Profile fields contributed by extension modules. Flattened into the profile JSON.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graphics: Option<graphics::Graphics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub navapi: Option<navapi::NavApi>,
}

/// Everything the generator chose for this profile.
#[allow(dead_code)]
pub(crate) struct ExtraContext<'a> {
    pub data: &'a Dataset,
    pub device: &'a DeviceDef,
    pub os: &'a OsDef,
    pub os_version: &'a OsVersionDef,
    pub browser: &'a BrowserDef,
    pub browser_id: &'a str,
    pub version: &'a VersionDef,
    pub locale: &'a LocaleDef,
}

impl ExtraContext<'_> {
    /// Deterministic RNG private to one module: same seed and module name, same stream.
    #[allow(dead_code)]
    pub fn rng(&self, seed: &str, module: &str) -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(fnv1a(format!("{seed}/{module}").as_bytes()))
    }
}

/// Fill extension fields after the core profile is built.
pub(crate) fn derive(_profile: &mut Profile, _ctx: &ExtraContext) {
    fonts_locale::derive(_profile);
    _profile.ext.graphics = graphics::derive(_profile);
    _profile.ext.navapi = navapi::derive(_profile, &mut _ctx.rng(&_profile.seed, "navapi"));
}

/// Extension rules. Must never fire on a generated profile.
pub(crate) fn validate(_data: &Dataset, _profile: &Profile, _c: &mut Collector) {
    fonts_locale::validate(_c);
    graphics::validate(_profile, _c);
    navapi::validate(_profile, _c);
}

/// Extra live comparisons: (JSON pointer into the collector output, expected value).
pub(crate) fn live_checks(_profile: &Profile) -> Vec<(String, Value)> {
    let mut out = Vec::new();
    out.extend(graphics::live_checks(_profile));
    out.extend(navapi::live_checks(_profile));
    out
}
