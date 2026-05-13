//! The [`use_droppable`] hook for Dioxus.

#![allow(clippy::needless_pass_by_value)]

use std::rc::Rc;

use dioxus::prelude::*;
use taino_dnd_core::{detect_axis, live_displacements, Axis, DragState, DroppableId, Rect};

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
        format!(
            "transform: translate({dx}px, {dy}px); \
             transition: transform 220ms cubic-bezier(0.2, 0, 0, 1);"
        )
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

    // Re-measure the bounding rect whenever the auto-scroll loop bumps
    // `measurement_tick`. After a viewport `scrollBy`, every droppable's
    // viewport-relative rect has shifted; without this the highlighted
    // target would lag behind while the page is scrolling.
    #[cfg(target_arch = "wasm32")]
    {
        let element_for_measure = element;

        // Re-measure when a drag starts (pointer: Idle -> Pressed, keyboard: Idle -> Dragging).
        // Using a memo prevents re-running on Pressed -> Dragging transition.
        let is_active = use_memo(move || {
            matches!(*ctx.state.read(), DragState::Pressed { .. } | DragState::Dragging { .. })
        });

        use_effect(move || {
            if *is_active.read() {
                if let Some(mounted) = element_for_measure.peek().as_ref() {
                    if let Some(rect) = crate::dom::bounding_rect_of(mounted) {
                        ctx.upsert_droppable(id, rect);
                    }
                }
            }
        });

        use_effect(move || {
            // Subscribe to the tick.
            let _ = *ctx.measurement_tick.read();
            if !matches!(*ctx.state.peek(), DragState::Dragging { .. }) {
                return;
            }
            if let Some(mounted) = element_for_measure.peek().as_ref() {
                if let Some(rect) = crate::dom::bounding_rect_of(mounted) {
                    ctx.upsert_droppable(id, rect);
                }
            }
        });
    }

    // Live drop-preview displacement. Re-evaluates whenever state, over,
    // or droppables changes. For typical list sizes (N < 100) the
    // per-droppable computation is fine.
    let displacement = use_memo(move || {
        let dragged = match *ctx.state.read() {
            DragState::Dragging { id, .. } => DroppableId(id.0),
            _ => return (0.0, 0.0),
        };
        let over = *ctx.over.read();
        let map = ctx.droppables.read();
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

    use_drop(move || ctx.remove_droppable(id));

    UseDroppable { id, element, is_over, displacement, ctx }
}
