//! Browser-side smoke test. Run with:
//!
//! ```sh
//! wasm-pack test --headless --chrome -p taino-dnd-leptos
//! ```
//!
//! Stage 1 keeps this minimal: a single test that asserts the public hooks
//! can be called inside a Leptos `Owner` scope without panicking. Real
//! end-to-end interaction tests land in Stage 2 once the keyboard sensor
//! gives us a deterministic input path.

#![cfg(target_arch = "wasm32")]

use leptos::prelude::*;
use leptos::reactive::owner::Owner;
use taino_dnd_core::{DragState, DraggableId, DroppableId};
use taino_dnd_leptos::{provide_dnd_context, use_dnd_context, use_draggable, use_droppable};
use wasm_bindgen_test::wasm_bindgen_test_configure;

wasm_bindgen_test_configure!(run_in_browser);

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

        let z = use_droppable(DroppableId(1));
        assert_eq!(z.id, DroppableId(1));
        assert!(!z.is_over.get_untracked());

        // The provided context is what we get back.
        let same = use_dnd_context();
        assert_eq!(same.state.get_untracked(), DragState::Idle);
    });
}
