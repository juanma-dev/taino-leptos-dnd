//! `DragOverlay` — a floating preview that mirrors the active drag.
//!
//! Render `<DragOverlay>...</DragOverlay>` once near the root of your app
//! (after `provide_dnd_context()`). While a drag is active, the overlay's
//! container becomes a fixed-position layer translated to the
//! (modifier-adjusted) pointer position. Children render only when there is
//! an active drag.
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

    // The fixed-position layer sits at the document origin; we move its
    // contents to the (modifier-adjusted) pointer position.
    let translate = Signal::derive(move || match ctx.state.get() {
        DragState::Dragging { start, current, .. } => {
            let raw = Vector::new(current.x - start.x, current.y - start.y);
            let m = ctx.modify(raw);
            (start.x + m.x, start.y + m.y)
        }
        _ => (0.0, 0.0),
    });

    let style = move || {
        let (x, y) = translate.get();
        format!(
            "position: fixed; top: 0; left: 0; \
             transform: translate({x}px, {y}px); \
             pointer-events: none; z-index: 9999; \
             will-change: transform;"
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
