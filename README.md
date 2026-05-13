# taino-leptos-dnd

[![CI](https://github.com/juanma-dev/taino-leptos-dnd/actions/workflows/ci.yml/badge.svg)](https://github.com/juanma-dev/taino-leptos-dnd/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue.svg)](#license)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue.svg)](rust-toolchain.toml)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](Cargo.toml)

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
| `taino-dnd-dioxus`     | Dioxus integration                       | Pre-alpha (S3 in progress) |
| `taino-dnd-yew`        | Yew integration                          | Planned (S3)  |

## Roadmap

See [`docs/ROADMAP.md`](docs/ROADMAP.md). The project ships in three stages:

1. **Functional MVP** — sortable list works end-to-end.
2. **Production-grade** — accessibility, keyboard sensor, animations, auto-scroll, modifiers.
3. **Multi-framework** — extract `taino-dnd-core`, build Dioxus binding as proof.

## Quick start

```rust
use leptos::prelude::*;
use taino_dnd_core::{DraggableId, DroppableId};
use taino_dnd_leptos::{
    provide_dnd_context, use_draggable, use_droppable, DndAnnouncer,
};

#[component]
fn Row(id: u64, label: String) -> impl IntoView {
    let d = use_draggable(DraggableId(id));
    let z = use_droppable(DroppableId(id));
    view! {
        <div node_ref=z.node_ref class:over=move || z.is_over.get()>
            <div
                node_ref=d.node_ref
                tabindex="0"
                role="button"
                aria-roledescription="draggable item"
                on:pointerdown=move |e| d.on_pointer_down(&e)
                on:pointermove=move |e| d.on_pointer_move(&e)
                on:pointerup=move |e| d.on_pointer_up(&e)
                on:pointercancel=move |e| d.on_pointer_cancel(&e)
                on:keydown=move |e| d.on_key_down(&e)
                style=move || d.style()
            >
                {label}
            </div>
        </div>
    }
}

#[component]
fn App() -> impl IntoView {
    provide_dnd_context();
    view! {
        <DndAnnouncer/>
        <Row id=1 label="First".into() />
        <Row id=2 label="Second".into() />
    }
}
```

See [`examples/sortable-list`](examples/sortable-list) for a full reordering demo.

## Building

```bash
# From the repo root, inside WSL or any *nix shell:
cargo check --workspace
cargo test  --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Documentation

- [Getting started](docs/guides/getting-started.md) — 5-minute tour from `cargo add` to a working sortable list.
- [Keyboard & accessibility](docs/guides/keyboard-and-a11y.md) — the a11y model in depth.
- [Architecture](docs/ARCHITECTURE.md) — why the code is shaped this way.
- [Roadmap](docs/ROADMAP.md) — three-stage delivery plan with acceptance criteria.
- [Contributing](docs/CONTRIBUTING.md) — workflow, local setup, pre-commit checks.

## Community

- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Security policy](SECURITY.md) — please report vulnerabilities privately.

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option. Contributions are accepted under the same terms (Apache-2.0 + MIT).
