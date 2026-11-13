package io.github.llcforge.veneer;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.node.ObjectNode;
import com.microsoft.playwright.BrowserType;
import com.microsoft.playwright.Playwright;
import com.sun.net.httpserver.HttpServer;
import io.github.llcforge.veneer.playwright.VeneerPlaywright;
import io.github.llcforge.veneer.selenium.VeneerSelenium;
import java.io.IOException;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.concurrent.CopyOnWriteArrayList;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.condition.EnabledIfEnvironmentVariable;
import org.openqa.selenium.chrome.ChromeDriver;
import org.openqa.selenium.chrome.ChromeOptions;

/** Real-browser checks; enabled with VENEER_E2E=1 (CI). */
@EnabledIfEnvironmentVariable(named = "VENEER_E2E", matches = "1")
class BrowserTest {
  private HttpServer server;
  private final List<Map<String, String>> seen = new CopyOnWriteArrayList<>();

  @BeforeEach
  void start() throws IOException {
    server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
    server.createContext(
        "/",
        exchange -> {
          Map<String, String> headers = new java.util.HashMap<>();
          exchange
              .getRequestHeaders()
              .forEach((k, v) -> headers.put(k.toLowerCase(Locale.ROOT), String.join(",", v)));
          seen.add(headers);
          byte[] body = "<!doctype html><title>veneer</title>".getBytes(StandardCharsets.UTF_8);
          exchange.getResponseHeaders().set("content-type", "text/html");
          exchange.sendResponseHeaders(200, body.length);
          try (OutputStream out = exchange.getResponseBody()) {
            out.write(body);
          }
        });
    server.start();
  }

  @AfterEach
  void stop() {
    server.stop(0);
  }

  private String url() {
    return "http://127.0.0.1:" + server.getAddress().getPort() + "/";
  }

  private void assertHeaders(ObjectNode profile) {
    Map<String, String> first = seen.get(0);
    for (Veneer.Header h : Veneer.navigationHeaders(profile)) {
      if (h.name().equals("user-agent")
          || h.name().equals("accept-language")
          || h.name().startsWith("sec-ch-ua")) {
        assertEquals(h.value(), first.get(h.name()), h.name());
      }
    }
  }

  @Test
  void playwright() {
    for (String os : List.of("windows", "android")) {
      seen.clear();
      ObjectNode profile =
          Veneer.generate(Map.of("os", os, "browser", "chrome", "seed", "java-e2e"));
      try (Playwright pw = Playwright.create()) {
        VeneerPlaywright.Launched l =
            VeneerPlaywright.launchWithProfile(pw, profile, new BrowserType.LaunchOptions());
        try {
          l.page().navigate(url());
          Veneer.Report r = VeneerPlaywright.verifyPage(l.page(), profile);
          assertTrue(r.valid(), r.issues().toString());
        } finally {
          l.browser().close();
        }
      }
      assertHeaders(profile);
    }
  }

  @Test
  void selenium() {
    seen.clear();
    ObjectNode profile =
        Veneer.generate(Map.of("os", "windows", "browser", "chrome", "seed", "selenium-e2e"));
    ChromeDriver driver = new ChromeDriver(new ChromeOptions().addArguments("--headless=new"));
    try {
      VeneerSelenium.apply(driver, profile);
      driver.get(url());
      Veneer.Report r = VeneerSelenium.verify(driver, profile);
      assertTrue(r.valid(), r.issues().toString());
    } finally {
      driver.quit();
    }
    assertHeaders(profile);
  }
}
