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
        use_signal(|| (1..=6_u64).map(|id| (id, format!("Item #{id}"))).collect::<Vec<_>>());

    // React to a successful drop: move `draggable` to the slot occupied
    // by `over`, then clear `last_drop` so the effect doesn't re-fire.
    use_effect(move || {
        if let Some(drop) = *ctx.last_drop.read() {
            if let Some(target) = drop.over {
                if drop.draggable.0 != target.0 {
                    items.with_mut(|v| reorder(v, drop.draggable.0, target.0));
                }
            }
            ctx.clear_last_drop();
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

/// Render the visual preview inside the [`DragOverlay`].
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
fn reorder(v: &mut Vec<(u64, String)>, from: u64, to: u64) {
    let Some(from_idx) = v.iter().position(|(i, _)| *i == from) else {
        return;
    };
    let item = v.remove(from_idx);
    let Some(to_idx) = v.iter().position(|(i, _)| *i == to) else {
        v.insert(from_idx.min(v.len()), item);
        return;
    };
    v.insert(to_idx, item);
}
