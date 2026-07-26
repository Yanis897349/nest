# Contributing

Focused bug fixes, tests, accessibility improvements, and documentation changes are welcome.

## Local setup

1. Install a Chromium-family browser and [Rustup](https://rustup.rs/).
2. Clone the repository. Rustup will install the pinned Rust 1.90.0 toolchain from
   `rust-toolchain.toml`.
3. Run `cargo run --release`.

Before opening a pull request, run:

```shell
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --document-private-items
```

When installed, also run `cargo audit` and `cargo deny check`.

## Tests and pull requests

- Keep each change scoped to one concern and explain its motivation.
- Add deterministic unit or fixture tests for behavior changes.
- Keep real-site checks ignored and opt-in; CI must not depend on Ziroom availability.
- Update the README for CLI, keyboard, export, security, or platform changes.
- Do not commit exported listings, diagnostic logs, browser profiles, credentials, or build output.

Use `NEST_LIVE_TESTS=1 cargo test discovers_live_beijing_filters -- --ignored --nocapture`
only when intentionally checking the current Ziroom site. It may require
`NEST_LIVE_HEADED=1` and can legitimately encounter a browser challenge.
