//! Shared drag-and-drop state for a region of a Dioxus tree.
//!
//! Every region that uses `taino-dnd-dioxus` needs a [`DndContext`] in
//! its provider chain. Install one with [`provide_dnd_context`]; consume
//! it from any descendant with [`use_dnd_context`].

use std::collections::HashMap;

use dioxus::prelude::*;
use taino_dnd_core::{
    closest_center, spatial_neighbor, Direction, DragState, DraggableId, DroppableId, Point, Rect,
};

/// Shared drag-and-drop state installed at the root of a region that
/// uses `taino-dnd-dioxus`.
///
/// `DndContext` is [`Copy`] (every field is a Dioxus `Signal` handle),
/// so it can be moved into event closures without cloning.
#[derive(Clone, Copy)]
pub struct DndContext {
    /// Reactive drag state. Read with `ctx.state()` (Dioxus call-syntax).
    pub state: Signal<DragState>,
    /// Registry of currently-mounted droppables, keyed by id with their
    /// last known bounding rect (viewport coordinates).
    pub(crate) droppables: Signal<HashMap<DroppableId, Rect>>,
    /// The droppable the pointer is currently hovering over, if any.
    pub over: Signal<Option<DroppableId>>,
    /// The draggable that emitted the most recent successful drop, with
    /// its destination. Cleared back to `None` when
    /// [`DndContext::clear_last_drop`] is called or when a new drag starts.
    pub last_drop: Signal<Option<DropResult>>,
    /// Latest screen-reader announcement. Mirrored into a visually-
    /// hidden `role="alert" aria-live="assertive"` region by
    /// [`DndAnnouncer`](crate::DndAnnouncer).
    pub announcement: Signal<String>,
    /// Bounding rect of the dragged element at drag-start. Populated by
    /// `use_draggable` on `keyboard-pickup`; used by the keyboard sensor
    /// to compute a synthetic `at` position. Cleared when state returns
    /// to Idle (see `provide_dnd_context`).
    pub(crate) dragged_element_rect: Signal<Option<Rect>>,
}

/// The outcome of a completed drag interaction.
///
/// Mirrors `taino_dnd_leptos::DropResult` so user code that reads either
/// binding can share the consume-the-drop helper unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropResult {
    /// The draggable that was dropped.
    pub draggable: DraggableId,
    /// The droppable the pointer was over at the moment of release, if
    /// any. `None` means the drag ended outside any registered droppable.
    pub over: Option<DroppableId>,
}

impl DndContext {
    /// Clear [`Self::last_drop`] after the caller has consumed it.
    pub fn clear_last_drop(mut self) {
        self.last_drop.set(None);
    }

    /// Register or update the bounding rect for a droppable.
    ///
    /// Only the wasm32 build path calls this; native builds keep the
    /// registry empty (there's no DOM to measure against).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn upsert_droppable(mut self, id: DroppableId, rect: Rect) {
        self.droppables.with_mut(|map| {
            map.insert(id, rect);
        });
    }

    /// Remove a droppable from the registry (call on unmount).
    pub(crate) fn remove_droppable(mut self, id: DroppableId) {
        self.droppables.with_mut(|map| {
            map.remove(&id);
        });
        if *self.over.peek() == Some(id) {
            self.over.set(None);
        }
    }

    /// Recompute which droppable the pointer is over and update `self.over`.
    pub(crate) fn update_over(mut self, pointer: Point) {
        let id = self
            .droppables
            .with(|map| closest_center(pointer, map.iter().map(|(id, rect)| (*id, *rect))));
        if *self.over.peek() != id {
            self.over.set(id);
        }
    }

    /// Move keyboard-driven focus to a neighbor droppable in `direction`.
    ///
    /// Returns the new `over` id (or the old one if no neighbor exists
    /// — i.e. we're at the edge of the layout in that direction).
    pub(crate) fn keyboard_step(mut self, direction: Direction) -> Option<DroppableId> {
        let from = (*self.over.peek())?;
        let next = self
            .droppables
            .with(|map| spatial_neighbor(from, direction, map.iter().map(|(id, r)| (*id, *r))));
        if let Some(id) = next {
            self.over.set(Some(id));
        }
        next.or(Some(from))
    }

    /// Push a screen-reader announcement onto the live region. Equivalent
    /// to writing into [`Self::announcement`] but kept as a method for
    /// symmetry with the Leptos binding.
    pub fn announce(mut self, message: impl Into<String>) {
        self.announcement.set(message.into());
    }
}

/// Install a [`DndContext`] for descendants. Call once near the root of
/// your drag-and-drop region (typically inside the top-level component
/// for a page or a board).
///
/// Also installs a lightweight effect that clears the cached
/// `dragged_element_rect` when state returns to Idle, so a stale rect
/// can't influence the next drag.
pub fn provide_dnd_context() -> DndContext {
    let state = use_signal(|| DragState::Idle);
    let droppables = use_signal::<HashMap<DroppableId, Rect>>(HashMap::new);
    let over = use_signal::<Option<DroppableId>>(|| None);
    let last_drop = use_signal::<Option<DropResult>>(|| None);
    let announcement = use_signal(String::new);
    let dragged_element_rect = use_signal::<Option<Rect>>(|| None);
    let ctx = DndContext { state, droppables, over, last_drop, announcement, dragged_element_rect };
    use_context_provider(|| ctx);

    // When state returns to Idle, clear the cached element rect.
    use_effect(move || {
        if matches!(*ctx.state.read(), DragState::Idle) {
            let mut rect = ctx.dragged_element_rect;
            if rect.peek().is_some() {
                rect.set(None);
            }
        }
    });

    ctx
}

/// Retrieve the nearest ancestor [`DndContext`].
///
/// # Panics
///
/// Panics if no `DndContext` has been provided in an ancestor. This is
/// intentional: calling a drag-and-drop hook outside a drag-and-drop
/// scope is a programmer error, not a recoverable runtime condition.
pub fn use_dnd_context() -> DndContext {
    use_context::<DndContext>()
}
