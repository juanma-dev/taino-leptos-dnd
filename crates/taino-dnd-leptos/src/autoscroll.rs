//! Auto-scroll driven by a `requestAnimationFrame` loop, plus a **capturing**
//! `scroll` listener that handles user-initiated scrolling (wheel, trackpad,
//! scrollbar) during a drag.
//!
//! `install` is called from [`provide_dnd_context`](crate::provide_dnd_context)
//! and wires:
//!
//! 1. An `Effect` that, when state enters `Dragging`, schedules a RAF loop.
//!    Each tick looks for the innermost **scroll container** under the pointer
//!    that can still scroll toward the pointer's edge and scrolls that;
//!    failing that, it scrolls the **window**. Edge math comes from
//!    [`taino_dnd_core::scroll_velocity`] (which works for any rect — viewport
//!    or container).
//! 2. A capturing `scroll` listener on the window. `scroll` events don't
//!    bubble, but the capture phase still reaches a window-level listener for
//!    scrolls of *any* descendant element — so this single listener catches
//!    both window scrolls and overflow-container scrolls. While dragging, it:
//!    handles **window scroll** by shifting every droppable rect by the
//!    inverse delta (`shift_droppable_rects`; cheap, no `getComputedStyle` per
//!    frame), and **container scroll** by `request_remeasure` (only that
//!    container's descendants moved, so a blanket shift would be wrong). Then
//!    it re-runs collision detection at the unchanged pointer position.
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
    use std::cell::Cell;
    use std::{cell::RefCell, rc::Rc};

    use leptos::prelude::*;
    use taino_dnd_core::{scroll_velocity, DragState, Rect};
    use wasm_bindgen::{closure::Closure, JsCast};

    use crate::context::DndContext;

    type CbCell = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

    pub(super) fn install(ctx: DndContext) {
        // Shared window-scroll tracker. The scroll listener computes the
        // delta from this value on each window scroll and updates it.
        let last_scroll = Rc::new(Cell::new(read_window_scroll()));

        install_scroll_listener(ctx, last_scroll);

        // Generation counter: bumped every time `start_loop` is called. Each
        // RAF closure captures the generation it was created at and
        // self-terminates if a newer generation exists — preventing loop
        // accumulation when a new drag starts before the previous loop has
        // seen the idle transition.
        let generation = Rc::new(Cell::new(0_u64));

        // `Memo` (not `Signal::derive`) is load-bearing: every `pointermove`
        // updates `state.current`, so a non-deduped derived signal would
        // re-fire this effect on every move and spawn a fresh RAF loop each
        // time. With `Memo`, the effect only fires on the actual
        // Idle/Pressed → Dragging transition.
        let is_dragging = Memo::new(move |_| matches!(ctx.state.get(), DragState::Dragging { .. }));
        Effect::new(move |_| {
            if is_dragging.get() {
                start_loop(ctx, &generation);
            }
        });
    }

    fn read_window_scroll() -> (f64, f64) {
        web_sys::window()
            .map_or((0.0, 0.0), |w| (w.scroll_x().unwrap_or(0.0), w.scroll_y().unwrap_or(0.0)))
    }

    /// Install a **capturing** window `scroll` listener that handles every
    /// scroll while dragging — programmatic (the RAF loop's `scrollBy` /
    /// `scrollTop`) and user-initiated (wheel, trackpad, scrollbar), for both
    /// the window and any overflow container.
    fn install_scroll_listener(ctx: DndContext, last_scroll: Rc<Cell<(f64, f64)>>) {
        let Some(win) = web_sys::window() else {
            return;
        };
        let win_for_listener = win.clone();
        let listener = Closure::wrap(Box::new(move |ev: web_sys::Event| {
            // Always keep the window-scroll tracker fresh so the first delta
            // of the next drag is correct.
            let cur_x = win_for_listener.scroll_x().unwrap_or(0.0);
            let cur_y = win_for_listener.scroll_y().unwrap_or(0.0);
            let (last_x, last_y) = last_scroll.get();
            let dx = cur_x - last_x;
            let dy = cur_y - last_y;
            last_scroll.set((cur_x, cur_y));

            if !matches!(ctx.state.get_untracked(), DragState::Dragging { .. }) {
                return;
            }

            // A window/document scroll has the document as its target; a
            // container scroll has the element as its target.
            let is_document =
                ev.target().is_none_or(|t| t.dyn_ref::<web_sys::Document>().is_some());

            if is_document {
                if dx == 0.0 && dy == 0.0 {
                    return;
                }
                // Viewport scrolled by (dx, dy): every rect's origin shifted
                // by -(dx, dy).
                ctx.shift_droppable_rects(-dx, -dy);
            } else {
                // Overflow container scrolled: only its descendants moved, so
                // re-measure (transform-safe) rather than shift everything.
                ctx.request_remeasure();
            }

            let DragState::Dragging { start, current, .. } = ctx.state.get_untracked() else {
                return;
            };
            let effective = ctx.effective_point(start, current);
            ctx.update_over(effective);
        }) as Box<dyn FnMut(web_sys::Event)>);
        // `true` = capture phase, so scrolls of descendant elements (which
        // don't bubble) still reach this single window-level listener.
        let _ = win.add_event_listener_with_callback_and_bool(
            "scroll",
            listener.as_ref().unchecked_ref(),
            true,
        );
        // The listener lives for the page lifetime — a context is installed
        // once near the root of a drag-and-drop region.
        listener.forget();
    }

    fn start_loop(ctx: DndContext, generation: &Rc<Cell<u64>>) {
        let Some(win) = web_sys::window() else {
            return;
        };

        // Bump generation: any older RAF loop will see a mismatch and
        // self-terminate on its next tick.
        let my_gen = generation.get().wrapping_add(1);
        generation.set(my_gen);
        let gen = generation.clone();

        // Standard wasm-bindgen RAF re-scheduling pattern: the closure holds a
        // shared handle to itself so it can re-`requestAnimationFrame`. We
        // drop the closure (by clearing the cell) when the drag ends.
        let cb: CbCell = Rc::new(RefCell::new(None));
        let cb_clone = cb.clone();
        let win_for_cb = win.clone();

        *cb.borrow_mut() = Some(Closure::wrap(Box::new(move || {
            // Stale loop from a previous drag? Terminate.
            if gen.get() != my_gen {
                cb_clone.borrow_mut().take();
                return;
            }

            let DragState::Dragging { current, .. } = ctx.state.get_untracked() else {
                // Drag ended — drop the closure to free `ctx`.
                cb_clone.borrow_mut().take();
                return;
            };

            let config = ctx.auto_scroll.get_untracked();

            // Prefer the innermost scroll container under the pointer that can
            // still scroll toward the pointer's edge. The resulting scroll
            // (element `scrollTop`/`scrollLeft` or `window.scrollBy`) fires the
            // capturing `scroll` listener, which is the single source of truth
            // for rect updates + collision detection.
            let mut handled = false;
            if let Some(target) = crate::dom::element_from_point(current.x, current.y) {
                for el in crate::dom::scrollable_ancestors(&target) {
                    let v = scroll_velocity(current, crate::dom::bounding_rect_raw(&el), config);
                    let moving = v.x.abs() > f64::EPSILON || v.y.abs() > f64::EPSILON;
                    if moving && crate::dom::can_scroll(&el, v.x, v.y) {
                        crate::dom::scroll_element_by(&el, v.x, v.y);
                        handled = true;
                        break;
                    }
                }
            }
            if !handled {
                let v = scroll_velocity(current, viewport_rect(&win_for_cb), config);
                if v.x.abs() > f64::EPSILON || v.y.abs() > f64::EPSILON {
                    win_for_cb.scroll_by_with_x_and_y(v.x, v.y);
                }
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
