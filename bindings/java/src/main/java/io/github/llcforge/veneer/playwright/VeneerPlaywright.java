package io.github.llcforge.veneer.playwright;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.microsoft.playwright.Browser;
import com.microsoft.playwright.BrowserContext;
import com.microsoft.playwright.BrowserType;
import com.microsoft.playwright.Page;
import com.microsoft.playwright.Playwright;
import com.microsoft.playwright.PlaywrightException;
import io.github.llcforge.veneer.Veneer;
import io.github.llcforge.veneer.VeneerException;
import java.util.Objects;

/**
 * Playwright for Java helpers, mirroring {@code newContext} / {@code newPage} / {@code
 * launchWithProfile} / {@code verifyPage} of the npm package.
 */
public final class VeneerPlaywright {
  /** Browser, context and page returned by {@link #launchWithProfile}. */
  public record Launched(Browser browser, BrowserContext context, Page page) {}

  private VeneerPlaywright() {}

  /** Browser type whose engine matches the profile: chromium, firefox or webkit. */
  public static BrowserType browserType(Playwright playwright, ObjectNode profile) {
    Objects.requireNonNull(playwright, "playwright must not be null");
    String engine = profile.path("browser").path("engine").asText();
    return switch (engine) {
      case "chromium" -> playwright.chromium();
      case "gecko" -> playwright.firefox();
      case "webkit" -> playwright.webkit();
      default -> throw new VeneerException("unknown engine " + engine);
    };
  }

  /** Options for {@link Browser#newContext(Browser.NewContextOptions)}. */
  public static Browser.NewContextOptions contextOptions(ObjectNode profile) {
    Objects.requireNonNull(profile, "profile must not be null");
    JsonNode screen = profile.path("screen");
    JsonNode viewport = profile.path("viewport");
    Browser.NewContextOptions options =
        new Browser.NewContextOptions()
            .setUserAgent(profile.path("navigator").path("userAgent").asText())
            .setLocale(profile.path("locale").asText())
            .setTimezoneId(profile.path("timezone").asText())
            .setViewportSize(viewport.path("width").asInt(), viewport.path("height").asInt())
            .setScreenSize(screen.path("width").asInt(), screen.path("height").asInt())
            .setDeviceScaleFactor(screen.path("devicePixelRatio").asDouble(1));
    // Playwright rejects isMobile on Firefox.
    if (Veneer.isHandheld(profile) && !profile.path("browser").path("engine").asText().equals("gecko")) {
      options.setIsMobile(true).setHasTouch(true);
    }
    return options;
  }

  private static boolean isChromium(Browser browser) {
    return browser != null && browser.browserType().name().equals("chromium");
  }

  private static JsonObject overrideArgs(ObjectNode profile) {
    return JsonParser.parseString(Veneer.userAgentOverride(profile).toString()).getAsJsonObject();
  }

  private static void applyCdp(BrowserContext context, Page page, JsonObject args) {
    context.newCDPSession(page).send("Emulation.setUserAgentOverride", args);
  }

  /**
   * A context configured for the profile. On Chromium, popups also get matching request headers via
   * CDP; a popup's very first request may race the override.
   */
  public static BrowserContext newContext(Browser browser, ObjectNode profile) {
    Objects.requireNonNull(browser, "browser must not be null");
    BrowserContext context = browser.newContext(contextOptions(profile));
    context.addInitScript(Veneer.initScript(profile));
    if (isChromium(browser)) {
      JsonObject args = overrideArgs(profile);
      context.onPage(
          page -> {
            try {
              applyCdp(context, page, args);
            } catch (PlaywrightException e) {
              // Page closed before the override could be applied.
            }
          });
    }
    return context;
  }

  /** A page whose request headers are overridden before its first navigation. */
  public static Page newPage(BrowserContext context, ObjectNode profile) {
    Objects.requireNonNull(context, "context must not be null");
    Page page = context.newPage();
    if (isChromium(context.browser())) {
      applyCdp(context, page, overrideArgs(profile));
    }
    return page;
  }

  /** Launches the engine that matches the profile and returns browser, context and page. */
  public static Launched launchWithProfile(
      Playwright playwright, ObjectNode profile, BrowserType.LaunchOptions launchOptions) {
    Browser browser = browserType(playwright, profile).launch(launchOptions);
    try {
      BrowserContext context = newContext(browser, profile);
      return new Launched(browser, context, newPage(context, profile));
    } catch (RuntimeException e) {
      browser.close();
      throw e;
    }
  }

  /** Runs the collector in the live page and compares it with the profile. */
  public static Veneer.Report verifyPage(Page page, ObjectNode profile) {
    Objects.requireNonNull(page, "page must not be null");
    String expression =
        "(async () => JSON.stringify(await (" + Veneer.collectorScript() + ")))()";
    Object observed = page.evaluate(expression);
    if (!(observed instanceof String json)) {
      throw new VeneerException("collector returned no JSON");
    }
    return Veneer.compareObserved(profile, json);
  }
}
