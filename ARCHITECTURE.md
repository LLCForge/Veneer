# Architecture

This document describes how Veneer is put together. Read it before changing the core.

## Goals

1. **Coherence over randomness.** A profile is useful only if no two of its values contradict each other.
2. **Real data only.** Every value comes from a documented source (`crates/veneer/data/SOURCES.md`); unverifiable values are left out.
3. **Realistic distribution.** Profiles appear roughly as often as their configurations do in the real world.
4. **Determinism.** The same seed and constraints always produce the same profile on every platform and runtime.
5. **Portability.** One Rust core, compiled natively (Windows, Linux, macOS), to WebAssembly and as a C ABI library.
6. **Verifiable.** Every profile can be checked offline (validator) and live in a real browser (collector + comparison).
7. **Parallel-friendly.** Data, browser patches and extensions are one file per topic, so contributors rarely touch the same file.

## Repository layout

```text
Veneer/
├── crates/
│   ├── veneer/                 core library (crates.io)
│   │   ├── build.rs            embeds data/ and the script modules, honours data-* features
│   │   ├── data/               os/, browsers/, devices/, extra/, locales.json, SOURCES.md
│   │   └── src/
│   │       ├── data.rs         dataset types, integrity checks, runtime extension
│   │       ├── generate.rs     joint sampling, value derivation, UA templates
│   │       ├── generate/       alias-table sampler, shareable profile ids
│   │       ├── extras/         extension modules (graphics, navigator APIs, locale fonts)
│   │       ├── validate.rs     core rules, issues with fixes, score
│   │       ├── validator/      capture, imports, plausibility, headers, SARIF, drift, custom rules
│   │       ├── live.rs         live comparison of collector output
│   │       ├── headers.rs      navigation headers and captured-header comparison
│   │       ├── network.rs      JA4, proxy location and WebRTC helpers
│   │       ├── inject.rs       assembles inject/*.js into the init script
│   │       ├── inject/         browser patches, one module per topic
│   │       ├── collect/        in-page collector modules
│   │       ├── catalog.rs      ids for every constraint, configuration count
│   │       └── model.rs        public types (Profile, Constraints, …)
│   ├── veneer-cli/             `veneer` binary: CLI, HTTP API, bilingual web UI (src/ui)
│   ├── veneer-wasm/            wasm-bindgen bindings
│   └── veneer-ffi/             C ABI (JSON in, JSON out) for other languages
├── packages/                   npm: veneer (core + Playwright/Puppeteer helpers + veneer-verify) and integrations
├── bindings/                   Python, Go, .NET, Java on top of veneer-ffi
├── tests/e2e/                  Playwright end-to-end, FingerprintJS and WASM-parity tests
├── docs/                       network notes, verification campaign
├── examples/                   project templates and integration examples
├── docker/                     Playwright + Veneer image
└── scripts/update-versions.mjs refreshes desktop browser versions from official feeds
```

## Data flow

```text
data/** ──build.rs──► embedded files ──► data.rs (parse, integrity check, optional --data overlay)
                                              │
Constraints + seed ──► generate.rs ──► Profile ──► extras::derive ──┬─► validate.rs ──► Report
                                                                    ├─► headers.rs  ──► request headers
                                                                    ├─► inject.rs   ──► init script ──► browser
                                                                    └─► live.rs ◄── collector output ◄── browser
```

## Dataset

| Location | Holds |
| --- | --- |
| `data/os/<id>.json` | `navigator.platform` (and a Firefox override), client-hint platform, architecture, Firefox `oscpu`/`appVersion` templates, insets, fonts, weighted versions with pinned browser versions, excluded browsers, UA tokens and iOS build numbers |
| `data/browsers/<id>.json` | engine, client-hint brand, toolbar heights, core cap, weighted versions (with Chromium version, app build, revision), and a **user-agent template per OS** |
| `data/devices/<family>.json` | per device: OS, form factor, model and Apple hardware id, weight, OS versions it runs, architecture override, browser shares, core/memory distributions, touch points, screens, WebGL strings per engine |
| `data/extra/<topic>.json` | data for extension modules (GPU capabilities, WebGPU, voices, keyboard layouts, locale fonts, release dates) |
| `data/locales.json` | `navigator.languages`, weight, weighted timezones |

User-agent placeholders are rendered by `UaContext::render` in `generate.rs`: browser version (`{major}`, `{full}`, `{build}`, `{revision}`, `{chromium}`, `{chromiumFull}`), form factor (`{mobile}`, `{formFx}`), OS (`{osUa}`, `{osVersion}`, `{osVersion_}`, `{osVersionFull_}`, `{osMajor}`, `{osBuild}`), device (`{hwModel}`), locale (`{locale_}`, `{lang}`) and screen (`{screenPx}`, `{scale}`, `{scaleInt}`).

`deny_unknown_fields` rejects typos and `data.rs::check` enforces cross-file invariants (ids exist and are unique, every device browser has a template and an OS version that runs it, `{hwModel}` only where a device has one, weights are non-zero, handhelds have touch points). Cargo features `data-desktop`, `data-mobile` and `data-other` embed only part of the devices. At runtime `extend_dataset_dir` (CLI `--data`) overlays extra files, checked by the same rules.

`veneer catalog` reports `combinations`: device × browser × OS version × browser version × screen × cores × memory × (locale, timezone).

## Generation

1. **Validate input** (known ids, length limits, sane ranges).
2. **Enumerate** every `(device, browser, OS version)` matching the constraints, weighted `device × browser share × OS version`, and sample it with an alias table.
3. **Pick** browser version (pinned or from the range), screen, cores, memory, locale and timezone in a fixed order.
4. **Derive** user agent, `appVersion`, `oscpu`, platform, vendor, masked WebGL, client hints (GREASE order seeded by the Chromium major), `deviceMemory` (Chromium 147+ buckets), insets, viewport, colour depth and fonts.
5. **Extensions** (`extras::derive`) add their fields with their own seeded RNG, so they never shift core values.

Randomness comes from one `ChaCha8Rng` seeded with the FNV-1a hash of the seed. Weights are integers, so native and WASM produce the same profile. Batches can be `unique`, and a profile id encodes constraints + seed to regenerate it exactly.

## Validation

`validate.rs` returns `Report { valid, score, issues, rulesVersion }`; every issue has a rule id, severity, message, hint and a machine-readable fix. Rules cover the generic engine facts, the dataset (template user agent, pinned versions, device ↔ OS version, client hints, touch points, GPU, fonts, locale/timezone), plausibility (cores and memory per device or GPU) and extension rules. `validator/` adds capture and imports, header and HAR checks, geography, custom JSON rules, SARIF, drift and coverage.

**Invariant:** the generator never produces a profile with any issue. `tests/profiles.rs` checks thousands of profiles across every OS, browser and device on each run.

## Browser side

The init script is `inject/*.js` joined in file order inside one IIFE with the profile embedded as `serde_json` output (no value can escape its string literal). `00-*` modules share helpers; every other module runs in its own block. Patches redefine getters where the browser defines them (keeping receiver checks), print as `[native code]`, delete properties the profiled browser does not have, and cover workers by re-running registered parts in the worker scope.

The collector is `collect/*.js` joined into one async expression. `compare_observed_in` diffs its output with the profile; in `LiveContext::Page` (a plain tab) mismatches that only emulation can fix — CSS media features — are warnings, and extension fields that were not collected are reported once as `live.not_collected.*`.

Automation helpers apply what a script cannot: user agent, locale, timezone, viewport, scale, touch, colour scheme and colour gamut through Playwright/Puppeteer options and CDP (`Emulation.setUserAgentOverride` with client-hint metadata, `Emulation.setEmulatedMedia`).

## CLI and HTTP API

`veneer-cli` builds one `veneer` binary (see `veneer --help`). `serve` runs an axum server with the bilingual web UI (`/`, `/probe`, `/validate`, `/contribute`) and the `/v1` JSON API:

- binds to `127.0.0.1` by default; any other address requires a token (fail closed)
- token via `Authorization: Bearer` or an HttpOnly, SameSite=Strict cookie, compared in constant time
- strict CSP (no inline scripts or styles; `blob:` only for workers), `X-Frame-Options: DENY`, `nosniff`, `no-referrer`, `no-store`
- 64 KB bodies, 64 requests in flight, at most 100 fingerprints per request, schema-checked inputs, re-encoded values in HTML
- structured JSON logs on stderr without tokens or bodies

## Other runtimes

- `veneer-wasm` → `packages/veneer` (Node.js) with Playwright/Puppeteer helpers and `veneer-verify`; integration packages build on it.
- `veneer-ffi` exposes `veneer_generate`, `veneer_validate`, `veneer_init_script`, … as JSON-in/JSON-out C functions with `veneer_free`; panics never cross the boundary. `bindings/` wrap it for Python, Go, .NET and Java.

## Keeping data fresh

`scripts/update-versions.mjs` reads Chrome, Edge and Firefox release feeds and rewrites those browsers' versions; the weekly workflow runs it with the tests and opens a pull request. iOS browsers, in-app browsers and OS versions are maintained by hand from the sources in SOURCES.md.

## Adding things

- **Device:** add it to the matching `data/devices/<family>.json` (or a new family file); run `cargo test -p veneer`.
- **Browser or in-app browser:** add `data/browsers/<id>.json` with a UA template per OS and add it to devices' `browsers`.
- **Browser API:** add `src/extras/<topic>.rs` (field, derivation, rules, live checks), `src/inject/NN-<topic>.js` and `src/collect/NN-<topic>.js`; register the module in `extras/mod.rs`; extend `index.d.ts`.
