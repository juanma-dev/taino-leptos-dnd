//! Viewport-edge auto-scroll driven by a `requestAnimationFrame` loop,
//! plus a window `scroll` listener that handles user-initiated
//! scrolling during a drag.
//!
//! Mirror of `taino_dnd_leptos::autoscroll`. `install` is called from
//! [`provide_dnd_context`](crate::provide_dnd_context) and wires:
//!
//! 1. A `use_effect` that, when state enters `Dragging`, schedules a
//!    RAF loop. Each tick computes a scroll velocity via
//!    [`taino_dnd_core::scroll_velocity`] and calls
//!    `window.scrollBy(dx, dy)`.
//! 2. A `scroll` event listener on the window. While dragging, any
//!    scroll (programmatic from the RAF, or user-initiated via wheel,
//!    trackpad, or scrollbar) bumps `measurement_tick` to refresh
//!    droppable rects and re-runs collision detection at the
//!    unchanged pointer position. Without this listener, mouse-wheel
//!    scrolling mid-drag left rects stale and the highlighted target
//!    stuck on whichever card was last under the cursor.
//!
//! Scope is intentionally limited to the document/viewport. Arbitrary
//! overflow ancestors are deferred — measuring scroll containers
//! requires walking the parent chain and inspecting computed styles,
//! which is more plumbing than the 80% case needs.
//!
//! On non-wasm targets the whole module compiles to a single no-op
//! [`install`] so the rest of the crate stays target-agnostic.

// DndContext is a bag of Dioxus Signal handles. Each is small but Dioxus's
// `Signal<T>` has a few generics that push the struct past clippy's default
// threshold. The handles are still cheap to copy (each is a generational-box
// id); passing by value is the idiomatic pattern in this codebase.
#![allow(clippy::large_types_passed_by_value, clippy::redundant_pub_crate)]

use crate::context::DndContext;

#[cfg(target_arch = "wasm32")]
pub(crate) fn install(ctx: DndContext) {
    imp::install(ctx);
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) const fn install(_ctx: DndContext) {}

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::cell::Cell;
    use std::{cell::RefCell, rc::Rc};

    use dioxus::prelude::*;
    use taino_dnd_core::{scroll_velocity, DragState, Rect};
    use wasm_bindgen::{closure::Closure, JsCast};

    use crate::context::DndContext;

    type CbCell = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

    pub(super) fn install(ctx: DndContext) {
        // `provide_dnd_context` runs on every render of its host
        // component (Dioxus components re-execute when their reactive
        // state changes). The window scroll listener and the RAF
        // generation counter must therefore be installed **once per
        // component instance**, not once per render — without
        // `use_hook` we'd attach a fresh `forget()`-ed scroll listener
        // on every render, and after a few drags the page would be
        // running N listeners per scroll event with the cost growing
        // linearly. Visible as auto-scroll velocity decreasing as more
        // drops accumulate.
        use_hook(|| install_scroll_listener(ctx));
        let generation: Rc<Cell<u64>> = use_hook(|| Rc::new(Cell::new(0_u64)));

        // `use_memo` (not a raw read inside `use_effect`) is
        // load-bearing: every `pointermove` updates `state.current`, so
        // the unguarded effect re-fires on every move and spawns a
        // fresh RAF loop each time. With `use_memo`, the effect only
        // fires on the actual Idle/Pressed → Dragging transition, so
        // exactly one loop runs per drag (and self-terminates when the
        // state leaves Dragging).
        let is_dragging = use_memo(move || matches!(*ctx.state.read(), DragState::Dragging { .. }));
        use_effect(move || {
            if *is_dragging.read() {
                start_loop(ctx, &generation);
            }
        });
    }

    /// Install a window `scroll` listener that, while we're dragging,
    /// asks every droppable to re-measure and re-runs collision
    /// detection at the (unchanged) pointer position.
    ///
    /// **Skips** when `raf_scrolling` is `true` — that means the scroll
    /// was caused by the RAF loop's own `scrollBy`, and the RAF loop
    /// already handles remeasure + collision itself. Without this
    /// guard, every auto-scroll frame would produce a double reactive
    /// cascade (RAF + scroll listener both firing).
    fn install_scroll_listener(ctx: DndContext) {
        let Some(win) = web_sys::window() else {
            return;
        };
        let listener = Closure::wrap(Box::new(move |_: web_sys::Event| {
            if !matches!(*ctx.state.peek(), DragState::Dragging { .. }) {
                return;
            }
            // Skip if this scroll was caused by the RAF loop's scrollBy.
            if *ctx.raf_scrolling.peek() {
                return;
            }
            // Call remeasure_all directly (not request_remeasure) so the
            // short-circuit check works immediately: if the RAF loop
            // already measured this frame, no rects will have changed
            // and remeasure_all returns without notifying subscribers.
            ctx.remeasure_all();
            let DragState::Dragging { start, current, .. } = *ctx.state.peek() else {
                return;
            };
            let effective = ctx.effective_point(start, current);
            ctx.update_over(effective);
        }) as Box<dyn FnMut(web_sys::Event)>);
        let _ = win.add_event_listener_with_callback("scroll", listener.as_ref().unchecked_ref());
        // The listener lives for the page lifetime — a context is
        // installed once near the root of a drag-and-drop region.
        listener.forget();
    }

    fn start_loop(mut ctx: DndContext, generation: &Rc<Cell<u64>>) {
        let Some(win) = web_sys::window() else {
            return;
        };

        // Bump generation: any older RAF loop will see a mismatch and
        // self-terminate on its next tick.
        let my_gen = generation.get().wrapping_add(1);
        generation.set(my_gen);
        let gen = generation.clone();

        // Standard wasm-bindgen RAF re-scheduling pattern: the closure
        // holds a shared handle to itself so it can call
        // `requestAnimationFrame` recursively. We drop the closure (by
        // clearing the cell) when the drag ends to free the borrowed
        // `ctx`.
        let cb: CbCell = Rc::new(RefCell::new(None));
        let cb_clone = cb.clone();
        let win_for_cb = win.clone();

        *cb.borrow_mut() = Some(Closure::wrap(Box::new(move || {
            // Stale loop from a previous drag? Terminate.
            if gen.get() != my_gen {
                cb_clone.borrow_mut().take();
                return;
            }

            let DragState::Dragging { current, .. } = *ctx.state.peek() else {
                // Drag ended — drop the closure to free `ctx`.
                cb_clone.borrow_mut().take();
                return;
            };

            let config = *ctx.auto_scroll.peek();
            let v = scroll_velocity(current, viewport_rect(&win_for_cb), config);

            if v.x.abs() > f64::EPSILON || v.y.abs() > f64::EPSILON {
                // Guard: suppress the scroll listener while we do our
                // own scrollBy + remeasure + update_over sequence.
                ctx.raf_scrolling.set(true);
                win_for_cb.scroll_by_with_x_and_y(v.x, v.y);

                // Directly remeasure and update collision — this is
                // synchronous, so the guard is still active and the
                // scroll listener (which may fire synchronously from
                // scrollBy in some browsers) is suppressed.
                ctx.remeasure_all();
                let DragState::Dragging { start, .. } = *ctx.state.peek() else {
                    ctx.raf_scrolling.set(false);
                    return;
                };
                let effective = ctx.effective_point(start, current);
                ctx.update_over(effective);
                ctx.raf_scrolling.set(false);
            }

            // Schedule the next tick while still dragging.
            let next = cb_clone.borrow();
            if let Some(closure) = next.as_ref() {
                let _ = win_for_cb.request_animation_frame(closure.as_ref().unchecked_ref());
            }
            drop(next);
        }) as Box<dyn FnMut()>));

        let first = cb.borrow();
        if let Some(closure) = first.as_ref() {
            let _ = win.request_animation_frame(closure.as_ref().unchecked_ref());
        }
        drop(first);
    }

    fn viewport_rect(win: &web_sys::Window) -> Rect {
        let w = win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
        let h = win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
        Rect::new(0.0, 0.0, w, h)
    }
}
