"""Playwright for Python + Veneer through the local HTTP API.

Usage:
    veneer serve --port 8787            # in another terminal
    pip install -r requirements.txt && playwright install chromium
    python main.py https://example.com/
"""

from __future__ import annotations

import json
import os
import sys
import urllib.parse
import urllib.request
from typing import Any

from playwright.sync_api import sync_playwright

API = os.environ.get("VENEER_API", "http://127.0.0.1:8787").rstrip("/")
TOKEN = os.environ.get("VENEER_TOKEN")  # only needed when the API was started with --token
TIMEOUT = 30


def api(path: str, body: dict[str, Any] | None = None) -> Any:
    data = json.dumps(body).encode() if body is not None else None
    request = urllib.request.Request(API + path, data=data, method="POST" if data else "GET")
    if data:
        request.add_header("content-type", "application/json")
    if TOKEN:
        request.add_header("authorization", f"Bearer {TOKEN}")
    with urllib.request.urlopen(request, timeout=TIMEOUT) as response:
        text = response.read().decode()
    return json.loads(text) if response.headers.get_content_type() == "application/json" else text


def user_agent_override(profile: dict[str, Any]) -> dict[str, Any]:
    """CDP Emulation.setUserAgentOverride parameters, the same as userAgentOverride() in the npm package."""
    nav = profile["navigator"]
    # Chromium adds the q-values itself, so it gets the plain list.
    params: dict[str, Any] = {"userAgent": nav["userAgent"], "acceptLanguage": ",".join(nav["languages"]), "platform": nav["platform"]}
    ch = nav.get("userAgentData")
    if ch:
        params["userAgentMetadata"] = {
            "brands": ch["brands"],
            "fullVersionList": ch["fullVersionList"],
            "fullVersion": ch["uaFullVersion"],
            "platform": ch["platform"],
            "platformVersion": ch["platformVersion"],
            "architecture": ch["architecture"],
            "model": ch["model"],
            "mobile": ch["mobile"],
            "bitness": ch["bitness"],
            "wow64": ch["wow64"],
            "formFactors": ch["formFactors"],
        }
    return params


def main() -> int:
    url = sys.argv[1] if len(sys.argv) > 1 else "https://example.com/"
    if not url.startswith(("https://", "http://")):
        print("the target must be an http(s) URL", file=sys.stderr)
        return 2

    query = urllib.parse.urlencode({"os": "windows", "browser": "chrome", "locale": "en-US", "include": "script"})
    fingerprint = api(f"/v1/fingerprint?{query}")["fingerprints"][0]
    profile, script = fingerprint["profile"], fingerprint["initScript"]
    webrtc = api("/v1/network/webrtc?engine=chromium&policy=public-interface-only")
    collector = api("/v1/collector.js").strip().rstrip(";")

    handheld = profile["device"]["kind"] in ("phone", "tablet")
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True, args=webrtc["args"])
        try:
            context = browser.new_context(
                user_agent=profile["navigator"]["userAgent"],
                locale=profile["locale"],
                timezone_id=profile["timezone"],
                viewport=profile["viewport"],
                screen={"width": profile["screen"]["width"], "height": profile["screen"]["height"]},
                device_scale_factor=profile["screen"]["devicePixelRatio"],
                is_mobile=handheld,
                has_touch=handheld,
            )
            context.add_init_script(script)
            page = context.new_page()
            context.new_cdp_session(page).send("Emulation.setUserAgentOverride", user_agent_override(profile))
            response = page.goto(url, wait_until="load")
            if response is None:
                raise RuntimeError(f"no response from {url}")
            # headers_array() is sorted by name, not wire order; an object makes the API skip the order check.
            sent = response.request.all_headers()
            page.evaluate(f"void ({collector})")
            observed = page.evaluate("window.veneerCollect()")
        finally:
            browser.close()

    page_report = api("/v1/compare", {"profile": profile, "observed": observed})
    header_report = api("/v1/headers/compare", {"profile": profile, "headers": sent})
    print(f"{profile['device']['id']} / {profile['browser']['name']} {profile['browser']['version']}")
    print("page:   ", json.dumps(page_report, indent=2))
    print("headers:", json.dumps(header_report, indent=2))
    return 0 if page_report["valid"] and header_report["valid"] else 1


if __name__ == "__main__":
    sys.exit(main())
