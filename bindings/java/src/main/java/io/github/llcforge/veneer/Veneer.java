package io.github.llcforge.veneer;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.lang.foreign.Arena;
import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.Linker;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.SymbolLookup;
import java.lang.foreign.ValueLayout;
import java.lang.invoke.MethodHandle;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.UUID;

/**
 * Coherent browser fingerprint profiles through the Veneer C ABI ({@code veneer_ffi}).
 *
 * <p>Profiles are Jackson {@link ObjectNode}s exactly as the core produced them, so nothing is lost
 * when they are passed back. The library is loaded from {@code VENEER_FFI_LIB} if set, otherwise by
 * name from the system library path. Run the JVM with {@code --enable-native-access=ALL-UNNAMED}.
 */
public final class Veneer {
  /** One request header. */
  public record Header(String name, String value) {}

  /** One consistency finding. */
  public record Issue(String rule, String severity, String message) {}

  /** Result of {@link #validate} and {@link #compareObserved}. */
  public record Report(boolean valid, int score, List<Issue> issues) {}

  // Unknown fields are ignored so reports keep parsing when the core adds fields.
  static final ObjectMapper MAPPER =
      new ObjectMapper().configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);

  private static final MethodHandle GENERATE;
  private static final MethodHandle VALIDATE;
  private static final MethodHandle INIT_SCRIPT;
  private static final MethodHandle HEADERS;
  private static final MethodHandle CATALOG;
  private static final MethodHandle COLLECTOR;
  private static final MethodHandle COMPARE;
  private static final MethodHandle FREE;

  static {
    String explicit = System.getenv("VENEER_FFI_LIB");
    SymbolLookup lib =
        explicit != null && !explicit.isEmpty()
            ? SymbolLookup.libraryLookup(Path.of(explicit), Arena.global())
            : SymbolLookup.libraryLookup(System.mapLibraryName("veneer_ffi"), Arena.global());
    Linker linker = Linker.nativeLinker();
    FunctionDescriptor one = FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS);
    FunctionDescriptor none = FunctionDescriptor.of(ValueLayout.ADDRESS);
    GENERATE = handle(linker, lib, "veneer_generate", one);
    VALIDATE = handle(linker, lib, "veneer_validate", one);
    INIT_SCRIPT = handle(linker, lib, "veneer_init_script", one);
    HEADERS = handle(linker, lib, "veneer_navigation_headers", one);
    CATALOG = handle(linker, lib, "veneer_catalog", none);
    COLLECTOR = handle(linker, lib, "veneer_collector_script", none);
    COMPARE =
        handle(
            linker,
            lib,
            "veneer_compare_observed",
            FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS));
    FREE = handle(linker, lib, "veneer_free", FunctionDescriptor.ofVoid(ValueLayout.ADDRESS));
  }

  private Veneer() {}

  private static MethodHandle handle(
      Linker linker, SymbolLookup lib, String name, FunctionDescriptor descriptor) {
    MemorySegment symbol =
        lib.find(name).orElseThrow(() -> new UnsatisfiedLinkError("veneer_ffi has no " + name));
    return linker.downcallHandle(symbol, descriptor);
  }

  private static JsonNode call(MethodHandle fn, String... args) {
    String text;
    try (Arena arena = Arena.ofConfined()) {
      MemorySegment result;
      try {
        result =
            switch (args.length) {
              case 0 -> (MemorySegment) fn.invokeExact();
              case 1 -> (MemorySegment) fn.invokeExact(arena.allocateFrom(args[0]));
              default ->
                  (MemorySegment) fn.invokeExact(arena.allocateFrom(args[0]), arena.allocateFrom(args[1]));
            };
      } catch (Throwable t) {
        throw new VeneerException("native call failed: " + t, t);
      }
      if (result.equals(MemorySegment.NULL)) {
        throw new VeneerException("result allocation failed");
      }
      try {
        // The envelope is NUL-terminated; its length is unknown until read.
        text = result.reinterpret(Long.MAX_VALUE).getString(0);
      } finally {
        try {
          FREE.invokeExact(result);
        } catch (Throwable t) {
          throw new VeneerException("veneer_free failed: " + t, t);
        }
      }
    }
    JsonNode envelope;
    try {
      envelope = MAPPER.readTree(text);
    } catch (JsonProcessingException e) {
      throw new VeneerException("malformed result", e);
    }
    if (envelope.has("error")) {
      throw new VeneerException(envelope.get("error").asText());
    }
    return envelope.get("ok");
  }

  private static String encode(JsonNode node, String what) {
    Objects.requireNonNull(node, what + " must not be null");
    // JSON escapes U+0000, so the C string is never truncated.
    return node.toString();
  }

  private static <T> T convert(JsonNode node, TypeReference<T> type) {
    try {
      return MAPPER.treeToValue(node, MAPPER.getTypeFactory().constructType(type));
    } catch (JsonProcessingException e) {
      throw new VeneerException("unexpected result shape", e);
    }
  }

  /**
   * Generates a coherent profile. Keys use the core's camelCase names, e.g. {@code os}, {@code
   * browser}, {@code minVersion}, {@code seed}. Without a seed a random one is stored in the profile.
   */
  public static ObjectNode generate(Map<String, ?> constraints) {
    ObjectNode merged =
        constraints == null ? MAPPER.createObjectNode() : MAPPER.valueToTree(constraints);
    if (!merged.hasNonNull("seed")) {
      merged.put("seed", UUID.randomUUID().toString());
    }
    return (ObjectNode) call(GENERATE, merged.toString());
  }

  /** Generates a profile without constraints and with a random seed. */
  public static ObjectNode generate() {
    return generate(null);
  }

  /** Checks a profile against every consistency rule. */
  public static Report validate(ObjectNode profile) {
    return convert(call(VALIDATE, encode(profile, "profile")), new TypeReference<Report>() {});
  }

  /** Init script to run before any page script. */
  public static String initScript(ObjectNode profile) {
    return call(INIT_SCRIPT, encode(profile, "profile")).asText();
  }

  /** Navigation request headers in the order the profiled browser sends them. */
  public static List<Header> navigationHeaders(ObjectNode profile) {
    return convert(call(HEADERS, encode(profile, "profile")), new TypeReference<List<Header>>() {});
  }

  /** Every OS, browser, device and locale the generator knows. */
  public static ObjectNode catalog() {
    return (ObjectNode) call(CATALOG);
  }

  /** Expression that resolves (as a promise) to what the page can observe. */
  public static String collectorScript() {
    return call(COLLECTOR).asText();
  }

  /** Compares collector output (JSON text) from a real browser against the profile. */
  public static Report compareObserved(ObjectNode profile, String observedJson) {
    Objects.requireNonNull(observedJson, "observed values must not be null");
    return convert(
        call(COMPARE, encode(profile, "profile"), observedJson), new TypeReference<Report>() {});
  }

  /**
   * CDP {@code Emulation.setUserAgentOverride} parameters: user agent, Accept-Language, platform
   * and, for browsers with client hints, {@code userAgentMetadata}.
   */
  public static ObjectNode userAgentOverride(ObjectNode profile) {
    Objects.requireNonNull(profile, "profile must not be null");
    JsonNode nav = profile.path("navigator");
    List<String> languages = new ArrayList<>();
    nav.path("languages").forEach(l -> languages.add(l.asText()));
    ObjectNode out = MAPPER.createObjectNode();
    out.put("userAgent", nav.path("userAgent").asText());
    // Chromium appends the q-values itself; passing the finished header would double them.
    out.put("acceptLanguage", String.join(",", languages));
    out.put("platform", nav.path("platform").asText());
    JsonNode u = nav.get("userAgentData");
    if (u != null && u.isObject()) {
      ObjectNode m = out.putObject("userAgentMetadata");
      m.set("brands", u.get("brands"));
      m.set("fullVersionList", u.get("fullVersionList"));
      m.set("fullVersion", u.get("uaFullVersion"));
      for (String key :
          List.of("platform", "platformVersion", "architecture", "model", "mobile", "bitness", "wow64")) {
        m.set(key, u.get(key));
      }
      m.set("formFactors", u.get("formFactors"));
    }
    return out;
  }

  /** {@code true} for phone and tablet profiles. */
  public static boolean isHandheld(ObjectNode profile) {
    String kind = profile.path("device").path("kind").asText();
    return kind.equals("phone") || kind.equals("tablet");
  }

  /** Converts a JSON node into plain maps, lists and primitives (for CDP argument maps). */
  public static Map<String, Object> toMap(ObjectNode node) {
    return MAPPER.convertValue(node, new TypeReference<Map<String, Object>>() {});
  }
}
