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

[Unreleased]: https://github.com/juanma-dev/taino-leptos-dnd/commits/main
