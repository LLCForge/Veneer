// Package veneerrod applies a Veneer profile to a go-rod page.
//
//	p, _ := veneer.Generate(veneer.Constraints{"os": "windows", "browser": "chrome"})
//	page := browser.MustPage()
//	if err := veneerrod.Apply(page, p); err != nil { ... }
//	page.MustNavigate(url)
package veneerrod

import (
	"encoding/json"
	"errors"

	"github.com/go-rod/rod"
	"github.com/go-rod/rod/lib/proto"

	veneer "github.com/LLCForge/Veneer/bindings/go"
)

// UserAgentOverride converts the profile's override into rod parameters, including client-hint
// metadata for Chromium-based profiles. rod's protocol types have no formFactors field, so
// Sec-CH-UA-Form-Factors keeps the browser default.
func UserAgentOverride(p *veneer.Profile) *proto.EmulationSetUserAgentOverride {
	o := p.Override()
	params := &proto.EmulationSetUserAgentOverride{
		UserAgent:      o.UserAgent,
		AcceptLanguage: o.AcceptLanguage,
		Platform:       o.Platform,
	}
	if u := o.UserAgentMetadata; u != nil {
		params.UserAgentMetadata = &proto.EmulationUserAgentMetadata{
			Brands:          brands(u.Brands),
			FullVersionList: brands(u.FullVersionList),
			Platform:        u.Platform,
			PlatformVersion: u.PlatformVersion,
			Architecture:    u.Architecture,
			Model:           u.Model,
			Mobile:          u.Mobile,
			Bitness:         u.Bitness,
			Wow64:           u.Wow64,
		}
	}
	return params
}

func brands(in []veneer.Brand) []*proto.EmulationUserAgentBrandVersion {
	out := make([]*proto.EmulationUserAgentBrandVersion, len(in))
	for i, b := range in {
		out[i] = &proto.EmulationUserAgentBrandVersion{Brand: b.Brand, Version: b.Version}
	}
	return out
}

// Apply configures the page before its first navigation: user agent with client hints and
// Accept-Language, timezone, locale, viewport/screen/scale, touch and the init script.
func Apply(page *rod.Page, p *veneer.Profile) error {
	if page == nil || p == nil {
		return errors.New("veneerrod: page and profile must not be nil")
	}
	script, err := veneer.InitScript(p)
	if err != nil {
		return err
	}
	if err := UserAgentOverride(p).Call(page); err != nil {
		return err
	}
	if err := (proto.EmulationSetTimezoneOverride{TimezoneID: p.Timezone}).Call(page); err != nil {
		return err
	}
	if err := (proto.EmulationSetLocaleOverride{Locale: p.Locale}).Call(page); err != nil {
		return err
	}
	screenW, screenH := p.Screen.Width, p.Screen.Height
	metrics := proto.EmulationSetDeviceMetricsOverride{
		Width:             p.Viewport.Width,
		Height:            p.Viewport.Height,
		DeviceScaleFactor: p.Screen.DevicePixelRatio,
		Mobile:            p.Handheld(),
		ScreenWidth:       &screenW,
		ScreenHeight:      &screenH,
	}
	if err := metrics.Call(page); err != nil {
		return err
	}
	points := max(p.Navigator.MaxTouchPoints, 1)
	touch := proto.EmulationSetTouchEmulationEnabled{Enabled: p.Navigator.MaxTouchPoints > 0, MaxTouchPoints: &points}
	if err := touch.Call(page); err != nil {
		return err
	}
	_, err = page.EvalOnNewDocument(script)
	return err
}

// Verify runs the collector in the page and compares what it observes with the profile.
func Verify(page *rod.Page, p *veneer.Profile) (*veneer.Report, error) {
	expr, err := veneer.CollectorScript()
	if err != nil {
		return nil, err
	}
	// rod evaluates JS as a function, so the collector expression is wrapped in one.
	res, err := page.Evaluate(&rod.EvalOptions{JS: "() => " + expr, AwaitPromise: true, ByValue: true})
	if err != nil {
		return nil, err
	}
	observed, err := json.Marshal(res.Value)
	if err != nil {
		return nil, err
	}
	return veneer.CompareObserved(p, observed)
}
