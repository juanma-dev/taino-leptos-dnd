//! The [`use_drag_container`] helper hook.
//!
//! Pairs with [`Modifier::RestrictToParent`](taino_dnd_core::Modifier) to
//! constrain drags inside a user-chosen container element.

use std::{fmt::Debug, hash::Hash};

use leptos::{html::Div, prelude::*};

use crate::context::use_dnd_context;

/// Register a container element. Returns a [`NodeRef<Div>`] for the user to
/// attach to whichever element should bound the drags.
///
/// The hook installs an `Effect` that mirrors the element's bounding rect
/// into `ctx.restrict_container` on mount and on every measurement tick
/// (so the rect stays accurate as the page scrolls during auto-scroll).
///
/// # Example
///
/// ```no_run
/// use leptos::prelude::*;
/// use taino_dnd_leptos::{provide_dnd_context, use_drag_container, Modifier};
///
/// #[component]
/// fn Board() -> impl IntoView {
///     let ctx = provide_dnd_context();
///     ctx.push_modifier(Modifier::RestrictToParent);
///     let container = use_drag_container();
///     view! {
///         <div node_ref=container class="board">
///             // ...draggable items inside...
///         </div>
///     }
/// }
/// ```
pub fn use_drag_container<T>() -> NodeRef<Div>
where
    T: Debug + Clone + Copy + PartialEq + Eq + Hash + PartialOrd + Ord + Send + Sync + 'static,
{
    let ctx = use_dnd_context::<T>();
    let node_ref = NodeRef::<Div>::new();

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        Effect::new(move |_| {
            // Subscribe to the measurement tick so the rect re-measures on
            // every auto-scroll step (the container's viewport rect shifts
            // as the document scrolls).
            ctx.measurement_tick.get();
            if let Some(el) = node_ref.get() {
                if let Some(el) = (*el).dyn_ref::<web_sys::Element>() {
                    let rect = crate::dom::bounding_rect(el);
                    ctx.set_restrict_container(Some(rect));
                }
            }
        });
    }

    on_cleanup(move || {
        ctx.set_restrict_container(None);
    });

    node_ref
}
