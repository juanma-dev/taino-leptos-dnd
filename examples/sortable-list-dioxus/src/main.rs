//! Sortable-list demo for `taino-dnd-dioxus`.
//!
//! Each list row is a droppable. Inside each row, a draggable handle
//! covers the row's content. Dragging item A onto row B inserts A at
//! B's slot. Demonstrates the full Stage-3 binding end-to-end: pointer
//! events, keyboard sensor, ARIA announcer, live drop-preview, drag
//! overlay, and viewport auto-scroll.
//!
//! Keyboard: Tab onto a row, Space/Enter to pick up, arrows to move,
//! Space/Enter to drop, Esc to cancel.

#![allow(non_snake_case)]

use dioxus::prelude::*;
use taino_dnd_dioxus::{
    provide_dnd_context, use_dnd_context, use_draggable, use_droppable, DndAnnouncer, DragOverlay,
    DragState, DraggableId, DroppableId,
};

fn main() {
    console_error_panic_hook::set_once();
    launch(App);
}

fn App() -> Element {
    let ctx = provide_dnd_context();

    let mut items =
        use_signal(|| (1..=20_u64).map(|id| (id, format!("Item #{id}"))).collect::<Vec<_>>());

    // React to a successful drop: move `draggable` to the slot occupied
    // by `over`. `take_last_drop` reads the value, subscribes to future
    // changes, and clears the signal — done safely across two statements
    // so Dioxus's `read()` guard is dropped before the `set()` runs.
    use_effect(move || {
        if let Some(drop) = ctx.take_last_drop() {
            if let Some(target) = drop.over {
                if drop.draggable.0 != target.0 {
                    items.with_mut(|v| reorder(v, drop.draggable.0, target.0));
                }
            }
        }
    });

    rsx! {
        DndAnnouncer {}
        h1 { "taino-dnd-dioxus — sortable list" }
        p { class: "hint",
            "Drag items to reorder. Mouse, touch, and keyboard all work — focus a row with \
             Tab, then Space to pick up, arrows to move, Space again to drop, Esc to cancel."
        }
        div { class: "list",
            for (id, label) in items.read().iter() {
                Row { key: "{id}", id: *id, label: label.clone() }
            }
        }
        DragOverlay {
            {render_overlay(items)}
        }
        footer { "Stage 3 demo · v0.0.1" }
    }
}

#[component]
fn Row(id: u64, label: String) -> Element {
    let d = use_draggable(DraggableId(id));
    let z = use_droppable(DroppableId(id));

    let row_class =
        use_memo(move || if *z.is_over.read() { "row over".to_owned() } else { "row".to_owned() });
    let item_class = use_memo(move || {
        if *d.is_dragging.read() {
            "item dragging".to_owned()
        } else {
            "item".to_owned()
        }
    });

    let label_for_aria = label.clone();
    rsx! {
        div {
            class: "{row_class}",
            onmounted: move |e| z.on_mounted(e),
            style: "{z.drop_preview_style()}",
            div {
                class: "{item_class}",
                onmounted: move |e| d.on_mounted(e),
                onpointerdown: move |e| d.on_pointer_down(e),
                onpointermove: move |e| d.on_pointer_move(e),
                onpointerup: move |e| d.on_pointer_up(e),
                onpointercancel: move |e| d.on_pointer_cancel(e),
                onkeydown: move |e| d.on_key_down(e),
                tabindex: "0",
                role: "button",
                "aria-roledescription": "draggable item",
                "aria-label": "{label_for_aria}",
                style: "{d.style_pinned()}",
                "{label}"
            }
        }
    }
}

/// Render the visual preview inside the `DragOverlay`.
fn render_overlay(items: Signal<Vec<(u64, String)>>) -> Element {
    let ctx = use_dnd_context();
    let DragState::Dragging { id, .. } = *ctx.state.read() else {
        return rsx! {};
    };
    let label = items.read().iter().find(|(i, _)| *i == id.0).map(|(_, l)| l.clone());
    let Some(label) = label else {
        return rsx! {};
    };
    rsx! {
        div { class: "overlay-card", "{label}" }
    }
}

/// Move the item with id `from` so that it occupies the slot currently
/// held by `to`. Other items shift accordingly.
///
/// Both indices are looked up **before** the removal — looking up `to`
/// after the remove returns the wrong slot for forward moves because the
/// target has already shifted down by one.
fn reorder(v: &mut Vec<(u64, String)>, from: u64, to: u64) {
    let Some(from_idx) = v.iter().position(|(i, _)| *i == from) else {
        return;
    };
    let Some(to_idx) = v.iter().position(|(i, _)| *i == to) else {
        return;
    };
    if from_idx == to_idx {
        return;
    }
    let item = v.remove(from_idx);
    // See the reorder() in examples/sortable-list/src/main.rs for the
    // index-math derivation. tl;dr: original `to_idx` is the right
    // insert position in both directions.
    v.insert(to_idx, item);
}
