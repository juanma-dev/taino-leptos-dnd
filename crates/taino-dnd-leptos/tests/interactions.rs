//! Browser-only synthetic-interaction tests.
//!
//! Mounts components into a fresh `<div>` per test, dispatches real
//! `PointerEvent` and `KeyboardEvent`, and asserts the binding's public
//! signals respond as expected. Complements:
//!
//! - the Node-runnable smoke in `tests/web.rs` (no DOM), and
//! - the native unit tests in `src/context.rs` (reactivity without a browser).
//!
//! Requires a real DOM, so run via:
//!
//! ```sh
//! wasm-pack test --chrome --headless -p taino-dnd-leptos
//! # or --firefox / --safari / drop --headless for visual debugging.
//! ```
//!
//! `cargo test --target wasm32-unknown-unknown` will *compile* but the
//! `run_in_browser` configure call below makes the tests refuse to run
//! outside a real browser — Node has no `document`.

#![cfg(target_arch = "wasm32")]
// The harness builds events through `Option::unwrap` on `Result`s that only
// fail if the document is missing (we just made it) or the event
// constructor's init-dict shape is wrong (statically known to be valid).
// Treating them as test-time invariants keeps the harness readable; the
// crate's `clippy::unwrap_used` lint is allowed in `#[cfg(test)]` anyway.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    // wasm is single-threaded; `Send` bounds on async tests are noise.
    clippy::future_not_send,
)]

use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use leptos::prelude::*;
use taino_dnd_core::{AnnounceEvent, DragState, DraggableId, DroppableId};
use taino_dnd_leptos::{
    provide_dnd_context, use_draggable, use_draggable_with, use_droppable, DndContext,
};
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, KeyboardEventInit, PointerEventInit};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

// ── Harness ────────────────────────────────────────────────────────────────

/// Append a fresh `<div>` to `<body>` and return it. Each test uses its own
/// root so signals/effects from earlier tests don't bleed in.
fn make_root() -> HtmlElement {
    let doc = web_sys::window().unwrap().document().unwrap();
    let el = doc.create_element("div").unwrap().dyn_into::<HtmlElement>().unwrap();
    // Anchor at (0, 0) with no padding so getBoundingClientRect on children is
    // predictable — the state-machine tests don't depend on it, but the
    // `over`-hover test below does.
    el.set_attribute("style", "position: absolute; top: 0; left: 0; padding: 0; margin: 0;")
        .unwrap();
    doc.body().unwrap().append_child(&el).unwrap();
    el
}

/// RAII guard: when dropped, unmounts the Leptos component and removes the
/// test root from the document.
struct Mounted {
    root: HtmlElement,
    // The mount handle is a generic Leptos type whose name we don't want to
    // spell; `Box<dyn Any>` keeps the API tidy and still runs the proper
    // destructor on drop.
    _unmount: Box<dyn Any>,
}

impl Drop for Mounted {
    fn drop(&mut self) {
        if let Some(parent) = self.root.parent_node() {
            let _ = parent.remove_child(&self.root);
        }
    }
}

/// Mount `view` into a fresh root. The view closure receives a setter that
/// the test uses to stash the [`DndContext`] for later assertions.
fn mount<V, F>(view: F) -> (Mounted, DndContext)
where
    F: FnOnce(Rc<Cell<Option<DndContext>>>) -> V + 'static,
    V: leptos::IntoView + 'static,
{
    let root = make_root();
    let ctx_slot: Rc<Cell<Option<DndContext>>> = Rc::new(Cell::new(None));
    let ctx_for_view = ctx_slot.clone();
    let handle = leptos::mount::mount_to(root.clone(), move || view(ctx_for_view));
    let ctx = ctx_slot.get().expect("App body must call ctx_slot.set(...)");
    (Mounted { root, _unmount: Box::new(handle) }, ctx)
}

/// Dispatch a synthetic `PointerEvent` of `kind` at viewport (`x`, `y`).
fn pointer(target: &web_sys::EventTarget, kind: &str, x: f64, y: f64) {
    let init = PointerEventInit::new();
    init.set_bubbles(true);
    init.set_cancelable(true);
    #[allow(clippy::cast_possible_truncation)]
    {
        init.set_client_x(x as i32);
        init.set_client_y(y as i32);
    }
    init.set_button(0);
    init.set_buttons(1);
    init.set_pointer_type("mouse");
    // web-sys exposes a single `new_with_event_init_dict` even though the IDL
    // dict type is `PointerEventInit`; `unchecked_ref` coerces the subtype.
    let ev = web_sys::PointerEvent::new_with_event_init_dict(kind, init.unchecked_ref()).unwrap();
    let _ = target.dispatch_event(&ev);
}

/// Dispatch a synthetic `KeyboardEvent` of `kind` with `key`.
fn key(target: &web_sys::EventTarget, kind: &str, key: &str) {
    let init = KeyboardEventInit::new();
    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_key(key);
    let ev = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict(kind, init.unchecked_ref())
        .unwrap();
    let _ = target.dispatch_event(&ev);
}

/// Find the first descendant of `root` with `data-handle="<name>"`.
fn find(root: &HtmlElement, name: &str) -> web_sys::Element {
    root.query_selector(&format!("[data-handle='{name}']")).unwrap().unwrap()
}

/// Yield to the event loop long enough for Leptos to flush effects scheduled
/// during the initial render — in particular `use_droppable`'s
/// rect-measurement effect that subscribes to `node_ref.get()` and only fires
/// once the element is in the document. Tests that read `ctx.over` /
/// `ctx.droppables` after mount must `tick().await` first; otherwise the
/// registry is still empty and `keyboard_step` finds no neighbour.
async fn tick() {
    gloo_timers::future::TimeoutFuture::new(16).await;
}

// ── Test app ───────────────────────────────────────────────────────────────

#[component]
fn TwoRows() -> impl IntoView {
    view! {
        <Row id=1 />
        <Row id=2 />
    }
}

#[component]
fn Row(id: u64) -> impl IntoView {
    let d = use_draggable(DraggableId(id));
    let z = use_droppable(DroppableId(id));
    let row_data = id.to_string();
    let item_data = id.to_string();
    // Bake `top` into the static `style` string. Using a reactive `style:top`
    // alongside `style=` left the wrappers stacked at y=0 in headless Chrome
    // (the effect that measures the rect for the droppable registry runs
    // before the individual style property is committed), which made
    // `spatial_neighbor(Down)` see no neighbour and broke the arrow-key
    // tests. A static string applies in one pass.
    let row_style = format!(
        "position: absolute; left: 0; top: {}px; width: 100px; height: 50px;",
        (id - 1) * 50
    );
    view! {
        <div
            node_ref=z.node_ref
            data-handle=format!("row-{row_data}")
            style=row_style
        >
            <div
                node_ref=d.node_ref
                data-handle=format!("item-{item_data}")
                tabindex="0"
                on:pointerdown=move |e| d.on_pointer_down(&e)
                on:pointermove=move |e| d.on_pointer_move(&e)
                on:pointerup=move |e| d.on_pointer_up(&e)
                on:pointercancel=move |e| d.on_pointer_cancel(&e)
                on:keydown=move |e| d.on_key_down(&e)
                style="width: 100%; height: 100%;"
            >
                "Row " {id.to_string()}
            </div>
        </div>
    }
}

#[component]
fn LockedRow(id: u64, locked: ReadSignal<bool>) -> impl IntoView {
    let d = use_draggable_with(DraggableId(id), locked.into());
    let z = use_droppable(DroppableId(id));
    view! {
        <div
            node_ref=z.node_ref
            data-handle=format!("row-{id}")
            style="position: absolute; left: 0; top: 0; width: 100px; height: 50px;"
        >
            <div
                node_ref=d.node_ref
                data-handle=format!("item-{id}")
                tabindex="0"
                on:pointerdown=move |e| d.on_pointer_down(&e)
                on:keydown=move |e| d.on_key_down(&e)
                style="width: 100%; height: 100%;"
            >
                "Locked"
            </div>
        </div>
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[wasm_bindgen_test::wasm_bindgen_test]
fn pointer_down_enters_pressed() {
    let (m, ctx) = mount(|slot| {
        let ctx = provide_dnd_context();
        slot.set(Some(ctx));
        view! { <TwoRows /> }
    });
    let item = find(&m.root, "item-1");
    pointer(&item, "pointerdown", 10.0, 10.0);
    assert!(matches!(ctx.state.get_untracked(), DragState::Pressed { .. }));
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn pointer_move_past_threshold_promotes_to_dragging() {
    let (m, ctx) = mount(|slot| {
        let ctx = provide_dnd_context();
        slot.set(Some(ctx));
        view! { <TwoRows /> }
    });
    let item = find(&m.root, "item-1");
    pointer(&item, "pointerdown", 10.0, 10.0);
    pointer(&item, "pointermove", 80.0, 10.0); // > 5 px threshold
    assert!(matches!(ctx.state.get_untracked(), DragState::Dragging { .. }));
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn pointer_up_after_short_move_is_a_click_not_a_drop() {
    let (m, ctx) = mount(|slot| {
        let ctx = provide_dnd_context();
        slot.set(Some(ctx));
        view! { <TwoRows /> }
    });
    let item = find(&m.root, "item-1");
    pointer(&item, "pointerdown", 10.0, 10.0);
    pointer(&item, "pointermove", 12.0, 10.0); // 2 px — below threshold
    pointer(&item, "pointerup", 12.0, 10.0);
    assert_eq!(ctx.state.get_untracked(), DragState::Idle);
    assert!(ctx.last_drop.get_untracked().is_none());
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn pointer_up_after_real_drag_enters_dropping_and_records_last_drop() {
    let (m, ctx) = mount(|slot| {
        let ctx = provide_dnd_context();
        slot.set(Some(ctx));
        view! { <TwoRows /> }
    });
    let item = find(&m.root, "item-1");
    pointer(&item, "pointerdown", 10.0, 10.0);
    pointer(&item, "pointermove", 80.0, 10.0);
    pointer(&item, "pointerup", 80.0, 10.0);

    // The drop-settle animation is scheduled on a timer, so immediately after
    // `pointerup` the state is `Dropping`. (The native `context` unit tests
    // cover the synchronous `Dropping → Idle` path; here we just witness the
    // transition before the timer fires.)
    assert!(matches!(ctx.state.get_untracked(), DragState::Dropping { .. }));
    let drop = ctx.last_drop.get_untracked().expect("a drop was recorded");
    assert_eq!(drop.draggable, DraggableId(1));
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn disabled_draggable_ignores_pointer_down() {
    let (m, ctx) = mount(|slot| {
        let ctx = provide_dnd_context();
        slot.set(Some(ctx));
        let (locked, _set) = signal(true);
        view! { <LockedRow id=1 locked /> }
    });
    let item = find(&m.root, "item-1");
    pointer(&item, "pointerdown", 10.0, 10.0);
    assert_eq!(ctx.state.get_untracked(), DragState::Idle);
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn keyboard_space_picks_up_and_drops() {
    let (m, ctx) = mount(|slot| {
        let ctx = provide_dnd_context();
        slot.set(Some(ctx));
        view! { <TwoRows /> }
    });
    let item = find(&m.root, "item-1");

    key(&item, "keydown", " ");
    assert!(
        matches!(ctx.state.get_untracked(), DragState::Dragging { id, .. } if id == DraggableId(1))
    );

    key(&item, "keydown", " ");
    assert!(
        matches!(ctx.state.get_untracked(), DragState::Dropping { id } if id == DraggableId(1))
    );
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn keyboard_escape_cancels_an_active_drag() {
    let (m, ctx) = mount(|slot| {
        let ctx = provide_dnd_context();
        slot.set(Some(ctx));
        view! { <TwoRows /> }
    });
    let item = find(&m.root, "item-1");
    key(&item, "keydown", " ");
    assert!(matches!(ctx.state.get_untracked(), DragState::Dragging { .. }));
    key(&item, "keydown", "Escape");
    assert_eq!(ctx.state.get_untracked(), DragState::Idle);
}

// This test was `#[ignore]`d for a long time: the `Effect::new` that
// `use_droppable` schedules to measure its rect never ran, so
// `keyboard_step(Down)` found no neighbour. Root cause: the test build
// compiled leptos without the `csr` feature, which leaves
// reactive_graph's `effects` feature off — `Effect::new` is inert in
// that configuration (server behavior). Fixed by enabling `csr` on the
// leptos dev-dependency.
#[wasm_bindgen_test::wasm_bindgen_test]
async fn keyboard_arrow_steps_over_to_the_neighbor() {
    let (m, ctx) = mount(|slot| {
        let ctx = provide_dnd_context();
        slot.set(Some(ctx));
        view! { <TwoRows /> }
    });
    tick().await;
    let item = find(&m.root, "item-1");
    key(&item, "keydown", " "); // pick up
    assert_eq!(ctx.over.get_untracked(), Some(DroppableId(1)));
    key(&item, "keydown", "ArrowDown");
    assert_eq!(ctx.over.get_untracked(), Some(DroppableId(2)));
}

// Was `#[ignore]`d for the same missing-`csr`-feature issue as
// `keyboard_arrow_steps_over_to_the_neighbor` above; see the comment
// there.
#[wasm_bindgen_test::wasm_bindgen_test]
async fn announcement_formatter_receives_lifecycle_events() {
    // Capture every event the formatter sees via a shared `Mutex<Vec<_>>`.
    // The formatter runs *synchronously* inside `announce_event`, so the
    // sequence below is captured deterministically once the registry-prepare
    // `tick().await` has fired.
    let seen: Arc<Mutex<Vec<AnnounceEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let seen_for_fmt = seen.clone();
    let (m, _ctx) = mount(move |slot| {
        let ctx = provide_dnd_context();
        slot.set(Some(ctx));
        ctx.set_announcement_formatter(move |ev| {
            seen_for_fmt.lock().unwrap().push(*ev);
            taino_dnd_core::default_announcement(ev)
        });
        view! { <TwoRows /> }
    });
    tick().await; // let use_droppable's effects flush so ArrowDown can find a neighbour
    let item = find(&m.root, "item-1");

    key(&item, "keydown", " "); // pick up
    key(&item, "keydown", "ArrowDown"); // move over neighbor
    key(&item, "keydown", "Escape"); // cancel

    let events = seen.lock().unwrap().clone();
    assert!(
        matches!(events.first(), Some(AnnounceEvent::PickedUp { draggable }) if *draggable == DraggableId(1)),
        "first event should be PickedUp, got {:?}",
        events.first()
    );
    assert!(
        events.iter().any(
            |e| matches!(e, AnnounceEvent::MovedOver { over: Some(o), .. } if *o == DroppableId(2))
        ),
        "should have a MovedOver event over droppable 2, got {events:?}"
    );
    assert!(
        matches!(events.last(), Some(AnnounceEvent::Cancelled { draggable }) if *draggable == DraggableId(1)),
        "last event should be Cancelled, got {:?}",
        events.last()
    );
}
