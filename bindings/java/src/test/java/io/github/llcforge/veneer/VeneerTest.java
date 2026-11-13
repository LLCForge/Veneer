package io.github.llcforge.veneer;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.Map;
import org.junit.jupiter.api.Test;

class VeneerTest {
  @Test
  void roundTrip() {
    ObjectNode p = Veneer.generate(Map.of("os", "windows", "browser", "chrome", "seed", "java"));
    assertEquals("java", p.get("seed").asText());
    Veneer.Report r = Veneer.validate(p);
    assertTrue(r.valid());
    assertTrue(r.issues().isEmpty());
    String ua = p.path("navigator").path("userAgent").asText();
    assertTrue(Veneer.initScript(p).contains(ua));
    assertTrue(Veneer.navigationHeaders(p).stream().anyMatch(h -> h.name().equals("sec-ch-ua")));
    assertTrue(Veneer.catalog().get("os").isArray());
    assertFalse(Veneer.collectorScript().isEmpty());
    assertFalse(Veneer.compareObserved(p, "{\"userAgent\":\"other\"}").valid());
  }

  @Test
  void determinismAndRandomSeed() {
    assertEquals(Veneer.generate(Map.of("seed", "same")), Veneer.generate(Map.of("seed", "same")));
    assertFalse(Veneer.generate().get("seed").asText().isEmpty());
  }

  @Test
  void overrides() {
    ObjectNode phone = Veneer.generate(Map.of("os", "android", "browser", "chrome", "seed", "p"));
    ObjectNode o = Veneer.userAgentOverride(phone);
    assertTrue(Veneer.isHandheld(phone));
    assertTrue(o.path("userAgentMetadata").path("mobile").asBoolean());
    assertFalse(o.get("acceptLanguage").asText().contains(";q="));
    ObjectNode fx = Veneer.generate(Map.of("browser", "firefox", "seed", "f"));
    assertNull(Veneer.userAgentOverride(fx).get("userAgentMetadata"));
  }

  @Test
  void errors() {
    assertThrows(VeneerException.class, () -> Veneer.generate(Map.of("os", "amiga")));
    VeneerException e =
        assertThrows(VeneerException.class, () -> Veneer.generate(Map.of("oss", "x")));
    assertTrue(e.getMessage().contains("invalid constraints"));
    assertThrows(
        VeneerException.class, () -> Veneer.validate(Veneer.MAPPER.createObjectNode().put("a", 1)));
    assertThrows(NullPointerException.class, () -> Veneer.validate(null));
  }
}
