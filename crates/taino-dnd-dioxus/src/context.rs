//! Shared drag-and-drop state for a region of a Dioxus tree.
//!
//! Every region that uses `taino-dnd-dioxus` needs a [`DndContext`] in
//! its provider chain. Install one with [`provide_dnd_context`]; consume
//! it from any descendant with [`use_dnd_context`].

use std::collections::HashMap;
use std::rc::Rc;

use dioxus::prelude::*;
use taino_dnd_core::{
    apply_chain, pointer_within, spatial_neighbor, AutoScrollConfig, Direction, DragState,
    DraggableId, DroppableId, Modifier, ModifierContext, Point, Rect, Vector,
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
    /// `use_draggable` on `pointerdown` and `keyboard-pickup`. Used by
    /// the keyboard sensor to compute a synthetic `at` position and by
    /// [`Modifier::RestrictToParent`] for clamping. Cleared when state
    /// returns to Idle (see `provide_dnd_context`).
    pub(crate) dragged_element_rect: Signal<Option<Rect>>,
    /// While in [`DragState::Dropping`], the viewport-space point the drag
    /// overlay should animate **to** — the top-left of the slot the item
    /// lands in, or the source's origin when dropped outside any droppable.
    /// `None` outside a drop animation. Drives [`DragOverlay`]'s exit glide
    /// (the post-release "fly to slot" settle).
    ///
    /// [`DragOverlay`]: crate::DragOverlay
    pub(crate) drop_target: Signal<Option<Point>>,
    /// Ordered list of [`Modifier`]s applied to the drag displacement
    /// before it's used for the visual transform and for collision
    /// detection. Empty by default. Mutate with
    /// [`DndContext::push_modifier`] / [`DndContext::set_modifiers`].
    pub modifiers: Signal<Vec<Modifier>>,
    /// Optional bounding rect of the container that
    /// [`Modifier::RestrictToParent`] should keep drags inside. Set via
    /// [`DndContext::set_restrict_container`] or via the
    /// [`use_drag_container`](crate::use_drag_container) helper hook.
    pub restrict_container: Signal<Option<Rect>>,
    /// Auto-scroll configuration. Drives the viewport-edge auto-scroll
    /// behavior during a drag. Set `enabled` to `false` to opt out.
    pub auto_scroll: Signal<AutoScrollConfig>,
    /// Non-reactive registry of droppable element handles. Populated by
    /// `use_droppable` on mount and cleared on drop. The centralized
    /// re-measure effect in `provide_dnd_context` iterates this map
    /// and writes all rects into `droppables` in a single batch,
    /// avoiding the O(N²) cascading notification problem that occurs
    /// when N individual effects each write one-by-one.
    pub(crate) elements: Signal<HashMap<DroppableId, Rc<MountedData>>>,
    /// Guard flag: `true` while the RAF auto-scroll loop is executing
    /// its `scrollBy` + remeasure + `update_over` sequence. The window
    /// `scroll` listener checks this and skips its own (duplicate) call
    /// when the scroll was caused by the RAF loop's `scrollBy`.
    ///
    /// Only read inside `#[cfg(target_arch = "wasm32")]` blocks in
    /// `autoscroll.rs`, so native `cargo check` sees it as unread.
    #[allow(dead_code)]
    pub(crate) raf_scrolling: Signal<bool>,
    /// Deduped memo of the currently dragged item's droppable ID.
    /// Changes only on drag start/end, NOT on every `pointermove`.
    /// Displacement memos subscribe to this instead of raw `state`,
    /// avoiding 18× re-evaluations per pointermove event.
    pub dragged_droppable: Memo<Option<DroppableId>>,
}

/// Duration of the drop-settle overlay animation, in milliseconds. The
/// overlay's CSS transition and the `Settle` timer share this value so the
/// state returns to `Idle` exactly when the glide finishes.
// `unreachable_pub` would prefer `pub(crate)` (the module is private), which
// then trips `redundant_pub_crate`; the const is genuinely crate-internal.
#[allow(clippy::redundant_pub_crate)]
pub(crate) const DROP_ANIMATION_MS: u64 = 200;

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

    /// Read **and** clear [`Self::last_drop`] in one call, returning the
    /// previous value (if any). Subscribes the calling effect / memo to
    /// `last_drop` changes.
    ///
    /// This is the preferred pattern for the "process the drop result,
    /// then reset" idiom. Doing it manually as
    /// `if let Some(d) = *ctx.last_drop.read() { ... ctx.clear_last_drop(); }`
    /// is a Dioxus borrow trap: `read()` returns a guard that lives
    /// through the entire `if let` body, so the subsequent `set(None)`
    /// from `clear_last_drop` would panic with a borrow conflict.
    /// `take_last_drop` does the read-then-write across two statements
    /// so the read borrow has already dropped before the write fires.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use dioxus::prelude::*;
    /// use taino_dnd_dioxus::use_dnd_context;
    ///
    /// let ctx = use_dnd_context();
    /// use_effect(move || {
    ///     if let Some(drop) = ctx.take_last_drop() {
    ///         // ... update your items vec from `drop.draggable` / `drop.over` ...
    ///     }
    /// });
    /// ```
    pub fn take_last_drop(mut self) -> Option<DropResult> {
        let value = *self.last_drop.read();
        if value.is_some() {
            self.last_drop.set(None);
        }
        value
    }

    /// Register or update the bounding rect for a droppable.
    ///
    /// Only the wasm32 build path calls this; native builds keep the
    /// registry empty (there's no DOM to measure against).
    ///
    /// **Short-circuits** when the stored rect already matches `rect`.
    /// This is critical for performance: the auto-scroll loop bumps
    /// `measurement_tick` once, causing N `use_droppable` effects to
    /// re-measure in the same microtask.  If every one of those writes
    /// into the `droppables` signal (even with the identical value),
    /// Dioxus notifies all subscribers N times and each subscriber's
    /// memo re-evaluates — O(N²) work per scroll tick.  Skipping
    /// the no-op write keeps the cost at O(N).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn upsert_droppable(mut self, id: DroppableId, rect: Rect) {
        // Peek first (non-reactive) to avoid subscribing effects that
        // call upsert_droppable to the droppables signal — that would
        // create a read→write→read loop.
        let current = self.droppables.peek();
        if current.get(&id) == Some(&rect) {
            return;
        }
        drop(current);
        self.droppables.with_mut(|map| {
            map.insert(id, rect);
        });
    }

    /// Store the element handle so the centralized re-measure effect
    /// can reach it. Called from `use_droppable`'s `on_mounted`.
    pub(crate) fn register_element(mut self, id: DroppableId, data: Rc<MountedData>) {
        self.elements.with_mut(|map| {
            map.insert(id, data);
        });
    }

    /// Remove the element handle. Called on droppable unmount.
    pub(crate) fn unregister_element(mut self, id: DroppableId) {
        self.elements.with_mut(|map| {
            map.remove(&id);
        });
    }

    /// Re-measure **all** registered droppable elements and write the
    /// updated rects into `self.droppables` in one batch. This avoids
    /// the O(N²) cascade that happens when N individual effects each
    /// write one rect at a time (every write notifies all displacement
    /// memos, and each memo reads the full map).
    ///
    /// **Only notifies subscribers when at least one rect actually
    /// changed.** `with_mut` in Dioxus always triggers a notification,
    /// so we peek first to decide whether the write is necessary.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn remeasure_all(mut self) {
        let elements = self.elements.peek().clone();

        // Phase 1: measure all elements and collect new rects.
        let mut new_rects: Vec<(DroppableId, Rect)> = Vec::with_capacity(elements.len());
        for (id, mounted) in &elements {
            if let Some(rect) = crate::dom::bounding_rect_of(mounted) {
                new_rects.push((*id, rect));
            }
        }

        // Phase 2: check if anything actually differs from the current
        // map. If not, skip the `with_mut` entirely so Dioxus doesn't
        // notify subscribers (which would trigger a full displacement
        // cascade for zero benefit).
        let current = self.droppables.peek();
        let any_changed = new_rects.iter().any(|(id, rect)| current.get(id) != Some(rect));
        drop(current);

        if !any_changed {
            return;
        }

        // Phase 3: apply all changes in one write → one notification.
        self.droppables.with_mut(|map| {
            for (id, rect) in new_rects {
                map.insert(id, rect);
            }
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
    ///
    /// Uses the containment-first policy from
    /// [`taino_dnd_core::pointer_within`]: the pointer must lie inside a
    /// droppable's rect to activate it. When the pointer is outside every
    /// droppable (e.g. in the gap between two stacked zones) `over` is
    /// `None` so cross-zone drop-preview shifts don't fire prematurely.
    pub(crate) fn update_over(mut self, pointer: Point) {
        let id = self
            .droppables
            .with(|map| pointer_within(pointer, map.iter().map(|(id, rect)| (*id, *rect))));
        if *self.over.peek() != id {
            self.over.set(id);
        }
    }

    /// Borrow the registry of mounted droppables and their last-known
    /// bounding rects. Subscribes the calling reactive scope (memo /
    /// effect) to registry updates so derived values re-run when
    /// droppables are added, removed, or remeasured.
    ///
    /// Use when building layouts where the library's built-in
    /// per-droppable `displacement` memo is too coarse — for example,
    /// multi-zone demos that need to run
    /// [`taino_dnd_core::live_displacements`] separately per zone with
    /// only that zone's cards.
    pub fn with_droppables<R>(self, f: impl FnOnce(&HashMap<DroppableId, Rect>) -> R) -> R {
        self.droppables.with(f)
    }

    /// Same as `with_droppables` but uses `peek()` under the hood. Does not subscribe the
    /// calling effect or memo to registry updates. Useful for derivations that only need
    /// to read rects when some *other* signal (like `over` or `dragged_droppable`) changes.
    pub fn peek_droppables<R>(self, f: impl FnOnce(&HashMap<DroppableId, Rect>) -> R) -> R {
        let droppables = self.droppables.peek();
        f(&droppables)
    }

    /// Move keyboard-driven focus to a neighbor droppable in `direction`.
    ///
    /// Returns the new `over` id (or the old one if no neighbor exists
    /// — i.e. we're at the edge of the layout in that direction).
    ///
    /// Beyond updating `over`, this also synthesizes a `current` value
    /// in the [`DragState::Dragging`] variant so the `DragOverlay`
    /// translates to the new target's slot — giving keyboard drags the
    /// same visual feel as mouse drags (the overlay follows the
    /// selection, neighbors part to make room). Without this the
    /// overlay would stay glued to the source position and the
    /// keyboard user would see "an invisible something" opening up the
    /// gap.
    pub(crate) fn keyboard_step(mut self, direction: Direction) -> Option<DroppableId> {
        let from = (*self.over.peek())?;
        let next = self
            .droppables
            .with(|map| spatial_neighbor(from, direction, map.iter().map(|(id, r)| (*id, *r))));
        let Some(id) = next else {
            return Some(from);
        };
        self.over.set(Some(id));

        // Copy the state value out before reading any other signal —
        // Dioxus's `peek()` guard would otherwise outlive a subsequent
        // `set` and panic with a borrow conflict.
        let snapshot = *self.state.peek();
        if let DragState::Dragging { id: dragged_id, start, .. } = snapshot {
            let target_rect = self.droppables.with(|map| map.get(&id).copied());
            if let Some(rect) = target_rect {
                let new_current = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
                self.state.set(DragState::Dragging { id: dragged_id, start, current: new_current });
            }
        }
        Some(id)
    }

    /// Push a screen-reader announcement onto the live region. Equivalent
    /// to writing into [`Self::announcement`] but kept as a method for
    /// symmetry with the Leptos binding.
    pub fn announce(mut self, message: impl Into<String>) {
        self.announcement.set(message.into());
    }

    /// Append a single [`Modifier`] to the chain.
    pub fn push_modifier(mut self, modifier: Modifier) {
        self.modifiers.with_mut(|ms| ms.push(modifier));
    }

    /// Replace the entire modifier chain.
    pub fn set_modifiers(mut self, modifiers: Vec<Modifier>) {
        self.modifiers.set(modifiers);
    }

    /// Set (or clear) the container rect used by
    /// [`Modifier::RestrictToParent`]. Typically called from a
    /// [`use_drag_container`](crate::use_drag_container) effect.
    pub fn set_restrict_container(mut self, rect: Option<Rect>) {
        self.restrict_container.set(rect);
    }

    /// Run the current modifier chain over a raw displacement.
    pub(crate) fn modify(self, raw: Vector) -> Vector {
        let mctx = ModifierContext {
            container: *self.restrict_container.peek(),
            element: *self.dragged_element_rect.peek(),
        };
        self.modifiers.with(|ms| apply_chain(ms, raw, &mctx))
    }

    /// Convenience: turn a raw pointer position into the post-modifier
    /// effective position, given the drag's starting point.
    pub(crate) fn effective_point(self, start: Point, raw: Point) -> Point {
        let v = self.modify(Vector::new(raw.x - start.x, raw.y - start.y));
        Point::new(start.x + v.x, start.y + v.y)
    }

    /// Settle a completed drop (state is already `Dropping`).
    ///
    /// When `to` is `Some` and motion is allowed, the overlay animates to `to`
    /// over [`DROP_ANIMATION_MS`] before the state returns to `Idle`;
    /// otherwise it settles immediately. `to` is the viewport-space top-left
    /// of the slot the item lands in (see [`Self::drop_target`]).
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables, unused_mut))]
    pub(crate) fn settle_drop(mut self, to: Option<Point>) {
        #[cfg(target_arch = "wasm32")]
        if let Some(to) = to {
            if !crate::dom::prefers_reduced_motion() {
                self.drop_target.set(Some(to));
                let ctx = self;
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                crate::dom::set_timeout(DROP_ANIMATION_MS as i32, move || ctx.finish_settle());
                return;
            }
        }
        self.finish_settle();
    }

    /// Complete the `Dropping → Idle` transition and clear drop-related state.
    /// Guarded so a drop animation that outlives the start of a *new* drag
    /// doesn't clobber it.
    fn finish_settle(mut self) {
        if matches!(*self.state.peek(), DragState::Dropping { .. }) {
            self.state.set(DragState::Idle);
            self.over.set(None);
        }
        self.drop_target.set(None);
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
    let drop_target = use_signal::<Option<Point>>(|| None);
    let modifiers = use_signal::<Vec<Modifier>>(Vec::new);
    let restrict_container = use_signal::<Option<Rect>>(|| None);
    let auto_scroll = use_signal(AutoScrollConfig::default);
    let elements = use_signal::<HashMap<DroppableId, Rc<MountedData>>>(HashMap::new);
    let raf_scrolling = use_signal(|| false);
    // Deduped dragged-droppable memo: only changes on drag start/end,
    // NOT on every pointermove. Displacement memos subscribe to this
    // instead of raw `state` to avoid N re-evaluations per move.
    let dragged_droppable = use_memo(move || match *state.read() {
        DragState::Dragging { id, .. } => Some(DroppableId(id.0)),
        _ => None,
    });
    let ctx = DndContext {
        state,
        droppables,
        over,
        last_drop,
        announcement,
        dragged_element_rect,
        drop_target,
        modifiers,
        restrict_container,
        auto_scroll,
        elements,
        raf_scrolling,
        dragged_droppable,
    };
    use_context_provider(|| ctx);
    crate::autoscroll::install(ctx);

    // When state returns to Idle, clear the cached element rect.
    use_effect(move || {
        if matches!(*ctx.state.read(), DragState::Idle) {
            let mut rect = ctx.dragged_element_rect;
            if rect.peek().is_some() {
                rect.set(None);
            }
        }
    });

    // ── Centralized re-measure effect ──────────────────────────────
    //
    // Re-measure all droppable rects when a drag starts. Layout can
    // change between drags (e.g. items reordered after a previous drop).
    //
    // Scroll-driven re-measurement is handled directly by the RAF loop
    // and the scroll listener (both call `remeasure_all()` inline),
    // so no `measurement_tick` effect is needed here.
    #[cfg(target_arch = "wasm32")]
    {
        let is_active = use_memo(move || {
            matches!(*ctx.state.read(), DragState::Pressed { .. } | DragState::Dragging { .. })
        });
        use_effect(move || {
            if *is_active.read() {
                ctx.remeasure_all();
            }
        });
    }

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
