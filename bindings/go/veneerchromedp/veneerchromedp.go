// Package veneerchromedp applies a Veneer profile to a chromedp tab.
//
//	p, _ := veneer.Generate(veneer.Constraints{"os": "windows", "browser": "chrome"})
//	err := chromedp.Run(ctx, veneerchromedp.Apply(p), chromedp.Navigate(url))
package veneerchromedp

import (
	"context"
	"encoding/json"
	"errors"

	"github.com/chromedp/cdproto/emulation"
	"github.com/chromedp/cdproto/page"
	"github.com/chromedp/cdproto/runtime"
	"github.com/chromedp/chromedp"

	veneer "github.com/LLCForge/Veneer/bindings/go"
)

// UserAgentOverride converts the profile's override into cdproto parameters, including
// client-hint metadata for Chromium-based profiles.
func UserAgentOverride(p *veneer.Profile) *emulation.SetUserAgentOverrideParams {
	o := p.Override()
	params := emulation.SetUserAgentOverride(o.UserAgent).
		WithAcceptLanguage(o.AcceptLanguage).
		WithPlatform(o.Platform)
	if u := o.UserAgentMetadata; u != nil {
		params = params.WithUserAgentMetadata(&emulation.UserAgentMetadata{
			Brands:          brands(u.Brands),
			FullVersionList: brands(u.FullVersionList),
			Platform:        u.Platform,
			PlatformVersion: u.PlatformVersion,
			Architecture:    u.Architecture,
			Model:           u.Model,
			Mobile:          u.Mobile,
			Bitness:         u.Bitness,
			Wow64:           u.Wow64,
			FormFactors:     append([]string(nil), u.FormFactors...),
		})
	}
	return params
}

func brands(in []veneer.Brand) []*emulation.UserAgentBrandVersion {
	out := make([]*emulation.UserAgentBrandVersion, len(in))
	for i, b := range in {
		out[i] = &emulation.UserAgentBrandVersion{Brand: b.Brand, Version: b.Version}
	}
	return out
}

// Apply configures the current tab before its first navigation: user agent with client hints
// and Accept-Language, timezone, locale, viewport/screen/scale, touch and the init script.
// It affects only this target; apply it again to tabs you open later.
func Apply(p *veneer.Profile) chromedp.Action {
	return chromedp.ActionFunc(func(ctx context.Context) error {
		if p == nil {
			return errors.New("veneerchromedp: profile must not be nil")
		}
		script, err := veneer.InitScript(p)
		if err != nil {
			return err
		}
		metrics := emulation.SetDeviceMetricsOverride(
			int64(p.Viewport.Width), int64(p.Viewport.Height), p.Screen.DevicePixelRatio, p.Handheld(),
		).WithScreenWidth(int64(p.Screen.Width)).WithScreenHeight(int64(p.Screen.Height))
		return chromedp.Tasks{
			UserAgentOverride(p),
			emulation.SetTimezoneOverride(p.Timezone),
			emulation.SetLocaleOverride().WithLocale(p.Locale),
			metrics,
			emulation.SetTouchEmulationEnabled(p.Navigator.MaxTouchPoints > 0).
				WithMaxTouchPoints(int64(max(p.Navigator.MaxTouchPoints, 1))),
			chromedp.ActionFunc(func(ctx context.Context) error {
				_, err := page.AddScriptToEvaluateOnNewDocument(script).Do(ctx)
				return err
			}),
		}.Do(ctx)
	})
}

// Verify runs the collector in the current page and stores the comparison with the profile in report.
func Verify(p *veneer.Profile, report *veneer.Report) chromedp.Action {
	return chromedp.ActionFunc(func(ctx context.Context) error {
		expr, err := veneer.CollectorScript()
		if err != nil {
			return err
		}
		var observed []byte
		await := func(e *runtime.EvaluateParams) *runtime.EvaluateParams { return e.WithAwaitPromise(true) }
		if err := chromedp.Evaluate(expr, &observed, await).Do(ctx); err != nil {
			return err
		}
		r, err := veneer.CompareObserved(p, json.RawMessage(observed))
		if err != nil {
			return err
		}
		*report = *r
		return nil
	})
}
