//! The [`use_draggable`] hook.

use std::{fmt::Debug, hash::Hash};

use leptos::{html::Div, prelude::*};
use taino_dnd_core::{
    transition, AnnounceEvent, Direction, DragEvent, DragState, DraggableId, Point,
    DEFAULT_DRAG_THRESHOLD,
};

use crate::context::{use_dnd_context, DndContext, DropResult};

/// Handle returned by [`use_draggable`]. Wire its fields onto a `<div>` in your
/// view.
///
/// `UseDraggable` is [`Copy`], so it can be moved into multiple event handlers
/// in the same view without cloning.
#[derive(Clone, Copy)]
pub struct UseDraggable<T>
where
    T: Debug + Clone + Copy + PartialEq + Eq + Hash + PartialOrd + Ord + Send + Sync + 'static,
{
    /// Attach to the draggable element with `node_ref={handle.node_ref}`.
    pub node_ref: NodeRef<Div>,
    /// `true` while this specific draggable is being dragged.
    pub is_dragging: Signal<bool>,
    /// Translation (in CSS pixels) to apply while dragging.
    /// `(0.0, 0.0)` when not dragging.
    pub transform: Signal<(f64, f64)>,
    /// Reactive disabled flag. While `true`, this draggable can't be picked
    /// up (pointer or keyboard). Read it to set `aria-disabled` / styling.
    /// Always `false` for [`use_draggable`]; set via [`use_draggable_with`].
    pub disabled: Signal<bool>,
    /// The identifier this hook was instantiated with.
    pub id: DraggableId<T>,
    ctx: DndContext<T>,
}

impl<T> UseDraggable<T>
where
    T: Debug + Clone + Copy + PartialEq + Eq + Hash + PartialOrd + Ord + Send + Sync + 'static,
{
    /// `on:pointerdown` handler. Starts a `Pressed` state; movement past the
    /// drag threshold promotes it to `Dragging`.
    pub fn on_pointer_down(self, ev: &web_sys::PointerEvent) {
        if ev.button() != 0 || self.disabled.get_untracked() {
            return;
        }
        let at = Point::new(ev.client_x().into(), ev.client_y().into());
        let current = self.ctx.state.get_untracked();
        if let Ok(state) =
            transition(current, DragEvent::PointerDown { id: self.id, at }, DEFAULT_DRAG_THRESHOLD)
        {
            self.ctx.state.set(state);
            self.ctx.last_drop.set(None);
            // Record the element rect so RestrictToParent has something to
            // clamp against.
            self.ctx.dragged_element_rect.set(self.element_rect());
            // Snapshot the multi-selection at drag start. Single-drag callers
            // see this set to `[self.id]`; multi-drag callers (selection
            // contained `self.id` and had >1 element) see the whole group.
            self.ctx.begin_drag_group(self.id);
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
        // Raw position drives the state machine (so the click-vs-drag
        // threshold isn't broken by an axis lock). The modifier chain runs
        // afterwards to produce the effective position used for collision.
        let at = Point::new(ev.client_x().into(), ev.client_y().into());
        let current = self.ctx.state.get_untracked();
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

    /// `on:pointerup` handler.
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
    pub fn on_pointer_up(self, ev: &web_sys::PointerEvent) {
        let current = self.ctx.state.get_untracked();
        if !self.is_my_state(current) {
            return;
        }
        // Genuine drop (not a click) records its destination and the slot the
        // overlay should glide to during the drop-settle animation.
        let to = if matches!(current, DragState::Dragging { .. }) {
            let additional = self.additional_from_group();
            self.ctx.last_drop.set(Some(DropResult {
                draggable: self.id,
                over: self.ctx.over.get_untracked(),
                additional,
            }));
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
            if let Some(el) = self.node_ref.get_untracked() {
                use wasm_bindgen::JsCast;
                if let Some(el) = (*el).dyn_ref::<web_sys::Element>() {
                    crate::dom::release_pointer(el, ev);
                }
            }
        }
    }

    /// Viewport-space top-left the drop overlay should glide to: the slot the
    /// item lands in (the current `over` droppable), or the source's origin
    /// when released outside any droppable (a snap-back).
    fn drop_landing(self) -> Point {
        if let Some(over) = self.ctx.over.get_untracked() {
            if let Some(rect) = self.ctx.droppables.with_untracked(|m| m.get(&over).copied()) {
                return Point::new(rect.x, rect.y);
            }
        }
        self.ctx
            .dragged_element_rect
            .get_untracked()
            .map_or(Point::new(0.0, 0.0), |r| Point::new(r.x, r.y))
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

    /// `on:keydown` handler for the keyboard sensor.
    ///
    /// Key model:
    ///
    /// | Key             | When focused (Idle) | When dragging this item |
    /// | --------------- | ------------------- | ----------------------- |
    /// | Space / Enter   | Pick up             | Drop                    |
    /// | Arrow keys      | (pass through)      | Move over neighbor      |
    /// | Escape          | (pass through)      | Cancel, restore         |
    ///
    /// The handler calls [`web_sys::Event::prevent_default`] on consumed keys
    /// so they don't double-fire as scroll/space-page actions.
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
    pub fn on_key_down(self, ev: &web_sys::KeyboardEvent) {
        let key = ev.key();
        let current = self.ctx.state.get_untracked();
        let is_dragging_me = matches!(current, DragState::Dragging { id, .. } if id == self.id);

        // Pickup path: Space/Enter while not dragging.
        if !is_dragging_me && matches!(current, DragState::Idle) && is_activation(&key) {
            if self.disabled.get_untracked() {
                return;
            }
            self.keyboard_pickup(ev);
            return;
        }

        if !is_dragging_me {
            return;
        }

        match key.as_str() {
            // Drop
            " " | "Enter" => {
                ev.prevent_default();
                if let Ok(state) = transition(current, DragEvent::PointerUp, DEFAULT_DRAG_THRESHOLD)
                {
                    let target = self.ctx.over.get_untracked();
                    let additional = self.additional_from_group();
                    self.ctx.last_drop.set(Some(DropResult {
                        draggable: self.id,
                        over: target,
                        additional,
                    }));
                    let to = self.drop_landing();
                    self.ctx.state.set(state);
                    if matches!(state, DragState::Dropping { .. }) {
                        self.ctx.settle_drop(Some(to));
                    }
                    self.ctx.announce_event(AnnounceEvent::Dropped {
                        draggable: self.id,
                        over: target,
                    });
                }
            }
            // Cancel
            "Escape" => {
                ev.prevent_default();
                if let Ok(state) = transition(current, DragEvent::Cancel, DEFAULT_DRAG_THRESHOLD) {
                    self.ctx.state.set(state);
                    self.ctx.over.set(None);
                    self.ctx.announce_event(AnnounceEvent::Cancelled { draggable: self.id });
                }
            }
            // Move
            other => {
                let dir = direction_for_key(other);
                if let Some(dir) = dir {
                    ev.prevent_default();
                    if let Some(new_over) = self.ctx.keyboard_step(dir) {
                        self.ctx.announce_event(AnnounceEvent::MovedOver {
                            draggable: self.id,
                            over: Some(new_over),
                        });
                    }
                }
            }
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
    fn keyboard_pickup(self, ev: &web_sys::KeyboardEvent) {
        ev.prevent_default();
        // Use the element's center as the synthetic start position. If the
        // element isn't mounted yet we fall back to (0, 0) — the actual values
        // only matter for the `transform` signal, which the user can disable
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
            // Snapshot the multi-selection at drag start (same logic as the
            // pointer-down path); single-item drags get `[self.id]`.
            self.ctx.begin_drag_group(self.id);
            // Default `over` to the draggable's own droppable id if registered,
            // so the user has a target to navigate from.
            self.ctx.over.set(Some(taino_dnd_core::DroppableId(self.id.0)));
            self.ctx.announce_event(AnnounceEvent::PickedUp { draggable: self.id });
        }
    }

    /// Build the [`DropResult::additional`] vector: everything in the active
    /// drag group except the primary `self.id`. Single-item drags return
    /// `Vec::new()`. Clears `dragged_group` as a side effect (call exactly
    /// once per drop, from the `pointerup` / keyboard-drop path).
    fn additional_from_group(self) -> Vec<DraggableId<T>> {
        self.ctx.take_drag_group().into_iter().filter(|id| *id != self.id).collect()
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
    fn element_rect(self) -> Option<taino_dnd_core::Rect> {
        use wasm_bindgen::JsCast;
        let el = self.node_ref.get_untracked()?;
        let el = (*el).dyn_ref::<web_sys::Element>()?;
        Some(crate::dom::bounding_rect(el))
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::unused_self)] // signature matches the wasm32 sibling
    const fn element_rect(self) -> Option<taino_dnd_core::Rect> {
        None
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

    /// Inline CSS for the source element **when a [`DragOverlay`] is in use**.
    ///
    /// Unlike [`Self::style`], the source element is **not** translated — the
    /// overlay takes over the visual preview, so leaving the source pinned in
    /// place is the correct behavior. `touch-action: none` and `user-select:
    /// none` are still applied so pointer events route cleanly.
    ///
    /// [`DragOverlay`]: crate::DragOverlay
    pub const fn style_pinned(self) -> &'static str {
        "touch-action: none; user-select: none;"
    }

    fn is_my_state(self, state: DragState<T>) -> bool {
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
/// use taino_dnd_leptos::{provide_dnd_context, use_draggable};
///
/// #[component]
/// fn Item() -> impl IntoView {
///     let d = use_draggable(1);
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
///     provide_dnd_context::<u64>();
///     view! { <Item/> }
/// }
/// ```
pub fn use_draggable<T>(id: impl Into<DraggableId<T>>) -> UseDraggable<T>
where
    T: Debug + Clone + Copy + PartialEq + Eq + Hash + PartialOrd + Ord + Send + Sync + 'static,
{
    use_draggable_with(id, Signal::derive(|| false))
}

/// Like [`use_draggable`], but with a reactive `disabled` flag.
///
/// While `disabled` reads `true`, the draggable can't be picked up by pointer
/// or keyboard — `on_pointer_down` and the Space/Enter pickup both no-op. The
/// flag is reactive: flip the signal and the next interaction respects it.
/// Read it back via [`UseDraggable::disabled`] to drive `aria-disabled` or a
/// `not-allowed` cursor.
///
/// # Example
///
/// ```no_run
/// use leptos::prelude::*;
/// use taino_dnd_leptos::{provide_dnd_context, use_draggable_with};
///
/// # #[component] fn Item() -> impl IntoView {
/// let locked = RwSignal::new(true);
/// let d = use_draggable_with(1, locked.into());
/// view! {
///     <div node_ref=d.node_ref aria-disabled=move || d.disabled.get().to_string()>
///         "locked while `locked` is true"
///     </div>
/// }
/// # }
/// ```
pub fn use_draggable_with<T>(
    id: impl Into<DraggableId<T>>,
    disabled: Signal<bool>,
) -> UseDraggable<T>
where
    T: Debug + Clone + Copy + PartialEq + Eq + Hash + PartialOrd + Ord + Send + Sync + 'static,
{
    let ctx = use_dnd_context();
    let node_ref = NodeRef::<Div>::new();

    let id = id.into();

    let is_dragging = Signal::derive(move || match ctx.state.get() {
        DragState::Dragging { id: dragged, .. } | DragState::Dropping { id: dragged } => {
            dragged == id
        }
        _ => false,
    });

    let transform = Signal::derive(move || match ctx.state.get() {
        DragState::Dragging { id: dragged, start, current } if dragged == id => {
            let raw = taino_dnd_core::Vector::new(current.x - start.x, current.y - start.y);
            let modified = ctx.modify(raw);
            (modified.x, modified.y)
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

    UseDraggable { node_ref, is_dragging, transform, disabled, id, ctx }
}

fn is_activation(key: &str) -> bool {
    matches!(key, " " | "Enter")
}

fn direction_for_key(key: &str) -> Option<Direction> {
    match key {
        "ArrowUp" => Some(Direction::Up),
        "ArrowDown" => Some(Direction::Down),
        "ArrowLeft" => Some(Direction::Left),
        "ArrowRight" => Some(Direction::Right),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{direction_for_key, is_activation};
    use taino_dnd_core::Direction;

    #[test]
    fn activation_keys_recognised() {
        assert!(is_activation(" "));
        assert!(is_activation("Enter"));
        assert!(!is_activation("Escape"));
        assert!(!is_activation("ArrowDown"));
    }

    #[test]
    fn arrow_keys_map_to_directions() {
        assert_eq!(direction_for_key("ArrowUp"), Some(Direction::Up));
        assert_eq!(direction_for_key("ArrowDown"), Some(Direction::Down));
        assert_eq!(direction_for_key("ArrowLeft"), Some(Direction::Left));
        assert_eq!(direction_for_key("ArrowRight"), Some(Direction::Right));
        assert_eq!(direction_for_key("a"), None);
    }
}
