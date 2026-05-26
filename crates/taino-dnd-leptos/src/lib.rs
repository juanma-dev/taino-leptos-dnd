//! Leptos hooks and components for accessible, pointer-events drag-and-drop.
//!
//! This crate is the framework-binding layer; see [`taino_dnd_core`] for the
//! framework-free primitives that power it.
//!
//! # Quick start
//!
//! ```no_run
//! use leptos::prelude::*;
//! use taino_dnd_core::{DraggableId, DroppableId};
//! use taino_dnd_leptos::{provide_dnd_context, use_draggable, use_droppable};
//!
//! #[component]
//! fn App() -> impl IntoView {
//!     provide_dnd_context();
//!     let d = use_draggable(DraggableId(1));
//!     let z = use_droppable(DroppableId(2));
//!     view! {
//!         <div
//!             node_ref=d.node_ref
//!             on:pointerdown=move |e| d.on_pointer_down(&e)
//!             on:pointermove=move |e| d.on_pointer_move(&e)
//!             on:pointerup=move |e| d.on_pointer_up(&e)
//!             on:pointercancel=move |e| d.on_pointer_cancel(&e)
//!             style=move || d.style()
//!         >
//!             "Drag me"
//!         </div>
//!         <div node_ref=z.node_ref class:over=move || z.is_over.get()>
//!             "Drop zone"
//!         </div>
//!     }
//! }
//! ```
//!
//! See [`docs/ROADMAP.md`](https://github.com/juanma-dev/taino-leptos-dnd/blob/main/docs/ROADMAP.md)
//! for the staged plan; Stage 1 ships the API in this quick-start, Stage 2 adds
//! keyboard sensors, animations, auto-scroll, and ARIA announcements.

#![doc(html_root_url = "https://docs.rs/taino-dnd-leptos/0.4.6")]
// Stage 2 acceptance: no `unwrap()` / `expect()` / `panic!` in public-facing
// paths. See `crates/taino-dnd-core/src/lib.rs` for the rationale; the one
// remaining `expect()` (in `use_dnd_context`) is `#[allow]`-ed inline because
// the panic is the documented behavior for "called outside a DndContext".
#![warn(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod announcer;
mod autoscroll;
mod container;
mod context;
#[cfg(target_arch = "wasm32")]
mod dom;
mod draggable;
mod droppable;
mod flip;
mod overlay;

pub use announcer::DndAnnouncer;
pub use container::use_drag_container;
pub use context::{provide_dnd_context, use_dnd_context, DndContext, DropResult};
pub use draggable::{use_draggable, use_draggable_with, UseDraggable};
pub use droppable::{use_droppable, use_droppable_with, UseDroppable};
pub use flip::{use_flip, use_flip_with, FlipConfig};
pub use overlay::DragOverlay;
// Re-exports for ergonomics: configuring modifiers and auto-scroll shouldn't
// require an extra dependency on `taino-dnd-core`.
pub use taino_dnd_core::{
    default_announcement, AnnounceEvent, AutoScrollConfig, Axis, Modifier, ModifierContext, Vector,
};

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        // Native compilation smoke test. Real browser tests live in `tests/web.rs`.
    }
}
