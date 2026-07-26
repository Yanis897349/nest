# nest

> A browser-backed terminal interface for browsing and exporting Beijing Ziroom rentals.

`nest` discovers Ziroom's current filters, walks listing pages sequentially, decodes sprite-based
rent prices conservatively, and exports stable JSON or CSV data. It uses a temporary
Chromium-family browser profile because Ziroom renders results dynamically and may present a
browser challenge.

## Highlights

- Region, subway, commute, rental-type, rent, bedroom, and sort filters
- Headless browsing with a visible recovery flow for browser challenges
- Bounded retries, navigation timeouts, sequential requests, cancellation, and pagination guards
- Lazy detail loading without disturbing the active search page
- Stable schema-versioned JSON and CSV exports that never overwrite existing files
- Strict HTTPS Ziroom URL boundaries and bounded price-sprite downloads
- Optional newline-delimited JSON diagnostics with sensitive search data excluded
- No login, credentials, database, persistent browser profile, or CAPTCHA automation

## Requirements and support

- macOS or Linux on x86_64 or ARM64
- Google Chrome, Chromium, Brave, Microsoft Edge, or Helium
- A UTF-8 terminal capable of displaying Chinese labels

Windows browser discovery remains available on a best-effort basis but is not a release-blocking
platform. Chromium is not bundled with release archives.

## Installation

Download the archive for your operating system and architecture from GitHub Releases, verify it
against `SHA256SUMS`, and place `nest` on your `PATH`. Release binaries are currently unsigned and
macOS binaries are not notarized, so review your operating system's warning before choosing to run
one.

To build from source, install [Rustup](https://rustup.rs/) and run:

```shell
git clone https://github.com/Yanis897349/nest.git
cd nest
cargo build --locked --release
./target/release/nest
```

The repository pins Rust 1.90.0 and the required `rustfmt` and `clippy` components through
`rust-toolchain.toml`.

## Usage

Run with the default headless browser:

```shell
nest
```

Useful options:

```shell
nest --browser "/Applications/Helium.app" --headed
nest --output-dir ./exports --navigation-timeout 60
nest --log-file ./logs/nest.jsonl --log-level debug
```

Environment equivalents are available for browser and diagnostics:

```text
NEST_BROWSER
NEST_LOG_FILE
NEST_LOG_LEVEL=error|warn|info|debug|trace
```

Use `nest --help` for the complete CLI and `nest --version` for the installed version.

## Keyboard controls

| Key | Action |
| --- | --- |
| `←` / `→`, `Tab` | Change filter group |
| `↑` / `↓` | Move through options or results |
| `Space`, `Enter` | Apply the focused filter using Ziroom's current link |
| `p` | Enter a custom rent range |
| `a` | Enter a commute address and bus-time limit |
| `s`, `c` | Scrape every result page / cancel |
| `d`, `o` | Load a selected detail page / open it in the system browser |
| `e` | Export all card-level results as JSON or CSV |
| `r` | Reset or return to filters |
| `?`, `q` | Help / quit |

Region mode exposes districts and the business areas returned for the selected district. Subway
mode similarly exposes current lines and stations. Bedrooms and business areas or stations use
Ziroom's own multi-select links.

## Browser challenges

If the search page does not clear its challenge headlessly, `nest` offers to relaunch a visible
browser with the same temporary profile. Complete any required interaction yourself, then return
to the TUI. The application does not solve CAPTCHAs, rotate proxies, log in, or attempt to
circumvent access controls.

Listings are fetched one page at a time with a delay, bounded retries, and a 100-page guard. Use
the tool responsibly and comply with the site's terms and applicable law.

## Data, diagnostics, and failures

JSON exports contain the source URL, selected Chinese filter labels, scrape timestamp, warnings,
reported count, and listing summaries. CSV exports retain stable columns and join tags with `|`.
Lazy detail pages are intentionally excluded. Writes are atomic; a numeric suffix is added if an
export filename already exists.

Ziroom prices use digit sprites. `nest` compares each cell with embedded templates and refuses
low-confidence matches. If the visual format changes, rent is exported as `null` or blank and the
report contains a warning; a value is never guessed.

Diagnostics are disabled by default. `--log-file` appends JSON records for lifecycle, timing,
counts, warnings, and error chains. Commute addresses, listing content, and URL query values are
not logged. Review diagnostics before sharing because external errors can contain local paths.

Common failures:

- **Browser not found:** pass `--browser PATH` or set `NEST_BROWSER`; macOS `.app` bundles work.
- **Challenge required:** press Enter for the visible recovery flow.
- **Filter update timeout:** retry once; Ziroom may have changed its page structure or be degraded.
- **Missing rent values:** inspect report warnings; the sprite format may have changed.
- **Export failure:** verify that the output directory is writable and has free space.

## Development

Run the same deterministic checks used by CI:

```shell
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --document-private-items
```

Dependency policy is checked with `cargo audit` and `cargo deny check`. The opt-in live test is
ignored by normal CI:

```shell
NEST_LIVE_TESTS=1 cargo test discovers_live_beijing_filters -- --ignored --nocapture
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidance and [SECURITY.md](SECURITY.md) for
private vulnerability reporting and the application's trust boundaries.

## License and disclaimer

Released under the [MIT License](LICENSE).

This project is independent of and not affiliated with Ziroom. Availability and page structure are
controlled by the upstream site.
