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
```

## Pre-commit checks

Before you push, run:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --no-deps --workspace
cargo deny check
cargo audit
```

CI runs the same set. PRs that don't pass CI will not be reviewed.

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
