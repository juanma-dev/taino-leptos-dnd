//! `DragOverlay` — a floating preview that mirrors the active drag.
//!
//! Render `<DragOverlay>...</DragOverlay>` once near the root of your app
//! (after `provide_dnd_context()`). While a drag is active, the overlay's
//! container is a fixed-position layer sized to the dragged element's
//! original bounding rect and translated so the cursor stays at the
//! original grab-point relative to the card. Children render only while
//! a drag is in progress.
//!
//! Using the overlay decouples the visual preview from the source element's
//! DOM position, which is what enables the dragged item to "escape" parent
//! `overflow: hidden`, transforms, and stacking contexts.
//!
//! See [`UseDraggable::style_pinned`](crate::UseDraggable::style_pinned) for
//! the matching source-element style that omits the inline `translate` so the
//! source doesn't double up with the overlay.

// The `#[component]` macro generates a `pub` function the `unreachable_pub`
// lint can't see through, just like `DndAnnouncer`.
#![allow(unreachable_pub)]

use leptos::prelude::*;
use taino_dnd_core::{DragState, Vector};

use crate::context::use_dnd_context;

/// A fixed-position layer that follows the active drag.
///
/// The overlay's container is sized to the dragged element's original
/// bounding rect (captured at `pointerdown` / `keyboard-pickup`) and
/// translated so the cursor stays at the same point within the card as
/// where the user grabbed it — the dnd-kit / react-beautiful-dnd
/// "grab-point preservation" pattern.
///
/// `children` is rendered inside the layer whenever a drag is in progress,
/// and removed when the state returns to [`DragState::Idle`]. The layer has
/// `pointer-events: none` so it never intercepts hit testing — the underlying
/// draggable still owns the pointer.
///
/// # Example
///
/// ```no_run
/// use leptos::prelude::*;
/// use taino_dnd_core::DragState;
/// use taino_dnd_leptos::{provide_dnd_context, use_dnd_context, DragOverlay};
///
/// #[component]
/// fn App() -> impl IntoView {
///     provide_dnd_context();
///     view! {
///         <DragOverlay>
///             {move || {
///                 let ctx = use_dnd_context();
///                 ctx.state.get().dragged_id().map(|id| view! {
///                     <div class="overlay-card">{format!("Item #{}", id.0)}</div>
///                 })
///             }}
///         </DragOverlay>
///     }
/// }
/// ```
#[component]
pub fn DragOverlay(children: ChildrenFn) -> impl IntoView {
    let ctx = use_dnd_context();

    let style = move || {
        let DragState::Dragging { start, current, .. } = ctx.state.get() else {
            return String::new();
        };
        let raw = Vector::new(current.x - start.x, current.y - start.y);
        let delta = ctx.modify(raw);

        // Position the overlay at the dragged element's original top-left
        // plus the modifier-adjusted drag delta. This preserves the cursor's
        // grab-point relative to the card. If we don't have the rect (rare —
        // it should always be set during a drag), fall back to placing the
        // overlay at the pointer.
        let (x, y, size) = ctx.dragged_element_rect.get().map_or_else(
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
    };

    view! {
        <Show
            when=move || ctx.state.get().is_dragging()
            fallback=|| ()
        >
            <div class="taino-dnd-overlay" style=style>
                {children()}
            </div>
        </Show>
    }
}
