# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.5] - 2026-05-26
### Added
- **Scroll-container auto-scroll (both bindings).** Auto-scroll now drives
  arbitrary overflow ancestors, not just the window. During a drag the RAF
  loop walks the scrollable-ancestor chain of the element under the pointer
  (innermost first) and scrolls the first one that can still move toward the
  pointer's edge, falling back to the window. A single **capturing** window
  `scroll` listener catches scrolls of any descendant container (scroll events
  don't bubble, but the capture phase reaches the window-level listener) and
  keeps the droppable registry correct: the window path shifts every rect by
  the inverse delta (cheap), while a container scroll re-measures (only that
  container's descendants moved). To make the container re-measure flicker-free,
  `taino-dnd-leptos`'s `dom::bounding_rect` now subtracts the element's computed
  `transform: translate(...)` (matching the Dioxus binding), so a mid-drag
  remeasure returns the layout position instead of feeding the drop-preview
  transform back into collision detection. Demonstrated by making the
  `sortable-list` examples (both frameworks) a bounded, scrollable region.
- **Drop-settle animation (both bindings).** On release the `DragOverlay` now
  glides from the drop position to the slot the item lands in (the `over`
  droppable, or the source's origin when dropped outside any target) before the
  state settles to `Idle` — the react-beautiful-dnd "fly into place" feel. The
  state machine already modelled the `Dropping → Settle` phase; the bindings now
  use it: `on_pointer_up` / keyboard-drop stash the target in a new
  `drop_target` signal, the overlay applies a CSS `transform` transition toward
  it, and a timer fires `Settle`. Respects `prefers-reduced-motion` (settles
  immediately) and is `None`-safe on native. Every example using `DragOverlay`
  gets it with no code change.
- **`taino-dnd-dioxus` reaches full API parity with `taino-dnd-leptos`.**
  - `use_flip` / `use_flip_with` / `FlipConfig` ported to the Dioxus
    binding (new `flip.rs`). Same First/Last/Invert/Play technique as the
    Leptos hook: it animates the *post-drop settle* (items glide to their
    new slots after release), is suppressed during an active drag, and
    respects `prefers-reduced-motion`. Takes the droppable's mounted-element
    signal (`UseDroppable::element`) where the Leptos version takes a
    `NodeRef`. Apply it to a wrapper with **no** reactive `style` binding so
    Dioxus's attribute reconciliation doesn't clobber the hook's direct
    transform writes (the complement to `drop_preview_style`).
  - `examples/kanban-dioxus`: the Dioxus twin of the Leptos kanban board.
    Three columns, cross-column moves, column-tail "drop at end" zones,
    pointer + touch + keyboard, `DragOverlay`, and `DndAnnouncer`. Uses
    `use_flip` for the reorder animation (contrast with the Dioxus
    sortable-list, which uses the live drop-preview).
  - `taino-dnd-dioxus` now opts into `clippy::{unwrap_used, expect_used,
    panic}` (`#![warn]`), matching the Leptos crate so CI's `-D warnings`
    makes any new occurrence in non-test code a hard error.
  - Enabled the `MediaQueryList` web-sys feature on `taino-dnd-dioxus`
    (required by `use_flip`'s `prefers-reduced-motion` check).

## [0.4.1] - 2026-05-15
### Fixed
- **Dioxus auto-scroll velocity decay across many drags.** The window
  `scroll` event listener and the RAF generation counter were
  installed inside `provide_dnd_context` without a hook guard, so
  Dioxus's re-execution of the host component on every signal change
  attached a fresh `forget()`-ed listener per render. After ~20 drops
  the page was running 20+ scroll listeners per event, each calling
  `remeasure_all` + `update_over`, producing a linear slowdown of the
  auto-scroll loop (mouse-wheel scrolling, which fires few events,
  stayed fast). Wrapping the listener install in `use_hook` makes it
  run exactly once per component instance.
- **Dioxus drop-preview flicker near viewport edges.** `bounding_rect`
  now subtracts the element's computed `transform: translate(...)`
  before returning. Without this, mid-drag `getBoundingClientRect`
  captures the drop-preview transform and feeds it back into the
  registry, producing a flicker loop. Required enabling the
  `CssStyleDeclaration` feature on `web-sys`.

## [0.4.0] - 2026-05-14
### Added
- **Stage 3 begins**: new `taino-dnd-dioxus` crate. First slice ships
  `DndContext`, `provide_dnd_context`, `use_dnd_context`, and the
  `DropResult` value type — same names and same shape as the Leptos
  binding. Built on Dioxus 0.6 with `Signal<T>` for reactive state.
  Confirms `taino-dnd-core` is genuinely framework-free: it serves both
  bindings without any code change.
- Initial workspace scaffolding: `taino-dnd-core`, `taino-dnd-leptos`.
- Roadmap, architecture, accessibility, and contribution docs.
- CI workflow with `fmt`, `clippy`, `test`, `audit`, `deny`, and `doc` jobs.
- Dual MIT / Apache-2.0 license.
- `taino-dnd-core::collision::closest_center` strategy.
- `taino-dnd-leptos`: `DndContext`, `provide_dnd_context`, `use_dnd_context`,
  `use_draggable`, `use_droppable`, `DropResult`.
- `examples/sortable-list`: end-to-end Trunk demo with reordering.
- `wasm-bindgen-test` smoke test in `crates/taino-dnd-leptos/tests/web.rs`.
- Stage 2 accessibility prep: `Direction` enum and
  `taino-dnd-core::spatial_neighbor` strategy for arrow-key navigation.
- Stage 2 keyboard sensor: `DragEvent::KeyboardPickUp`; new
  `UseDraggable::on_key_down` covering Space/Enter pickup & drop, arrow keys
  for movement, and Escape for cancel.
- Stage 2 ARIA: `DndContext::announcement` signal and `DndAnnouncer`
  component rendering a polite live region. Pickup / move / drop / cancel
  all emit announcements.
- README updated with a self-contained quick-start snippet.
- Stage 2 modifiers: `Modifier::RestrictToAxis`, `Modifier::SnapToGrid`,
  `Axis`, `Vector`, and `apply_chain` in `taino-dnd-core`.
  `DndContext::modifiers` signal plus `push_modifier` / `set_modifiers`
  helpers in `taino-dnd-leptos`. Modifiers run on the *output* of the state
  machine (preserving the click-vs-drag threshold) and feed both the visual
  `transform` signal and the collision-detection point.
- Stage 2 `DragOverlay` component: a fixed-position, pointer-events-none
  layer that mirrors the active drag at the modifier-adjusted pointer
  position. Plus `UseDraggable::style_pinned()` for source elements that
  delegate their visual preview to the overlay, and helpers
  `DragState::dragged_id()` / `DragState::is_dragging()` on the core enum.
  The sortable-list example now demonstrates the overlay pattern.
- Stage 2 viewport auto-scroll. New `taino-dnd-core::autoscroll` module
  with a pure `scroll_velocity` function and `AutoScrollConfig`
  (threshold + max speed + enabled flag). `provide_dnd_context()`
  installs a `requestAnimationFrame` loop that, while a drag is active,
  scrolls the window when the pointer approaches a viewport edge.
  `DndContext.measurement_tick` makes `use_droppable` re-measure
  bounding rects on each scroll step so collision detection stays
  accurate.
- Stage 2 `Modifier::RestrictToParent`. Clamps drag displacement so the
  dragged element stays inside a user-chosen container rect. Driven
  through a new `ModifierContext { container, element }` passed to
  `Modifier::apply`/`apply_chain`. New `use_drag_container()` hook
  returns a `NodeRef` that mirrors the container's bounding rect into
  `DndContext.restrict_container` (re-measured on each auto-scroll
  step). Element rect is captured on drag start and cleared on settle.
- Stage 2 FLIP animations. New `use_flip(node_ref)` /
  `use_flip_with(node_ref, FlipConfig)` hooks animate layout changes
  of tracked elements (typically sortable-list rows) using the
  classic First/Last/Invert/Play technique with CSS transitions and
  a forced reflow. Suppressed during active drag, respects
  `prefers-reduced-motion`. Demonstrated in the sortable-list example.
- Stage 2 WASM size budget. New `scripts/size-check.sh` builds the
  sortable-list example, runs it through `wasm-bindgen` + `wasm-opt -Oz`,
  gzips the result, and fails when it exceeds a budget (default 400 KB
  gzipped, overridable via `BUDGET_GZIP_KB`). The new `size-budget` CI
  job runs the same script on every push/PR with prebuilt tools from
  `taiki-e/install-action`.
- Stage 2 acceptance example: `examples/kanban`. Three-column board
  exercising cross-droppable moves with pointer, touch, and keyboard.
  Column-tail droppables (id-range `10_000+`) sit below each card list
  so items can be appended to the end or dropped into an empty column;
  card-slot droppables share their card's id for "insert before this
  card" semantics. Wires `DragOverlay`, `use_flip`, and the existing
  ARIA announcer.
- Stage 2 lint-enforcement: `taino-dnd-core` and `taino-dnd-leptos`
  both opt in (`#![warn(...)]`) to `clippy::unwrap_used`,
  `clippy::expect_used`, and `clippy::panic`. CI's `-D warnings` makes
  any new occurrence in non-test code a hard error. The one documented
  exception (`use_dnd_context`'s missing-provider panic) carries an
  inline `#[allow(clippy::expect_used)]`.

### Fixed
- **Drop-preview flicker during mid-drag scroll.** `getBoundingClientRect`
  returns rects that **include** the element's CSS `transform`, and the
  drop-preview applies `transform: translate(...)` to displaced cards.
  The previous mid-drag remeasure path (subscribed to `measurement_tick`)
  fed those transformed rects back into the registry, which made
  `update_over` report no containment, which cleared the transform,
  which put the cursor back over the un-transformed card — repeating at
  frame rate. Now the drag path **never** calls `getBoundingClientRect`
  after pickup: scroll deltas are applied to the registry via a new
  `shift_droppable_rects` operation that's mathematically equivalent for
  pure scroll and inert to transforms. Pickup-time `remeasure_all` /
  per-droppable `getBoundingClientRect` still runs (no transforms
  applied yet at that moment).
- Window `scroll` event listener now the single source of truth for
  mid-drag rect updates. The auto-scroll RAF loop just calls `scrollBy`;
  the listener catches both the programmatic scroll and any
  user-initiated wheel / trackpad / scrollbar scroll, shifts rects by
  the delta, and re-runs `update_over`. Previously, user wheel
  scrolling mid-drag bypassed the RAF path entirely and `over` would
  freeze on whichever card was last under the cursor.
- Containment-first `update_over` (via
  `taino_dnd_core::pointer_within`) replaces the previous greedy
  `closest_center` default. The pointer must lie inside a droppable's
  rect to activate it. Fixes premature drop-preview activation in
  multi-zone layouts (the gap between two zones no longer "steals" the
  drop slot of the nearest neighbor).
- `DndContext::announce` now blanks the live region and re-sets the text
  on a short timer (50 ms) on wasm targets. Without this, screen readers
  (NVDA observed; JAWS / `VoiceOver` documented to behave the same)
  de-duplicate identical consecutive `aria-live` updates, so e.g. a
  second pickup of the same item read silent. Native builds keep the
  direct `set()` behavior (no DOM).
- `DndAnnouncer` switched from `role="status" aria-live="polite"` to
  `role="alert" aria-live="assertive"`. With `polite`, the focus-change
  announcement that NVDA emits when the user `Tab`s onto a card and then
  presses `Space` was queued in front of the pickup message and observed
  to drop it. `assertive` interrupts the focus announcement so pickup /
  move / drop / cancel all reach the user — this is the pattern
  `react-beautiful-dnd` settled on after testing.

### Verified
- Stage 2 screen-reader smoke-test passed: NVDA on Microsoft Edge on
  Windows 11, kanban example, 2026-05-12. All five announcement classes
  (focus, pickup, move, drop, cancel) reach the user. Raw numeric ids
  in the move/drop messages are tracked as a follow-up in
  `docs/ROADMAP.md` (semantic-labels callback).

### Changed
- `Modifier::apply` and `apply_chain` now take a `&ModifierContext`.
  Existing variants (`RestrictToAxis`, `SnapToGrid`) ignore the
  context, but call sites need to pass one — typically
  `&ModifierContext::default()` for pure-vector code.

[0.4.5]: https://github.com/juanma-dev/taino-leptos-dnd/compare/v0.4.1...v0.4.5
