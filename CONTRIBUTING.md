# Contributing to Veneer

Thank you for your interest in Veneer. The project is fully open source, and every kind of contribution is welcome: bug reports, device data, browser versions, validator rules, bindings, documentation and translations.

By participating you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Ways to contribute

- **Report a bug.** Open an issue with the Veneer version, runtime (Rust, Node.js, Bun, Deno), the constraints you used, and what you expected.
- **Report an inconsistent or detectable profile.** Include the seed and constraints so it can be reproduced exactly, and explain which values give it away.
- **Verify a device.** Many console, TV and VR entries are marked low confidence in `crates/veneer/data/SOURCES.md`. If you own one, real values from it are extremely valuable.
- **Add a device or browser version.** See [Adding data](#adding-data).
- **Add a validator rule.** See [Adding a validator rule](#adding-a-validator-rule).
- **Improve docs or translations.** `README.md` and `README.tr.md` must stay in sync; update both in the same pull request.

For larger changes (new public API, dataset format changes), open an issue first so the design can be discussed before you invest time.

## Development setup

Requirements:

- Rust 1.82 or newer with the `wasm32-unknown-unknown` target (installed automatically through `rust-toolchain.toml` when using rustup)
- Node.js 20 or newer
- `wasm-bindgen-cli` matching the version in `Cargo.lock`, only for building the npm package

```bash
git clone https://github.com/LLCForge/Veneer.git
cd Veneer
cargo test --workspace
```

Build and test the npm package:

```bash
cargo install wasm-bindgen-cli --locked --version 0.2.130
```

```bash
cd packages/veneer && npm run build && npm test
```

Generate a profile or run the local API and web UI:

```bash
cargo run -p veneer-cli -- generate --os android --browser samsung --locale tr --seed demo
```

```bash
cargo run -p veneer-cli -- serve
```

Run the real-browser end-to-end suite (Chromium locally; CI also runs Firefox and WebKit):

```bash
cd tests/e2e && npm ci && npx playwright install chromium && npx playwright test --project=chromium
```

Bindings for other languages build on `crates/veneer-ffi`; see the README in each `bindings/` folder and the matching CI job.

Refresh desktop browser versions from the official release feeds:

```bash
node scripts/update-versions.mjs
```

## Before opening a pull request

All of these must pass; CI runs the same checks:

```bash
cargo fmt --all -- --check
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
cargo test --workspace
```

```bash
cargo deny check
```

## Conventions

- **Commits** follow [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `data:`, `docs:`, `refactor:`, `test:`, `chore:`.
- **Rust:** no `unsafe` (except the documented C boundary in `veneer-ffi`), no `unwrap()`/`expect()` outside tests, errors through `veneer::Error`.
- **JavaScript:** keep `index.d.ts` in sync with `index.js` and the Rust `model.rs`.
- **Tests:** every bug fix gets a regression test; every validator rule gets a test that triggers it.
- **Determinism:** all randomness comes from the seeded RNG in `generate.rs`. Never add another source of randomness to the generator, and never change the order of draws without noting that profiles for existing seeds will change.

## Adding data

Data lives in `crates/veneer/data/`. The format of every file is described in [ARCHITECTURE.md](ARCHITECTURE.md#dataset).

1. Edit the JSON. A new device goes into the matching `data/devices/<family>.json` (with the OS versions it can run); a new browser or in-app browser is a new `data/browsers/<id>.json` with a user-agent template per OS; a new platform is a new `data/os/<id>.json`. Web UI text lives in `crates/veneer-cli/src/ui/i18n.js`; add both Turkish and English strings.
2. Record where the values come from in `data/SOURCES.md`, with a confidence level. Data must be redistributable under terms compatible with MIT.
3. Run `cargo test`. The dataset is integrity-checked, and the test suite generates thousands of profiles and fails on any inconsistency.
4. Explain in the pull request how the values were obtained (real device, vendor documentation, public database).

Data must **never** contain personal information: no IP addresses, cookies, account names, user ids, browsing history or free-text fields from real users. Pull requests containing personal data will be closed and the data purged.

## Adding a browser API

Add `crates/veneer/src/extras/<topic>.rs` (profile field, derivation, rules, live checks), `crates/veneer/src/inject/NN-<topic>.js` (the patch) and `crates/veneer/src/collect/NN-<topic>.js` (what a page observes), register the module in `extras/mod.rs`, and extend `packages/veneer/index.d.ts`. Verify it in a real browser with the e2e suite.

## Adding a validator rule

1. Add the rule to `generic_rules` (applies to any browser) or `check` (needs the dataset) in `crates/veneer/src/validate.rs`, to a module in `crates/veneer/src/validator/`, or to an extension module in `crates/veneer/src/extras/`. Give it a hint and fix in `validator/rules.rs`.
2. Give it a stable dotted id (`webgl.masked`), `error` for contradictions or `warning` for rare-but-possible values, and a one-line message.
3. Add a test in `crates/veneer/tests/profiles.rs` that breaks a generated profile and asserts the rule fires.
4. Run `cargo test` to confirm the generator never trips the new rule.

## Pull request process

1. Fork the repository and create a branch from `main`.
2. Keep pull requests focused: one topic per pull request.
3. Describe the change and link related issues.
4. A maintainer will review. Expect feedback; it is part of the process, not a judgment.
5. Pull requests are squash-merged once CI is green and at least one maintainer approves.

## Licensing of contributions

By submitting a contribution you agree that it is licensed under the [MIT License](LICENSE), the same license as the project, and that you have the right to submit it.
