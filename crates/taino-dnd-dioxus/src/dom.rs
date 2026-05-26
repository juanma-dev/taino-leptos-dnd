//! Browser-only DOM helpers. Gated to wasm32 so the native build never
//! touches `web_sys`. Mirrors `taino_dnd_leptos::dom` so the bindings
//! stay easy to compare side by side.

// `dom::*` items are crate-internal; we keep them `pub(crate)` even
// though clippy would prefer plain `pub` for a private module.
#![allow(clippy::redundant_pub_crate)]

use std::rc::Rc;

use dioxus::prelude::*;
use taino_dnd_core::Rect;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
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

/// Raw bounding rect — `getBoundingClientRect` with no transform adjustment.
/// Used for scroll *containers* (which never carry a drop-preview transform),
/// where the visual box is exactly what the auto-scroll edge math wants.
pub(crate) fn bounding_rect_raw(element: &Element) -> Rect {
    let r = element.get_bounding_client_rect();
    Rect::new(r.x(), r.y(), r.width(), r.height())
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
/// root scroller (`<html>` / `<body>`) — that is the window's job, handled by
/// the viewport auto-scroll path.
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

/// Whether the user has requested reduced motion. Callers skip non-essential
/// animation (e.g. the drop-settle glide) when this is `true`.
pub(crate) fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
        .is_some_and(|m| m.matches())
}

/// Run `f` once after `ms` milliseconds via `window.setTimeout`. Dioxus has no
/// framework timer, so we go through `web_sys` directly. `Closure::once_into_js`
/// keeps the closure alive until it fires (one-shot, then dropped by the JS GC).
pub(crate) fn set_timeout(ms: i32, f: impl FnOnce() + 'static) {
    let Some(win) = web_sys::window() else {
        return;
    };
    let cb = Closure::once_into_js(f);
    let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
        cb.unchecked_ref::<js_sys::Function>(),
        ms,
    );
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
