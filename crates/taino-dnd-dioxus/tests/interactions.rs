//! Browser-only synthetic-interaction tests for the Dioxus binding.
//!
//! Mounts the app into a dedicated `<div>` per test (via `dioxus_web`'s
//! `Config::rootname`), dispatches real `PointerEvent` / `KeyboardEvent`,
//! yields to Dioxus's render loop, and asserts the public signals respond.
//! Mirror of `taino-dnd-leptos::tests::interactions`.
//!
//! Requires a real DOM:
//!
//! ```sh
//! wasm-pack test --chrome --headless -p taino-dnd-dioxus
//! ```
//!
//! The `dioxus_web::launch_cfg` call is fire-and-forget — it spawns the
//! render loop into the wasm event queue and returns. Tests `.await` a
//! short `TimeoutFuture(...)` to let the queued work run before the
//! event dispatch and again before the assertion.

#![cfg(target_arch = "wasm32")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    // wasm is single-threaded; `Send` bounds are noise for browser tests.
    clippy::future_not_send,
    non_snake_case,
)]

use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use dioxus::prelude::*;
use taino_dnd_core::{AnnounceEvent, DragState, DraggableId, DroppableId};
use taino_dnd_dioxus::{
    provide_dnd_context, use_draggable, use_draggable_with, use_droppable, DndContext,
};
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, KeyboardEventInit, PointerEventInit};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

// ── Cross-launch wiring ────────────────────────────────────────────────────
//
// `dioxus_web::launch_cfg` takes a zero-arg `fn() -> Element`. We can't
// thread per-test state in through arguments, so we set it on a thread-local
// before each launch and the `app` fn reads it once, on first render, via
// `use_hook` (which freezes the value into the VirtualDom for that instance).

#[derive(Clone, Copy)]
enum AppKind {
    TwoRows,
    LockedRow,
    /// Two rows + a custom announcement formatter installed in `App` that
    /// records each event into `FORMATTER_LOG`.
    FormatterCapture,
}

thread_local! {
    static APP_KIND: RefCell<AppKind> = const { RefCell::new(AppKind::TwoRows) };
    static CTX_OUT: RefCell<Option<DndContext>> = const { RefCell::new(None) };
    /// Side-channel for the formatter test. `Arc<Mutex<...>>` because the
    /// formatter closure is stored inside Dioxus and must be `'static`.
    static FORMATTER_LOG: RefCell<Option<Arc<Mutex<Vec<AnnounceEvent>>>>> =
        const { RefCell::new(None) };
}

// ── Components ─────────────────────────────────────────────────────────────

fn app() -> Element {
    let ctx = provide_dnd_context();
    // Snapshot the test config into this VirtualDom on first render only.
    use_hook(|| CTX_OUT.with(|c| *c.borrow_mut() = Some(ctx)));
    let kind = use_hook(|| APP_KIND.with(|k| *k.borrow()));
    use_hook(|| {
        if matches!(kind, AppKind::FormatterCapture) {
            let log = FORMATTER_LOG
                .with(|l| l.borrow().clone())
                .expect("FORMATTER_LOG must be set before launch");
            ctx.set_announcement_formatter(move |ev| {
                log.lock().unwrap().push(*ev);
                taino_dnd_core::default_announcement(ev)
            });
        }
    });
    match kind {
        AppKind::TwoRows | AppKind::FormatterCapture => rsx! { TwoRows {} },
        AppKind::LockedRow => rsx! { LockedRow {} },
    }
}

#[component]
fn TwoRows() -> Element {
    rsx! {
        Row { id: 1 }
        Row { id: 2 }
    }
}

#[component]
fn Row(id: u64) -> Element {
    let d = use_draggable(DraggableId(id));
    let z = use_droppable(DroppableId(id));
    let row_handle = format!("row-{id}");
    let item_handle = format!("item-{id}");
    let top = (id - 1) * 50;
    rsx! {
        div {
            "data-handle": "{row_handle}",
            onmounted: move |e| z.on_mounted(e),
            style: "position: absolute; left: 0; top: {top}px; width: 100px; height: 50px;",
            div {
                "data-handle": "{item_handle}",
                onmounted: move |e| d.on_mounted(e),
                onpointerdown: move |e| d.on_pointer_down(e),
                onpointermove: move |e| d.on_pointer_move(e),
                onpointerup: move |e| d.on_pointer_up(e),
                onpointercancel: move |e| d.on_pointer_cancel(e),
                onkeydown: move |e| d.on_key_down(e),
                tabindex: "0",
                style: "width: 100%; height: 100%;",
                "Row {id}"
            }
        }
    }
}

#[component]
fn LockedRow() -> Element {
    let locked = use_signal(|| true);
    let d = use_draggable_with(DraggableId(1), locked);
    let z = use_droppable(DroppableId(1));
    rsx! {
        div {
            "data-handle": "row-1",
            onmounted: move |e| z.on_mounted(e),
            style: "position: absolute; left: 0; top: 0; width: 100px; height: 50px;",
            div {
                "data-handle": "item-1",
                onmounted: move |e| d.on_mounted(e),
                onpointerdown: move |e| d.on_pointer_down(e),
                onkeydown: move |e| d.on_key_down(e),
                tabindex: "0",
                style: "width: 100%; height: 100%;",
                "Locked"
            }
        }
    }
}

// ── Harness ────────────────────────────────────────────────────────────────

fn make_root(name: &str) -> HtmlElement {
    let doc = web_sys::window().unwrap().document().unwrap();
    let el = doc.create_element("div").unwrap().dyn_into::<HtmlElement>().unwrap();
    el.set_id(name);
    el.set_attribute("style", "position: absolute; top: 0; left: 0;").unwrap();
    doc.body().unwrap().append_child(&el).unwrap();
    el
}

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
    let ev = web_sys::PointerEvent::new_with_event_init_dict(kind, init.unchecked_ref()).unwrap();
    let _ = target.dispatch_event(&ev);
}

fn key(target: &web_sys::EventTarget, kind: &str, key: &str) {
    let init = KeyboardEventInit::new();
    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_key(key);
    let ev = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict(kind, init.unchecked_ref())
        .unwrap();
    let _ = target.dispatch_event(&ev);
}

fn find(root_name: &str, handle: &str) -> web_sys::Element {
    let doc = web_sys::window().unwrap().document().unwrap();
    let root = doc.get_element_by_id(root_name).unwrap();
    root.query_selector(&format!("[data-handle='{handle}']")).unwrap().unwrap()
}

/// Yield to the event loop long enough for Dioxus to flush mounted /
/// rendered work. A handful of milliseconds is plenty in headless Chrome.
async fn tick() {
    gloo_timers::future::TimeoutFuture::new(10).await;
}

/// Launch a fresh app rooted at `<div id="root_name">` and yield until the
/// component has rendered + `CTX_OUT` is populated. Returns the captured
/// `DndContext`.
async fn mount(root_name: &str, kind: AppKind) -> DndContext {
    APP_KIND.with(|k| *k.borrow_mut() = kind);
    CTX_OUT.with(|c| *c.borrow_mut() = None);
    make_root(root_name);
    let cfg = dioxus_web::Config::new().rootname(root_name.to_string());
    dioxus_web::launch::launch_cfg(app, cfg);
    tick().await;
    CTX_OUT.with(|c| c.borrow().expect("App must have rendered and set CTX_OUT"))
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[wasm_bindgen_test::wasm_bindgen_test]
async fn pointer_down_enters_pressed() {
    let ctx = mount("dx-down", AppKind::TwoRows).await;
    let item = find("dx-down", "item-1");
    pointer(&item, "pointerdown", 10.0, 10.0);
    tick().await;
    assert!(matches!(*ctx.state.peek(), DragState::Pressed { .. }));
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn pointer_move_past_threshold_promotes_to_dragging() {
    let ctx = mount("dx-thresh", AppKind::TwoRows).await;
    let item = find("dx-thresh", "item-1");
    pointer(&item, "pointerdown", 10.0, 10.0);
    pointer(&item, "pointermove", 80.0, 10.0);
    tick().await;
    assert!(matches!(*ctx.state.peek(), DragState::Dragging { .. }));
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn pointer_up_after_short_move_is_a_click_not_a_drop() {
    let ctx = mount("dx-click", AppKind::TwoRows).await;
    let item = find("dx-click", "item-1");
    pointer(&item, "pointerdown", 10.0, 10.0);
    pointer(&item, "pointermove", 12.0, 10.0);
    pointer(&item, "pointerup", 12.0, 10.0);
    tick().await;
    assert_eq!(*ctx.state.peek(), DragState::Idle);
    assert!(ctx.last_drop.peek().is_none());
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn pointer_up_after_real_drag_enters_dropping_and_records_last_drop() {
    let ctx = mount("dx-drop", AppKind::TwoRows).await;
    let item = find("dx-drop", "item-1");
    pointer(&item, "pointerdown", 10.0, 10.0);
    pointer(&item, "pointermove", 80.0, 10.0);
    pointer(&item, "pointerup", 80.0, 10.0);
    tick().await;
    assert!(matches!(*ctx.state.peek(), DragState::Dropping { .. }));
    // `DropResult` is no longer `Copy` (gained `additional: Vec<DraggableId>`
    // for multi-drag), so the peek guard must be cloned out.
    let drop = ctx.last_drop.peek().clone().expect("a drop was recorded");
    assert_eq!(drop.draggable, DraggableId(1));
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn disabled_draggable_ignores_pointer_down() {
    let ctx = mount("dx-locked", AppKind::LockedRow).await;
    let item = find("dx-locked", "item-1");
    pointer(&item, "pointerdown", 10.0, 10.0);
    tick().await;
    assert_eq!(*ctx.state.peek(), DragState::Idle);
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn keyboard_space_picks_up_and_drops() {
    let ctx = mount("dx-kbd", AppKind::TwoRows).await;
    let item = find("dx-kbd", "item-1");

    key(&item, "keydown", " ");
    tick().await;
    assert!(matches!(*ctx.state.peek(), DragState::Dragging { id, .. } if id == DraggableId(1)));

    key(&item, "keydown", " ");
    tick().await;
    assert!(matches!(*ctx.state.peek(), DragState::Dropping { id } if id == DraggableId(1)));
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn keyboard_escape_cancels_an_active_drag() {
    let ctx = mount("dx-esc", AppKind::TwoRows).await;
    let item = find("dx-esc", "item-1");
    key(&item, "keydown", " ");
    tick().await;
    assert!(matches!(*ctx.state.peek(), DragState::Dragging { .. }));
    key(&item, "keydown", "Escape");
    tick().await;
    assert_eq!(*ctx.state.peek(), DragState::Idle);
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn keyboard_arrow_steps_over_to_the_neighbor() {
    let ctx = mount("dx-arrow", AppKind::TwoRows).await;
    let item = find("dx-arrow", "item-1");
    key(&item, "keydown", " ");
    tick().await;
    assert_eq!(*ctx.over.peek(), Some(DroppableId(1)));
    key(&item, "keydown", "ArrowDown");
    tick().await;
    assert_eq!(*ctx.over.peek(), Some(DroppableId(2)));
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn announcement_formatter_receives_lifecycle_events() {
    let log: Arc<Mutex<Vec<AnnounceEvent>>> = Arc::new(Mutex::new(Vec::new()));
    FORMATTER_LOG.with(|l| *l.borrow_mut() = Some(log.clone()));
    let _ctx = mount("dx-fmt", AppKind::FormatterCapture).await;
    let item = find("dx-fmt", "item-1");

    key(&item, "keydown", " ");
    tick().await;
    key(&item, "keydown", "ArrowDown");
    tick().await;
    key(&item, "keydown", "Escape");
    tick().await;

    let events = log.lock().unwrap().clone();
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
