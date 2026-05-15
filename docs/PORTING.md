# Porting Taino DnD to a New Framework

`taino-dnd` is designed to be framework-agnostic. The core logic (geometry, state machine, and collision detection) lives entirely in the pure-Rust `taino-dnd-core` crate, which does not depend on `wasm-bindgen`, the DOM, or any specific UI framework.

This guide outlines the steps to build a new framework binding (e.g., `taino-dnd-yew`).

## 1. The Core Concepts

A framework binding is responsible for bridging `taino-dnd-core` with the framework's reactive system and DOM event handling. You will need to implement:

- **`DndContext`**: A struct (usually provided via the framework's context API) holding the reactive state.
- **`use_draggable` (or equivalent)**: A hook/component to attach pointer and keyboard event listeners to draggable DOM elements.
- **`use_droppable` (or equivalent)**: A hook/component to register droppable DOM elements and provide their live drop-preview displacement.
- **Auto-scroll Loop**: A `requestAnimationFrame` (RAF) loop that reads pointer position near the viewport edges and scrolls the window.

## 2. Setting Up the Context

The core state machine is `taino_dnd_core::DragState`. Your context needs to hold this in a reactive primitive (e.g., a Signal in Leptos/Dioxus, or a Reducer/State in Yew).

You will also need to store:
- A registry of droppables: `HashMap<DroppableId, Rect>`.
- The currently hovered droppable: `Option<DroppableId>`.
- The last drop result: `Option<DropResult>`.

### The "Remeasure" Problem
Because DOM reads (`getBoundingClientRect`) are expensive and force synchronous layout, avoid measuring elements continuously. But there's a second, more subtle reason to avoid mid-drag remeasures: **`getBoundingClientRect` includes any CSS `transform` currently applied to the element**, and the drop-preview applies `transform: translate(...)` to displaced cards. A mid-drag remeasure would feed the preview transform back into the registry, producing a flicker loop (transform applies → measure → registry shifts → `update_over` reports no containment → transform clears → repeat at frame rate).

The established pattern in `taino` bindings is:
1. Draggables only update `DragState` (pointer coordinates).
2. The context provides a centralized `remeasure_all` (or per-droppable equivalent) that iterates DOM nodes, measures them via `getBoundingClientRect`, and writes the rect registry in a single batch. **Call it only at pickup** (the Idle → Pressed/Dragging transition), where no drop-preview transforms are applied yet.
3. For mid-drag scroll updates, use a `shift_droppable_rects(dx, dy)` operation that translates every rect in the registry. Scroll is a pure linear shift of viewport-relative coordinates, so this stays mathematically correct without ever calling `getBoundingClientRect`.

## 3. Draggable Implementation

The `use_draggable` equivalent needs to attach the following DOM events:
- `pointerdown`: Triggers `DragState::Pressed`. Capture the pointer (`setPointerCapture`).
- `pointermove`: Triggers `DragState::Dragging` if movement exceeds the activation threshold.
- `pointerup` / `pointercancel`: Triggers drop resolution or cancellation. Release pointer capture.
- `keydown`: Handle `Space`/`Enter` for pickup/drop, `Escape` for cancel, and `Arrow` keys for spatial navigation.

Use `taino_dnd_core::state` transition methods to update the context state.

## 4. Droppable Implementation

The `use_droppable` equivalent needs to:
1. Register its DOM node with the context when mounted.
2. Provide a reactive "displacement" value (`Vector` or `(f64, f64)`).

### Displacement Optimization
To prevent O(N²) reactivity cascades during scroll, droppables should calculate their displacement using `taino_dnd_core::live_displacements`. 

**Critical Performance Tip**: Displacement memos should only subscribe to the `over` target and the `dragged` ID (which only changes at drag start/end). They should *peek* (read non-reactively) the droppable rect registry. Since displacements are scroll-invariant, they don't need to re-evaluate when the window scrolls.

## 5. Auto-scroll

Port the `autoscroll.rs` logic. There are two pieces:

### 5.1 RAF loop
A self-terminating `requestAnimationFrame` loop scheduled when state enters `Dragging`. Each tick computes a scroll velocity via [`taino_dnd_core::scroll_velocity`] and calls `window.scrollBy(dx, dy)`. **That's the whole tick.** No remeasure, no `update_over`, no rect manipulation.

The trigger effect that schedules the loop must **deduplicate by `is_dragging` bool** (Memo / `use_memo`) — every `pointermove` updates `state.current`, so a raw subscription to `state` would spawn a new RAF loop on every move. Combine that with a generation counter so a new drag started before the previous loop has seen the idle transition still runs exactly one loop.

### 5.2 Scroll listener
Install a `window.addEventListener("scroll", ...)` once at context setup. The listener is the **single source of truth** for mid-drag rect updates and collision detection:

```
on scroll:
  delta = current_scroll - last_scroll
  last_scroll = current_scroll
  if not dragging: return
  ctx.shift_droppable_rects(-delta.x, -delta.y)
  ctx.update_over(current_pointer_position)
```

This single path covers both programmatic scrolls from the RAF loop (which fire the same `scroll` event) and user-initiated scrolls (wheel, trackpad, scrollbar). Don't try to call `update_over` from inside the RAF tick — let the listener handle it. Browsers may fire the `scroll` event one frame after `scrollBy()`, which adds at most one frame of lag and is invisible in practice.
