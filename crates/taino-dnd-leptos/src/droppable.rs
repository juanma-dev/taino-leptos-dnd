//! The [`use_droppable`] hook.

use leptos::{html::Div, prelude::*};
#[cfg(target_arch = "wasm32")]
use taino_dnd_core::DragState;
use taino_dnd_core::{detect_axis, live_displacements, DroppableId};

use crate::context::{use_dnd_context, DndContext};

/// Handle returned by [`use_droppable`]. Wire its `node_ref` to your element
/// and read `is_over` for hover styling.
#[derive(Clone, Copy)]
pub struct UseDroppable {
    /// Attach to the element with `node_ref={handle.node_ref}`.
    pub node_ref: NodeRef<Div>,
    /// `true` while a drag is in progress and the pointer is closest to this
    /// droppable.
    pub is_over: Signal<bool>,
    /// Live drop-preview displacement: how far this droppable should
    /// visually translate (in CSS pixels) to make room where the
    /// dragged item would land. `(0.0, 0.0)` when no drag is active
    /// or when this slot is not in the affected range.
    ///
    /// Wire with [`Self::drop_preview_style`] for the typical pattern,
    /// or read raw if you need full control of the transition.
    pub displacement: Signal<(f64, f64)>,
    /// The identifier this hook was instantiated with.
    pub id: DroppableId,
    /// Reactive disabled flag. While `true`, this droppable is removed from
    /// the registry: it's never reported as `over`, never participates in the
    /// drop-preview, and can't receive a drop. Always `false` for
    /// [`use_droppable`]; set via [`use_droppable_with`].
    pub disabled: Signal<bool>,
    #[allow(dead_code)]
    ctx: DndContext,
}

impl UseDroppable {
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
        let (dx, dy) = self.displacement.get();
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
/// The element's bounding rect is measured when the node is attached and
/// re-measured at the start of every drag. Stage 1 does not observe resize or
/// scroll between drags — for a list whose layout doesn't change during a
/// drag, this is sufficient.
///
/// # Example
///
/// ```no_run
/// use leptos::prelude::*;
/// use taino_dnd_core::DroppableId;
/// use taino_dnd_leptos::{provide_dnd_context, use_droppable};
///
/// #[component]
/// fn Zone() -> impl IntoView {
///     let z = use_droppable(DroppableId(42));
///     view! {
///         <div node_ref=z.node_ref class:over=move || z.is_over.get()>
///             "drop here"
///         </div>
///     }
/// }
/// ```
pub fn use_droppable(id: DroppableId) -> UseDroppable {
    use_droppable_with(id, Signal::derive(|| false))
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
/// ```no_run
/// use leptos::prelude::*;
/// use taino_dnd_core::DroppableId;
/// use taino_dnd_leptos::{provide_dnd_context, use_droppable_with};
///
/// # #[component] fn Zone() -> impl IntoView {
/// let full = RwSignal::new(true);
/// let z = use_droppable_with(DroppableId(9), full.into());
/// view! { <div node_ref=z.node_ref class:full=move || z.disabled.get()>"zone"</div> }
/// # }
/// ```
pub fn use_droppable_with(id: DroppableId, disabled: Signal<bool>) -> UseDroppable {
    let ctx = use_dnd_context();
    let node_ref = NodeRef::<Div>::new();

    let is_over = Signal::derive(move || ctx.over.get() == Some(id));

    // Live drop-preview displacement: while a drag is active, neighbors
    // of the hovered slot shift to show where the dragged item would
    // land. We re-evaluate per render whenever state / over / the
    // droppables map changes. Cost is O(N) per droppable for typical
    // list sizes (N < 100); the cheaper amortized version (one shared
    // memo) is a future optimization.
    let displacement = Signal::derive(move || {
        let state = ctx.state.get();
        // Use the same DraggableId↔DroppableId convention as the rest
        // of the library: in sortable lists they share the numeric id.
        let dragged = match state {
            taino_dnd_core::DragState::Dragging { id, .. } => DroppableId(id.0),
            _ => return (0.0, 0.0),
        };
        let over = ctx.over.get();
        ctx.droppables.with(|map| {
            // Sort by axis-relevant edge so the index order matches the
            // visual stacking order.
            let mut items: Vec<(DroppableId, taino_dnd_core::Rect)> =
                map.iter().map(|(d, r)| (*d, *r)).collect();
            let axis = detect_axis(&items);
            items.sort_by(|a, b| {
                let (sa, sb) = match axis {
                    taino_dnd_core::Axis::Y => (a.1.y, b.1.y),
                    taino_dnd_core::Axis::X => (a.1.x, b.1.x),
                };
                sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
            });
            let displacements = live_displacements(dragged, over, &items, axis);
            displacements
                .into_iter()
                .find(|(d, _)| *d == id)
                .map_or((0.0, 0.0), |(_, v)| (v.x, v.y))
        })
    });

    // Registration authority: react to both node mount and `disabled`. When
    // disabled, pull this droppable out of the registry so collision / preview
    // skip it; when enabled (and mounted), measure and (re-)register it.
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        Effect::new(move |_| {
            if disabled.get() {
                ctx.remove_droppable(id);
                return;
            }
            if let Some(el) = node_ref.get() {
                if let Some(el) = (*el).dyn_ref::<web_sys::Element>() {
                    let rect = crate::dom::bounding_rect(el);
                    ctx.upsert_droppable(id, rect);
                }
            }
        });

        // Re-measure when a drag starts. Layout can change between drags
        // (e.g. items reordered after a previous drop).
        //
        // `Memo` (not `Signal::derive`) is load-bearing: the keyboard sensor
        // updates `state.current` on every arrow step, and a non-deduped
        // derived signal would re-fire this effect mid-drag. Re-measuring
        // mid-drag captures the *post-CSS-transform* rect of displaced
        // wrappers (since `getBoundingClientRect` includes transforms),
        // which then corrupts spatial-neighbor and displacement math for
        // subsequent arrow presses. With `Memo`, the effect only fires on
        // the actual Idle → Pressed/Dragging transition.
        let is_active = Memo::new(move |_| {
            matches!(ctx.state.get(), DragState::Pressed { .. } | DragState::Dragging { .. })
        });

        Effect::new(move |_| {
            if is_active.get() && !disabled.get_untracked() {
                if let Some(el) = node_ref.get_untracked() {
                    if let Some(el) = (*el).dyn_ref::<web_sys::Element>() {
                        let rect = crate::dom::bounding_rect(el);
                        ctx.upsert_droppable(id, rect);
                    }
                }
            }
        });

        // Window-scroll re-measurement is handled by
        // `DndContext::shift_droppable_rects` (cheaper: pure arithmetic,
        // no `getComputedStyle` per frame). **Container** scroll, however,
        // moves only the descendants of the scrolled overflow ancestor —
        // a blanket shift would be wrong — so the capturing scroll
        // listener bumps `measurement_tick` and we re-measure here.
        //
        // This is safe (no flicker) because `bounding_rect` subtracts the
        // drop-preview `transform: translate(...)`, returning the layout
        // position. Measuring the transform-included rect was the original
        // flicker source; with the transform removed, remeasure is inert
        // to the preview and matches `shift` for the pure-scroll case.
        Effect::new(move |_| {
            ctx.measurement_tick.get(); // subscribe to container-scroll ticks
            if matches!(ctx.state.get_untracked(), DragState::Dragging { .. })
                && !disabled.get_untracked()
            {
                if let Some(el) = node_ref.get_untracked() {
                    if let Some(el) = (*el).dyn_ref::<web_sys::Element>() {
                        let rect = crate::dom::bounding_rect(el);
                        ctx.upsert_droppable(id, rect);
                    }
                }
            }
        });
    }

    on_cleanup(move || {
        ctx.remove_droppable(id);
    });

    UseDroppable { node_ref, is_over, displacement, id, disabled, ctx }
}
