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

/// Bounding rect of `element` in CSS pixels (viewport coordinates),
/// **with the element's CSS `transform: translate(...)` subtracted out**
/// so the returned rect reflects the element's *layout* position.
///
/// `getBoundingClientRect` includes any transform applied to the
/// element. The drop-preview applies `transform: translate(...)` to
/// displaced cards during a drag; if we kept the transform-included
/// rect in our registry, the cursor's containment check would track
/// the *visual* position. Every mid-drag remeasure (auto-scroll RAF,
/// scroll listener) would then feed the preview transform back into
/// the registry and produce a flicker loop: transform applies → rect
/// captured → `update_over` reports no containment → transform clears
/// → measure again → repeat at frame rate.
///
/// Subtracting the computed translate gives the same answer
/// `getBoundingClientRect` would have returned if no transform were
/// applied — i.e. the element's true layout position — which is what
/// collision detection actually wants.
pub(crate) fn bounding_rect(element: &Element) -> Rect {
    let r = element.get_bounding_client_rect();
    let (tx, ty) = layout_translate(element);
    Rect::new(r.x() - tx, r.y() - ty, r.width(), r.height())
}

/// Bounding rect of a Dioxus-mounted element, if it's a real DOM node.
/// Returns `None` for non-DOM renderers (desktop, mobile, etc.) where
/// `MountedData::downcast::<Element>` doesn't apply.
pub(crate) fn bounding_rect_of(mounted: &Rc<MountedData>) -> Option<Rect> {
    let el = mounted.downcast::<Element>()?;
    Some(bounding_rect(el))
}

/// Read the element's computed `transform` and extract its `(tx, ty)`
/// translate components. Returns `(0.0, 0.0)` when there's no transform
/// or when the computed value can't be parsed (e.g. `none`, `rotate`,
/// `skew`).
///
/// Only `matrix(...)` and `matrix3d(...)` are decoded — those are what
/// `getComputedStyle` always normalises to in modern browsers, so any
/// `translate(...)` / `translate3d(...)` / mixed-transform input from
/// CSS reaches us as a matrix here.
fn layout_translate(el: &Element) -> (f64, f64) {
    let Some(win) = web_sys::window() else {
        return (0.0, 0.0);
    };
    let Ok(Some(style)) = win.get_computed_style(el) else {
        return (0.0, 0.0);
    };
    let transform = style.get_property_value("transform").unwrap_or_default();
    parse_matrix_translate(&transform)
}

fn parse_matrix_translate(transform: &str) -> (f64, f64) {
    let transform = transform.trim();
    if transform.is_empty() || transform == "none" {
        return (0.0, 0.0);
    }
    if let Some(inner) = transform.strip_prefix("matrix(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() == 6 {
            return (
                parts[4].parse::<f64>().unwrap_or(0.0),
                parts[5].parse::<f64>().unwrap_or(0.0),
            );
        }
    }
    if let Some(inner) = transform.strip_prefix("matrix3d(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() == 16 {
            return (
                parts[12].parse::<f64>().unwrap_or(0.0),
                parts[13].parse::<f64>().unwrap_or(0.0),
            );
        }
    }
    (0.0, 0.0)
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
