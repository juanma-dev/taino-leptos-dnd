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

use crate::context::{use_dnd_context, DROP_ANIMATION_MS};

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
        // The overlay container is sized to the dragged element's original
        // bounding rect so the preview matches the source's footprint.
        let size = ctx
            .dragged_element_rect
            .get()
            .map_or(String::new(), |r| format!("width: {}px; height: {}px;", r.width, r.height));

        let (x, y, transition) = match ctx.state.get() {
            // Active drag: track the modifier-adjusted pointer. Position the
            // overlay at the dragged element's original top-left plus the
            // drag delta, preserving the cursor's grab-point within the card.
            DragState::Dragging { start, current, .. } => {
                let delta = ctx.modify(Vector::new(current.x - start.x, current.y - start.y));
                let (bx, by) =
                    ctx.dragged_element_rect.get().map_or((start.x, start.y), |r| (r.x, r.y));
                (bx + delta.x, by + delta.y, String::new())
            }
            // Drop-settle: glide from the release position to the landing slot
            // (a CSS transition; the `Settle` timer unmounts us when it ends).
            DragState::Dropping { .. } => match ctx.drop_target.get() {
                Some(p) => (
                    p.x,
                    p.y,
                    format!(
                        "transition: transform {DROP_ANIMATION_MS}ms cubic-bezier(0.2, 0, 0, 1); "
                    ),
                ),
                None => return String::new(),
            },
            _ => return String::new(),
        };

        format!(
            "position: fixed; top: 0; left: 0; \
             transform: translate({x}px, {y}px); \
             pointer-events: none; z-index: 9999; \
             will-change: transform; {transition}{size}"
        )
    };

    view! {
        <Show
            when=move || {
                matches!(ctx.state.get(), DragState::Dragging { .. } | DragState::Dropping { .. })
            }
            fallback=|| ()
        >
            <div class="taino-dnd-overlay" style=style>
                {children()}
            </div>
        </Show>
    }
}
