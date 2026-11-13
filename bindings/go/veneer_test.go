package veneer

import (
	"encoding/json"
	"errors"
	"strings"
	"testing"
)

func TestRoundTrip(t *testing.T) {
	p, err := Generate(Constraints{"os": "windows", "browser": "chrome", "seed": "go"})
	if err != nil {
		t.Fatal(err)
	}
	if p.Seed != "go" || p.Browser.Engine != "chromium" || p.Navigator.UserAgentData == nil {
		t.Fatalf("unexpected profile: %+v", p)
	}
	r, err := Validate(p)
	if err != nil || !r.Valid || len(r.Issues) != 0 {
		t.Fatalf("validate: %v %+v", err, r)
	}
	script, err := InitScript(p)
	if err != nil || !strings.Contains(script, p.Navigator.UserAgent) {
		t.Fatalf("init script: %v", err)
	}
	h, err := NavigationHeaders(p)
	if err != nil || len(h) == 0 {
		t.Fatalf("headers: %v %v", err, h)
	}
	c, err := Catalog()
	if err != nil || c["os"] == nil {
		t.Fatalf("catalog: %v", err)
	}
	if s, err := CollectorScript(); err != nil || s == "" {
		t.Fatalf("collector: %v", err)
	}
	cmp, err := CompareObserved(p, json.RawMessage(`{"userAgent":"other"}`))
	if err != nil || cmp.Valid {
		t.Fatalf("compare: %v %+v", err, cmp)
	}
	o := p.Override()
	if o.AcceptLanguage != strings.Join(p.Navigator.Languages, ",") || o.UserAgentMetadata == nil {
		t.Fatalf("override: %+v", o)
	}
}

func TestLosslessJSON(t *testing.T) {
	a, err := Generate(Constraints{"seed": "same"})
	if err != nil {
		t.Fatal(err)
	}
	raw, err := json.Marshal(a)
	if err != nil {
		t.Fatal(err)
	}
	var b Profile
	if err := json.Unmarshal(raw, &b); err != nil {
		t.Fatal(err)
	}
	if r, err := Validate(&b); err != nil || !r.Valid {
		t.Fatalf("re-parsed profile: %v %+v", err, r)
	}
	if _, err := Validate(&Profile{}); err == nil {
		t.Fatal("hand-built profile without JSON must be rejected")
	}
	rnd, err := Generate(nil)
	if err != nil || rnd.Seed == "" {
		t.Fatalf("random seed: %v", err)
	}
}

func TestErrors(t *testing.T) {
	var ve *Error
	if _, err := Generate(Constraints{"os": "amiga"}); !errors.As(err, &ve) {
		t.Fatalf("want *Error, got %v", err)
	}
	if _, err := Generate(Constraints{"oss": "windows"}); err == nil || !strings.Contains(err.Error(), "invalid constraints") {
		t.Fatalf("unknown key: %v", err)
	}
	if _, err := Validate(nil); err == nil {
		t.Fatal("nil profile must fail")
	}
	p, _ := Generate(Constraints{"seed": "nul"})
	if _, err := CompareObserved(p, json.RawMessage("{\"a\":\"\x00\"}")); err == nil {
		t.Fatal("NUL byte must fail")
	}
}
