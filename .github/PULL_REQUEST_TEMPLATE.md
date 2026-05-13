<!-- Thanks for the patch. Tick what applies, delete what doesn't. -->

## What and why

<!-- 1–3 sentences. The "why" matters more than the "what" — the diff already
shows the what. -->

## Roadmap stage

- [ ] Stage 1 (functional MVP)
- [ ] Stage 2 (production-grade)
- [ ] Stage 3 (multi-framework)
- [ ] Outside the roadmap — explain below.

## Public API

- [ ] No public API change.
- [ ] Additive only (new function/component/type).
- [ ] Breaking change (pre-1.0 ok; describe migration below).

## Checklist

- [ ] `cargo fmt --all`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` (native + `wasm32-unknown-unknown`)
- [ ] `cargo test --workspace`
- [ ] `cargo doc --no-deps --workspace`
- [ ] `cargo deny check`
- [ ] `cargo audit`
- [ ] `bash scripts/size-check.sh` if the patch could affect bundle size.
- [ ] `CHANGELOG.md` updated under `## [Unreleased]`.

## Notes for the reviewer

<!-- Risk areas, things you're unsure about, follow-ups deliberately punted. -->
