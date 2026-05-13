//! The [`use_drag_container`] helper hook.
//!
//! Pairs with [`Modifier::RestrictToParent`](taino_dnd_core::Modifier)
//! to constrain drags inside a user-chosen container element.

#![allow(clippy::needless_pass_by_value)]

use std::rc::Rc;

use dioxus::prelude::*;

use crate::context::{use_dnd_context, DndContext};

/// Handle returned by [`use_drag_container`]. Wire its `on_mounted`
/// callback to whichever element should bound the drags.
#[derive(Clone, Copy)]
pub struct UseDragContainer {
    /// The mounted container (after `onmounted` fires).
    pub element: Signal<Option<Rc<MountedData>>>,
    /// Only read from the wasm32 `on_mounted` path (where we measure
    /// and write the container rect). On native it's stored but
    /// unused — kept for API symmetry with the wasm32 build.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    ctx: DndContext,
}

impl UseDragContainer {
    /// `onmounted` handler. Captures the element handle and immediately
    /// measures its bounding rect into the shared
    /// [`DndContext::restrict_container`].
    pub fn on_mounted(mut self, ev: Event<MountedData>) {
        let data = ev.data();
        #[cfg(target_arch = "wasm32")]
        if let Some(rect) = crate::dom::bounding_rect_of(&data) {
            self.ctx.set_restrict_container(Some(rect));
        }
        self.element.set(Some(data));
    }
}

/// Register a container element that bounds drags inside the current
/// [`DndContext`](crate::DndContext).
///
/// Pairs with [`Modifier::RestrictToParent`](taino_dnd_core::Modifier):
/// push the modifier onto the context, then attach the container's
/// `onmounted` to this hook. The container's bounding rect is measured
/// when the element mounts; a follow-up slice will add re-measurement
/// on every press-start and auto-scroll tick.
///
/// On unmount the container rect is cleared so a stale rect doesn't
/// outlive the element.
///
/// # Example
///
/// ```ignore
/// use dioxus::prelude::*;
/// use taino_dnd_dioxus::{provide_dnd_context, use_drag_container, Modifier};
///
/// fn Board() -> Element {
///     let ctx = provide_dnd_context();
///     ctx.push_modifier(Modifier::RestrictToParent);
///     let container = use_drag_container();
///     rsx! {
///         div {
///             onmounted: move |e| container.on_mounted(e),
///             class: "board",
///             // ...draggable items inside...
///         }
///     }
/// }
/// ```
pub fn use_drag_container() -> UseDragContainer {
    let ctx = use_dnd_context();
    let element = use_signal::<Option<Rc<MountedData>>>(|| None);

    use_drop(move || ctx.set_restrict_container(None));

    UseDragContainer { element, ctx }
}
