//! Browser-only DOM helpers. The whole module is gated to wasm32 so the
//! native build never touches `web_sys`.

// `unreachable_pub` (rust) wants `pub(crate)` here; `clippy::redundant_pub_crate`
// disagrees because `dom` is itself private. We prefer the explicit semantic and
// silence the clippy variant.
#![allow(clippy::redundant_pub_crate)]

use taino_dnd_core::Rect;
use wasm_bindgen::JsCast;
use web_sys::{Element, PointerEvent};

/// Bounding rect of `element` in CSS pixels (viewport coordinates, matching
/// `PointerEvent::client_x` / `client_y`), **with the element's CSS
/// `transform: translate(...)` subtracted out** so the returned rect reflects
/// the element's *layout* position.
///
/// `getBoundingClientRect` includes any transform applied to the element, and
/// the live drop-preview applies `transform: translate(...)` to displaced
/// droppable wrappers during a drag. If we kept the transform-included rect in
/// the registry, a mid-drag remeasure would feed the preview transform back
/// into collision detection and produce a flicker loop. Subtracting the
/// computed translate yields the same answer `getBoundingClientRect` would
/// have returned with no transform applied — the true layout position, which
/// is what collision detection wants. (This mirrors the Dioxus binding so the
/// two stay measurement-equivalent.)
pub(crate) fn bounding_rect(element: &Element) -> Rect {
    let r = element.get_bounding_client_rect();
    let (tx, ty) = layout_translate(element);
    Rect::new(r.x() - tx, r.y() - ty, r.width(), r.height())
}

/// Raw bounding rect — `getBoundingClientRect` with no transform adjustment.
/// Used for scroll *containers* (which never carry a drop-preview transform),
/// where the visual box is exactly what we want for the auto-scroll edge math.
pub(crate) fn bounding_rect_raw(element: &Element) -> Rect {
    let r = element.get_bounding_client_rect();
    Rect::new(r.x(), r.y(), r.width(), r.height())
}

/// Read the element's computed `transform` and extract its `(tx, ty)`
/// translate components. Returns `(0.0, 0.0)` when there's no transform or the
/// computed value can't be parsed (e.g. `none`, `rotate`, `skew`).
///
/// Only `matrix(...)` and `matrix3d(...)` are decoded — those are what
/// `getComputedStyle` always normalises to, so any `translate(...)` /
/// `translate3d(...)` input from CSS reaches us as a matrix here.
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

/// Topmost element at viewport point `(x, y)`, if any. The drag overlay sets
/// `pointer-events: none`, so it never shadows the real content here.
pub(crate) fn element_from_point(x: f64, y: f64) -> Option<Element> {
    let win = web_sys::window()?;
    let doc = win.document()?;
    #[allow(clippy::cast_possible_truncation)]
    doc.element_from_point(x as f32, y as f32)
}

/// Scrollable ancestors of `el`, **innermost first**. Excludes the document
/// root scroller (`<html>` / `<body>`) — that is the window's job and is
/// handled by the viewport auto-scroll path.
pub(crate) fn scrollable_ancestors(el: &Element) -> Vec<Element> {
    let mut out = Vec::new();
    let root = web_sys::window().and_then(|w| w.document()).and_then(|d| d.document_element());
    let mut node = el.parent_element();
    while let Some(parent) = node {
        if root.as_ref() == Some(&parent) {
            break;
        }
        if is_scrollable(&parent) {
            out.push(parent.clone());
        }
        node = parent.parent_element();
    }
    out
}

/// Whether `el` is an overflow scroll container that currently has room to
/// scroll on at least one axis.
fn is_scrollable(el: &Element) -> bool {
    let overflows =
        (el.scroll_height() > el.client_height()) || (el.scroll_width() > el.client_width());
    if !overflows {
        return false;
    }
    let Some(win) = web_sys::window() else {
        return false;
    };
    let Ok(Some(style)) = win.get_computed_style(el) else {
        return false;
    };
    let ox = style.get_property_value("overflow-x").unwrap_or_default();
    let oy = style.get_property_value("overflow-y").unwrap_or_default();
    is_scroll_value(&oy) || is_scroll_value(&ox)
}

fn is_scroll_value(v: &str) -> bool {
    matches!(v.trim(), "auto" | "scroll" | "overlay")
}

/// Whether `el` can still scroll in the direction implied by `(dx, dy)`.
pub(crate) fn can_scroll(el: &Element, dx: f64, dy: f64) -> bool {
    let max_x = el.scroll_width() - el.client_width();
    let max_y = el.scroll_height() - el.client_height();
    let sl = el.scroll_left();
    let st = el.scroll_top();
    let can_x = (dx < 0.0 && sl > 0) || (dx > 0.0 && sl < max_x);
    let can_y = (dy < 0.0 && st > 0) || (dy > 0.0 && st < max_y);
    (dx != 0.0 && can_x) || (dy != 0.0 && can_y)
}

/// Scroll `el` by `(dx, dy)` CSS pixels (rounded to whole pixels — element
/// scroll offsets are integers).
pub(crate) fn scroll_element_by(el: &Element, dx: f64, dy: f64) {
    #[allow(clippy::cast_possible_truncation)]
    if dx != 0.0 {
        el.set_scroll_left(el.scroll_left() + dx.round() as i32);
    }
    #[allow(clippy::cast_possible_truncation)]
    if dy != 0.0 {
        el.set_scroll_top(el.scroll_top() + dy.round() as i32);
    }
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

/// Whether the user has requested reduced motion. Callers skip non-essential
/// animation (e.g. the drop-settle glide) when this is `true`.
pub(crate) fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
        .is_some_and(|m| m.matches())
}
