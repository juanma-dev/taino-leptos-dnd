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

use crate::context::use_dnd_context;

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
        let DragState::Dragging { start, current, .. } = *ctx.state.read() else {
            return "display: none;".to_owned();
        };
        let raw = Vector::new(current.x - start.x, current.y - start.y);
        let delta = ctx.modify(raw);

        // Position the overlay at the dragged element's original top-left
        // plus the modifier-adjusted drag delta. This preserves the cursor's
        // grab-point relative to the card. If we don't have the rect (rare —
        // it should always be set during a drag), fall back to placing the
        // overlay at the pointer.
        let (x, y, size) = ctx.dragged_element_rect.read().map_or_else(
            || (start.x + delta.x, start.y + delta.y, String::new()),
            |rect| {
                (
                    rect.x + delta.x,
                    rect.y + delta.y,
                    format!("width: {}px; height: {}px;", rect.width, rect.height),
                )
            },
        );

        format!(
            "position: fixed; top: 0; left: 0; \
             transform: translate({x}px, {y}px); \
             pointer-events: none; z-index: 9999; \
             will-change: transform; {size}"
        )
    });

    rsx! {
        div { class: "taino-dnd-overlay", style: "{style}", {children} }
    }
}
