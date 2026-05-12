# taino-leptos-dnd

A **drag & drop** library for [Leptos](https://leptos.dev), inspired by the ergonomics of
`react-beautiful-dnd` / `dnd-kit`, with a framework-agnostic core that can power bindings
for Dioxus, Yew, and others.

> ⚠️ **Status: Pre-alpha (v0.0.1).** Public APIs will change without notice until v0.1.

## Why

The Rust frontend ecosystem lacks a polished, accessible drag-and-drop primitive. Existing
options are mostly thin wrappers around HTML5 DnD (which is broken on touch devices,
inaccessible, and visually inconsistent). `taino-leptos-dnd` aims to provide:

- ✋ Pointer Events under the hood (mouse + touch + pen unified)
- ♿ Full WAI-ARIA support and keyboard sensors out of the box
- 🎯 SSR-safe (no `window` access at module load)
- 🪶 Tree-shakeable, `wasm-opt`-friendly, `#![forbid(unsafe_code)]`
- 🧩 A core crate (`taino-dnd-core`) free of any UI framework

## Crates

| Crate                  | Purpose                                  | Status        |
| ---------------------- | ---------------------------------------- | ------------- |
| `taino-dnd-core`       | Geometry, state machine, framework-free  | Pre-alpha     |
| `taino-dnd-leptos`     | Leptos hooks and components              | Pre-alpha     |
| `taino-dnd-dioxus`     | Dioxus integration                       | Planned (S3)  |
| `taino-dnd-yew`        | Yew integration                          | Planned (S3)  |

## Roadmap

See [`docs/ROADMAP.md`](docs/ROADMAP.md). The project ships in three stages:

1. **Functional MVP** — sortable list works end-to-end.
2. **Production-grade** — accessibility, keyboard sensor, animations, auto-scroll, modifiers.
3. **Multi-framework** — extract `taino-dnd-core`, build Dioxus binding as proof.

## Building

```bash
# From the repo root, inside WSL or any *nix shell:
cargo check --workspace
cargo test  --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option. Contributions are accepted under the same terms (Apache-2.0 + MIT).
