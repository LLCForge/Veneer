package veneerrod

import (
	"net/http"
	"net/http/httptest"
	"os"
	"sync"
	"testing"

	"github.com/go-rod/rod"
	"github.com/go-rod/rod/lib/launcher"
	"github.com/go-rod/rod/lib/proto"

	veneer "github.com/LLCForge/Veneer/bindings/go"
)

// Needs Chrome/Chromium: set VENEER_CHROME to its executable, or let rod find/download one.
func TestApplyInRealBrowser(t *testing.T) {
	var mu sync.Mutex
	var seen []http.Header
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		mu.Lock()
		seen = append(seen, r.Header.Clone())
		mu.Unlock()
		w.Header().Set("content-type", "text/html")
		_, _ = w.Write([]byte("<!doctype html><title>veneer</title>"))
	}))
	defer srv.Close()

	l := launcher.New().Headless(true)
	if path := os.Getenv("VENEER_CHROME"); path != "" {
		l = l.Bin(path)
	}
	u, err := l.Launch()
	if err != nil {
		t.Skipf("no browser: %v", err)
	}
	browser := rod.New().ControlURL(u)
	if err := browser.Connect(); err != nil {
		t.Fatal(err)
	}
	defer browser.Close()

	for _, c := range []veneer.Constraints{
		{"os": "windows", "browser": "chrome", "seed": "rod-e2e"},
		{"os": "android", "browser": "chrome", "seed": "rod-e2e"},
	} {
		p, err := veneer.Generate(c)
		if err != nil {
			t.Fatal(err)
		}
		mu.Lock()
		seen = nil
		mu.Unlock()
		page, err := browser.Page(proto.TargetCreateTarget{})
		if err != nil {
			t.Fatal(err)
		}
		if err := Apply(page, p); err != nil {
			t.Fatal(err)
		}
		if err := page.Navigate(srv.URL); err != nil {
			t.Fatal(err)
		}
		if err := page.WaitLoad(); err != nil {
			t.Fatal(err)
		}
		report, err := Verify(page, p)
		if err != nil {
			t.Fatal(err)
		}
		if !report.Valid {
			t.Fatalf("%v: live comparison failed: %+v", c, report.Issues)
		}
		headers, err := veneer.NavigationHeaders(p)
		if err != nil {
			t.Fatal(err)
		}
		mu.Lock()
		got := seen[0]
		mu.Unlock()
		for _, h := range headers {
			if h.Name == "user-agent" || h.Name == "accept-language" || h.Name == "sec-ch-ua" || h.Name == "sec-ch-ua-mobile" || h.Name == "sec-ch-ua-platform" {
				if got.Get(h.Name) != h.Value {
					t.Errorf("%v: header %s = %q, want %q", c, h.Name, got.Get(h.Name), h.Value)
				}
			}
		}
		_ = page.Close()
	}
}
