# Veneer for Go

Go bindings for [Veneer](../../README.md) through cgo and the C ABI library from [`crates/veneer-ffi`](../../crates/veneer-ffi).

| Module | Purpose |
| --- | --- |
| `github.com/LLCForge/Veneer/bindings/go` | `Generate`, `Validate`, `InitScript`, `NavigationHeaders`, `Catalog`, `CollectorScript`, `CompareObserved` |
| `.../bindings/go/veneerchromedp` | chromedp: `Apply(profile)` and `Verify(profile, &report)` actions |
| `.../bindings/go/veneerrod` | rod helpers |

## Setup

```bash
cargo build -p veneer-ffi --release
```

Linux and macOS: set `CGO_LDFLAGS=-L<repo>/target/release` and the library path (`LD_LIBRARY_PATH` / `DYLD_LIBRARY_PATH`). Windows: put `veneer_ffi.dll` next to the executable or on `PATH`.

## Usage

```go
profile, err := veneer.Generate(veneer.Constraints{"os": "windows", "browser": "chrome", "seed": "session-42"})
if err != nil {
	return err
}
var report veneer.Report
err = chromedp.Run(ctx,
	veneerchromedp.Apply(profile),
	chromedp.Navigate("https://example.com"),
	veneerchromedp.Verify(profile, &report),
)
```

## Tests

```bash
cd bindings/go && go test ./...
```
