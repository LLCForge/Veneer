// Package veneer generates coherent browser fingerprint profiles through the Veneer C ABI
// (crates/veneer-ffi). Build the library with `cargo build -p veneer-ffi --release`, then
// point cgo at it, e.g. CGO_LDFLAGS="-L<repo>/target/release", and make it loadable at run
// time (PATH on Windows, LD_LIBRARY_PATH on Linux, DYLD_LIBRARY_PATH on macOS).
//
// Integrations live in the sibling modules veneerchromedp and veneerrod.
package veneer

/*
#cgo !windows LDFLAGS: -lveneer_ffi
// The release directory also holds an MSVC static veneer_ffi.lib that MinGW cannot link; name the DLL.
#cgo windows LDFLAGS: -l:veneer_ffi.dll
#include <stdlib.h>

char *veneer_generate(const char *constraints_json);
char *veneer_validate(const char *profile_json);
char *veneer_init_script(const char *profile_json);
char *veneer_navigation_headers(const char *profile_json);
char *veneer_catalog(void);
char *veneer_collector_script(void);
char *veneer_compare_observed(const char *profile_json, const char *observed_json);
void veneer_free(char *ptr);
*/
import "C"

import (
	"bytes"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"strings"
	"unsafe"
)

// Error is returned for invalid input or unsatisfiable constraints.
type Error struct{ Message string }

func (e *Error) Error() string { return "veneer: " + e.Message }

// Constraints narrow what Generate may pick, using the core's camelCase keys, e.g.
// {"os": "windows", "browser": "chrome", "minVersion": 140, "seed": "s1"}. Unknown keys are
// rejected by the core, so new constraint fields work without a binding release.
type Constraints map[string]any

// Brand is one client-hint brand entry.
type Brand struct {
	Brand   string `json:"brand"`
	Version string `json:"version"`
}

// UserAgentData mirrors navigator.userAgentData (Chromium-based browsers only).
type UserAgentData struct {
	Brands          []Brand  `json:"brands"`
	FullVersionList []Brand  `json:"fullVersionList"`
	UAFullVersion   string   `json:"uaFullVersion"`
	Platform        string   `json:"platform"`
	PlatformVersion string   `json:"platformVersion"`
	Architecture    string   `json:"architecture"`
	Model           string   `json:"model"`
	Mobile          bool     `json:"mobile"`
	Bitness         string   `json:"bitness"`
	Wow64           bool     `json:"wow64"`
	FormFactors     []string `json:"formFactors"`
}

// Profile is a typed, read-only view of a generated profile. The exact JSON produced by the
// core is kept and sent back unchanged, so fields not modelled here are never lost.
type Profile struct {
	Seed      string `json:"seed"`
	Locale    string `json:"locale"`
	Timezone  string `json:"timezone"`
	Navigator struct {
		UserAgent      string         `json:"userAgent"`
		Platform       string         `json:"platform"`
		Languages      []string       `json:"languages"`
		MaxTouchPoints int            `json:"maxTouchPoints"`
		UserAgentData  *UserAgentData `json:"userAgentData"`
	} `json:"navigator"`
	Screen struct {
		Width            int     `json:"width"`
		Height           int     `json:"height"`
		DevicePixelRatio float64 `json:"devicePixelRatio"`
	} `json:"screen"`
	Viewport struct {
		Width  int `json:"width"`
		Height int `json:"height"`
	} `json:"viewport"`
	Device struct {
		Kind  string `json:"kind"`
		Model string `json:"model"`
	} `json:"device"`
	Browser struct {
		ID     string `json:"id"`
		Engine string `json:"engine"`
	} `json:"browser"`

	raw json.RawMessage
}

type profileFields Profile

// UnmarshalJSON keeps the original document next to the typed fields.
func (p *Profile) UnmarshalJSON(data []byte) error {
	var f profileFields
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}
	*p = Profile(f)
	p.raw = append(json.RawMessage(nil), data...)
	return nil
}

// MarshalJSON returns the profile exactly as the core produced it.
func (p Profile) MarshalJSON() ([]byte, error) {
	if p.raw == nil {
		return nil, errors.New("veneer: profile was not produced by Generate or json.Unmarshal")
	}
	return p.raw, nil
}

// Handheld reports whether the profile is a phone or tablet.
func (p *Profile) Handheld() bool { return p.Device.Kind == "phone" || p.Device.Kind == "tablet" }

// Issue is one consistency finding.
type Issue struct {
	Rule     string `json:"rule"`
	Severity string `json:"severity"`
	Message  string `json:"message"`
}

// Report is the result of Validate and CompareObserved.
type Report struct {
	Valid  bool    `json:"valid"`
	Score  int     `json:"score"`
	Issues []Issue `json:"issues"`
}

// Header is one request header.
type Header struct {
	Name  string `json:"name"`
	Value string `json:"value"`
}

func call(out any, fn func(args []*C.char) *C.char, args ...[]byte) error {
	cargs := make([]*C.char, len(args))
	for i, a := range args {
		if bytes.IndexByte(a, 0) >= 0 {
			return &Error{Message: "argument contains a NUL byte"}
		}
		cargs[i] = C.CString(string(a))
	}
	defer func() {
		for _, c := range cargs {
			C.free(unsafe.Pointer(c))
		}
	}()
	ptr := fn(cargs)
	if ptr == nil {
		return errors.New("veneer: result allocation failed")
	}
	text := C.GoString(ptr)
	C.veneer_free(ptr)

	var env struct {
		Ok    json.RawMessage `json:"ok"`
		Error *string         `json:"error"`
	}
	if err := json.Unmarshal([]byte(text), &env); err != nil {
		return fmt.Errorf("veneer: malformed result: %w", err)
	}
	if env.Error != nil {
		return &Error{Message: *env.Error}
	}
	return json.Unmarshal(env.Ok, out)
}

func randomSeed() (string, error) {
	b := make([]byte, 16)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return hex.EncodeToString(b), nil
}

// Generate returns a coherent profile. Without a seed, a random one is chosen and stored in Profile.Seed.
func Generate(c Constraints) (*Profile, error) {
	merged := Constraints{}
	for k, v := range c {
		merged[k] = v
	}
	if v, ok := merged["seed"]; !ok || v == nil {
		seed, err := randomSeed()
		if err != nil {
			return nil, err
		}
		merged["seed"] = seed
	}
	in, err := json.Marshal(merged)
	if err != nil {
		return nil, err
	}
	var p Profile
	err = call(&p, func(a []*C.char) *C.char { return C.veneer_generate(a[0]) }, in)
	if err != nil {
		return nil, err
	}
	return &p, nil
}

func withProfile(p *Profile, out any, fn func(a []*C.char) *C.char, extra ...[]byte) error {
	if p == nil {
		return &Error{Message: "profile must not be nil"}
	}
	raw, err := p.MarshalJSON()
	if err != nil {
		return err
	}
	return call(out, fn, append([][]byte{raw}, extra...)...)
}

// Validate checks a profile against every consistency rule.
func Validate(p *Profile) (*Report, error) {
	var r Report
	return &r, withProfile(p, &r, func(a []*C.char) *C.char { return C.veneer_validate(a[0]) })
}

// InitScript returns the script to run before any page script.
func InitScript(p *Profile) (string, error) {
	var s string
	return s, withProfile(p, &s, func(a []*C.char) *C.char { return C.veneer_init_script(a[0]) })
}

// NavigationHeaders returns navigation request headers in the order the profiled browser sends them.
func NavigationHeaders(p *Profile) ([]Header, error) {
	var h []Header
	return h, withProfile(p, &h, func(a []*C.char) *C.char { return C.veneer_navigation_headers(a[0]) })
}

// Catalog returns every OS, browser, device and locale the generator knows.
func Catalog() (map[string]any, error) {
	var c map[string]any
	return c, call(&c, func([]*C.char) *C.char { return C.veneer_catalog() })
}

// CollectorScript returns an expression that resolves (as a promise) to what the page can observe.
func CollectorScript() (string, error) {
	var s string
	return s, call(&s, func([]*C.char) *C.char { return C.veneer_collector_script() })
}

// CompareObserved compares collector output (raw JSON) from a real browser against the profile.
func CompareObserved(p *Profile, observed json.RawMessage) (*Report, error) {
	var r Report
	return &r, withProfile(p, &r, func(a []*C.char) *C.char { return C.veneer_compare_observed(a[0], a[1]) }, observed)
}

// UserAgentOverride holds CDP Emulation.setUserAgentOverride parameters.
type UserAgentOverride struct {
	UserAgent         string         `json:"userAgent"`
	AcceptLanguage    string         `json:"acceptLanguage"`
	Platform          string         `json:"platform"`
	UserAgentMetadata *UserAgentData `json:"userAgentMetadata,omitempty"`
}

// Override returns the CDP user-agent override for the profile. UserAgentMetadata is nil for
// browsers without client hints. Chromium appends q-values to AcceptLanguage itself, so it is the
// plain language list.
func (p *Profile) Override() UserAgentOverride {
	return UserAgentOverride{
		UserAgent:         p.Navigator.UserAgent,
		AcceptLanguage:    strings.Join(p.Navigator.Languages, ","),
		Platform:          p.Navigator.Platform,
		UserAgentMetadata: p.Navigator.UserAgentData,
	}
}
