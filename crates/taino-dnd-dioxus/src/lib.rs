//! Dioxus hooks for accessible, pointer-events drag-and-drop.
//!
//! Stage 3 binding. The framework-free state machine, geometry,
//! modifiers, and collision strategies live in [`taino_dnd_core`]; this
//! crate is the thin glue between those primitives and Dioxus's
//! reactivity + event system.
//!
//! ## Status
//!
//! - ✅ `DndContext` + [`provide_dnd_context`] / [`use_dnd_context`]
//! - ✅ [`use_draggable`] with the four pointer-event handlers,
//!   `on_key_down` (keyboard sensor: Space/Enter pickup & drop, arrow
//!   keys for movement, Escape for cancel) and `style` / `style_pinned`.
//! - ✅ [`use_droppable`] with `onmounted` rect registration.
//! - ✅ [`DndAnnouncer`] — visually-hidden `role="alert"
//!   aria-live="assertive"` region for screen-reader announcements.
//! - ✅ Modifiers ([`Modifier::RestrictToAxis`], [`Modifier::SnapToGrid`],
//!   [`Modifier::RestrictToParent`]) + [`use_drag_container`] for the
//!   parent-restrict pattern.
//! - ⏳ `DragOverlay`, `use_flip`, auto-scroll — coming in follow-up
//!   commits as we port each piece from `taino-dnd-leptos`.
//!
//! Wherever possible the API mirrors `taino-dnd-leptos` 1:1 so users
//! who know one binding can read the other.

#![doc(html_root_url = "https://docs.rs/taino-dnd-dioxus/0.0.1")]

mod announcer;
mod container;
mod context;
#[cfg(target_arch = "wasm32")]
mod dom;
mod draggable;
mod droppable;

pub use announcer::DndAnnouncer;
pub use container::{use_drag_container, UseDragContainer};
pub use context::{provide_dnd_context, use_dnd_context, DndContext, DropResult};
pub use draggable::{use_draggable, UseDraggable};
pub use droppable::{use_droppable, UseDroppable};

// Re-exports so user code doesn't need a separate `taino-dnd-core` dep
// for the value types it'll commonly reach for.
pub use taino_dnd_core::{
    Axis, DragState, DraggableId, DroppableId, Modifier, ModifierContext, Vector,
};
