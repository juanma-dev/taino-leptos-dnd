# Roadmap

This document is the single source of truth for what gets built and in what order.
It is intentionally aggressive about *not* building things until they earn their place.

Three stages, gated by acceptance criteria. Do not start stage N+1 until stage N is
green on CI and has at least one user-facing example.

---

## Stage 1 — Functional MVP (`v0.0.x → v0.1.0`)

**Goal:** A Leptos user can `cargo add taino-dnd-leptos`, write ~30 lines, and reorder
items in a single vertical list end-to-end. Pointer + touch supported. No keyboard yet.

### Scope

- Workspace scaffolding (this commit).
- `taino-dnd-core`:
  - `DragId(u64)`, `DroppableId(u64)` newtypes with `Display`/`Debug`/`Hash`.
  - `Rect { x, y, width, height }` and `Point { x, y }`.
  - `DragState` enum: `Idle | Pressed { start } | Dragging { id, offset } | Dropping`.
  - Collision detection: closest-center, then bounding-box intersection.
  - Pure functions, no I/O, no globals, no `web_sys`.
- `taino-dnd-leptos`:
  - `provide_dnd_context()` to install a `DndContext` in the component tree.
  - `use_draggable(id)` hook returning `(handlers, signals)`.
  - `use_droppable(id)` hook.
  - Pointer event glue via `web_sys::PointerEvent` (pointerdown/move/up/cancel).
  - SSR-safe (all browser access wrapped in `Effect::new`).
- `examples/sortable-list` running with Trunk.
- Tests:
  - `taino-dnd-core`: unit tests for geometry + state machine, > 90% line coverage.
  - `taino-dnd-leptos`: `wasm-bindgen-test` smoke test that mounts a list and emits
    one synthetic pointer drag.

### Out of scope (Stage 1)

- Keyboard sensor.
- Animations.
- Auto-scroll.
- Multi-drag.
- Nested drop zones with priority.
- ARIA announcements.

### Acceptance

- [ ] `cargo check --workspace` passes.
- [ ] `cargo test --workspace` passes (native + wasm targets).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.
- [ ] `cargo doc --no-deps -D warnings` clean.
- [ ] `examples/sortable-list` builds with `trunk build --release`.
- [ ] README shows a working 30-line snippet that matches the example.

---

## Stage 2 — Production-grade (`v0.1 → v0.3`)

**Goal:** Feature parity with the *good parts* of `react-beautiful-dnd` and `dnd-kit`.
A non-trivial Kanban example must work, including accessibility.

### Scope

#### Accessibility (must land first in this stage)
- ARIA live region for announcements (pick up / moved / dropped / cancelled).
- Visual focus ring for keyboard sensor.
- `aria-roledescription` on draggables.

#### Keyboard sensor
- Space/Enter: pick up & drop.
- Arrow keys: move between adjacent droppables.
- Escape: cancel drag, restore original position.

#### Animation
- FLIP-based reordering (First-Last-Invert-Play).
- Configurable easing/duration per `DndContext`.
- Animation can be disabled (motion-reduced users / tests).

#### Auto-scroll
- Detect overflow ancestors on drag start.
- Scroll containers when pointer is within configurable threshold of edge.
- Configurable speed curve.

#### Sensors as trait
- `Sensor` trait in core (`activate`, `update`, `end`, `cancel`).
- Built-ins: `PointerSensor`, `KeyboardSensor`.
- User can register custom sensors via `DndContextOptions { sensors: [...] }`.

#### Modifiers
- `restrict_to_axis(Axis::X | Axis::Y)`.
- `restrict_to_parent()`.
- `snap_to_grid(grid_size)`.
- Composable via `[Box<dyn Modifier>]`.

#### DragOverlay
- Portal-rendered preview, decoupled from the source element.
- Source element can stay in place, be hidden, or fade.

#### SSR / WASM polish
- Audit for `panic!`-free public APIs (errors are `Result`s).
- `wasm-opt -Oz` step in CI, with a size budget assertion (e.g. `taino-dnd-leptos`
  contributes < 30 KB gzip to a hello-world).

### Out of scope (Stage 2)
- Multi-drag selection.
- Virtual lists (assume user wires their own virtualizer).

### Acceptance

- [ ] All Stage 1 gates still green.
- [ ] `examples/kanban` works with mouse, touch, **and keyboard**.
- [ ] Screen reader smoke-test pass (NVDA on Windows, VoiceOver on macOS).
- [ ] No `unwrap()` / `panic!` in public-facing paths (lint-enforced).
- [ ] Bundle-size CI check passes.

---

## Stage 3 — Multi-framework (`v0.3 → v0.4+`)

**Goal:** Prove `taino-dnd-core` is truly framework-agnostic by porting it to a second
framework. Set up the pattern so a third (Yew) is trivial.

### Scope

- Audit `taino-dnd-core` API. Anything that imports `leptos` is a bug.
- Introduce a `Reactive` trait abstracting signal-like values:
  ```rust
  pub trait Reactive<T> {
      fn get(&self) -> T;
      fn set(&self, value: T);
      fn subscribe(&self, f: Box<dyn FnMut(&T)>) -> Subscription;
  }
  ```
- Move framework-specific reactivity into the bindings crates.
- Build `taino-dnd-dioxus` mirroring the Leptos API surface where possible.
- Publish all three crates on crates.io with synchronized minor versions.
- Add `docs/PORTING.md` describing how to write a new binding.

### Acceptance

- [ ] `taino-dnd-dioxus` example `sortable-list` works.
- [ ] `taino-dnd-core` has **zero** `leptos`, `dioxus`, or framework deps.
- [ ] Published `0.4.0` of all three crates on crates.io.

---

## Versioning

- Pre-1.0: minor version bumps may include breaking changes (per SemVer caveat for 0.x).
- Each release tagged in git with an annotated tag and a `CHANGELOG.md` section.
- Each release gets a `cargo-deny check` and `security-review` pass before tagging.

## Non-goals

We deliberately *will not* ship:

- Re-implementations of HTML5 Drag and Drop. Pointer Events only.
- File-drag support from OS into the page. Use the platform `dragover` for that.
- A general-purpose animation library. We pick FLIP and stop.
