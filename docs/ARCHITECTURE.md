# Architecture

This document explains *why* the code is shaped the way it is. If you find yourself
fighting the structure during a change, update this doc first and the code second.

## Crate layout

```
taino-dnd-core   ──►  pure logic, no framework, no DOM
        ▲
        │ depends on
        │
taino-dnd-leptos  ──►  Leptos hooks/components, web_sys glue
taino-dnd-dioxus  ──►  (Stage 3) Dioxus hooks, same glue, different reactivity
taino-dnd-yew     ──►  (Stage 3) Yew components
```

**Hard rule:** `taino-dnd-core` must compile without any UI framework feature flag.
Run `cargo check -p taino-dnd-core --no-default-features` in CI to enforce it.

## Layers inside `taino-dnd-leptos`

```
                        ┌────────────────────────────────────┐
   User component  ──►  │  <Draggable /> / <Droppable />      │  (Stage 1)
                        └─────────────────┬──────────────────┘
                                          │
                        ┌─────────────────▼──────────────────┐
                        │  use_draggable / use_droppable     │  hooks
                        └─────────────────┬──────────────────┘
                                          │
                        ┌─────────────────▼──────────────────┐
                        │  DndContext (provide_context)      │  shared state
                        └─────────────────┬──────────────────┘
                                          │
                        ┌─────────────────▼──────────────────┐
                        │  taino-dnd-core::DragState         │  pure state machine
                        └────────────────────────────────────┘
```

The user-facing components (`<Draggable>`, `<Droppable>`) are thin convenience wrappers
around the hooks. The hooks are thin glue between Leptos signals and the core state
machine. The core has no idea Leptos exists.

## State machine (core)

```
                  pointerdown
       Idle  ─────────────────────────►  Pressed { start }
        ▲                                       │
        │                                       │  movement > threshold
        │                                       ▼
        │                                  Dragging { id, offset }
        │                                       │
        │                                       │  pointerup / Esc / cancel
        │                                       ▼
        └─────────────────────────────────  Dropping
                       (settles)
```

Threshold avoids accidentally starting a drag on click. Default: 5 px.

## Reactivity boundary

Signals (Leptos `RwSignal` or Dioxus `Signal`) are owned by the **bindings crate**.
Core takes plain `&mut DragState` and returns plain `DragOutcome`. The bindings call
core inside their reactive runtime.

This is what makes Stage 3 possible: swap the reactivity, keep everything else.

## Why Pointer Events, not HTML5 DnD?

HTML5 DnD (`dragstart`/`dragover`/`drop`) is:
- Broken on iOS Safari touch without polyfills.
- Inconsistent across browsers (ghost image rendering, allowed effects).
- Inaccessible — keyboard support is essentially nonexistent.
- Limited in what you can style during drag.

Pointer Events (`pointerdown`/`pointermove`/`pointerup`/`pointercancel`):
- One unified event model for mouse, touch, pen.
- Predictable across browsers.
- Don't fight us on styling or DOM manipulation.
- Compose cleanly with our keyboard sensor (Stage 2).

## SSR safety

Anything that touches `window`, `document`, or `web_sys::*` lives inside an effect
(`Effect::new` in Leptos). Module-level statics with browser handles are banned.

CI runs `cargo check --target wasm32-unknown-unknown` and the (Leptos) server-side
build to catch regressions.

## Why `#![forbid(unsafe_code)]`?

Drag-and-drop is a UI library. There is no place for `unsafe` here. Forbidding it
upfront removes a whole class of supply-chain and review concerns, and makes our
`cargo-deny` story simpler.

## Error handling

- Public APIs that can fail return `Result<_, taino_dnd_core::Error>`.
- `Error` is `#[non_exhaustive]` from day one so we can add variants without a major bump.
- Internal invariants use `debug_assert!`. We do **not** use `unwrap()`/`expect()` in
  paths reachable from user code — lint-enforced from Stage 2.
