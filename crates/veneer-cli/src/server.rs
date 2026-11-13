//! Local HTTP API: fresh, validated fingerprints on every request, plus a web UI and a live probe page.

use std::hash::BuildHasher;
use std::net::IpAddr;
use std::sync::Arc;

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{DefaultBodyLimit, Query, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::Semaphore;
use veneer::{Constraints, Engine, Kind, Profile};

const MAX_BODY: usize = 64 * 1024;
const MAX_COUNT: u32 = 100;
const MAX_IN_FLIGHT: usize = 64;
const COOKIE: &str = "veneer_session";

const INDEX_HTML: &str = include_str!("ui/index.html");
const APP_JS: &str = include_str!("ui/app.js");
const PROBE_HTML: &str = include_str!("ui/probe.html");
const PROBE_JS: &str = include_str!("ui/probe.js");
const STYLE_CSS: &str = include_str!("ui/style.css");
const I18N_JS: &str = include_str!("ui/i18n.js");
const CONTRIBUTE_HTML: &str = include_str!("ui/contribute.html");
const CONTRIBUTE_JS: &str = include_str!("ui/contribute.js");
const VALIDATE_HTML: &str = include_str!("ui/validate.html");
const VALIDATE_JS: &str = include_str!("ui/validate.js");
const LOGO: &[u8] = include_bytes!("../../../assets/fingerprint-tile-256.png");
const FAVICON: &[u8] = include_bytes!("../../../assets/favicon.ico");

#[derive(Clone)]
struct AppState {
    token: Option<Arc<str>>,
    permits: Arc<Semaphore>,
}

pub async fn serve(host: &str, port: u16, token: Option<String>) -> Result<(), String> {
    let ip: IpAddr = host
        .parse()
        .map_err(|_| format!("`{host}` is not an IP address"))?;
    if let Some(t) = &token {
        if t.len() < 16
            || !t
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(
                "--token must be at least 16 characters of A-Z, a-z, 0-9, '-' or '_'".into(),
            );
        }
    } else if !ip.is_loopback() {
        // Fail closed: never expose an unauthenticated API beyond this machine.
        return Err(format!(
            "binding to {ip} requires --token (or VENEER_TOKEN)"
        ));
    }
    veneer::catalog().map_err(|e| e.to_string())?;

    let state = AppState {
        token: token.map(Arc::from),
        permits: Arc::new(Semaphore::new(MAX_IN_FLIGHT)),
    };
    let app = Router::new()
        .route("/", get(|| async { Html(INDEX_HTML) }))
        .route(
            "/app.js",
            get(|| async { asset("text/javascript; charset=utf-8", APP_JS) }),
        )
        .route(
            "/i18n.js",
            get(|| async { asset("text/javascript; charset=utf-8", I18N_JS) }),
        )
        .route(
            "/probe.js",
            get(|| async { asset("text/javascript; charset=utf-8", PROBE_JS) }),
        )
        .route(
            "/style.css",
            get(|| async { asset("text/css; charset=utf-8", STYLE_CSS) }),
        )
        .route("/logo.png", get(|| async { asset("image/png", LOGO) }))
        .route(
            "/favicon.ico",
            get(|| async { asset("image/x-icon", FAVICON) }),
        )
        .route("/probe", get(probe))
        // Volunteer report page: runs the collector in the visitor's browser; the server stores and sends nothing.
        // Outside /v1 so a TV or console on the LAN can open it without the API token.
        .route("/contribute", get(|| async { Html(CONTRIBUTE_HTML) }))
        .route(
            "/contribute.js",
            get(|| async { asset("text/javascript; charset=utf-8", CONTRIBUTE_JS) }),
        )
        .route("/contribute/collector.js", get(collector))
        .route("/v1/health", get(health))
        .route("/v1/session", post(session))
        .route("/v1/catalog", get(catalog))
        .route("/v1/fingerprint", get(fingerprint))
        .route("/v1/script", get(script))
        .route("/v1/collector.js", get(collector))
        .route("/v1/validate", post(validate))
        .route("/validate", get(|| async { Html(VALIDATE_HTML) }))
        .route(
            "/validate.js",
            get(|| async { asset("text/javascript; charset=utf-8", VALIDATE_JS) }),
        )
        .route("/v1/check", post(check))
        .route("/v1/capture", post(capture))
        .route("/v1/sarif", post(sarif))
        .route("/v1/drift", post(drift))
        .route("/v1/rules", get(rules))
        .route("/v1/compare", post(compare))
        .route("/v1/headers/compare", post(compare_headers))
        .route("/v1/network/ja4", post(network_ja4))
        .route("/v1/network/location", post(network_location))
        .route("/v1/network/webrtc", get(network_webrtc))
        .layer(middleware::from_fn_with_state(state.clone(), guard))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind((ip, port))
        .await
        .map_err(|e| format!("cannot listen on {ip}:{port}: {e}"))?;
    log(
        "info",
        "listening",
        json!({ "url": format!("http://{ip}:{port}") }),
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .map_err(|e| e.to_string())
}

/// One structured log line on stderr. Never includes tokens or request bodies.
fn log(level: &str, event: &str, fields: serde_json::Value) {
    eprintln!(
        "{}",
        json!({ "level": level, "event": event, "fields": fields })
    );
}

fn asset<B: Into<axum::body::Body>>(content_type: &'static str, body: B) -> Response {
    ([(header::CONTENT_TYPE, content_type)], body.into()).into_response()
}

/// Concurrency cap, authentication and security headers for every request.
async fn guard(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let Ok(_permit) = state.permits.clone().try_acquire_owned() else {
        return ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "server busy, retry shortly".into(),
        )
        .into_response();
    };
    let path = req.uri().path();
    let open = !path.starts_with("/v1/") || path == "/v1/health" || path == "/v1/session";
    if let (Some(token), false) = (&state.token, open) {
        if !authorized(req.headers(), token) {
            return ApiError(StatusCode::UNAUTHORIZED, "missing or invalid token".into())
                .into_response();
        }
    }
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    for (name, value) in [
        ("x-content-type-options", "nosniff"),
        ("x-frame-options", "DENY"),
        ("referrer-policy", "no-referrer"),
        ("permissions-policy", "camera=(), microphone=(), geolocation=()"),
        ("cross-origin-opener-policy", "same-origin"),
        ("cache-control", "no-store"),
        (
            "content-security-policy",
            "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self' blob:; worker-src 'self' blob:; frame-ancestors 'none'; base-uri 'none'; form-action 'self'",
        ),
    ] {
        h.insert(name, HeaderValue::from_static(value));
    }
    res
}

fn authorized(headers: &HeaderMap, token: &str) -> bool {
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let cookie = headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .find_map(|c| {
            c.trim()
                .strip_prefix(&format!("{COOKIE}="))
                .map(str::to_owned)
        });
    bearer.is_some_and(|b| constant_time_eq(b, token))
        || cookie.is_some_and(|c| constant_time_eq(&c, token))
}

/// Comparison time does not depend on where the inputs differ.
fn constant_time_eq(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl From<veneer::Error> for ApiError {
    fn from(e: veneer::Error) -> Self {
        match e {
            veneer::Error::InvalidConstraint(m) | veneer::Error::Unsatisfiable(m) => {
                ApiError(StatusCode::BAD_REQUEST, m)
            }
            other => {
                log("error", "internal", json!({ "error": other.to_string() }));
                ApiError(StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
            }
        }
    }
}

impl From<QueryRejection> for ApiError {
    fn from(e: QueryRejection) -> Self {
        ApiError(StatusCode::BAD_REQUEST, e.body_text())
    }
}

impl From<JsonRejection> for ApiError {
    fn from(e: JsonRejection) -> Self {
        ApiError(e.status(), e.body_text())
    }
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FingerprintQuery {
    os: Option<String>,
    browser: Option<String>,
    kind: Option<Kind>,
    device: Option<String>,
    engine: Option<Engine>,
    gpu_vendor: Option<String>,
    min_version: Option<u32>,
    max_version: Option<u32>,
    locale: Option<String>,
    country: Option<String>,
    timezone: Option<String>,
    seed: Option<String>,
    min_width: Option<u32>,
    max_width: Option<u32>,
    min_height: Option<u32>,
    max_height: Option<u32>,
    min_dpr: Option<f64>,
    max_dpr: Option<f64>,
    /// Never repeat a (device, browser, version, screen, locale) within one `count` batch.
    unique: Option<bool>,
    /// Short profile id (from a previous response) that regenerates that exact profile.
    id: Option<String>,
    count: Option<u32>,
    /// Comma-separated: `headers`, `script`.
    include: Option<String>,
    /// Interface language of the web UI (`tr` or `en`); ignored by the API.
    lang: Option<UiLang>,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum UiLang {
    Tr,
    En,
}

impl FingerprintQuery {
    fn constraints(&self) -> Constraints {
        Constraints {
            os: self.os.clone(),
            browser: self.browser.clone(),
            kind: self.kind,
            device: self.device.clone(),
            engine: self.engine,
            gpu_vendor: self.gpu_vendor.clone(),
            min_version: self.min_version,
            max_version: self.max_version,
            locale: self.locale.clone(),
            country: self.country.clone(),
            timezone: self.timezone.clone(),
            seed: self.seed.clone(),
            min_width: self.min_width,
            max_width: self.max_width,
            min_height: self.min_height,
            max_height: self.max_height,
            min_dpr: self.min_dpr,
            max_dpr: self.max_dpr,
        }
    }

    /// The profile named by `id`, or `None` when the query has no id. An id stands alone.
    fn profile_from_id(&self) -> Result<Option<Profile>, ApiError> {
        let Some(id) = &self.id else { return Ok(None) };
        if self.constraints() != Constraints::default() || self.unique.is_some() {
            return Err(ApiError(
                StatusCode::BAD_REQUEST,
                "id cannot be combined with constraints, seed or unique".into(),
            ));
        }
        if self.count.is_some_and(|c| c != 1) {
            return Err(ApiError(
                StatusCode::BAD_REQUEST,
                "id names one profile; count must be 1".into(),
            ));
        }
        Ok(Some(veneer::generate_from_id(id)?))
    }

    fn profile(&self) -> Result<Profile, ApiError> {
        match self.profile_from_id()? {
            Some(profile) => Ok(profile),
            None => Ok(veneer::generate(&self.constraints())?),
        }
    }

    fn includes(&self, part: &str) -> bool {
        self.include
            .as_deref()
            .is_some_and(|list| list.split(',').any(|p| p.trim() == part))
    }

    /// Re-encoded query string: only validated fields, percent-encoded, safe inside HTML attributes.
    fn encoded(&self) -> String {
        let value = serde_json::to_value(self).unwrap_or_default();
        value
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(_, v)| !v.is_null())
            .map(|(k, v)| {
                let raw = v.as_str().map_or_else(|| v.to_string(), str::to_owned);
                format!("{}={}", percent_encode(k), percent_encode(&raw))
            })
            .collect::<Vec<_>>()
            .join("&")
    }
}

fn percent_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionBody {
    token: String,
}

/// Exchange the token for an HttpOnly cookie so the web UI and probe page work without exposing it to scripts.
async fn session(
    State(state): State<AppState>,
    body: Result<Json<SessionBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(body) = body?;
    let Some(token) = &state.token else {
        return Ok(StatusCode::NO_CONTENT.into_response());
    };
    if !constant_time_eq(&body.token, token) {
        return Err(ApiError(StatusCode::UNAUTHORIZED, "invalid token".into()));
    }
    let cookie = format!("{COOKIE}={token}; HttpOnly; SameSite=Strict; Path=/");
    let value = HeaderValue::from_str(&cookie)
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "internal error".into()))?;
    Ok(([(header::SET_COOKIE, value)], StatusCode::NO_CONTENT).into_response())
}

async fn catalog() -> Result<Json<veneer::Catalog>, ApiError> {
    Ok(Json(veneer::catalog()?))
}

async fn fingerprint(
    query: Result<Query<FingerprintQuery>, QueryRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Query(q) = query?;
    let count = q.count.unwrap_or(1);
    if !(1..=MAX_COUNT).contains(&count) {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            format!("count must be between 1 and {MAX_COUNT}"),
        ));
    }
    let list = match q.profile_from_id()? {
        Some(profile) => vec![crate::envelope(
            profile,
            q.id.clone(),
            q.includes("headers"),
            q.includes("script"),
        )?],
        None => crate::fingerprint_batch(
            &q.constraints(),
            count,
            q.unique.unwrap_or(false),
            q.includes("headers"),
            q.includes("script"),
        )?,
    };
    Ok(Json(json!({ "count": list.len(), "fingerprints": list })))
}

async fn script(
    query: Result<Query<FingerprintQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let Query(q) = query?;
    let profile = q.profile()?;
    Ok(asset(
        "text/javascript; charset=utf-8",
        veneer::to_init_script(&profile)?,
    ))
}

async fn collector() -> Response {
    asset(
        "text/javascript; charset=utf-8",
        format!(
            "window.veneerCollect = () => {};\n",
            veneer::collector_script()
        ),
    )
}

async fn validate(
    body: Result<Json<Profile>, JsonRejection>,
) -> Result<Json<veneer::Report>, ApiError> {
    let Json(profile) = body?;
    Ok(Json(veneer::validate(&profile)))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CompareBody {
    profile: Profile,
    observed: serde_json::Value,
    #[serde(default)]
    context: veneer::LiveContext,
}

async fn compare(
    body: Result<Json<CompareBody>, JsonRejection>,
) -> Result<Json<veneer::Report>, ApiError> {
    let Json(body) = body?;
    Ok(Json(veneer::compare_observed_in(
        &body.profile,
        &body.observed,
        body.context,
    )))
}

/// Applies a fingerprint to this very page and checks what the browser now reports.
async fn probe(
    query: Result<Query<FingerprintQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let Query(mut q) = query?;
    q.count = None;
    q.include = None;
    q.unique = None;
    if q.seed.is_none() && q.id.is_none() {
        // A fixed seed lets the init script and the expected profile be generated identically.
        q.seed = Some(format!(
            "probe-{:016x}",
            std::collections::hash_map::RandomState::new().hash_one(std::time::SystemTime::now())
        ));
        return Ok(Redirect::to(&format!("/probe?{}", q.encoded())).into_response());
    }
    q.profile()?;
    Ok(Html(PROBE_HTML.replace("__QUERY__", &q.encoded())).into_response())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HeadersBody {
    profile: Profile,
    /// `[{name, value}]`, `[[name, value]]` or `{name: value}` (order check skipped for objects).
    headers: serde_json::Value,
}

async fn compare_headers(
    body: Result<Json<HeadersBody>, JsonRejection>,
) -> Result<Json<veneer::Report>, ApiError> {
    let Json(body) = body?;
    Ok(Json(veneer::compare_headers_json(
        &body.profile,
        &body.headers,
    )?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ja4Body {
    profile: Profile,
    ja4: String,
}

async fn network_ja4(
    body: Result<Json<Ja4Body>, JsonRejection>,
) -> Result<Json<veneer::NetworkCheck>, ApiError> {
    let Json(body) = body?;
    Ok(Json(veneer::check_ja4(&body.profile, &body.ja4)?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LocationBody {
    profile: Profile,
    country: Option<String>,
    timezone: Option<String>,
}

async fn network_location(
    body: Result<Json<LocationBody>, JsonRejection>,
) -> Result<Json<veneer::NetworkCheck>, ApiError> {
    let Json(body) = body?;
    Ok(Json(veneer::check_location(
        &body.profile,
        body.country.as_deref(),
        body.timezone.as_deref(),
    )?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WebRtcQuery {
    engine: Engine,
    policy: Option<veneer::WebRtcPolicy>,
}

async fn network_webrtc(
    query: Result<Query<WebRtcQuery>, QueryRejection>,
) -> Result<Json<veneer::WebRtcSettings>, ApiError> {
    let Query(q) = query?;
    Ok(Json(veneer::webrtc_settings(
        q.engine,
        q.policy
            .unwrap_or(veneer::WebRtcPolicy::PublicInterfaceOnly),
    )))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CheckBody {
    /// A Veneer profile, collector output, fingerprint-suite output or flat JSON.
    input: serde_json::Value,
    #[serde(default)]
    format: veneer::ImportFormat,
    /// HAR log whose document request supplies the request headers.
    #[serde(default)]
    har: Option<serde_json::Value>,
    #[serde(default)]
    options: veneer::ValidateOptions,
}

/// Import a profile in any supported format and validate it with options.
async fn check(
    body: Result<Json<CheckBody>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Json(b) = body?;
    let imported = veneer::import_profile(&b.input, b.format, &[])?;
    let mut options = b.options;
    if let Some(har) = &b.har {
        options.headers = Some(veneer::headers_from_har(har)?);
    }
    audited(imported, options)
}

fn audited(
    imported: veneer::Imported,
    mut options: veneer::ValidateOptions,
) -> Result<Json<serde_json::Value>, ApiError> {
    if options.headers.is_none() && !imported.headers.is_empty() {
        options.headers = Some(imported.headers.clone());
    }
    let report = veneer::validate_with(&imported.profile, &options)?;
    Ok(Json(json!({
        "profile": imported.profile,
        "format": imported.format,
        "missing": imported.missing,
        "report": report,
    })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CaptureBody {
    /// Output of the in-page collector (`/v1/collector.js`).
    observed: serde_json::Value,
    /// Request headers of the same page load, in sent order.
    #[serde(default)]
    headers: Vec<veneer::Header>,
    #[serde(default)]
    options: veneer::ValidateOptions,
}

/// Build a full profile from what a real browser reported, then validate it.
async fn capture(
    body: Result<Json<CaptureBody>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Json(b) = body?;
    let imported = veneer::capture(&b.observed, &b.headers)?;
    audited(imported, b.options)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SarifBody {
    profiles: Vec<serde_json::Value>,
    #[serde(default)]
    format: veneer::ImportFormat,
    #[serde(default)]
    options: veneer::ValidateOptions,
}

async fn sarif(
    body: Result<Json<SarifBody>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Json(b) = body?;
    let reports = b
        .profiles
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let imported = veneer::import_profile(v, b.format, &[])?;
            let report = veneer::validate_with(&imported.profile, &b.options)?;
            Ok((format!("profiles/{i}.json"), report))
        })
        .collect::<Result<Vec<_>, veneer::Error>>()?;
    Ok(Json(veneer::to_sarif(&reports, &b.options.rules)))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DriftBody {
    profiles: Vec<Profile>,
}

async fn drift(
    body: Result<Json<DriftBody>, JsonRejection>,
) -> Result<Json<veneer::DriftReport>, ApiError> {
    let Json(b) = body?;
    Ok(Json(veneer::drift(&b.profiles)?))
}

async fn rules() -> Result<Json<veneer::Coverage>, ApiError> {
    Ok(Json(veneer::rule_coverage(&[])?))
}
