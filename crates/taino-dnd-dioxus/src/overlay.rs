//! `DragOverlay` — a floating preview that mirrors the active drag.
//!
//! Render `<DragOverlay>...</DragOverlay>` once near the root of your
//! Dioxus app (after `provide_dnd_context()`). While a drag is active,
//! the overlay's container becomes a fixed-position layer translated to
//! the (modifier-adjusted) pointer position. The user composes the
//! preview content inside, typically reading
//! <code>[DndContext::state](crate::DndContext::state).dragged_id()</code>
//! to pick what to render.
//!
//! Mirrors `taino-dnd-leptos::DragOverlay` so user code reads identically.

#![allow(clippy::needless_pass_by_value, unreachable_pub)]

use dioxus::prelude::*;
use taino_dnd_core::{DragState, Vector};

use crate::context::{use_dnd_context, DROP_ANIMATION_MS};

/// A fixed-position layer that follows the active drag.
///
/// `children` is rendered inside the layer. While the state machine is
/// not in `Dragging`, the layer is hidden with `display: none`, so the
/// children stay mounted but invisible (cheap and avoids re-mount
/// flicker). The layer has `pointer-events: none` so it never
/// intercepts hit testing — the underlying draggable still owns the
/// pointer.
///
/// # Example
///
/// ```ignore
/// use dioxus::prelude::*;
/// use taino_dnd_dioxus::{provide_dnd_context, use_dnd_context, DragOverlay};
///
/// fn App() -> Element {
///     provide_dnd_context();
///     rsx! {
///         DragOverlay {
///             {
///                 let ctx = use_dnd_context();
///                 ctx.state.read().dragged_id().map(|id| rsx! {
///                     div { class: "overlay-card", "Item #{id.0}" }
///                 })
///             }
///         }
///     }
/// }
/// ```
#[component]
pub fn DragOverlay(children: Element) -> Element {
    let ctx = use_dnd_context();

    let style = use_memo(move || {
        // The overlay container is sized to the dragged element's original
        // bounding rect so the preview matches the source's footprint.
        let size = ctx
            .dragged_element_rect
            .read()
            .map_or(String::new(), |r| format!("width: {}px; height: {}px;", r.width, r.height));

        let (x, y, transition) = match *ctx.state.read() {
            // Active drag: track the modifier-adjusted pointer. Position the
            // overlay at the dragged element's original top-left plus the drag
            // delta, preserving the cursor's grab-point within the card.
            DragState::Dragging { start, current, .. } => {
                let delta = ctx.modify(Vector::new(current.x - start.x, current.y - start.y));
                let (bx, by) =
                    ctx.dragged_element_rect.read().map_or((start.x, start.y), |r| (r.x, r.y));
                (bx + delta.x, by + delta.y, String::new())
            }
            // Drop-settle: glide from the release position to the landing slot
            // (a CSS transition; the `Settle` timer hides us when it ends).
            DragState::Dropping { .. } => match *ctx.drop_target.read() {
                Some(p) => (
                    p.x,
                    p.y,
                    format!(
                        "transition: transform {DROP_ANIMATION_MS}ms cubic-bezier(0.2, 0, 0, 1); "
                    ),
                ),
                None => return "display: none;".to_owned(),
            },
            _ => return "display: none;".to_owned(),
        };

        format!(
            "position: fixed; top: 0; left: 0; \
             transform: translate({x}px, {y}px); \
             pointer-events: none; z-index: 9999; \
             will-change: transform; {transition}{size}"
        )
    });

    rsx! {
        div { class: "taino-dnd-overlay", style: "{style}", {children} }
    }
}
