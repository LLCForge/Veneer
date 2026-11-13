package veneerchromedp

import (
	"context"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/chromedp/chromedp"

	veneer "github.com/LLCForge/Veneer/bindings/go"
)

// Needs Chrome/Chromium: set VENEER_CHROME to its executable, or have one on the default path.
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

	opts := append([]chromedp.ExecAllocatorOption{}, chromedp.DefaultExecAllocatorOptions[:]...)
	if path := os.Getenv("VENEER_CHROME"); path != "" {
		opts = append(opts, chromedp.ExecPath(path))
	}
	allocCtx, cancelAlloc := chromedp.NewExecAllocator(context.Background(), opts...)
	defer cancelAlloc()

	for _, c := range []veneer.Constraints{
		{"os": "windows", "browser": "chrome", "seed": "go-e2e"},
		{"os": "android", "browser": "chrome", "seed": "go-e2e"},
	} {
		p, err := veneer.Generate(c)
		if err != nil {
			t.Fatal(err)
		}
		ctx, cancel := chromedp.NewContext(allocCtx)
		ctx, cancelTimeout := context.WithTimeout(ctx, 60*time.Second)
		mu.Lock()
		seen = nil
		mu.Unlock()
		var report veneer.Report
		err = chromedp.Run(ctx, Apply(p), chromedp.Navigate(srv.URL), Verify(p, &report))
		cancelTimeout()
		cancel()
		if err != nil {
			if strings.Contains(err.Error(), "executable file not found") {
				t.Skip("no Chrome found; set VENEER_CHROME")
			}
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
			if h.Name == "user-agent" || h.Name == "accept-language" || strings.HasPrefix(h.Name, "sec-ch-ua") {
				if got.Get(h.Name) != h.Value {
					t.Errorf("%v: header %s = %q, want %q", c, h.Name, got.Get(h.Name), h.Value)
				}
			}
		}
	}
}
