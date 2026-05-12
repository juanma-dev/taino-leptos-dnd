//! The [`use_droppable`] hook.

use leptos::{html::Div, prelude::*};
#[cfg(target_arch = "wasm32")]
use taino_dnd_core::DragState;
use taino_dnd_core::DroppableId;

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
    /// The identifier this hook was instantiated with.
    pub id: DroppableId,
    #[allow(dead_code)]
    ctx: DndContext,
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
    let ctx = use_dnd_context();
    let node_ref = NodeRef::<Div>::new();

    let is_over = Signal::derive(move || ctx.over.get() == Some(id));

    // Measure the rect once the node is attached.
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        Effect::new(move |_| {
            if let Some(el) = node_ref.get() {
                if let Some(el) = (*el).dyn_ref::<web_sys::Element>() {
                    let rect = crate::dom::bounding_rect(el);
                    ctx.upsert_droppable(id, rect);
                }
            }
        });

        // Re-measure when a drag starts. Layout can change between drags
        // (e.g. items reordered after a previous drop).
        Effect::new(move |_| {
            if matches!(ctx.state.get(), DragState::Pressed { .. }) {
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

    UseDroppable { node_ref, is_over, id, ctx }
}
