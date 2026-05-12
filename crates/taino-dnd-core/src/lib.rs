//! Framework-agnostic drag-and-drop primitives.
//!
//! This crate is intentionally free of any UI framework. It provides the geometry,
//! state machine, and collision detection used by the binding crates
//! (`taino-dnd-leptos`, future `taino-dnd-dioxus`, etc).
//!
//! See the workspace [`docs/ARCHITECTURE.md`] for the layering rationale.
//!
//! [`docs/ARCHITECTURE.md`]: https://github.com/juanma-dev/taino-leptos-dnd/blob/main/docs/ARCHITECTURE.md

#![doc(html_root_url = "https://docs.rs/taino-dnd-core/0.0.1")]

pub mod collision;
pub mod error;
pub mod geometry;
pub mod state;

pub use collision::closest_center;
pub use error::Error;
pub use geometry::{Point, Rect};
pub use state::{
    transition, DragEvent, DragState, DraggableId, DroppableId, DEFAULT_DRAG_THRESHOLD,
};
