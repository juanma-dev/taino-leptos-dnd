//! The [`use_draggable`] hook.

use leptos::{html::Div, prelude::*};
use taino_dnd_core::{
    transition, DragEvent, DragState, DraggableId, Point, DEFAULT_DRAG_THRESHOLD,
};

use crate::context::{use_dnd_context, DndContext, DropResult};

/// Handle returned by [`use_draggable`]. Wire its fields onto a `<div>` in your
/// view.
///
/// `UseDraggable` is [`Copy`], so it can be moved into multiple event handlers
/// in the same view without cloning.
#[derive(Clone, Copy)]
pub struct UseDraggable {
    /// Attach to the draggable element with `node_ref={handle.node_ref}`.
    pub node_ref: NodeRef<Div>,
    /// `true` while this specific draggable is being dragged.
    pub is_dragging: Signal<bool>,
    /// Translation (in CSS pixels) to apply while dragging.
    /// `(0.0, 0.0)` when not dragging.
    pub transform: Signal<(f64, f64)>,
    /// The identifier this hook was instantiated with.
    pub id: DraggableId,
    ctx: DndContext,
}

impl UseDraggable {
    /// `on:pointerdown` handler. Starts a `Pressed` state; movement past the
    /// drag threshold promotes it to `Dragging`.
    pub fn on_pointer_down(self, ev: &web_sys::PointerEvent) {
        if ev.button() != 0 {
            return;
        }
        let at = Point::new(ev.client_x().into(), ev.client_y().into());
        let current = self.ctx.state.get_untracked();
        if let Ok(state) =
            transition(current, DragEvent::PointerDown { id: self.id, at }, DEFAULT_DRAG_THRESHOLD)
        {
            self.ctx.state.set(state);
            self.ctx.last_drop.set(None);
            #[cfg(target_arch = "wasm32")]
            if let Some(el) = self.node_ref.get_untracked() {
                use wasm_bindgen::JsCast;
                if let Some(el) = (*el).dyn_ref::<web_sys::Element>() {
                    crate::dom::capture_pointer(el, ev);
                }
            }
        }
    }

    /// `on:pointermove` handler.
    pub fn on_pointer_move(self, ev: &web_sys::PointerEvent) {
        let at = Point::new(ev.client_x().into(), ev.client_y().into());
        let current = self.ctx.state.get_untracked();
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

    /// `on:pointerup` handler.
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
    pub fn on_pointer_up(self, ev: &web_sys::PointerEvent) {
        let current = self.ctx.state.get_untracked();
        if !self.is_my_state(current) {
            return;
        }
        // Genuine drop (not a click) records its destination.
        if matches!(current, DragState::Dragging { .. }) {
            self.ctx
                .last_drop
                .set(Some(DropResult { draggable: self.id, over: self.ctx.over.get_untracked() }));
        }
        if let Ok(state) = transition(current, DragEvent::PointerUp, DEFAULT_DRAG_THRESHOLD) {
            self.ctx.state.set(state);
            // Stage 1: no exit animation — settle synchronously.
            if matches!(state, DragState::Dropping { .. }) {
                if let Ok(idle) = transition(state, DragEvent::Settle, DEFAULT_DRAG_THRESHOLD) {
                    self.ctx.state.set(idle);
                    self.ctx.over.set(None);
                }
            }
            #[cfg(target_arch = "wasm32")]
            if let Some(el) = self.node_ref.get_untracked() {
                use wasm_bindgen::JsCast;
                if let Some(el) = (*el).dyn_ref::<web_sys::Element>() {
                    crate::dom::release_pointer(el, ev);
                }
            }
        }
    }

    /// `on:pointercancel` handler. Wire this **and** `on:pointerup` — pointer
    /// capture loss (e.g. a system gesture preempting the drag) fires
    /// `pointercancel`, not `pointerup`.
    pub fn on_pointer_cancel(self, _ev: &web_sys::PointerEvent) {
        let current = self.ctx.state.get_untracked();
        if !self.is_my_state(current) {
            return;
        }
        if let Ok(state) = transition(current, DragEvent::Cancel, DEFAULT_DRAG_THRESHOLD) {
            self.ctx.state.set(state);
            self.ctx.over.set(None);
        }
    }

    /// Inline CSS for the element: `transform: translate(...)` while dragging,
    /// plus `touch-action: none` always (so the browser doesn't pre-empt our
    /// pointer events for scroll/zoom gestures).
    pub fn style(self) -> String {
        let (dx, dy) = self.transform.get();
        if self.is_dragging.get() {
            format!(
                "transform: translate({dx}px, {dy}px); touch-action: none; \
                 user-select: none; z-index: 1000;"
            )
        } else {
            "touch-action: none; user-select: none;".to_owned()
        }
    }

    fn is_my_state(self, state: DragState) -> bool {
        matches!(state,
            DragState::Pressed { id, .. } | DragState::Dragging { id, .. } if id == self.id
        )
    }
}

/// Register an element as a draggable identified by `id`.
///
/// Returns a [`UseDraggable`] handle whose fields wire into the element's
/// `node_ref`, `on:pointerdown` / `on:pointermove` / `on:pointerup` /
/// `on:pointercancel`, and `style`.
///
/// The hook installs an `on_cleanup` callback that cancels any in-flight drag
/// if the element unmounts mid-drag.
///
/// # Example
///
/// ```no_run
/// use leptos::prelude::*;
/// use taino_dnd_core::DraggableId;
/// use taino_dnd_leptos::{provide_dnd_context, use_draggable};
///
/// #[component]
/// fn Item() -> impl IntoView {
///     let d = use_draggable(DraggableId(1));
///     view! {
///         <div
///             node_ref=d.node_ref
///             on:pointerdown=move |e| d.on_pointer_down(&e)
///             on:pointermove=move |e| d.on_pointer_move(&e)
///             on:pointerup=move |e| d.on_pointer_up(&e)
///             on:pointercancel=move |e| d.on_pointer_cancel(&e)
///             style=move || d.style()
///         >
///             "drag me"
///         </div>
///     }
/// }
///
/// #[component]
/// fn App() -> impl IntoView {
///     provide_dnd_context();
///     view! { <Item/> }
/// }
/// ```
pub fn use_draggable(id: DraggableId) -> UseDraggable {
    let ctx = use_dnd_context();
    let node_ref = NodeRef::<Div>::new();

    let is_dragging = Signal::derive(move || match ctx.state.get() {
        DragState::Dragging { id: dragged, .. } | DragState::Dropping { id: dragged } => {
            dragged == id
        }
        _ => false,
    });

    let transform = Signal::derive(move || match ctx.state.get() {
        DragState::Dragging { id: dragged, start, current } if dragged == id => {
            (current.x - start.x, current.y - start.y)
        }
        _ => (0.0, 0.0),
    });

    on_cleanup(move || {
        let s = ctx.state.get_untracked();
        let mine = matches!(
            s,
            DragState::Pressed { id: i, .. } | DragState::Dragging { id: i, .. } if i == id
        );
        if mine {
            ctx.state.set(DragState::Idle);
            ctx.over.set(None);
        }
    });

    UseDraggable { node_ref, is_dragging, transform, id, ctx }
}
