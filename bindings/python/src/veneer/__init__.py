"""Coherent browser fingerprint profiles, backed by the Veneer C ABI (``veneer_ffi``).

The shared library is located, in order, from the ``VENEER_FFI_LIB`` environment variable,
next to this package, or on the system library path.
"""

from __future__ import annotations

import ctypes
import ctypes.util
import json
import os
import sys
import uuid
from pathlib import Path
from typing import Any, Mapping

__all__ = [
    "VeneerError",
    "generate",
    "validate",
    "to_init_script",
    "catalog",
    "navigation_headers",
    "collector_script",
    "compare_observed",
    "user_agent_metadata",
    "user_agent_override",
    "context_options",
    "playwright_engine",
]

Profile = dict[str, Any]

_PLAYWRIGHT_ENGINES = {"chromium": "chromium", "gecko": "firefox", "webkit": "webkit"}


class VeneerError(Exception):
    """Raised for invalid input or unsatisfiable constraints."""


def _library_name() -> str:
    if sys.platform == "win32":
        return "veneer_ffi.dll"
    if sys.platform == "darwin":
        return "libveneer_ffi.dylib"
    return "libveneer_ffi.so"


def _load() -> ctypes.CDLL:
    explicit = os.environ.get("VENEER_FFI_LIB")
    if explicit:
        return ctypes.CDLL(explicit)
    bundled = Path(__file__).with_name(_library_name())
    if bundled.is_file():
        return ctypes.CDLL(str(bundled))
    found = ctypes.util.find_library("veneer_ffi")
    if found is None:
        raise OSError(
            "veneer_ffi shared library not found; build it with "
            "`cargo build -p veneer-ffi --release` and set VENEER_FFI_LIB to its path"
        )
    return ctypes.CDLL(found)


_lib = _load()
for _name, _argc in (
    ("veneer_generate", 1),
    ("veneer_validate", 1),
    ("veneer_init_script", 1),
    ("veneer_navigation_headers", 1),
    ("veneer_catalog", 0),
    ("veneer_collector_script", 0),
    ("veneer_compare_observed", 2),
):
    _fn = getattr(_lib, _name)
    _fn.argtypes = [ctypes.c_char_p] * _argc
    # c_void_p keeps the raw pointer so it can be handed back to veneer_free.
    _fn.restype = ctypes.c_void_p
_lib.veneer_free.argtypes = [ctypes.c_void_p]
_lib.veneer_free.restype = None


def _call(name: str, *args: bytes | None) -> Any:
    ptr = getattr(_lib, name)(*args)
    if not ptr:
        raise MemoryError(f"{name} could not allocate its result")
    try:
        envelope = json.loads(ctypes.string_at(ptr).decode("utf-8"))
    finally:
        _lib.veneer_free(ptr)
    if "error" in envelope:
        raise VeneerError(envelope["error"])
    return envelope["ok"]


def _encode(value: Mapping[str, Any], what: str) -> bytes:
    if not isinstance(value, Mapping):
        raise TypeError(f"{what} must be a mapping")
    return json.dumps(value, ensure_ascii=False).encode("utf-8")


def generate(constraints: Mapping[str, Any] | None = None, **kwargs: Any) -> Profile:
    """Generate a coherent profile. Without a seed, a random one is chosen and stored in ``profile["seed"]``."""
    merged = {**(constraints or {}), **kwargs}
    if merged.get("seed") is None:
        merged["seed"] = str(uuid.uuid4())
    return _call("veneer_generate", _encode(merged, "constraints"))


def validate(profile: Profile) -> dict[str, Any]:
    """Check a profile against every consistency rule: ``{valid, score, issues}``."""
    return _call("veneer_validate", _encode(profile, "profile"))


def to_init_script(profile: Profile) -> str:
    """Init script for Playwright ``add_init_script`` / CDP ``Page.addScriptToEvaluateOnNewDocument``."""
    return _call("veneer_init_script", _encode(profile, "profile"))


def catalog() -> dict[str, Any]:
    """Every OS, browser, device and locale the generator knows."""
    return _call("veneer_catalog")


def navigation_headers(profile: Profile) -> list[dict[str, str]]:
    """Navigation request headers in the order the profiled browser sends them."""
    return _call("veneer_navigation_headers", _encode(profile, "profile"))


def collector_script() -> str:
    """Expression for ``page.evaluate`` that resolves to what the page can observe."""
    return _call("veneer_collector_script")


def compare_observed(profile: Profile, observed: Mapping[str, Any]) -> dict[str, Any]:
    """Compare collector output from a real browser against the profile."""
    return _call(
        "veneer_compare_observed", _encode(profile, "profile"), _encode(observed, "observed values")
    )


def playwright_engine(profile: Profile) -> str:
    """Playwright browser type whose engine matches the profile: ``chromium``, ``firefox`` or ``webkit``."""
    return _PLAYWRIGHT_ENGINES[profile["browser"]["engine"]]


def context_options(profile: Profile) -> dict[str, Any]:
    """Keyword arguments for Playwright ``browser.new_context()``."""
    options: dict[str, Any] = {
        "user_agent": profile["navigator"]["userAgent"],
        "locale": profile["locale"],
        "timezone_id": profile["timezone"],
        "viewport": dict(profile["viewport"]),
        "screen": {"width": profile["screen"]["width"], "height": profile["screen"]["height"]},
        "device_scale_factor": profile["screen"]["devicePixelRatio"],
    }
    # Playwright rejects is_mobile on Firefox.
    if profile["device"]["kind"] in ("phone", "tablet") and profile["browser"]["engine"] != "gecko":
        options["is_mobile"] = True
        options["has_touch"] = True
    return options


def user_agent_metadata(profile: Profile) -> dict[str, Any] | None:
    """CDP ``UserAgentMetadata`` so Sec-CH-UA headers match, or ``None`` for browsers without client hints."""
    u = profile["navigator"].get("userAgentData")
    if not u:
        return None
    return {
        "brands": [dict(b) for b in u["brands"]],
        "fullVersionList": [dict(b) for b in u["fullVersionList"]],
        "fullVersion": u["uaFullVersion"],
        "platform": u["platform"],
        "platformVersion": u["platformVersion"],
        "architecture": u["architecture"],
        "model": u["model"],
        "mobile": u["mobile"],
        "bitness": u["bitness"],
        "wow64": u["wow64"],
        "formFactors": list(u["formFactors"]),
    }


def user_agent_override(profile: Profile) -> dict[str, Any]:
    """CDP ``Emulation.setUserAgentOverride`` parameters: user agent, Accept-Language, platform, client hints."""
    params: dict[str, Any] = {
        "userAgent": profile["navigator"]["userAgent"],
        # Chromium appends the q-values itself; passing the finished header would double them.
        "acceptLanguage": ",".join(profile["navigator"]["languages"]),
        "platform": profile["navigator"]["platform"],
    }
    metadata = user_agent_metadata(profile)
    if metadata is not None:
        params["userAgentMetadata"] = metadata
    return params
