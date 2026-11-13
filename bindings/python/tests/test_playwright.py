"""Real-browser check: needs Playwright for Python and its Chromium (`python -m playwright install chromium`)."""

import asyncio
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest

import veneer

sync_api = pytest.importorskip("playwright.sync_api")


class _Server:
    def __init__(self):
        self.seen: list[dict[str, str]] = []
        seen = self.seen

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                seen.append({"path": self.path, **{k.lower(): v for k, v in self.headers.items()}})
                body = b"<!doctype html><title>veneer</title><p>ok</p>"
                self.send_response(200)
                self.send_header("content-type", "text/html")
                self.send_header("content-length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

            def log_message(self, *args):
                pass

        self.httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.url = f"http://127.0.0.1:{self.httpd.server_address[1]}/"
        threading.Thread(target=self.httpd.serve_forever, daemon=True).start()

    def close(self):
        self.httpd.shutdown()


@pytest.fixture()
def server():
    s = _Server()
    yield s
    s.close()


def _launch_or_skip(launch):
    try:
        return launch()
    except sync_api.Error as e:
        if "Executable doesn't exist" in str(e):
            pytest.skip("Playwright Chromium is not installed")
        raise


@pytest.mark.parametrize("constraints", [{"os": "windows", "browser": "chrome"}, {"os": "android", "browser": "chrome"}])
def test_sync_profile_applies(server, constraints):
    from veneer.playwright import launch_with_profile, verify_page

    profile = veneer.generate(constraints, seed="py-e2e")
    with sync_api.sync_playwright() as p:
        browser, context, page = _launch_or_skip(lambda: launch_with_profile(p, profile))
        try:
            page.goto(server.url)
            report = verify_page(page, profile)
            assert report["valid"], report["issues"]
            # A popup gets the CDP override through the context "page" handler. Its very first
            # request can race the handler, so it opens blank and navigates afterwards.
            with page.expect_popup() as popup_info:
                page.evaluate("() => { window.open('about:blank'); }")
            popup = popup_info.value
            popup.wait_for_timeout(200)
            popup.goto(server.url + "popup")
        finally:
            browser.close()

    expected = {
        h["name"]: h["value"]
        for h in veneer.navigation_headers(profile)
        if h["name"] in ("user-agent", "accept-language") or h["name"].startswith("sec-ch-ua")
    }
    assert len(server.seen) >= 2  # main page and popup
    for request in server.seen:
        for name, value in expected.items():
            assert request.get(name) == value, (request["path"], name)


def test_async_profile_applies(server):
    from playwright.async_api import async_playwright

    from veneer.async_playwright import launch_with_profile, verify_page

    profile = veneer.generate(os="macos", browser="chrome", seed="py-async")

    async def run():
        async with async_playwright() as p:
            try:
                browser, _, page = await launch_with_profile(p, profile)
            except Exception as e:
                if "Executable doesn't exist" in str(e):
                    pytest.skip("Playwright Chromium is not installed")
                raise
            try:
                await page.goto(server.url)
                return await verify_page(page, profile)
            finally:
                await browser.close()

    report = asyncio.run(run())
    assert report["valid"], report["issues"]
    assert server.seen[0]["user-agent"] == profile["navigator"]["userAgent"]
