# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
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

[Unreleased]: https://github.com/juanma-dev/taino-leptos-dnd/commits/main
