//! Screen-reader announcement infrastructure for Dioxus.
//!
//! Render `<DndAnnouncer/>` once near the root of your drag-and-drop
//! region (after `provide_dnd_context()` has been called in an
//! ancestor). It mounts a visually-hidden
//! `role="alert" aria-live="assertive"` region that mirrors the current
//! value of [`DndContext::announcement`](crate::DndContext::announcement).
//!
//! `assertive` (rather than `polite`) is intentional. During a drag the
//! user has just pressed a key and is waiting to hear what happened; if
//! a focus change is still being spoken (`Tab` → card label), a polite
//! live region gets queued behind it and observed in practice to be
//! dropped by NVDA when the next move arrives. `assertive` interrupts
//! the current speech so every pickup / move / drop / cancel reliably
//! reaches the user. This matches the pattern the Leptos binding
//! settled on after NVDA testing.

use dioxus::prelude::*;

use crate::context::use_dnd_context;

/// A visually-hidden ARIA live region that announces drag-and-drop
/// activity for the surrounding [`DndContext`](crate::DndContext).
///
/// Render once per region. The element is visible only to assistive
/// technology — it is rendered off-screen using the standard "sr-only"
/// CSS technique.
#[component]
pub fn DndAnnouncer() -> Element {
    let ctx = use_dnd_context();
    rsx! {
        div {
            role: "alert",
            "aria-live": "assertive",
            "aria-atomic": "true",
            style: "position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; \
                    overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0;",
            "{ctx.announcement}"
        }
    }
}
