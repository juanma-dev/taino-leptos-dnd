//! Kanban-board demo for `taino-dnd-dioxus`.
//!
//! Three columns, each with a vertical list of cards. Drag cards within a
//! column to reorder, drag across columns to move. Pointer, touch, and
//! keyboard all work — focus a card with Tab, Space/Enter to pick up, arrow
//! keys to move (Up/Down within a column, Left/Right across columns and onto
//! a column's "drop at end" zone), Space/Enter to drop, Escape to cancel.
//!
//! This is the Dioxus twin of `examples/kanban` (Leptos). The component
//! structure and the `move_card` / `locate` reorder math are identical; only
//! the rendering layer differs. It also showcases the *other* reorder
//! animation: where the Dioxus sortable-list uses the live drop-preview
//! (neighbors shift mid-drag), this board uses [`use_flip`] for the
//! post-drop settle (cards glide to their new slots after release).
//!
//! ID scheme: each card's draggable and card-slot droppable share its
//! integer id. Each column's "tail" droppable (the strip below the last
//! card, used to drop a card at the end of the column) uses
//! [`COLUMN_TAIL_BASE`] `+ col_idx`. The base is chosen well above any card id
//! used in this demo so the two ranges can be distinguished cheaply.

#![allow(non_snake_case)]

use dioxus::prelude::*;
use taino_dnd_dioxus::{
    provide_dnd_context, use_dnd_context, use_draggable, use_droppable, use_flip, DndAnnouncer,
    DragOverlay, DragState, DraggableId, DroppableId,
};

const COLUMN_TAIL_BASE: u64 = 10_000;

const fn column_tail_id(idx: usize) -> DroppableId {
    DroppableId(COLUMN_TAIL_BASE + idx as u64)
}

fn column_idx_from_tail(id: DroppableId) -> Option<usize> {
    id.0.checked_sub(COLUMN_TAIL_BASE).and_then(|i| usize::try_from(i).ok())
}

#[derive(Clone, PartialEq, Eq)]
struct Card {
    id: u64,
    title: String,
}

#[derive(Clone)]
struct Column {
    title: &'static str,
    cards: Vec<Card>,
}

fn main() {
    console_error_panic_hook::set_once();
    launch(App);
}

fn App() -> Element {
    let ctx = provide_dnd_context();

    let mut columns = use_signal(|| {
        vec![
            Column {
                title: "To do",
                cards: vec![
                    card(1, "Write kanban example"),
                    card(2, "Smoke-test keyboard nav"),
                    card(3, "Polish ARIA messages"),
                ],
            },
            Column {
                title: "In progress",
                cards: vec![card(4, "Wire FLIP animations"), card(5, "Tune drag overlay")],
            },
            Column {
                title: "Done",
                cards: vec![card(6, "Stage 1 MVP"), card(7, "Sortable example")],
            },
        ]
    });

    // React to a successful drop: move `draggable` to the slot described by
    // `over`. `take_last_drop` reads, subscribes, and clears across two
    // statements so Dioxus's `read()` guard is dropped before the `set()`.
    use_effect(move || {
        if let Some(drop) = ctx.take_last_drop() {
            if let Some(over) = drop.over {
                if drop.draggable.0 != over.0 {
                    columns.with_mut(|cols| move_card(cols, drop.draggable.0, over));
                }
            }
        }
    });

    rsx! {
        DndAnnouncer {}
        h1 { "taino-dnd-dioxus — kanban" }
        p { class: "hint",
            "Drag cards across columns to move, or within a column to reorder. \
             Pointer, touch, and keyboard all work — focus a card with Tab, Space \
             or Enter to pick up, arrows to move (Left/Right between columns), \
             Space or Enter to drop, Esc to cancel."
        }
        div { class: "board",
            for idx in 0..columns.read().len() {
                ColumnView { key: "{idx}", idx, columns }
            }
        }
        DragOverlay {
            {render_overlay(columns)}
        }
        footer { "Stage 3 demo · v0.0.1" }
    }
}

#[component]
fn ColumnView(idx: usize, columns: Signal<Vec<Column>>) -> Element {
    let tail = use_droppable(column_tail_id(idx));

    let title = use_memo(move || columns.read().get(idx).map_or("", |c| c.title));
    let cards =
        use_memo(move || columns.read().get(idx).map(|c| c.cards.clone()).unwrap_or_default());
    let is_empty = use_memo(move || cards.read().is_empty());
    let tail_class = use_memo(move || {
        let mut s = String::from("column-tail");
        if *is_empty.read() {
            s.push_str(" empty");
        }
        if *tail.is_over.read() {
            s.push_str(" over");
        }
        s
    });

    rsx! {
        section { class: "column", "aria-label": "{title}",
            header {
                h2 { "{title}" }
            }
            div { class: "cards",
                for c in cards.read().iter() {
                    CardView { key: "{c.id}", card: c.clone() }
                }
                div {
                    class: "{tail_class}",
                    onmounted: move |e| tail.on_mounted(e),
                    "aria-hidden": "true",
                    span {
                        {if *is_empty.read() { "Drop a card here" } else { "Drop at end" }}
                    }
                }
            }
        }
    }
}

#[component]
fn CardView(card: Card) -> Element {
    let id = card.id;
    let d = use_draggable(DraggableId(id));
    let z = use_droppable(DroppableId(id));
    // FLIP post-drop settle. The card-slot has **no** reactive `style`
    // binding, so Dioxus never reconciles its `style` attribute and the
    // hook's direct transform mutations survive re-renders. (Contrast with
    // the sortable-list, where the wrapper uses `drop_preview_style`.)
    use_flip(z.element);

    let slot_class =
        use_memo(move || if *z.is_over.read() { "card-slot over" } else { "card-slot" });
    let card_class = use_memo(move || if *d.is_dragging.read() { "card dragging" } else { "card" });

    let title = card.title;
    rsx! {
        div {
            class: "{slot_class}",
            onmounted: move |e| z.on_mounted(e),
            div {
                class: "{card_class}",
                onmounted: move |e| d.on_mounted(e),
                onpointerdown: move |e| d.on_pointer_down(e),
                onpointermove: move |e| d.on_pointer_move(e),
                onpointerup: move |e| d.on_pointer_up(e),
                onpointercancel: move |e| d.on_pointer_cancel(e),
                onkeydown: move |e| d.on_key_down(e),
                tabindex: "0",
                role: "button",
                "aria-roledescription": "draggable card",
                "aria-label": "{title}",
                style: "{d.style_pinned()}",
                "{title}"
            }
        }
    }
}

/// Render the visual preview inside the `DragOverlay`.
fn render_overlay(columns: Signal<Vec<Column>>) -> Element {
    let ctx = use_dnd_context();
    let DragState::Dragging { id, .. } = *ctx.state.read() else {
        return rsx! {};
    };
    let title = columns
        .read()
        .iter()
        .flat_map(|c| c.cards.iter())
        .find(|c| c.id == id.0)
        .map(|c| c.title.clone());
    let Some(title) = title else {
        return rsx! {};
    };
    rsx! {
        div { class: "overlay-card", "{title}" }
    }
}

/// Move the card identified by `from` to the slot described by `to`.
///
/// `to` may be either a card-slot droppable (insert *before* that card,
/// possibly in another column) or a column-tail droppable (append at the end
/// of that column). Source and destination columns can differ.
fn move_card(cols: &mut [Column], from: u64, to: DroppableId) {
    let Some((src_col, src_idx)) = locate(cols, from) else { return };

    // Resolve the destination **before** removing the source. Locating the
    // target after the remove returns the wrong slot for same-column forward
    // moves (the target has already shifted down by one). It also lets us
    // skip the put-back fallbacks entirely — if we can't find a destination,
    // we never remove in the first place.
    if let Some(dest_col) = column_idx_from_tail(to) {
        if dest_col >= cols.len() {
            return; // bogus tail id — leave state untouched
        }
        let card = cols[src_col].cards.remove(src_idx);
        cols[dest_col].cards.push(card);
        return;
    }

    let Some((dest_col, dest_idx)) = locate(cols, to.0) else { return };
    if (src_col, src_idx) == (dest_col, dest_idx) {
        return; // dropped on self — no-op
    }

    let card = cols[src_col].cards.remove(src_idx);
    // `dest_idx` is the original target position. For same-column forward
    // moves the target now sits at `dest_idx - 1` after the remove; inserting
    // at the original `dest_idx` puts `card` just past it. For backward /
    // cross-column moves the index is unchanged. Both match the visual
    // drop-preview.
    cols[dest_col].cards.insert(dest_idx, card);
}

fn locate(cols: &[Column], id: u64) -> Option<(usize, usize)> {
    cols.iter()
        .enumerate()
        .find_map(|(ci, c)| c.cards.iter().position(|card| card.id == id).map(|idx| (ci, idx)))
}

fn card(id: u64, title: &str) -> Card {
    Card { id, title: title.to_owned() }
}
