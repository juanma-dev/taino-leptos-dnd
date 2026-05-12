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

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option. Contributions are accepted under the same terms (Apache-2.0 + MIT).
