import pytest

import veneer


def test_generate_validate_round_trip():
    profile = veneer.generate(os="windows", browser="chrome", seed="py")
    assert profile["seed"] == "py"
    report = veneer.validate(profile)
    assert report["valid"] and report["issues"] == []
    assert profile["navigator"]["userAgent"] in veneer.to_init_script(profile)
    names = [h["name"] for h in veneer.navigation_headers(profile)]
    assert "user-agent" in names and "sec-ch-ua" in names
    assert veneer.catalog()["os"]
    assert isinstance(veneer.collector_script(), str)


def test_random_seed_and_determinism():
    assert veneer.generate()["seed"]
    assert veneer.generate({"seed": "same"}) == veneer.generate(seed="same")


def test_errors_raise():
    with pytest.raises(veneer.VeneerError):
        veneer.generate(os="amiga")
    with pytest.raises(veneer.VeneerError, match="invalid constraints"):
        veneer.generate(oss="windows")
    with pytest.raises(veneer.VeneerError, match="invalid profile"):
        veneer.validate({"nope": 1})
    with pytest.raises(TypeError):
        veneer.validate("not a mapping")


def test_compare_detects_mismatch():
    profile = veneer.generate(seed="cmp")
    report = veneer.compare_observed(profile, {"userAgent": "something else"})
    assert not report["valid"]


def test_context_options_and_override_chromium_phone():
    profile = veneer.generate(os="android", browser="chrome", seed="phone")
    opts = veneer.context_options(profile)
    assert opts["is_mobile"] and opts["has_touch"]
    assert opts["user_agent"] == profile["navigator"]["userAgent"]
    override = veneer.user_agent_override(profile)
    assert override["userAgentMetadata"]["mobile"] is True
    assert override["acceptLanguage"]
    assert veneer.playwright_engine(profile) == "chromium"


def test_override_without_client_hints():
    profile = veneer.generate(browser="firefox", seed="fx")
    assert veneer.user_agent_metadata(profile) is None
    assert "userAgentMetadata" not in veneer.user_agent_override(profile)
    assert "is_mobile" not in veneer.context_options(profile)
    assert veneer.playwright_engine(profile) == "firefox"
