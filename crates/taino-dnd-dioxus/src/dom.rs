//! Browser-only DOM helpers. Gated to wasm32 so the native build never
//! touches `web_sys`. Mirrors `taino_dnd_leptos::dom` so the bindings
//! stay easy to compare side by side.

// `dom::*` items are crate-internal; we keep them `pub(crate)` even
// though clippy would prefer plain `pub` for a private module.
#![allow(clippy::redundant_pub_crate)]

use std::rc::Rc;

use dioxus::prelude::*;
use taino_dnd_core::Rect;
use web_sys::{Element, PointerEvent};

/// Bounding rect of `element` in CSS pixels (viewport coordinates).
pub(crate) fn bounding_rect(element: &Element) -> Rect {
    let r = element.get_bounding_client_rect();
    Rect::new(r.x(), r.y(), r.width(), r.height())
}

/// Bounding rect of a Dioxus-mounted element, if it's a real DOM node.
/// Returns `None` for non-DOM renderers (desktop, mobile, etc.) where
/// `MountedData::downcast::<Element>` doesn't apply.
pub(crate) fn bounding_rect_of(mounted: &Rc<MountedData>) -> Option<Rect> {
    let el = mounted.downcast::<Element>()?;
    Some(bounding_rect(el))
}

/// Get the underlying DOM `Element` of a mounted Dioxus node.
pub(crate) fn element_of(mounted: &Rc<MountedData>) -> Option<Element> {
    mounted.downcast::<Element>().cloned()
}

/// Capture the pointer for an element. Subsequent pointer events for
/// this pointer id are delivered to `element` regardless of hit testing.
/// Failure is swallowed: capture loss isn't fatal, the drag just loses
/// a few `pointermove` events when the pointer leaves the element.
pub(crate) fn capture_pointer(element: &Element, event: &PointerEvent) {
    let _ = element.set_pointer_capture(event.pointer_id());
}

/// Release a previously-captured pointer.
pub(crate) fn release_pointer(element: &Element, event: &PointerEvent) {
    let _ = element.release_pointer_capture(event.pointer_id());
}
