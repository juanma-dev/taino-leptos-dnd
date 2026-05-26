//! FLIP-based reorder animations for non-dragged items (Dioxus binding).
//!
//! FLIP = First, Last, Invert, Play. After a layout change the element is
//! already in its **L**ast position; we measure the delta to where it was
//! (**F**irst), apply the inverse transform so it visually stays at the old
//! position, then transition the transform back to identity (**P**lay).
//!
//! Mirrors [`taino_dnd_leptos::use_flip`](https://docs.rs/taino-dnd-leptos)
//! 1:1 in behavior. The only API difference is the handle you pass in:
//! where the Leptos binding takes a `NodeRef`, the Dioxus binding takes the
//! mounted-element signal exposed by [`use_droppable`](crate::use_droppable)
//! as [`UseDroppable::element`](crate::UseDroppable::element).
//!
//! # Behavior
//!
//! - Suppressed while the state machine is in `Pressed` or `Dragging` —
//!   measuring during a drag would race the drag transform.
//! - Respects `prefers-reduced-motion: reduce` (no-op when set).
//! - Uses CSS transitions with a forced reflow, so the animation is GPU
//!   accelerated.
//!
//! # Choosing FLIP vs. live drop-preview
//!
//! This crate ships two complementary reorder animations:
//!
//! - **Live drop-preview** ([`UseDroppable::drop_preview_style`](crate::UseDroppable::drop_preview_style)):
//!   neighbors shift *during* the drag to reveal where the item will land.
//! - **FLIP** (this module): the *post-drop* settle — items glide from
//!   their old slots to their new ones after the user releases.
//!
//! Use one or the other on a given element, **not both**: they each drive
//! the element's `transform`, and `drop_preview_style` binds it reactively
//! through Dioxus's `style` attribute while FLIP mutates it directly on the
//! DOM node. If both target the same element, Dioxus's reconciliation of
//! the `style` attribute clobbers FLIP's inverted transform. Apply FLIP to
//! a wrapper element that has **no reactive `style` binding** (see the
//! `kanban-dioxus` example's `card-slot`).

#![allow(clippy::redundant_pub_crate)]

use std::rc::Rc;

use dioxus::prelude::*;

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

/// Install FLIP animation tracking on the element behind a droppable's
/// [`element`](crate::UseDroppable::element) signal.
///
/// Equivalent to [`use_flip_with`] with [`FlipConfig::default`].
///
/// # Example
///
/// ```ignore
/// use dioxus::prelude::*;
/// use taino_dnd_core::DroppableId;
/// use taino_dnd_dioxus::{use_droppable, use_flip};
///
/// #[component]
/// fn Row(id: u64) -> Element {
///     let z = use_droppable(DroppableId(id));
///     use_flip(z.element);
///     rsx! {
///         div { class: "row", onmounted: move |e| z.on_mounted(e), "item" }
///     }
/// }
/// ```
pub fn use_flip(element: Signal<Option<Rc<MountedData>>>) {
    use_flip_with(element, FlipConfig::default());
}

/// Like [`use_flip`] but with a custom [`FlipConfig`].
#[cfg_attr(not(target_arch = "wasm32"), allow(clippy::missing_const_for_fn, unused_variables))]
pub fn use_flip_with(element: Signal<Option<Rc<MountedData>>>, config: FlipConfig) {
    #[cfg(target_arch = "wasm32")]
    imp::install(element, config);
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::rc::Rc;

    use dioxus::prelude::*;
    use taino_dnd_core::{DragState, Rect};
    use wasm_bindgen::JsCast;

    use super::FlipConfig;
    use crate::context::use_dnd_context;

    pub(super) fn install(element: Signal<Option<Rc<MountedData>>>, config: FlipConfig) {
        let ctx = use_dnd_context();
        // Holds the element's last-known layout rect. Read only via
        // `peek()` and written via `set()` so the effect never subscribes
        // to it — otherwise the `set()` below would re-trigger the effect
        // in a loop.
        let mut last_rect = use_signal(|| None::<Rect>);

        use_effect(move || {
            // Subscribe to state so the effect re-runs after every drag
            // transition (including Dropping → Idle, which is when the
            // user's items vec is typically reordered). `DragState` is
            // `Copy`, so this reads the value out and drops the borrow.
            let state = *ctx.state.read();
            // Don't measure or animate during an active drag. The dragged
            // element's inline transform would constantly trigger FLIPs on
            // the other items via the same effect.
            if matches!(state, DragState::Pressed { .. } | DragState::Dragging { .. }) {
                return;
            }

            // Clone the `Rc` out so the `element` read-borrow is released
            // before we touch `last_rect`.
            let Some(mounted) = element.read().clone() else {
                return;
            };
            let Some(el) = crate::dom::element_of(&mounted) else {
                return;
            };
            let new_rect = crate::dom::bounding_rect(&el);

            if let Some(prev) = *last_rect.peek() {
                let dx = prev.x - new_rect.x;
                let dy = prev.y - new_rect.y;
                if dx.abs() > 1.0 || dy.abs() > 1.0 {
                    play(&el, dx, dy, config);
                }
            }
            last_rect.set(Some(new_rect));
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
