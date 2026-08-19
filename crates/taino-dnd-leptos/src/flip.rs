//! FLIP-based reorder animations for non-dragged items.
//!
//! FLIP = First, Last, Invert, Play. After a layout change the element is
//! already in its **L**ast position; we measure the delta to where it was
//! (**F**irst), apply the inverse transform so it visually stays at the old
//! position, then transition the transform back to identity (**P**lay).
//!
//! Wire `use_flip(node_ref)` to any element whose layout position you want
//! animated (typically the row container of a sortable list — *not* the
//! draggable handle, which manages its own inline transform).
//!
//! # Behavior
//!
//! - Suppressed while the state machine is in `Pressed` or `Dragging` —
//!   measuring during a drag would race the drag transform.
//! - Respects `prefers-reduced-motion: reduce` (no-op when set).
//! - Uses CSS transitions with a forced reflow, so the animation is GPU
//!   accelerated and survives Leptos re-renders.

#![allow(clippy::redundant_pub_crate)]

use std::{fmt::Debug, hash::Hash};

use leptos::{html::Div, prelude::*};

/// Configuration for [`use_flip_with`]. The default is 220 ms with a
/// standard ease-out curve.
#[derive(Debug, Clone, Copy)]
pub struct FlipConfig {
    /// Animation duration in milliseconds.
    pub duration_ms: u32,
    /// CSS timing function.
    pub easing: &'static str,
}

impl Default for FlipConfig {
    fn default() -> Self {
        Self { duration_ms: 220, easing: "cubic-bezier(0.2, 0, 0, 1)" }
    }
}

/// Install FLIP animation tracking on the element behind `node_ref`.
///
/// Equivalent to [`use_flip_with`] with [`FlipConfig::default`].
///
/// # Example
///
/// ```no_run
/// use leptos::prelude::*;
/// use taino_dnd_core::DroppableId;
/// use taino_dnd_leptos::{use_droppable, use_flip};
///
/// #[component]
/// fn Row(id: u64) -> impl IntoView {
///     let z = use_droppable(DroppableId(id));
///     use_flip(z.node_ref);
///     view! { <div node_ref=z.node_ref class="row">"item"</div> }
/// }
/// ```
pub fn use_flip<T>(node_ref: NodeRef<Div>)
where
    T: Debug + Clone + Copy + PartialEq + Eq + Hash + PartialOrd + Ord + Send + Sync + 'static,
{
    use_flip_with::<T>(node_ref, FlipConfig::default());
}

/// Like [`use_flip`] but with a custom [`FlipConfig`].
#[cfg_attr(not(target_arch = "wasm32"), allow(clippy::missing_const_for_fn, unused_variables))]
pub fn use_flip_with<T>(node_ref: NodeRef<Div>, config: FlipConfig)
where
    T: Debug + Clone + Copy + PartialEq + Eq + Hash + PartialOrd + Ord + Send + Sync + 'static,
{
    #[cfg(target_arch = "wasm32")]
    imp::install::<T>(node_ref, config);
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::{fmt::Debug, hash::Hash};

    use leptos::{html::Div, prelude::*};
    use taino_dnd_core::{DragState, Rect};
    use wasm_bindgen::JsCast;

    use super::FlipConfig;
    use crate::context::use_dnd_context;

    pub(super) fn install<T>(node_ref: NodeRef<Div>, config: FlipConfig)
    where
        T: Debug + Clone + Copy + PartialEq + Eq + Hash + PartialOrd + Ord + Send + Sync + 'static,
    {
        let ctx = use_dnd_context::<T>();
        let last_rect = StoredValue::new(None::<Rect>);

        Effect::new(move |_| {
            // Subscribe to state so the effect re-runs after every drag
            // transition (including Dropping → Idle, which is when the
            // user's items vec is typically reordered).
            let state = ctx.state.get();
            // Don't measure or animate during an active drag. The dragged
            // element's inline transform would constantly trigger FLIPs on
            // the other items via the same effect.
            if matches!(state, DragState::Pressed { .. } | DragState::Dragging { .. }) {
                return;
            }

            let Some(div) = node_ref.get() else { return };
            let element: &web_sys::Element = (*div).as_ref();
            let new_rect = crate::dom::bounding_rect(element);

            if let Some(prev) = last_rect.get_value() {
                let dx = prev.x - new_rect.x;
                let dy = prev.y - new_rect.y;
                if dx.abs() > 1.0 || dy.abs() > 1.0 {
                    play(element, dx, dy, config);
                }
            }
            last_rect.set_value(Some(new_rect));
        });
    }

    fn play(el: &web_sys::Element, dx: f64, dy: f64, config: FlipConfig) {
        if prefers_reduced_motion() {
            return;
        }
        let Some(html_el) = el.dyn_ref::<web_sys::HtmlElement>() else {
            return;
        };
        let style = html_el.style();
        // 1. Disable transitions, jump to the First (inverted) position.
        let _ = style.set_property("transition", "none");
        let _ = style.set_property("transform", &format!("translate({dx}px, {dy}px)"));
        // 2. Force a synchronous reflow so the browser commits the
        //    no-transition state before we re-enable transitions.
        let _ = html_el.offset_height();
        // 3. Enable the transition and animate back to identity.
        let _ = style.set_property(
            "transition",
            &format!("transform {}ms {}", config.duration_ms, config.easing),
        );
        let _ = style.set_property("transform", "translate(0, 0)");
    }

    fn prefers_reduced_motion() -> bool {
        web_sys::window()
            .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
            .is_some_and(|m| m.matches())
    }
}
