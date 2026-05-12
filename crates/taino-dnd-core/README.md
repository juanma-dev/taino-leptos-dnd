# taino-dnd-core

Framework-agnostic primitives for `taino-leptos-dnd`.

This crate intentionally has **zero UI-framework dependencies**. It contains:

- Geometric types (`Point`, `Rect`).
- The drag state machine (`DragState`, `DragEvent`, transitions).
- Collision detection strategies.
- An `Error` type.

If you are looking for the user-facing hooks/components, see the framework-specific
companion crates:

- [`taino-dnd-leptos`](https://crates.io/crates/taino-dnd-leptos)
- *(Stage 3)* `taino-dnd-dioxus`, `taino-dnd-yew`

License: MIT OR Apache-2.0
