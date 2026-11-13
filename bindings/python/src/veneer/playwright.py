"""Playwright (sync API) helpers. Mirrors ``newContext`` / ``newPage`` / ``launchWithProfile`` of the npm package."""

from __future__ import annotations

from typing import Any

from playwright.sync_api import Browser, BrowserContext, Error, Page, Playwright

from . import Profile, collector_script, compare_observed, context_options, playwright_engine
from . import to_init_script, user_agent_override

__all__ = ["new_context", "new_page", "launch_with_profile", "verify_page"]


def _is_chromium(browser: Browser | None) -> bool:
    return browser is not None and browser.browser_type.name == "chromium"


def _apply_cdp(context: BrowserContext, page: Page, override: dict[str, Any]) -> None:
    context.new_cdp_session(page).send("Emulation.setUserAgentOverride", override)


def new_context(browser: Browser, profile: Profile, **options: Any) -> BrowserContext:
    """A context configured for the profile. On Chromium, popups also get matching request headers via CDP.

    The override is applied asynchronously, so a popup's very first request may still carry the
    browser's own client hints; use :func:`new_page` for pages you open yourself.
    """
    context = browser.new_context(**{**context_options(profile), **options})
    context.add_init_script(to_init_script(profile))
    if _is_chromium(browser):
        override = user_agent_override(profile)

        def on_page(page: Page) -> None:
            try:
                _apply_cdp(context, page, override)
            except Error:
                pass  # Page closed before the override could be applied.

        context.on("page", on_page)
    return context


def new_page(context: BrowserContext, profile: Profile) -> Page:
    """A page whose request headers are overridden before its first navigation."""
    page = context.new_page()
    if _is_chromium(context.browser):
        _apply_cdp(context, page, user_agent_override(profile))
    return page


def launch_with_profile(
    playwright: Playwright, profile: Profile, **launch_options: Any
) -> tuple[Browser, BrowserContext, Page]:
    """Launch the engine that matches the profile and return ``(browser, context, page)``."""
    browser = getattr(playwright, playwright_engine(profile)).launch(**launch_options)
    try:
        context = new_context(browser, profile)
        return browser, context, new_page(context, profile)
    except BaseException:
        browser.close()
        raise


def verify_page(page: Page, profile: Profile) -> dict[str, Any]:
    """Run the collector in the live page and compare it with the profile."""
    return compare_observed(profile, page.evaluate(collector_script()))
