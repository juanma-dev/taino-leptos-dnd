//! Browser-only DOM helpers. The whole module is gated to wasm32 so the
//! native build never touches `web_sys`.

// `unreachable_pub` (rust) wants `pub(crate)` here; `clippy::redundant_pub_crate`
// disagrees because `dom` is itself private. We prefer the explicit semantic and
// silence the clippy variant.
#![allow(clippy::redundant_pub_crate)]

use taino_dnd_core::Rect;
use wasm_bindgen::JsCast;
use web_sys::{Element, PointerEvent};

/// Bounding rect of `element` in CSS pixels, in the viewport coordinate system
/// (matching `PointerEvent::client_x` / `client_y`).
pub(crate) fn bounding_rect(element: &Element) -> Rect {
    let r = element.get_bounding_client_rect();
    Rect::new(r.x(), r.y(), r.width(), r.height())
}

/// Capture the pointer for an element. Subsequent pointer events for this
/// pointer id are delivered to `element` regardless of hit testing.
///
/// Capture failure is swallowed: the drag still works, the user might just
/// lose a `pointermove` when the pointer leaves the element bounds.
pub(crate) fn capture_pointer(element: &Element, event: &PointerEvent) {
    let _ = element.set_pointer_capture(event.pointer_id());
}

/// Release a previously-captured pointer. Same error-handling stance as
/// [`capture_pointer`].
pub(crate) fn release_pointer(element: &Element, event: &PointerEvent) {
    let _ = element.release_pointer_capture(event.pointer_id());
}

/// Downcast an [`web_sys::EventTarget`] to [`web_sys::Element`].
#[allow(dead_code)]
pub(crate) fn as_element(target: &web_sys::EventTarget) -> Option<Element> {
    target.dyn_ref::<Element>().cloned()
}
