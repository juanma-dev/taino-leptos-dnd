//! The [`use_droppable`] hook for Dioxus.

#![allow(clippy::needless_pass_by_value)]

use std::rc::Rc;

use dioxus::prelude::*;
use taino_dnd_core::DroppableId;

use crate::context::{use_dnd_context, DndContext};

/// Handle returned by [`use_droppable`]. Wire its `onmounted` callback
/// to the drop-target element and read `is_over` for hover styling.
#[derive(Clone, Copy)]
pub struct UseDroppable {
    /// The identifier this hook was instantiated with.
    pub id: DroppableId,
    /// The mounted element (after `onmounted` fires).
    pub element: Signal<Option<Rc<MountedData>>>,
    /// `true` while a drag is in progress and the pointer is over this
    /// droppable.
    pub is_over: Memo<bool>,
    /// Kept around for future helpers (e.g. an `on_mounted` that
    /// re-measures rect on drag start) — currently the cleanup hook
    /// captures its own copy via `use_drop`, so this field isn't read
    /// in the Stage-3 MVP body.
    #[allow(dead_code)]
    ctx: DndContext,
}

impl UseDroppable {
    /// `onmounted` handler. Captures the element handle and immediately
    /// measures its bounding rect into the shared droppable registry.
    pub fn on_mounted(mut self, ev: Event<MountedData>) {
        let data = ev.data();
        #[cfg(target_arch = "wasm32")]
        if let Some(rect) = crate::dom::bounding_rect_of(&data) {
            self.ctx.upsert_droppable(self.id, rect);
        }
        self.element.set(Some(data));
    }
}

/// Register an element as a drop target identified by `id`.
///
/// The element's bounding rect is measured when `onmounted` fires. A
/// follow-up slice will add a re-measure on every press-start (to
/// catch layout changes between drags) and on every auto-scroll tick;
/// for the Stage-3 MVP, one measurement per mount is enough for the
/// example to demonstrate the binding works end-to-end.
///
/// # Example
///
/// ```ignore
/// use dioxus::prelude::*;
/// use taino_dnd_core::DroppableId;
/// use taino_dnd_dioxus::use_droppable;
///
/// fn Zone() -> Element {
///     let z = use_droppable(DroppableId(42));
///     let class = if *z.is_over.read() { "zone over" } else { "zone" };
///     rsx! {
///         div {
///             onmounted: move |e| z.on_mounted(e),
///             class: "{class}",
///             "drop here"
///         }
///     }
/// }
/// ```
pub fn use_droppable(id: DroppableId) -> UseDroppable {
    let ctx = use_dnd_context();
    let element = use_signal::<Option<Rc<MountedData>>>(|| None);

    let is_over = use_memo(move || *ctx.over.read() == Some(id));

    use_drop(move || ctx.remove_droppable(id));

    UseDroppable { id, element, is_over, ctx }
}
