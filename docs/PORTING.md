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
Because DOM reads (`getBoundingClientRect`) are expensive and force synchronous layout, avoid measuring elements continuously. The established pattern in `taino` bindings is:
1. Draggables only update `DragState` (specifically the pointer coordinates).
2. The context provides a centralized `remeasure_all` function that iterates through DOM nodes, measures them, and writes the `HashMap<DroppableId, Rect>` in a single batch.
3. Call `remeasure_all` **only** when a drag starts, or when the auto-scroll loop ticks. Do **not** measure on `pointermove`.

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

Port the `autoscroll.rs` logic. It uses a self-terminating `requestAnimationFrame` loop.
When the pointer is near the edge of the viewport, the loop calls `window.scrollBy()`.

**Important**: Browsers fire a `scroll` event asynchronously after `scrollBy()`. If you also have a window `scroll` event listener to catch user wheel-scrolling, you must implement a "generation counter" or guard flag to prevent the RAF loop and the `scroll` event from both triggering `remeasure_all` in the same frame. See the Dioxus/Leptos implementations for the generation counter pattern.
