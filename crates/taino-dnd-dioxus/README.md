# taino-dnd-dioxus

Accessible, pointer-events drag-and-drop for [Dioxus](https://dioxuslabs.com).

> ⚠️ **Pre-alpha.** Stage 3 of the [roadmap](../../docs/ROADMAP.md):
> *prove the framework-free core by porting it to a second framework*.
> This binding now mirrors the full `taino-dnd-leptos` API surface 1:1.

```toml
[dependencies]
taino-dnd-dioxus = "0.4"
dioxus = { version = "0.6", features = ["web"] }
```

The framework-free primitives — `DragState`, `DraggableId`, `Modifier`,
`scroll_velocity`, the FSM `transition` function — live in
[`taino-dnd-core`](https://crates.io/crates/taino-dnd-core) and are
shared with the Leptos binding without changes. The Dioxus crate adds
the hooks layer and the DOM glue.

## What's here

- `provide_dnd_context` / `use_dnd_context` / `DndContext` / `DropResult`
- `use_draggable` — pointer (mouse + touch + pen) and keyboard sensors
  (Space/Enter pick up & drop, arrows to move, Esc to cancel)
- `use_droppable` — hover state plus the live drop-preview `displacement`
- `DndAnnouncer` — `aria-live` region for screen-reader announcements
- `DragOverlay` — portal-style preview that follows the modifier-adjusted
  pointer
- `use_flip` / `use_flip_with` — FLIP post-drop settle animation
- `use_drag_container` + `Modifier::{RestrictToAxis, RestrictToParent, SnapToGrid}`
- viewport auto-scroll when the pointer nears an edge during a drag

See the runnable demos: [`examples/sortable-list-dioxus`](../../examples/sortable-list-dioxus)
(live drop-preview) and [`examples/kanban-dioxus`](../../examples/kanban-dioxus)
(cross-column moves + FLIP).

License: MIT OR Apache-2.0
