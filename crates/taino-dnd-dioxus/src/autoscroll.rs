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
        // Shared scroll position tracker, written by **both** the RAF
        // tick (after its programmatic `scrollBy`) and the window
        // `scroll` listener (on every event). Sharing this is what
        // lets the listener naturally skip RAF-induced scroll events:
        // by the time the async `scroll` event fires for the RAF's
        // `scrollBy`, the RAF has already updated `last_scroll` to the
        // post-scroll position, so the listener computes a zero delta
        // and bails. No `raf_scrolling` flag needed.
        let last_scroll = Rc::new(Cell::new(read_window_scroll()));

        install_scroll_listener(ctx, last_scroll.clone());

        // Generation counter: bumped every time `start_loop` is called.
        // Each RAF closure captures the generation it was created at and
        // self-terminates if a newer generation exists — this prevents
        // loop accumulation when a new drag starts before the previous
        // loop has had a chance to see the idle transition.
        let generation = Rc::new(Cell::new(0_u64));

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
                start_loop(ctx, &generation, last_scroll.clone());
            }
        });
    }

    fn read_window_scroll() -> (f64, f64) {
        web_sys::window()
            .map_or((0.0, 0.0), |w| (w.scroll_x().unwrap_or(0.0), w.scroll_y().unwrap_or(0.0)))
    }

    /// Install a window `scroll` listener that handles **all** scrolls
    /// while dragging — both the programmatic ones produced by the RAF
    /// loop's `scrollBy` and user-initiated ones (wheel, trackpad,
    /// scrollbar). On each event it computes the scroll delta against
    /// the last observed position and applies it to the droppable
    /// registry via `shift_droppable_rects`, then re-runs collision at
    /// the unchanged pointer position.
    ///
    /// Shift (instead of `remeasure_all`) is the key detail: a
    /// remeasure mid-drag would call `getBoundingClientRect` on
    /// elements that have the drop-preview `transform: translate(...)`
    /// applied, feeding the transform back into the registry and
    /// producing a flicker loop.
    fn install_scroll_listener(ctx: DndContext, last_scroll: Rc<Cell<(f64, f64)>>) {
        let Some(win) = web_sys::window() else {
            return;
        };
        let win_for_listener = win.clone();
        let listener = Closure::wrap(Box::new(move |_: web_sys::Event| {
            let cur_x = win_for_listener.scroll_x().unwrap_or(0.0);
            let cur_y = win_for_listener.scroll_y().unwrap_or(0.0);
            let (last_x, last_y) = last_scroll.get();
            let dx = cur_x - last_x;
            let dy = cur_y - last_y;
            last_scroll.set((cur_x, cur_y));

            if !matches!(*ctx.state.peek(), DragState::Dragging { .. }) {
                return;
            }
            if dx == 0.0 && dy == 0.0 {
                return;
            }

            ctx.shift_droppable_rects(-dx, -dy);

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

    fn start_loop(ctx: DndContext, generation: &Rc<Cell<u64>>, last_scroll: Rc<Cell<(f64, f64)>>) {
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
                // Measure scroll position before and after so the
                // shift is sized by what actually moved (clamped to 0
                // at page edges). Update `last_scroll` to the
                // post-scroll value: the browser's async `scroll`
                // event will then fire on the listener with a zero
                // delta and bail, so the RAF and listener don't both
                // try to shift for the same scroll.
                let (before_x, before_y) = (
                    win_for_cb.scroll_x().unwrap_or(0.0),
                    win_for_cb.scroll_y().unwrap_or(0.0),
                );
                win_for_cb.scroll_by_with_x_and_y(v.x, v.y);
                let (after_x, after_y) = (
                    win_for_cb.scroll_x().unwrap_or(0.0),
                    win_for_cb.scroll_y().unwrap_or(0.0),
                );
                last_scroll.set((after_x, after_y));

                let dx = after_x - before_x;
                let dy = after_y - before_y;
                if dx != 0.0 || dy != 0.0 {
                    ctx.shift_droppable_rects(-dx, -dy);
                    if let DragState::Dragging { start, .. } = *ctx.state.peek() {
                        let effective = ctx.effective_point(start, current);
                        ctx.update_over(effective);
                    }
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
