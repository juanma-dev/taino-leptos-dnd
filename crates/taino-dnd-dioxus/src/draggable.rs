//! The [`use_draggable`] hook for Dioxus.

// Dioxus event handlers idiomatically take `Event<T>` by value (the
// type is a thin Rc handle, so passing by value is cheap and matches
// `rsx!` ergonomics). Take the lint at the module level once.
#![allow(clippy::needless_pass_by_value)]

use std::rc::Rc;

use dioxus::html::input_data::MouseButton;
use dioxus::prelude::*;
use taino_dnd_core::{
    transition, Direction, DragEvent, DragState, DraggableId, DroppableId, Point, Rect,
    DEFAULT_DRAG_THRESHOLD,
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
    /// Reactive disabled flag. While `true`, this draggable can't be picked
    /// up (pointer or keyboard). Read it to set `aria-disabled` / styling.
    /// Always `false` for [`use_draggable`]; set via [`use_draggable_with`].
    pub disabled: Signal<bool>,
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
        if ev.trigger_button() != Some(MouseButton::Primary) || *self.disabled.peek() {
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
            // Record the element rect so RestrictToParent has something
            // to clamp against and the keyboard sensor can compute a
            // synthetic `at` position if it takes over mid-drag.
            self.ctx.dragged_element_rect.set(self.element_rect());
            #[cfg(target_arch = "wasm32")]
            self.capture_pointer(&ev);
        }
    }

    /// `onpointermove` handler.
    pub fn on_pointer_move(mut self, ev: Event<PointerData>) {
        // Raw position drives the state machine (so the click-vs-drag
        // threshold isn't broken by an axis lock). The modifier chain
        // runs afterwards to produce the effective position used for
        // collision detection.
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
            if let DragState::Dragging { start, .. } = state {
                let effective = self.ctx.effective_point(start, at);
                self.ctx.update_over(effective);
            }
        }
    }

    /// `onpointerup` handler.
    pub fn on_pointer_up(mut self, ev: Event<PointerData>) {
        let current = *self.ctx.state.peek();
        if !self.is_my_state(current) {
            return;
        }
        // Genuine drop (not a click) records its destination and the slot the
        // overlay should glide to during the drop-settle animation.
        let to = if matches!(current, DragState::Dragging { .. }) {
            self.ctx
                .last_drop
                .set(Some(DropResult { draggable: self.id, over: *self.ctx.over.peek() }));
            Some(self.drop_landing())
        } else {
            None
        };
        if let Ok(state) = transition(current, DragEvent::PointerUp, DEFAULT_DRAG_THRESHOLD) {
            self.ctx.state.set(state);
            // Dragging → Dropping: animate the overlay to its slot, then settle.
            // (Pressed → Idle is a click, not a drop — nothing to animate.)
            if matches!(state, DragState::Dropping { .. }) {
                self.ctx.settle_drop(to);
            }
            #[cfg(target_arch = "wasm32")]
            self.release_pointer(&ev);
        }
        let _ = &ev;
    }

    /// Viewport-space top-left the drop overlay should glide to: the slot the
    /// item lands in (the current `over` droppable), or the source's origin
    /// when released outside any droppable (a snap-back).
    fn drop_landing(self) -> Point {
        if let Some(over) = *self.ctx.over.peek() {
            if let Some(rect) = self.ctx.droppables.peek().get(&over).copied() {
                return Point::new(rect.x, rect.y);
            }
        }
        (*self.ctx.dragged_element_rect.peek())
            .map_or(Point::new(0.0, 0.0), |r| Point::new(r.x, r.y))
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

    /// `onkeydown` handler for the keyboard sensor.
    ///
    /// Key model:
    ///
    /// | Key             | When focused (Idle) | When dragging this item |
    /// | --------------- | ------------------- | ----------------------- |
    /// | Space / Enter   | Pick up             | Drop                    |
    /// | Arrow keys      | (pass through)      | Move over neighbor      |
    /// | Escape          | (pass through)      | Cancel, restore         |
    ///
    /// The handler calls `event.prevent_default()` on consumed keys so
    /// they don't double-fire as scroll/space-page actions.
    pub fn on_key_down(mut self, ev: Event<KeyboardData>) {
        let key = ev.key();
        let current = *self.ctx.state.peek();
        let is_dragging_me = matches!(current, DragState::Dragging { id, .. } if id == self.id);

        // Pickup path: Space/Enter while not dragging.
        if !is_dragging_me && matches!(current, DragState::Idle) && is_activation(&key) {
            if *self.disabled.peek() {
                return;
            }
            self.keyboard_pickup(&ev);
            return;
        }

        if !is_dragging_me {
            return;
        }

        // Drop.
        if is_activation(&key) {
            ev.prevent_default();
            if let Ok(state) = transition(current, DragEvent::PointerUp, DEFAULT_DRAG_THRESHOLD) {
                let target = *self.ctx.over.peek();
                self.ctx.last_drop.set(Some(DropResult { draggable: self.id, over: target }));
                let to = self.drop_landing();
                self.ctx.state.set(state);
                if matches!(state, DragState::Dropping { .. }) {
                    self.ctx.settle_drop(Some(to));
                }
                let msg = target.map_or_else(
                    || format!("Dropped item {} outside any target.", self.id.0),
                    |t| format!("Dropped item {} on target {}.", self.id.0, t.0),
                );
                self.ctx.announce(msg);
            }
            return;
        }

        // Cancel.
        if matches!(key, Key::Escape) {
            ev.prevent_default();
            if let Ok(state) = transition(current, DragEvent::Cancel, DEFAULT_DRAG_THRESHOLD) {
                self.ctx.state.set(state);
                self.ctx.over.set(None);
                self.ctx.announce(format!("Cancelled drag of item {}.", self.id.0));
            }
            return;
        }

        // Arrow keys → step over to neighbor droppable.
        if let Some(dir) = direction_for_key(&key) {
            ev.prevent_default();
            if let Some(new_over) = self.ctx.keyboard_step(dir) {
                self.ctx.announce(format!("Item {} moved over target {}.", self.id.0, new_over.0));
            }
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

    fn keyboard_pickup(mut self, ev: &Event<KeyboardData>) {
        ev.prevent_default();
        // Use the element's center as the synthetic start position. On
        // non-wasm targets, fall back to (0, 0) — the actual values only
        // matter for the `transform` signal, which the user can disable
        // during keyboard drags via CSS.
        let at = self.element_center().unwrap_or_default();
        if let Ok(state) = transition(
            DragState::Idle,
            DragEvent::KeyboardPickUp { id: self.id, at },
            DEFAULT_DRAG_THRESHOLD,
        ) {
            self.ctx.last_drop.set(None);
            self.ctx.state.set(state);
            self.ctx.dragged_element_rect.set(self.element_rect());
            // Default `over` to the draggable's own droppable id if
            // registered, so the user has a target to navigate from.
            self.ctx.over.set(Some(DroppableId(self.id.0)));
            self.ctx.announce(format!(
                "Picked up item {}. Use arrow keys to move, space or enter to drop, escape to cancel.",
                self.id.0
            ));
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn element_center(self) -> Option<Point> {
        let r = self.element_rect()?;
        Some(Point::new(r.x + r.width / 2.0, r.y + r.height / 2.0))
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::unused_self)] // signature matches the wasm32 sibling
    const fn element_center(self) -> Option<Point> {
        None
    }

    #[cfg(target_arch = "wasm32")]
    fn element_rect(self) -> Option<Rect> {
        let mounted = self.element.peek().clone()?;
        crate::dom::bounding_rect_of(&mounted)
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::unused_self)] // signature matches the wasm32 sibling
    const fn element_rect(self) -> Option<Rect> {
        None
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
    let disabled = use_signal(|| false);
    use_draggable_with(id, disabled)
}

/// Like [`use_draggable`], but with a reactive `disabled` flag.
///
/// While `disabled` reads `true`, the draggable can't be picked up by pointer
/// or keyboard — `on_pointer_down` and the Space/Enter pickup both no-op. Flip
/// the signal and the next interaction respects it. Read it back via
/// [`UseDraggable::disabled`] to drive `aria-disabled` or a `not-allowed`
/// cursor.
///
/// # Example
///
/// ```ignore
/// use dioxus::prelude::*;
/// use taino_dnd_core::DraggableId;
/// use taino_dnd_dioxus::use_draggable_with;
///
/// fn Item() -> Element {
///     let locked = use_signal(|| true);
///     let d = use_draggable_with(DraggableId(1), locked);
///     rsx! {
///         div {
///             onmounted: move |e| d.on_mounted(e),
///             onpointerdown: move |e| d.on_pointer_down(e),
///             "aria-disabled": "{d.disabled}",
///             "locked while `locked` is true"
///         }
///     }
/// }
/// ```
pub fn use_draggable_with(id: DraggableId, disabled: Signal<bool>) -> UseDraggable {
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
            let raw = taino_dnd_core::Vector::new(current.x - start.x, current.y - start.y);
            let modified = ctx.modify(raw);
            (modified.x, modified.y)
        }
        _ => (0.0, 0.0),
    });

    UseDraggable { id, element, is_dragging, transform, disabled, ctx }
}

fn is_activation(key: &Key) -> bool {
    match key {
        Key::Enter => true,
        Key::Character(s) => s == " ",
        _ => false,
    }
}

const fn direction_for_key(key: &Key) -> Option<Direction> {
    match key {
        Key::ArrowUp => Some(Direction::Up),
        Key::ArrowDown => Some(Direction::Down),
        Key::ArrowLeft => Some(Direction::Left),
        Key::ArrowRight => Some(Direction::Right),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{direction_for_key, is_activation};
    use dioxus::prelude::Key;
    use taino_dnd_core::Direction;

    #[test]
    fn activation_keys_recognised() {
        assert!(is_activation(&Key::Enter));
        assert!(is_activation(&Key::Character(" ".to_owned())));
        assert!(!is_activation(&Key::Escape));
        assert!(!is_activation(&Key::ArrowDown));
    }

    #[test]
    fn arrow_keys_map_to_directions() {
        assert_eq!(direction_for_key(&Key::ArrowUp), Some(Direction::Up));
        assert_eq!(direction_for_key(&Key::ArrowDown), Some(Direction::Down));
        assert_eq!(direction_for_key(&Key::ArrowLeft), Some(Direction::Left));
        assert_eq!(direction_for_key(&Key::ArrowRight), Some(Direction::Right));
        assert_eq!(direction_for_key(&Key::Character("a".to_owned())), None);
    }
}
