//! Viewport-edge auto-scroll driven by a `requestAnimationFrame` loop.
//!
//! `install` is called from [`provide_dnd_context`](crate::provide_dnd_context)
//! and wires an `Effect` that, while the state is `Dragging`, schedules RAF
//! ticks that:
//!
//! 1. Compute a scroll velocity from the current pointer position and the
//!    viewport rect, via [`taino_dnd_core::scroll_velocity`].
//! 2. `window.scrollBy(dx, dy)` when non-zero.
//! 3. Bump `ctx.measurement_tick` so each `use_droppable` re-measures.
//! 4. Re-run collision detection at the (unchanged) pointer position.
//!
//! Scope is intentionally limited to the document/viewport. Arbitrary
//! overflow ancestors are deferred — measuring scroll containers requires
//! walking the parent chain and inspecting computed styles, which is more
//! plumbing than the 80% case needs.
//!
//! On non-wasm targets the whole module compiles to a single no-op
//! [`install`] so the rest of the crate stays target-agnostic.

#![allow(clippy::redundant_pub_crate)]

use crate::context::DndContext;

#[cfg(target_arch = "wasm32")]
pub(crate) fn install(ctx: DndContext) {
    imp::install(ctx);
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) const fn install(_ctx: DndContext) {}

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::{cell::RefCell, rc::Rc};

    use leptos::prelude::*;
    use taino_dnd_core::{scroll_velocity, DragState, Rect};
    use wasm_bindgen::{closure::Closure, JsCast};

    use crate::context::DndContext;

    type CbCell = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

    pub(super) fn install(ctx: DndContext) {
        // `Memo` (not `Signal::derive`) is load-bearing: every
        // `pointermove` updates `state.current`, so a non-deduped
        // derived signal would re-fire this effect on every move and
        // spawn a fresh RAF loop each time. With `Memo`, the effect
        // only fires on the actual Idle/Pressed → Dragging transition,
        // so exactly one loop runs per drag (and self-terminates when
        // the state leaves Dragging).
        let is_dragging = Memo::new(move |_| matches!(ctx.state.get(), DragState::Dragging { .. }));
        Effect::new(move |_| {
            if is_dragging.get() {
                start_loop(ctx);
            }
        });
    }

    fn start_loop(ctx: DndContext) {
        let Some(win) = web_sys::window() else {
            return;
        };

        // Standard wasm-bindgen RAF re-scheduling pattern: the closure holds
        // a shared handle to itself so it can call `requestAnimationFrame`
        // recursively. We drop the closure (by clearing the cell) when the
        // drag ends to free the borrowed `ctx`.
        let cb: CbCell = Rc::new(RefCell::new(None));
        let cb_clone = cb.clone();
        let win_for_cb = win.clone();

        *cb.borrow_mut() = Some(Closure::wrap(Box::new(move || {
            let DragState::Dragging { current, .. } = ctx.state.get_untracked() else {
                // Drag ended — drop the closure to free `ctx`.
                cb_clone.borrow_mut().take();
                return;
            };

            let config = ctx.auto_scroll.get_untracked();

            let viewport_rect = viewport_rect(&win_for_cb);
            let v = scroll_velocity(current, viewport_rect, config);

            if v.x.abs() > f64::EPSILON || v.y.abs() > f64::EPSILON {
                win_for_cb.scroll_by_with_x_and_y(v.x, v.y);
                ctx.request_remeasure();
                // Re-run collision at the (unchanged) pointer position so the
                // highlighted droppable updates as droppables move under the
                // cursor.
                let DragState::Dragging { start, .. } = ctx.state.get_untracked() else {
                    return;
                };
                let effective = ctx.effective_point(start, current);
                ctx.update_over(effective);
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
