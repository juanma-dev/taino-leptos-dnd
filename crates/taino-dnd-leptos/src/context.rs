//! Shared drag-and-drop state for a region of a Leptos tree.
//!
//! Every region that wants to use the drag-and-drop hooks needs a [`DndContext`]
//! available via Leptos's context API. Install one with [`provide_dnd_context`].

use std::collections::HashMap;

use leptos::prelude::*;
use taino_dnd_core::{closest_center, DragState, DraggableId, DroppableId, Point, Rect};

/// Shared drag-and-drop state installed at the root of a region that uses
/// `taino-dnd-leptos`.
///
/// `DndContext` is [`Copy`]; clone freely and pass by value.
#[derive(Clone, Copy)]
pub struct DndContext {
    /// Reactive drag state. Read with `.state.get()`.
    pub state: RwSignal<DragState>,
    /// Registry of currently-mounted droppables, keyed by id with their last
    /// known bounding rect.
    pub(crate) droppables: RwSignal<HashMap<DroppableId, Rect>>,
    /// The droppable the pointer is currently hovering over, if any.
    pub over: RwSignal<Option<DroppableId>>,
    /// The draggable that emitted the most recent successful drop, with its
    /// destination. Cleared back to `None` when [`DndContext::clear_last_drop`]
    /// is called or when a new drag starts.
    pub last_drop: RwSignal<Option<DropResult>>,
}

/// The outcome of a completed drag interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropResult {
    /// The draggable that was dropped.
    pub draggable: DraggableId,
    /// The droppable the pointer was over at the moment of release, if any.
    /// `None` means the drag ended outside any registered droppable.
    pub over: Option<DroppableId>,
}

impl Default for DndContext {
    fn default() -> Self {
        Self {
            state: RwSignal::new(DragState::Idle),
            droppables: RwSignal::new(HashMap::new()),
            over: RwSignal::new(None),
            last_drop: RwSignal::new(None),
        }
    }
}

impl DndContext {
    /// Register or update the bounding rect for a droppable.
    ///
    /// Only the wasm32 build path calls this; native builds keep the registry
    /// empty (there's no DOM to measure against).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn upsert_droppable(self, id: DroppableId, rect: Rect) {
        self.droppables.update(|map| {
            map.insert(id, rect);
        });
    }

    /// Remove a droppable from the registry (call on unmount).
    pub(crate) fn remove_droppable(self, id: DroppableId) {
        self.droppables.update(|map| {
            map.remove(&id);
        });
        // If the removed droppable was the one currently hovered, clear `over`.
        self.over.update(|current| {
            if *current == Some(id) {
                *current = None;
            }
        });
    }

    /// Recompute which droppable the pointer is over and update `self.over`.
    pub(crate) fn update_over(self, pointer: Point) {
        let id = self
            .droppables
            .with(|map| closest_center(pointer, map.iter().map(|(id, rect)| (*id, *rect))));
        if self.over.get_untracked() != id {
            self.over.set(id);
        }
    }

    /// Clear [`DndContext::last_drop`] after the caller has consumed it.
    pub fn clear_last_drop(self) {
        self.last_drop.set(None);
    }
}

/// Install a [`DndContext`] for descendants. Call once near the root of your
/// drag-and-drop region (typically inside the top-level component for a page
/// or a board).
///
/// Returns the context so the caller can keep a handle if desired.
pub fn provide_dnd_context() -> DndContext {
    let ctx = DndContext::default();
    provide_context(ctx);
    ctx
}

/// Retrieve the nearest ancestor [`DndContext`].
///
/// # Panics
///
/// Panics if no `DndContext` has been provided in an ancestor. This is intentional:
/// calling a drag-and-drop hook outside a drag-and-drop scope is a programmer
/// error, not a recoverable runtime condition.
pub fn use_dnd_context() -> DndContext {
    use_context::<DndContext>()
        .expect("taino-dnd: provide_dnd_context() must be called in an ancestor")
}
