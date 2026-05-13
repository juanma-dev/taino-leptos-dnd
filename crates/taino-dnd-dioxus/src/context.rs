//! Shared drag-and-drop state for a region of a Dioxus tree.
//!
//! Every region that uses `taino-dnd-dioxus` needs a [`DndContext`] in
//! its provider chain. Install one with [`provide_dnd_context`]; consume
//! it from any descendant with [`use_dnd_context`].

use std::collections::HashMap;

use dioxus::prelude::*;
use taino_dnd_core::{closest_center, DragState, DraggableId, DroppableId, Point, Rect};

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
}

/// Install a [`DndContext`] for descendants. Call once near the root of
/// your drag-and-drop region (typically inside the top-level component
/// for a page or a board).
pub fn provide_dnd_context() -> DndContext {
    let state = use_signal(|| DragState::Idle);
    let droppables = use_signal::<HashMap<DroppableId, Rect>>(HashMap::new);
    let over = use_signal::<Option<DroppableId>>(|| None);
    let last_drop = use_signal::<Option<DropResult>>(|| None);
    let ctx = DndContext { state, droppables, over, last_drop };
    use_context_provider(|| ctx);
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
