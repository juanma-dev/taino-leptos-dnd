//! wasm32 smoke test for the Dioxus binding.
//!
//! Mounts a component that calls the public hooks inside a real `VirtualDom`
//! and rebuilds it once, exercising `provide_dnd_context` / `use_draggable` /
//! `use_droppable` end-to-end without a browser DOM.
//!
//! ```sh
//! # local (Node):
//! cargo test -p taino-dnd-dioxus --target wasm32-unknown-unknown
//!
//! # with a real browser (CI installs chromedriver):
//! wasm-pack test --chrome -p taino-dnd-dioxus
//! ```
//!
//! Deeper DOM-interaction tests (synthetic pointer / keyboard events,
//! collision assertions) belong in a future browser-only suite. The
//! framework-free collision / keyboard / announcement logic this binding
//! drives is unit-tested in `taino-dnd-core` and `taino-dnd-leptos`.

#![cfg(target_arch = "wasm32")]

use dioxus::prelude::*;
use taino_dnd_dioxus::{
    provide_dnd_context, use_draggable, use_droppable, DragState, DraggableId, DroppableId,
};

fn smoke_app() -> Element {
    let ctx = provide_dnd_context();
    debug_assert!(matches!(*ctx.state.read(), DragState::Idle));

    let d = use_draggable(DraggableId(1));
    debug_assert_eq!(d.id, DraggableId(1));
    debug_assert!(!*d.is_dragging.read());
    debug_assert!(!*d.disabled.read());

    let z = use_droppable(DroppableId(1));
    debug_assert_eq!(z.id, DroppableId(1));
    debug_assert!(!*z.is_over.read());
    debug_assert!(!*z.disabled.read());

    rsx! { div {} }
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn public_api_smoke() {
    // Build the component tree once. This runs the hooks inside a live Dioxus
    // runtime; if any of them panic the rebuild propagates it and the test
    // fails.
    let mut dom = VirtualDom::new(smoke_app);
    dom.rebuild_in_place();
}
