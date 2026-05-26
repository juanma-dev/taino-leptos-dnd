//! Shared drag-and-drop state for a region of a Leptos tree.
//!
//! Every region that wants to use the drag-and-drop hooks needs a [`DndContext`]
//! available via Leptos's context API. Install one with [`provide_dnd_context`].

use std::collections::HashMap;

use leptos::prelude::*;
use taino_dnd_core::{
    apply_chain, pointer_within, spatial_neighbor, AutoScrollConfig, Direction, DragState,
    DraggableId, DroppableId, Modifier, ModifierContext, Point, Rect, Vector,
};

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
    /// Latest screen-reader announcement. Mirrored into a polite ARIA live
    /// region by [`DndAnnouncer`](crate::DndAnnouncer).
    pub announcement: RwSignal<String>,
    /// Ordered list of [`Modifier`]s applied to the drag displacement before
    /// it's used for the visual transform and for collision detection.
    ///
    /// The list is empty by default. Mutate with [`DndContext::push_modifier`]
    /// / [`DndContext::set_modifiers`], or read/write the signal directly for
    /// reactive control.
    pub modifiers: RwSignal<Vec<Modifier>>,
    /// Auto-scroll configuration. Drives the viewport-edge auto-scroll
    /// behavior during a drag. Set `enabled` to `false` to opt out.
    pub auto_scroll: RwSignal<AutoScrollConfig>,
    /// Bump-counter signal that asks all `use_droppable` instances to
    /// re-measure their bounding rects on the next tick. Incremented by the
    /// auto-scroll loop after a `scrollBy` so collision detection uses
    /// up-to-date rects.
    pub(crate) measurement_tick: RwSignal<u64>,
    /// Optional bounding rect of the container that
    /// [`Modifier::RestrictToParent`] should keep drags inside. Set via
    /// [`DndContext::set_restrict_container`] or via the
    /// [`use_drag_container`](crate::use_drag_container) helper hook.
    pub restrict_container: RwSignal<Option<Rect>>,
    /// Bounding rect of the dragged element at drag-start. Populated by
    /// `use_draggable` on `pointerdown`/keyboard-pickup; cleared on settle.
    pub(crate) dragged_element_rect: RwSignal<Option<Rect>>,
    /// While in [`DragState::Dropping`], the viewport-space point the drag
    /// overlay should animate **to** — the top-left of the slot the item
    /// lands in, or the source's origin when dropped outside any droppable.
    /// `None` outside a drop animation. Drives [`DragOverlay`]'s exit
    /// transition (the post-release "fly to slot" settle).
    ///
    /// [`DragOverlay`]: crate::DragOverlay
    pub(crate) drop_target: RwSignal<Option<Point>>,
}

/// Duration of the drop-settle overlay animation, in milliseconds. The
/// overlay's CSS transition and the `Settle` timer share this value so the
/// state returns to `Idle` exactly when the glide finishes.
// `unreachable_pub` would prefer `pub(crate)` (the module is private), which
// then trips `redundant_pub_crate`; the const is genuinely crate-internal.
#[allow(clippy::redundant_pub_crate)]
pub(crate) const DROP_ANIMATION_MS: u64 = 200;

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
            announcement: RwSignal::new(String::new()),
            modifiers: RwSignal::new(Vec::new()),
            auto_scroll: RwSignal::new(AutoScrollConfig::default()),
            measurement_tick: RwSignal::new(0),
            restrict_container: RwSignal::new(None),
            dragged_element_rect: RwSignal::new(None),
            drop_target: RwSignal::new(None),
        }
    }
}

impl DndContext {
    /// Register or update the bounding rect for a droppable.
    ///
    /// Only the wasm32 build path calls this; native builds keep the registry
    /// empty (there's no DOM to measure against).
    ///
    /// **Short-circuits** when the stored rect already matches `rect`,
    /// avoiding a redundant notification to all subscribers.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn upsert_droppable(self, id: DroppableId, rect: Rect) {
        let dominated = self.droppables.with_untracked(|map| map.get(&id) == Some(&rect));
        if dominated {
            return;
        }
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
    ///
    /// Uses the containment-first policy from
    /// [`taino_dnd_core::pointer_within`]: the pointer must lie inside a
    /// droppable's rect to activate it. When the pointer is outside every
    /// droppable (e.g. in the gap between two stacked zones) `over` is
    /// `None` so cross-zone drop-preview shifts don't fire prematurely.
    pub(crate) fn update_over(self, pointer: Point) {
        let id = self
            .droppables
            .with(|map| pointer_within(pointer, map.iter().map(|(id, rect)| (*id, *rect))));
        if self.over.get_untracked() != id {
            self.over.set(id);
        }
    }

    /// Borrow the registry of mounted droppables and their last-known
    /// bounding rects. Subscribes the calling reactive scope to
    /// registry updates so derived signals re-run when droppables are
    /// added, removed, or remeasured.
    ///
    /// Use when building layouts where the library's built-in
    /// per-droppable `displacement` signal is too coarse — for example,
    /// multi-zone demos that need to run
    /// [`taino_dnd_core::live_displacements`] separately per zone with
    /// only that zone's cards.
    pub fn with_droppables<R>(self, f: impl FnOnce(&HashMap<DroppableId, Rect>) -> R) -> R {
        self.droppables.with(f)
    }

    /// Clear [`DndContext::last_drop`] after the caller has consumed it.
    pub fn clear_last_drop(self) {
        self.last_drop.set(None);
    }

    /// Read **and** clear [`DndContext::last_drop`] in one call,
    /// returning the previous value (if any). Subscribes the calling
    /// effect to `last_drop` changes.
    ///
    /// Prefer this over the manual `if let Some(d) = ctx.last_drop.get()
    /// { ... ctx.clear_last_drop(); }` pattern — it keeps the example
    /// code identical with the Dioxus binding (where the manual form
    /// hits a borrow-conflict footgun) and avoids the double-trigger
    /// edge case where the effect re-fires once for the set and once
    /// for the clear.
    pub fn take_last_drop(self) -> Option<DropResult> {
        let value = self.last_drop.get();
        if value.is_some() {
            self.last_drop.set(None);
        }
        value
    }

    /// Move keyboard-driven focus to a neighbor droppable in `direction`.
    ///
    /// Returns the new `over` id (or the old one if no neighbor exists —
    /// i.e. we're at the edge of the layout in that direction).
    ///
    /// Beyond updating `over`, this also synthesizes a `current` value
    /// in the [`DragState::Dragging`] variant so the `DragOverlay`
    /// translates to the new target's slot — giving keyboard drags the
    /// same visual feel as mouse drags (the overlay follows the
    /// selection, neighbors part to make room). Without this the
    /// overlay would stay glued to the source position and the
    /// keyboard user would see "an invisible something" opening up the
    /// gap.
    pub(crate) fn keyboard_step(self, direction: Direction) -> Option<DroppableId> {
        let from = self.over.get_untracked()?;
        let next = self
            .droppables
            .with(|map| spatial_neighbor(from, direction, map.iter().map(|(id, r)| (*id, *r))));
        let Some(id) = next else {
            return Some(from);
        };
        self.over.set(Some(id));

        if let DragState::Dragging { id: dragged_id, start, .. } = self.state.get_untracked() {
            let target_rect = self.droppables.with(|map| map.get(&id).copied());
            if let Some(rect) = target_rect {
                let new_current = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
                self.state.set(DragState::Dragging { id: dragged_id, start, current: new_current });
            }
        }
        Some(id)
    }

    /// Push a screen-reader announcement.
    ///
    /// Screen readers (NVDA, JAWS, `VoiceOver`) de-duplicate identical
    /// consecutive updates to `aria-live` regions, so naively re-setting the
    /// same string silently drops, e.g., the second "Picked up item 1" when
    /// the user cancels and immediately picks up the same item again. To
    /// guarantee every call re-reads, the wasm implementation blanks the
    /// region and then sets the real text after a short delay so the live
    /// region mutation observer sees two distinct states. Native builds
    /// just set the signal directly (no DOM, nothing to dedupe).
    pub fn announce(self, message: impl Into<String>) {
        let msg = message.into();
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.announcement.set(msg);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.announcement.set(String::new());
            let signal = self.announcement;
            leptos::prelude::set_timeout(
                move || signal.set(msg),
                std::time::Duration::from_millis(50),
            );
        }
    }

    /// Append a single [`Modifier`] to the chain.
    ///
    /// ```no_run
    /// use taino_dnd_leptos::{provide_dnd_context, Axis, Modifier};
    ///
    /// let ctx = provide_dnd_context();
    /// // Lock all drags in this scope to the vertical axis.
    /// ctx.push_modifier(Modifier::RestrictToAxis(Axis::Y));
    /// // ...and snap to an 8 px grid on the way down.
    /// ctx.push_modifier(Modifier::SnapToGrid { x: 8.0, y: 8.0 });
    /// ```
    pub fn push_modifier(self, modifier: Modifier) {
        self.modifiers.update(|ms| ms.push(modifier));
    }

    /// Replace the entire modifier chain.
    pub fn set_modifiers(self, modifiers: Vec<Modifier>) {
        self.modifiers.set(modifiers);
    }

    /// Run the current modifier chain over a raw displacement.
    pub(crate) fn modify(self, raw: Vector) -> Vector {
        let ctx = ModifierContext {
            container: self.restrict_container.get_untracked(),
            element: self.dragged_element_rect.get_untracked(),
        };
        self.modifiers.with(|ms| apply_chain(ms, raw, &ctx))
    }

    /// Set (or clear) the container rect used by
    /// [`Modifier::RestrictToParent`]. Typically called from an `Effect`
    /// watching the container element's bounding rect.
    pub fn set_restrict_container(self, rect: Option<Rect>) {
        self.restrict_container.set(rect);
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
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
    pub(crate) fn settle_drop(self, to: Option<Point>) {
        #[cfg(target_arch = "wasm32")]
        if let Some(to) = to {
            if !crate::dom::prefers_reduced_motion() {
                self.drop_target.set(Some(to));
                let ctx = self;
                leptos::prelude::set_timeout(
                    move || ctx.finish_settle(),
                    std::time::Duration::from_millis(DROP_ANIMATION_MS),
                );
                return;
            }
        }
        self.finish_settle();
    }

    /// Complete the `Dropping → Idle` transition and clear drop-related state.
    /// Guarded so a drop animation that outlives the start of a *new* drag
    /// doesn't clobber it.
    fn finish_settle(self) {
        if matches!(self.state.get_untracked(), DragState::Dropping { .. }) {
            self.state.set(DragState::Idle);
            self.over.set(None);
        }
        self.drop_target.set(None);
    }

    /// Ask all live `use_droppable` instances to re-measure their bounding
    /// rects on the next reactive tick.
    ///
    /// Called by the capturing scroll listener when an **overflow ancestor**
    /// (a scroll container, not the window) scrolls during a drag: only that
    /// container's descendants move, so a blanket
    /// [`Self::shift_droppable_rects`] would be wrong — each droppable must
    /// re-measure its own rect. This is safe because `dom::bounding_rect`
    /// subtracts the drop-preview `transform`, so the measured value is the
    /// layout position and can't feed the flicker loop that the original
    /// transform-included remeasure produced. The window-scroll path still
    /// prefers `shift_droppable_rects` (no `getComputedStyle` per frame).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn request_remeasure(self) {
        self.measurement_tick.update(|t| *t = t.wrapping_add(1));
    }

    /// Translate every rect in the droppable registry by `(dx, dy)`.
    ///
    /// Called by the auto-scroll RAF loop after `window.scrollBy(...)`
    /// and by the window `scroll` listener after a user-initiated
    /// scroll (wheel, trackpad, scrollbar). The shift is mathematically
    /// equivalent to a remeasure for the *pure scroll* case but, unlike
    /// `getBoundingClientRect`, it ignores any CSS transform currently
    /// applied to the element. That matters because the drop-preview
    /// applies a `transform: translate(...)` to displaced cards: a
    /// post-scroll remeasure would capture the **transformed** position
    /// and `update_over` would then report `None`, the transform would
    /// clear, the cursor would be over the un-transformed card again,
    /// and the cycle would repeat at frame rate.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn shift_droppable_rects(self, dx: f64, dy: f64) {
        if dx == 0.0 && dy == 0.0 {
            return;
        }
        self.droppables.update(|map| {
            for rect in map.values_mut() {
                rect.x += dx;
                rect.y += dy;
            }
        });
    }
}

/// Install a [`DndContext`] for descendants. Call once near the root of your
/// drag-and-drop region (typically inside the top-level component for a page
/// or a board).
///
/// Also installs the viewport auto-scroll loop, which kicks in whenever the
/// state enters `Dragging` and the pointer is near a viewport edge. Disable
/// via `ctx.auto_scroll.update(|c| c.enabled = false)`.
///
/// Returns the context so the caller can keep a handle if desired.
pub fn provide_dnd_context() -> DndContext {
    let ctx = DndContext::default();
    provide_context(ctx);
    crate::autoscroll::install(ctx);
    // Whenever a drag ends and the state returns to Idle, clear the dragged
    // element rect so a stale rect can't influence the next drag.
    Effect::new(move |_| {
        if matches!(ctx.state.get(), DragState::Idle) {
            ctx.dragged_element_rect.set(None);
        }
    });
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
    // Intentional panic: see the doc-comment above. The `expect_used` lint is
    // crate-warn by default; this is the one documented exception.
    #[allow(clippy::expect_used)]
    use_context::<DndContext>()
        .expect("taino-dnd: provide_dnd_context() must be called in an ancestor")
}
