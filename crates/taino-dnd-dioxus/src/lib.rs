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
//! - ✅ Viewport auto-scroll: a `requestAnimationFrame` loop installed by
//!   [`provide_dnd_context`] scrolls the window when the pointer
//!   approaches a viewport edge during a drag. Configurable via
//!   [`AutoScrollConfig`].
//! - ✅ Live drop-preview reorder via [`use_droppable`]'s `displacement`
//!   signal + `drop_preview_style()` helper. Neighbors visibly shift to
//!   show where the dragged item would land.
//! - ✅ [`DragOverlay`] — fixed-position preview layer that mirrors the
//!   active drag at the modifier-adjusted pointer position.
//! - ✅ [`use_flip`] / [`use_flip_with`] — FLIP-based post-drop settle
//!   animation for the reorder, the complement to the live drop-preview.
//! - ✅ Auto-scroll of arbitrary overflow ancestors, not just the window:
//!   the RAF loop scrolls the innermost scroll container under the pointer.
//! - ✅ Drop-settle animation: the [`DragOverlay`] glides to the landing
//!   slot on release before the state returns to idle.
//!
//! Wherever possible the API mirrors `taino-dnd-leptos` 1:1 so users
//! who know one binding can read the other.

#![doc(html_root_url = "https://docs.rs/taino-dnd-dioxus/0.5.0")]
// Parity with `taino-dnd-leptos`: no `unwrap()` / `expect()` / `panic!` in
// public-facing paths. The one documented exception (`use_dnd_context`'s
// missing-provider panic, raised by Dioxus's `use_context`) is inherent to
// the framework call, not an `expect()` we own, so no inline `#[allow]` is
// needed here.
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
pub use container::{use_drag_container, UseDragContainer};
pub use context::{provide_dnd_context, use_dnd_context, DndContext, DropResult};
pub use draggable::{use_draggable, use_draggable_with, UseDraggable};
pub use droppable::{use_droppable, use_droppable_with, UseDroppable};
pub use flip::{use_flip, use_flip_with, FlipConfig};
pub use overlay::DragOverlay;

// Re-exports so user code doesn't need a separate `taino-dnd-core` dep
// for the value types it'll commonly reach for.
pub use taino_dnd_core::{
    default_announcement, AnnounceEvent, AutoScrollConfig, Axis, DragState, DraggableId,
    DroppableId, Modifier, ModifierContext, Vector,
};
