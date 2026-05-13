//! The [`use_draggable`] hook for Dioxus.

// Dioxus event handlers idiomatically take `Event<T>` by value (the
// type is a thin Rc handle, so passing by value is cheap and matches
// `rsx!` ergonomics). Take the lint at the module level once.
#![allow(clippy::needless_pass_by_value)]

use std::rc::Rc;

use dioxus::html::input_data::MouseButton;
use dioxus::prelude::*;
use taino_dnd_core::{
    transition, DragEvent, DragState, DraggableId, Point, DEFAULT_DRAG_THRESHOLD,
};

use crate::context::{use_dnd_context, DndContext, DropResult};

/// Handle returned by [`use_draggable`]. Wire its fields onto a `<div>`
/// in your `rsx!`.
///
/// `UseDraggable` is [`Copy`] — every field is a Dioxus `Signal` /
/// `Memo` handle, so it can be moved into multiple event closures in
/// the same component without cloning.
#[derive(Clone, Copy)]
pub struct UseDraggable {
    /// The identifier this hook was instantiated with.
    pub id: DraggableId,
    /// The mounted `<div>` (after `onmounted` fires). Wire via
    /// `onmounted: move |evt| handle.on_mounted(evt)`.
    pub element: Signal<Option<Rc<MountedData>>>,
    /// `true` while this specific draggable is being dragged.
    pub is_dragging: Memo<bool>,
    /// Translation (in CSS pixels) to apply while dragging. `(0.0, 0.0)`
    /// when not dragging.
    pub transform: Memo<(f64, f64)>,
    ctx: DndContext,
}

impl UseDraggable {
    /// `onmounted` handler. Stores a handle to the underlying element so
    /// subsequent pointer-capture / bounding-rect calls can reach it.
    pub fn on_mounted(mut self, ev: Event<MountedData>) {
        self.element.set(Some(ev.data()));
    }

    /// `onpointerdown` handler. Starts a `Pressed` state; movement past
    /// the drag threshold promotes it to `Dragging`.
    pub fn on_pointer_down(mut self, ev: Event<PointerData>) {
        if ev.trigger_button() != Some(MouseButton::Primary) {
            return;
        }
        let coords = ev.client_coordinates();
        let at = Point::new(coords.x, coords.y);
        let current = *self.ctx.state.peek();
        if let Ok(state) =
            transition(current, DragEvent::PointerDown { id: self.id, at }, DEFAULT_DRAG_THRESHOLD)
        {
            self.ctx.state.set(state);
            self.ctx.last_drop.set(None);
            #[cfg(target_arch = "wasm32")]
            self.capture_pointer(&ev);
        }
    }

    /// `onpointermove` handler.
    pub fn on_pointer_move(mut self, ev: Event<PointerData>) {
        let coords = ev.client_coordinates();
        let at = Point::new(coords.x, coords.y);
        let current = *self.ctx.state.peek();
        if !self.is_my_state(current) {
            return;
        }
        if let Ok(state) =
            transition(current, DragEvent::PointerMove { at }, DEFAULT_DRAG_THRESHOLD)
        {
            self.ctx.state.set(state);
            if matches!(state, DragState::Dragging { .. }) {
                self.ctx.update_over(at);
            }
        }
    }

    /// `onpointerup` handler.
    pub fn on_pointer_up(mut self, ev: Event<PointerData>) {
        let current = *self.ctx.state.peek();
        if !self.is_my_state(current) {
            return;
        }
        // Genuine drop (not a click) records its destination.
        if matches!(current, DragState::Dragging { .. }) {
            self.ctx
                .last_drop
                .set(Some(DropResult { draggable: self.id, over: *self.ctx.over.peek() }));
        }
        if let Ok(state) = transition(current, DragEvent::PointerUp, DEFAULT_DRAG_THRESHOLD) {
            self.ctx.state.set(state);
            // Stage-3 MVP: no exit animation — settle synchronously.
            if matches!(state, DragState::Dropping { .. }) {
                if let Ok(idle) = transition(state, DragEvent::Settle, DEFAULT_DRAG_THRESHOLD) {
                    self.ctx.state.set(idle);
                    self.ctx.over.set(None);
                }
            }
            #[cfg(target_arch = "wasm32")]
            self.release_pointer(&ev);
        }
        let _ = &ev;
    }

    /// `onpointercancel` handler. Wire this **and** `onpointerup` —
    /// pointer-capture loss (e.g. a system gesture preempting the drag)
    /// fires `pointercancel`, not `pointerup`.
    pub fn on_pointer_cancel(mut self, _ev: Event<PointerData>) {
        let current = *self.ctx.state.peek();
        if !self.is_my_state(current) {
            return;
        }
        if let Ok(state) = transition(current, DragEvent::Cancel, DEFAULT_DRAG_THRESHOLD) {
            self.ctx.state.set(state);
            self.ctx.over.set(None);
        }
    }

    /// Inline CSS for the element: `transform: translate(...)` while
    /// dragging, plus `touch-action: none` always (so the browser
    /// doesn't pre-empt pointer events for scroll/zoom gestures).
    pub fn style(self) -> String {
        let (dx, dy) = *self.transform.read();
        if *self.is_dragging.read() {
            format!(
                "transform: translate({dx}px, {dy}px); touch-action: none; \
                 user-select: none; z-index: 1000;"
            )
        } else {
            "touch-action: none; user-select: none;".to_owned()
        }
    }

    /// Inline CSS for the source element when a separate drag overlay
    /// will draw the preview. Omits the `translate` so the source stays
    /// pinned in place while the overlay (added in a follow-up slice)
    /// does the visual work.
    pub const fn style_pinned(self) -> &'static str {
        "touch-action: none; user-select: none;"
    }

    fn is_my_state(self, state: DragState) -> bool {
        matches!(state,
            DragState::Pressed { id, .. } | DragState::Dragging { id, .. } if id == self.id
        )
    }

    #[cfg(target_arch = "wasm32")]
    fn capture_pointer(self, ev: &Event<PointerData>) {
        if let Some(mounted) = self.element.peek().as_ref() {
            if let Some(el) = crate::dom::element_of(mounted) {
                if let Some(raw) = ev.downcast::<web_sys::PointerEvent>() {
                    crate::dom::capture_pointer(&el, raw);
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn release_pointer(self, ev: &Event<PointerData>) {
        if let Some(mounted) = self.element.peek().as_ref() {
            if let Some(el) = crate::dom::element_of(mounted) {
                if let Some(raw) = ev.downcast::<web_sys::PointerEvent>() {
                    crate::dom::release_pointer(&el, raw);
                }
            }
        }
    }
}

/// Register an element as a draggable identified by `id`.
///
/// Returns a [`UseDraggable`] handle whose methods you wire to the
/// element's `onmounted`, `onpointerdown`, `onpointermove`,
/// `onpointerup`, and `onpointercancel` event slots, and whose
/// [`style`](UseDraggable::style) helper goes on the `style:` attribute.
///
/// # Example
///
/// ```ignore
/// use dioxus::prelude::*;
/// use taino_dnd_core::DraggableId;
/// use taino_dnd_dioxus::{provide_dnd_context, use_draggable};
///
/// fn App() -> Element {
///     provide_dnd_context();
///     rsx! { Item {} }
/// }
///
/// fn Item() -> Element {
///     let d = use_draggable(DraggableId(1));
///     rsx! {
///         div {
///             onmounted: move |e| d.on_mounted(e),
///             onpointerdown: move |e| d.on_pointer_down(e),
///             onpointermove: move |e| d.on_pointer_move(e),
///             onpointerup: move |e| d.on_pointer_up(e),
///             onpointercancel: move |e| d.on_pointer_cancel(e),
///             style: "{d.style()}",
///             "drag me"
///         }
///     }
/// }
/// ```
pub fn use_draggable(id: DraggableId) -> UseDraggable {
    let ctx = use_dnd_context();
    let element = use_signal::<Option<Rc<MountedData>>>(|| None);

    let is_dragging = use_memo(move || match *ctx.state.read() {
        DragState::Dragging { id: dragged, .. } | DragState::Dropping { id: dragged } => {
            dragged == id
        }
        _ => false,
    });

    let transform = use_memo(move || match *ctx.state.read() {
        DragState::Dragging { id: dragged, start, current } if dragged == id => {
            (current.x - start.x, current.y - start.y)
        }
        _ => (0.0, 0.0),
    });

    UseDraggable { id, element, is_dragging, transform, ctx }
}
