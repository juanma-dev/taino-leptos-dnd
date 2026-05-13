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
//! - ⏳ `use_draggable`, `use_droppable`, `DragOverlay`, modifiers, FLIP,
//!   auto-scroll — coming in follow-up commits as we port each piece
//!   from `taino-dnd-leptos` and confirm the core primitives don't
//!   need framework-specific tweaks.
//!
//! Wherever possible the API mirrors `taino-dnd-leptos` 1:1 so users
//! who know one binding can read the other.

#![doc(html_root_url = "https://docs.rs/taino-dnd-dioxus/0.0.1")]

mod context;

pub use context::{provide_dnd_context, use_dnd_context, DndContext, DropResult};

// Re-exports so user code doesn't need a separate `taino-dnd-core` dep
// for the value types it'll commonly reach for.
pub use taino_dnd_core::{DragState, DraggableId, DroppableId};
