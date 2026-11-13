package io.github.llcforge.veneer.selenium;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import io.github.llcforge.veneer.Veneer;
import io.github.llcforge.veneer.VeneerException;
import java.util.Map;
import java.util.Objects;
import org.openqa.selenium.JavascriptExecutor;
import org.openqa.selenium.WebDriver;
import org.openqa.selenium.chromium.ChromiumDriver;

/**
 * Selenium helpers. The profile is applied through CDP, so it needs a Chromium-based driver
 * (ChromeDriver or EdgeDriver) and a Chromium-engine profile. CDP overrides apply to the current
 * tab only; call {@link #apply} again after switching to a new window.
 */
public final class VeneerSelenium {
  private VeneerSelenium() {}

  /**
   * Applies the profile to the current tab before navigating: user agent with client hints and
   * Accept-Language, timezone, locale, viewport/screen/scale, touch and the init script.
   */
  public static void apply(ChromiumDriver driver, ObjectNode profile) {
    Objects.requireNonNull(driver, "driver must not be null");
    Objects.requireNonNull(profile, "profile must not be null");
    if (!profile.path("browser").path("engine").asText().equals("chromium")) {
      throw new VeneerException("Selenium CDP needs a Chromium-engine profile");
    }
    String script = Veneer.initScript(profile);
    JsonNode screen = profile.path("screen");
    JsonNode viewport = profile.path("viewport");
    int touchPoints = profile.path("navigator").path("maxTouchPoints").asInt();

    driver.executeCdpCommand(
        "Emulation.setUserAgentOverride", Veneer.toMap(Veneer.userAgentOverride(profile)));
    driver.executeCdpCommand(
        "Emulation.setTimezoneOverride", Map.of("timezoneId", profile.path("timezone").asText()));
    driver.executeCdpCommand(
        "Emulation.setLocaleOverride", Map.of("locale", profile.path("locale").asText()));
    driver.executeCdpCommand(
        "Emulation.setDeviceMetricsOverride",
        Map.of(
            "width", viewport.path("width").asInt(),
            "height", viewport.path("height").asInt(),
            "deviceScaleFactor", screen.path("devicePixelRatio").asDouble(1),
            "mobile", Veneer.isHandheld(profile),
            "screenWidth", screen.path("width").asInt(),
            "screenHeight", screen.path("height").asInt()));
    driver.executeCdpCommand(
        "Emulation.setTouchEmulationEnabled",
        Map.of("enabled", touchPoints > 0, "maxTouchPoints", Math.max(touchPoints, 1)));
    driver.executeCdpCommand("Page.addScriptToEvaluateOnNewDocument", Map.of("source", script));
  }

  /** Runs the collector in the current page and compares it with the profile. */
  public static Veneer.Report verify(WebDriver driver, ObjectNode profile) {
    if (!(driver instanceof JavascriptExecutor js)) {
      throw new VeneerException("driver cannot execute JavaScript");
    }
    // WebDriver awaits a returned promise.
    Object observed =
        js.executeScript(
            "return (async () => JSON.stringify(await (" + Veneer.collectorScript() + ")))()");
    if (!(observed instanceof String json)) {
      throw new VeneerException("collector returned no JSON");
    }
    return Veneer.compareObserved(profile, json);
  }
}
