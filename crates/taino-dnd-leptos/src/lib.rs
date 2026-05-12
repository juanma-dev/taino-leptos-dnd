//! Leptos hooks and components for accessible, pointer-events drag-and-drop.
//!
//! This crate is the framework-binding layer; see [`taino_dnd_core`] for the
//! framework-free primitives that power it.
//!
//! # Stage 1 status
//!
//! The MVP exposes:
//!
//! - [`provide_dnd_context`] — install the shared `DndContext` in the component tree.
//! - [`use_dnd_context`] — pull the context out of any descendant.
//!
//! Hooks for `use_draggable` and `use_droppable` are next on the roadmap; see
//! [`docs/ROADMAP.md`](https://github.com/REPLACE_ME/taino-leptos-dnd/blob/main/docs/ROADMAP.md).

#![doc(html_root_url = "https://docs.rs/taino-dnd-leptos/0.0.1")]

use leptos::prelude::*;
use taino_dnd_core::DragState;

/// Shared drag-and-drop state installed at the root of a region that uses
/// `taino-dnd-leptos`.
///
/// Wrap the part of your tree that participates in drag-and-drop with a call to
/// [`provide_dnd_context`], then call [`use_dnd_context`] from descendants.
#[derive(Clone, Copy)]
pub struct DndContext {
    /// The current drag state. Reactive; subscribe with `.get()`.
    pub state: RwSignal<DragState>,
}

impl Default for DndContext {
    fn default() -> Self {
        Self { state: RwSignal::new(DragState::Idle) }
    }
}

/// Install a [`DndContext`] for descendants. Call once near the root of your
/// drag-and-drop region (typically inside the top-level component for a page or a
/// board).
///
/// Returns the context so the caller can keep a handle if desired.
pub fn provide_dnd_context() -> DndContext {
    let ctx = DndContext::default();
    provide_context(ctx);
    ctx
}

/// Retrieve the nearest ancestor [`DndContext`].
///
/// # Panics
///
/// Panics if no `DndContext` has been provided in an ancestor. This is intentional:
/// calling a drag-and-drop hook outside a drag-and-drop scope is a programmer error,
/// not a recoverable runtime condition.
pub fn use_dnd_context() -> DndContext {
    use_context::<DndContext>()
        .expect("taino-dnd: provide_dnd_context() must be called in an ancestor")
}

#[cfg(test)]
mod tests {
    //! Smoke tests that don't require a DOM. Real browser tests will live under
    //! `tests/web.rs` and run with `wasm-bindgen-test` once Stage 1 hooks land.

    #[test]
    fn crate_compiles() {
        // Intentionally empty: this test exists so `cargo test -p taino-dnd-leptos`
        // exercises the crate's native build path.
    }
}
