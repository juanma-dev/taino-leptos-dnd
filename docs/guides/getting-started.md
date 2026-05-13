# Getting started

A five-minute tour. By the end you'll have a Leptos app where the user can
reorder items with mouse, touch, or keyboard, with announcements for screen
readers.

## Prerequisites

- Rust 1.85 or newer (`rustup default stable`).
- The wasm32 target: `rustup target add wasm32-unknown-unknown`.
- Trunk (`cargo install --locked trunk`) — recommended for development.

## 1. Cargo setup

In your Leptos app's `Cargo.toml`:

```toml
[dependencies]
leptos = { version = "0.7", default-features = false, features = ["csr"] }
taino-dnd-leptos = "0.0"
console_error_panic_hook = "0.1"   # nicer panic backtraces in the browser
```

The `taino-dnd-core` crate is re-exported through `taino-dnd-leptos`; you
only need it directly if you're writing a binding for another framework.

## 2. Install the context

Drag-and-drop is region-scoped. Wrap the area of your app that participates
with a single context call:

```rust
use leptos::prelude::*;
use taino_dnd_leptos::{provide_dnd_context, DndAnnouncer};

#[component]
fn App() -> impl IntoView {
    provide_dnd_context();
    view! {
        <DndAnnouncer/>           // visually-hidden ARIA live region
        <Board/>
    }
}
```

`DndAnnouncer` adds a `role="status" aria-live="polite"` region so screen
readers hear pickup / move / drop announcements. It renders nothing visible.

## 3. A draggable

Use `use_draggable` on whatever element the user grabs. Wire its handlers:

```rust
use taino_dnd_core::DraggableId;
use taino_dnd_leptos::use_draggable;

#[component]
fn Card(id: u64, label: String) -> impl IntoView {
    let d = use_draggable(DraggableId(id));
    view! {
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
    }
}
```

`d.style()` returns `transform: translate(...)` while dragging plus
`touch-action: none; user-select: none;` so the browser doesn't intercept
your pointer events for scroll / text-selection gestures.

The `tabindex` / `role` / `aria-roledescription` attributes are how
assistive tech surfaces this as a moveable thing. Don't skip them.

## 4. A droppable

Drop zones use `use_droppable`. Read `is_over` for hover styling:

```rust
use taino_dnd_core::DroppableId;
use taino_dnd_leptos::use_droppable;

#[component]
fn Slot(id: u64) -> impl IntoView {
    let z = use_droppable(DroppableId(id));
    view! {
        <div
            node_ref=z.node_ref
            class:over=move || z.is_over.get()
        >
            "drop here"
        </div>
    }
}
```

## 5. React to drops

The context exposes `last_drop` as a `RwSignal<Option<DropResult>>`. Watch it
with an effect, update your data, then clear it:

```rust
use taino_dnd_leptos::use_dnd_context;

let ctx = use_dnd_context();
Effect::new(move |_| {
    if let Some(drop) = ctx.last_drop.get() {
        if let Some(target) = drop.over {
            // move drop.draggable into target's slot
        }
        ctx.clear_last_drop();
    }
});
```

`DropResult` contains both the dragged id and the droppable id the pointer
was over (if any). You decide what "move into" means — swap, insert-before,
insert-after — depending on your model.

## What you get for free

Once those four pieces are wired:

- **Mouse, touch, and pen** via Pointer Events.
- **Keyboard sensor**: tab to a draggable, space/enter to pick up, arrows
  to move between drop targets, escape to cancel.
- **Viewport auto-scroll** when the pointer nears a screen edge during a drag.
- **Screen-reader announcements** on every transition.

## Next steps

- [`keyboard-and-a11y.md`](keyboard-and-a11y.md) — the accessibility model
  in depth.
- [`../ROADMAP.md`](../ROADMAP.md) — what's shipped and what's coming.
- [`../ARCHITECTURE.md`](../ARCHITECTURE.md) — why the code is shaped this way.
