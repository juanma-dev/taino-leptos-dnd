//! wasm32 smoke test. Runs in Node by default (via `wasm-bindgen-test-runner`),
//! exercising only the pure-reactivity layer of the public API.
//!
//! ```sh
//! # local (Node):
//! cargo test -p taino-dnd-leptos --target wasm32-unknown-unknown
//!
//! # with a real browser (CI installs chromedriver):
//! NO_HEADLESS=1 wasm-pack test --chrome -p taino-dnd-leptos
//! ```
//!
//! Real DOM-interaction tests (mounting components, dispatching synthetic
//! pointer events) belong in a future browser-only test suite; this file
//! intentionally avoids DOM so it can run anywhere wasm-bindgen-test does.

#![cfg(target_arch = "wasm32")]

use leptos::prelude::*;
use leptos::reactive::owner::Owner;
use taino_dnd_core::{DragState, DraggableId, DroppableId};
use taino_dnd_leptos::{provide_dnd_context, use_dnd_context, use_draggable, use_droppable};

#[wasm_bindgen_test::wasm_bindgen_test]
fn public_api_smoke() {
    let owner = Owner::new();
    owner.with(|| {
        let ctx = provide_dnd_context();
        assert_eq!(ctx.state.get_untracked(), DragState::Idle);
        assert!(ctx.last_drop.get_untracked().is_none());

        let d = use_draggable(DraggableId(1));
        assert_eq!(d.id, DraggableId(1));
        assert!(!d.is_dragging.get_untracked());
        assert!(!d.disabled.get_untracked());

        let z = use_droppable(DroppableId(1));
        assert_eq!(z.id, DroppableId(1));
        assert!(!z.is_over.get_untracked());
        assert!(!z.disabled.get_untracked());

        // The provided context is what we get back.
        let same = use_dnd_context();
        assert_eq!(same.state.get_untracked(), DragState::Idle);
    });
}
