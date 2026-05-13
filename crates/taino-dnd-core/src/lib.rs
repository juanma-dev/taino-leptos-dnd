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
// Stage 2 acceptance: no `unwrap()` / `expect()` / `panic!` in public-facing
// paths. The `restriction`-group lints below are off by default; we opt in
// crate-wide and allow them inside `#[cfg(test)]` modules where they're the
// idiomatic way to assert invariants.
#![warn(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod autoscroll;
pub mod collision;
pub mod error;
pub mod geometry;
pub mod modifier;
pub mod state;

pub use autoscroll::{scroll_velocity, AutoScrollConfig};
pub use collision::{closest_center, spatial_neighbor, Direction};
pub use error::Error;
pub use geometry::{Point, Rect};
pub use modifier::{apply_chain, Axis, Modifier, ModifierContext, Vector};
pub use state::{
    transition, DragEvent, DragState, DraggableId, DroppableId, DEFAULT_DRAG_THRESHOLD,
};
