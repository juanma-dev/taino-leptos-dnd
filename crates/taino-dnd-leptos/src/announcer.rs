//! Screen-reader announcement infrastructure.
//!
//! Render `<DndAnnouncer/>` once near the root of your drag-and-drop region
//! (after `provide_dnd_context()` has been called in an ancestor). It mounts a
//! visually-hidden `role="alert" aria-live="assertive"` region that mirrors
//! the current value of [`DndContext::announcement`](crate::DndContext::announcement).
//!
//! `assertive` (rather than `polite`) is intentional. During a drag the user
//! has just pressed a key and is waiting to hear what happened; if a focus
//! change is still being spoken (`Tab` → card label), a polite live region
//! gets queued behind it and observed in practice to be dropped by NVDA when
//! the next move arrives. `assertive` interrupts the current speech so every
//! pickup / move / drop / cancel reliably reaches the user. This matches the
//! pattern that `react-beautiful-dnd` settled on after testing.

// `DndAnnouncer` is re-exported from lib.rs as `taino_dnd_leptos::DndAnnouncer`.
// The `unreachable_pub` lint can't see through the `#[component]` macro
// expansion to know the function is actually reachable, so we silence it here.
#![allow(unreachable_pub)]

use std::{fmt::Debug, hash::Hash, marker::PhantomData};

use leptos::prelude::*;

use crate::context::use_dnd_context;

/// A visually-hidden ARIA live region that announces drag-and-drop activity.
///
/// Render once per [`DndContext`](crate::DndContext) scope. Inside the live
/// region, the latest announcement is exposed via `aria-live="polite"` so
/// screen readers read it without interrupting the current utterance.
///
/// The element is visible only to assistive technology; it is rendered
/// off-screen using the standard "sr-only" CSS technique.
#[component]
pub fn DndAnnouncer<T>(#[prop(optional)] _marker: PhantomData<T>) -> impl IntoView
where
    T: Debug + Clone + Copy + PartialEq + Eq + Hash + PartialOrd + Ord + Send + Sync + 'static,
{
    let ctx = use_dnd_context::<T>();
    view! {
        <div
            role="alert"
            aria-live="assertive"
            aria-atomic="true"
            style="position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; \
                   overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0;"
        >
            {move || ctx.announcement.get()}
        </div>
    }
}
