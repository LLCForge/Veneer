# veneer-fingerprint (Python)

Python bindings for [Veneer](../../README.md): coherent browser fingerprints with Playwright helpers.
They load the native library built from [`crates/veneer-ffi`](../../crates/veneer-ffi) through `ctypes`.

## Setup

```bash
cargo build -p veneer-ffi --release
```

```bash
pip install -e bindings/python
```

Point `VENEER_FFI_LIB` at the built library (`target/release/veneer_ffi.dll`, `libveneer_ffi.so` or `libveneer_ffi.dylib`).

## Usage

```python
from playwright.sync_api import sync_playwright
import veneer
from veneer.playwright import launch_with_profile, verify_page

profile = veneer.generate(os="macos", browser="chrome", locale="tr", seed="session-42")

with sync_playwright() as p:
    browser, context, page = launch_with_profile(p, profile)
    page.goto("https://example.com")
    print(verify_page(page, profile)["valid"])
    browser.close()
```

The async API has the same functions in `veneer.async_playwright`. Core functions: `generate`, `validate`, `to_init_script`, `navigation_headers`, `catalog`, `collector_script`, `compare_observed`, `context_options`, `user_agent_override`.

## Tests

```bash
cd bindings/python && python -m pytest -q
```
