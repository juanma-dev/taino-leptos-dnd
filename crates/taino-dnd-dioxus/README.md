# taino-dnd-dioxus

Accessible, pointer-events drag-and-drop for [Dioxus](https://dioxuslabs.com).

> ⚠️ **Pre-alpha.** Stage 3 of the [roadmap](../../docs/ROADMAP.md):
> *prove the framework-free core by porting it to a second framework*. The
> initial slice ships the `DndContext` provider and the matching
> consume-context hook. `use_draggable` / `use_droppable` land in the
> following commits.

```toml
[dependencies]
taino-dnd-dioxus = "0.0.1"
dioxus = { version = "0.6", features = ["web"] }
```

The framework-free primitives — `DragState`, `DraggableId`, `Modifier`,
`scroll_velocity`, the FSM `transition` function — live in
[`taino-dnd-core`](https://crates.io/crates/taino-dnd-core) and are
shared with the Leptos binding without changes. The Dioxus crate adds
the hooks layer and the DOM glue.

License: MIT OR Apache-2.0
