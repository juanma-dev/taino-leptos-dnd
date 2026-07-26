# Contributing

Thanks for considering a contribution. This document explains the workflow we expect
so reviews stay fast.

## Ground rules

- All code is dual-licensed MIT / Apache-2.0. By submitting a PR you agree to that.
- `#![forbid(unsafe_code)]` is non-negotiable.
- Public APIs require doc comments with at least one `# Examples` block.
- New features in `taino-dnd-leptos` must keep `taino-dnd-core` framework-free.

## Local setup

Inside WSL (or any Unix-like shell):

```bash
rustup target add wasm32-unknown-unknown
cargo install --locked cargo-deny cargo-audit trunk wasm-bindgen-cli
# Binaryen for the size budget. Distros vary:
#   Debian/Ubuntu: sudo apt-get install binaryen
#   macOS:         brew install binaryen
#   Arch:          sudo pacman -S binaryen
```

`wasm-bindgen-cli`'s version must match the `wasm-bindgen` crate in
`Cargo.lock`. CI pins the version explicitly; if you see a mismatch
locally, re-run `cargo install --locked wasm-bindgen-cli@<version>` with
the version from the lockfile.

## Pre-commit checks

Before you push, run:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --no-deps --workspace
cargo deny check
cargo audit
# Size budget (needs wasm-bindgen-cli + binaryen, see Local setup)
bash scripts/size-check.sh
```

CI runs the same set. PRs that don't pass CI will not be reviewed.

### Browser interaction tests

Synthetic-event tests for both bindings live in each crate's
`tests/interactions.rs`. They require a real browser DOM (Node has no
`document`), so they're driven by `wasm-pack` against headless Chrome.
Note that `wasm-pack test -p <crate>` does **not** work with this
virtual workspace — run from inside the crate directory:

```bash
cargo install --locked wasm-pack
cd crates/taino-dnd-leptos  && wasm-pack test --chrome --headless
cd crates/taino-dnd-dioxus && wasm-pack test --chrome --headless
```

If you don't have a system Chrome (e.g. WSL), download a matched
[Chrome for Testing](https://googlechromelabs.github.io/chrome-for-testing/)
`chrome-headless-shell` + `chromedriver` pair, set
`CHROMEDRIVER=<path>` in the environment, and drop a `webdriver.json`
next to the crate's `Cargo.toml` (gitignored) pointing at the browser:

```json
{
  "goog:chromeOptions": {
    "binary": "/path/to/chrome-headless-shell",
    "args": ["--no-sandbox", "--disable-dev-shm-usage"]
  }
}
```

CI runs both suites via the `browser-tests` job with exactly this pinned
setup (the Dioxus 0.7 suite hangs under the runner's preinstalled full
Chrome). If you're iterating locally and don't have Chrome handy, you can
compile-check the suite without running it:
`cargo test -p taino-dnd-leptos --target wasm32-unknown-unknown --no-run`.

### End-to-end smoke (real examples, real drags)

The interaction tests exercise the hooks; `scripts/e2e-smoke.py` exercises
the shipped examples. It trunk-builds the two `multi-zone` demos, serves
them, and drives a real cross-zone pointer drag (vertical list → vertical
list, and horizontal bar → horizontal bar) through headless Chrome via
the WebDriver protocol, asserting the card actually moved in the DOM:

```bash
scripts/e2e-smoke.py                 # both frameworks
scripts/e2e-smoke.py multi-zone      # just the Leptos demo
E2E_NO_BUILD=1 scripts/e2e-smoke.py  # reuse the existing dist/
```

It reuses the same `CHROME` / `CHROMEDRIVER` binaries as above and needs
`trunk` on the PATH (or `TRUNK=<path>`). Run it after dependency bumps or
changes to collision/geometry code — it catches integration breakage that
the synthetic hook tests can't see.

## Commits

- Conventional Commits style is encouraged but not enforced:
  `feat(core): add closest-center collision strategy`
  `fix(leptos): SSR panic when use_droppable called pre-mount`
- Keep commits focused. If you find unrelated cleanup, that's a separate PR.

## Pull requests

- Title under 70 characters. Use the body for detail.
- Link the roadmap stage (e.g. *Stage 2 — keyboard sensor*).
- Update `CHANGELOG.md` under `## [Unreleased]` in the same PR.
- Screenshots / short clips for UI behavior changes.

## Issues / discussions

- Bugs: a minimal reproduction (ideally a `cargo new` project) goes a long way.
- Feature requests: please scan `docs/ROADMAP.md` first. If your idea fits a stage
  that hasn't started, propose it as an issue rather than a PR.

## Releasing (maintainers)

1. Update `CHANGELOG.md` with the version and date.
2. Bump versions in workspace `Cargo.toml`.
3. `cargo deny check && cargo audit`.
4. Run `/security-review` (Claude Code) on the staged diff.
5. Tag: `git tag -a vX.Y.Z -m "release: vX.Y.Z"`.
6. Push tag.
7. `cargo publish -p taino-dnd-core && cargo publish -p taino-dnd-leptos`.
