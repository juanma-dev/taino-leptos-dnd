//! The [`use_droppable`] hook for Dioxus.

#![allow(clippy::needless_pass_by_value)]

use std::rc::Rc;

use dioxus::prelude::*;
use taino_dnd_core::{detect_axis, live_displacements, Axis, DroppableId, Rect};

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
    /// Live drop-preview displacement: how far this droppable should
    /// visually translate (in CSS pixels) to make room where the
    /// dragged item would land. `(0.0, 0.0)` when no drag is active
    /// or when this slot is not in the affected range.
    ///
    /// Wire with [`Self::drop_preview_style`] for the typical pattern,
    /// or read raw for full control of the transition.
    pub displacement: Memo<(f64, f64)>,
    /// Reactive disabled flag. While `true`, this droppable is removed from
    /// the registry: it's never reported as `over`, never participates in the
    /// drop-preview, and can't receive a drop. Always `false` for
    /// [`use_droppable`]; set via [`use_droppable_with`].
    pub disabled: Signal<bool>,
    #[allow(dead_code)]
    ctx: DndContext,
}

impl UseDroppable {
    /// `onmounted` handler. Captures the element handle, immediately
    /// measures its bounding rect into the shared droppable registry,
    /// and registers the handle with the context so the centralized
    /// re-measure effect can reach it on subsequent drag-start and
    /// auto-scroll ticks.
    pub fn on_mounted(mut self, ev: Event<MountedData>) {
        // Just stash the handle. The registration effect in `use_droppable_with`
        // (which also reacts to `disabled`) is the single authority that
        // measures the rect and registers / unregisters the element.
        self.element.set(Some(ev.data()));
    }

    /// Inline CSS for the drop-preview transform.
    ///
    /// Returns `transform: translate(...); transition: transform 220ms ...;`
    /// while a drag is active and this slot is in the affected range.
    /// Returns just the transition rule otherwise, so the element
    /// animates *back* smoothly when the displacement clears.
    ///
    /// Apply to the *droppable wrapper* element, **not** the draggable
    /// handle (which carries its own drag transform).
    pub fn drop_preview_style(self) -> String {
        let (dx, dy) = *self.displacement.read();
        let z_index = if dx.abs() > 0.001 || dy.abs() > 0.001 { "z-index: 1;" } else { "" };
        format!(
            "transform: translate({dx}px, {dy}px); \
             transition: transform 220ms cubic-bezier(0.2, 0, 0, 1); \
             {z_index}"
        )
    }
}

/// Register an element as a drop target identified by `id`.
///
/// The element's bounding rect is measured when `onmounted` fires.
/// Re-measurement on drag-start and auto-scroll ticks is handled by
/// a centralized effect in [`provide_dnd_context`](crate::provide_dnd_context)
/// that iterates all registered element handles in a single batch,
/// avoiding O(N²) cascading reactive notifications.
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
    let disabled = use_signal(|| false);
    use_droppable_with(id, disabled)
}

/// Like [`use_droppable`], but with a reactive `disabled` flag.
///
/// While `disabled` reads `true`, the droppable is pulled from the registry:
/// it's never reported as `over`, never shifts in the drop-preview, and can't
/// receive a drop. Flip the signal and it rejoins on the next tick (re-measured
/// from the DOM). Read it back via [`UseDroppable::disabled`] for styling.
///
/// # Example
///
/// ```ignore
/// use dioxus::prelude::*;
/// use taino_dnd_core::DroppableId;
/// use taino_dnd_dioxus::use_droppable_with;
///
/// fn Zone() -> Element {
///     let full = use_signal(|| true);
///     let z = use_droppable_with(DroppableId(9), full);
///     rsx! { div { onmounted: move |e| z.on_mounted(e), "zone" } }
/// }
/// ```
pub fn use_droppable_with(id: DroppableId, disabled: Signal<bool>) -> UseDroppable {
    let ctx = use_dnd_context();
    let element = use_signal::<Option<Rc<MountedData>>>(|| None);

    let is_over = use_memo(move || *ctx.over.read() == Some(id));

    // Live drop-preview displacement. Subscribes to `dragged_droppable`
    // (deduped, only changes on drag start/end) and `over` (changes on
    // hover target switch). Uses peek() for droppables because
    // displacements are scroll-invariant — all rects shift by the same
    // delta during scroll, so relative order and step sizes don't change.
    let displacement = use_memo(move || {
        let Some(dragged) = *ctx.dragged_droppable.read() else {
            return (0.0, 0.0);
        };
        let over = *ctx.over.read();
        let map = ctx.droppables.peek();
        let mut items: Vec<(DroppableId, Rect)> = map.iter().map(|(d, r)| (*d, *r)).collect();
        let axis = detect_axis(&items);
        items.sort_by(|a, b| {
            let (sa, sb) = match axis {
                Axis::Y => (a.1.y, b.1.y),
                Axis::X => (a.1.x, b.1.x),
            };
            sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
        });
        let displacements = live_displacements(dragged, over, &items, axis);
        displacements.into_iter().find(|(d, _)| *d == id).map_or((0.0, 0.0), |(_, v)| (v.x, v.y))
    });

    // Registration authority: react to both the mounted element and
    // `disabled`. When disabled, pull this droppable out of both registries so
    // collision / preview / re-measure skip it; when enabled (and mounted),
    // measure and (re-)register it. Replaces the old register-in-`on_mounted`
    // path so toggling `disabled` is honoured without a remount.
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        if *disabled.read() {
            ctx.remove_droppable(id);
            ctx.unregister_element(id);
            return;
        }
        if let Some(data) = element.read().clone() {
            if let Some(rect) = crate::dom::bounding_rect_of(&data) {
                ctx.upsert_droppable(id, rect);
            }
            ctx.register_element(id, data);
        }
    });

    use_drop(move || {
        ctx.remove_droppable(id);
        ctx.unregister_element(id);
    });

    UseDroppable { id, element, is_over, displacement, disabled, ctx }
}
