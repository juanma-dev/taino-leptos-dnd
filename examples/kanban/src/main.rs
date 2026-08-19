//! Kanban-board demo for taino-leptos-dnd.
//!
//! Three columns, each with a vertical list of cards. Drag cards within a
//! column to reorder, drag across columns to move. Pointer, touch, and
//! keyboard all work — focus a card with Tab, Space/Enter to pick up, arrow
//! keys to move (Up/Down within a column, Left/Right across columns and onto
//! a column's "drop at end" zone), Space/Enter to drop, Escape to cancel.
//!
//! ID scheme: each card's draggable and card-slot droppable share its
//! integer id. Each column's "tail" droppable (the strip below the last
//! card, used to drop a card at the end of the column) uses
//! [`COLUMN_TAIL_BASE`] `+ col_idx`. The base is chosen well above any card id
//! used in this demo so the two ranges can be distinguished cheaply.

use leptos::prelude::*;
use taino_dnd_core::AnnounceEvent;
use taino_dnd_leptos::{
    provide_dnd_context, use_dnd_context, use_draggable, use_droppable, use_flip, DndAnnouncer,
    DragOverlay,
};

const COLUMN_TAIL_BASE: u64 = 10_000;

const fn column_tail_id(idx: usize) -> u64 {
    COLUMN_TAIL_BASE + idx as u64
}

fn column_idx_from_tail(id: u64) -> Option<usize> {
    id.checked_sub(COLUMN_TAIL_BASE).and_then(|i| usize::try_from(i).ok())
}

#[derive(Clone)]
struct Card {
    id: u64,
    title: String,
}

#[derive(Clone)]
struct Column {
    title: &'static str,
    cards: Vec<Card>,
}

#[component]
fn App() -> impl IntoView {
    let ctx = provide_dnd_context();

    let columns = RwSignal::new(vec![
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
        Column { title: "Done", cards: vec![card(6, "Stage 1 MVP"), card(7, "Sortable example")] },
    ]);

    // Speak human-readable labels instead of the default numeric ids. This is
    // the `announcement_formatter` follow-up from docs/ROADMAP.md: a screen
    // reader now hears "Write kanban example moved to the end of Done" rather
    // than "Item 1 moved over target 10002".
    ctx.set_announcement_formatter(move |ev| announce(columns, ev));

    Effect::new(move |_| {
        if let Some(drop) = ctx.take_last_drop() {
            if let Some(over) = drop.over {
                if drop.draggable.0 != over.0 {
                    columns.update(|cols| move_card(cols, drop.draggable.0, over.0));
                }
            }
        }
    });

    let col_count = Signal::derive(move || columns.with(Vec::len));

    view! {
        <DndAnnouncer<u64> />
        <h1>"taino-leptos-dnd — kanban"</h1>
        <p class="hint">
            "Drag cards across columns to move, or within a column to reorder. \
             Pointer, touch, and keyboard all work — focus a card with Tab, Space \
             or Enter to pick up, arrows to move (Left/Right between columns), \
             Space or Enter to drop, Esc to cancel."
        </p>
        <div class="board">
            <For
                each=move || { let n = col_count.get(); (0..n).collect::<Vec<_>>() }
                key=|idx| *idx
                children=move |idx| view! { <ColumnView idx=idx columns=columns /> }
            />
        </div>
        <DragOverlay<u64>>
            {move || {
                let ctx = use_dnd_context::<u64>();
                ctx.state.get().dragged_id().and_then(|id| {
                    columns
                        .with(|cols| {
                            cols.iter()
                                .flat_map(|c| c.cards.iter())
                                .find(|c| c.id == id.0)
                                .map(|c| c.title.clone())
                        })
                        .map(|title| view! { <div class="overlay-card">{title}</div> })
                })
            }}
        </DragOverlay<u64>>
        <footer>"Stage 2 demo · v0.0.1"</footer>
    }
}

#[component]
fn ColumnView(idx: usize, columns: RwSignal<Vec<Column>>) -> impl IntoView {
    let tail = use_droppable(column_tail_id(idx));
    let title = Signal::derive(move || columns.with(|cs| cs.get(idx).map_or("", |c| c.title)));
    let cards = Signal::derive(move || {
        columns.with(|cs| cs.get(idx).map(|c| c.cards.clone()).unwrap_or_default())
    });
    let is_empty = Signal::derive(move || cards.with(Vec::is_empty));

    view! {
        <section class="column" aria-label=move || title.get()>
            <header><h2>{move || title.get()}</h2></header>
            <div class="cards">
                <For
                    each=move || cards.get()
                    key=|c| c.id
                    children=move |c| view! { <CardView card=c /> }
                />
                <div
                    class="column-tail"
                    node_ref=tail.node_ref
                    class:over=move || tail.is_over.get()
                    class:empty=move || is_empty.get()
                    aria-hidden="true"
                >
                    <span>
                        {move || if is_empty.get() { "Drop a card here" } else { "Drop at end" }}
                    </span>
                </div>
            </div>
        </section>
    }
}

#[component]
fn CardView(card: Card) -> impl IntoView {
    let id = card.id;
    let d = use_draggable(id);
    let z = use_droppable(id);
    use_flip::<u64>(z.node_ref);
    let label = card.title.clone();

    view! {
        <div class="card-slot" node_ref=z.node_ref class:over=move || z.is_over.get()>
            <div
                class="card"
                class:dragging=move || d.is_dragging.get()
                node_ref=d.node_ref
                tabindex="0"
                role="button"
                aria-roledescription="draggable card"
                aria-label=label
                on:pointerdown=move |e| d.on_pointer_down(&e)
                on:pointermove=move |e| d.on_pointer_move(&e)
                on:pointerup=move |e| d.on_pointer_up(&e)
                on:pointercancel=move |e| d.on_pointer_cancel(&e)
                on:keydown=move |e| d.on_key_down(&e)
                style=move || d.style_pinned()
            >
                {card.title}
            </div>
        </div>
    }
}

/// Move the card identified by `from` to the slot described by `to`.
///
/// `to` may be either a card-slot droppable (insert *before* that card,
/// possibly in another column) or a column-tail droppable (append at the end
/// of that column). Source and destination columns can differ.
fn move_card(cols: &mut [Column], from: u64, to: u64) {
    let Some((src_col, src_idx)) = locate(cols, from) else { return };

    // Resolve the destination **before** removing the source. Locating
    // the target after the remove returns the wrong slot for same-column
    // forward moves (the target has already shifted down by one). It
    // also lets us skip the put-back fallbacks entirely — if we can't
    // find a destination, we never remove in the first place.
    if let Some(dest_col) = column_idx_from_tail(to) {
        if dest_col >= cols.len() {
            return; // bogus tail id — leave state untouched
        }
        let card = cols[src_col].cards.remove(src_idx);
        cols[dest_col].cards.push(card);
        return;
    }

    let Some((dest_col, dest_idx)) = locate(cols, to) else { return };
    if (src_col, src_idx) == (dest_col, dest_idx) {
        return; // dropped on self — no-op
    }

    let card = cols[src_col].cards.remove(src_idx);
    // `dest_idx` is the original target position. For same-column
    // forward moves the target now sits at `dest_idx - 1` after the
    // remove; inserting at the original `dest_idx` puts `card` just
    // past it. For backward / cross-column moves the index is
    // unchanged. Both match the visual drop-preview.
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

/// Human-readable screen-reader text for a drag-lifecycle event, resolving
/// numeric ids to card titles and column names.
fn announce(columns: RwSignal<Vec<Column>>, ev: &AnnounceEvent<u64>) -> String {
    let card_title = |id: u64| {
        columns.with_untracked(|cols| {
            cols.iter().flat_map(|c| c.cards.iter()).find(|c| c.id == id).map(|c| c.title.clone())
        })
    };
    let title = |id: u64| card_title(id).unwrap_or_else(|| format!("item {id}"));
    let target = |over: Option<u64>| -> String {
        let Some(o) = over else { return "no column".to_owned() };
        if let Some(col) = column_idx_from_tail(o) {
            return columns.with_untracked(|cols| {
                cols.get(col)
                    .map_or_else(|| "a column".to_owned(), |c| format!("the end of {}", c.title))
            });
        }
        card_title(o).map_or_else(|| "a card".to_owned(), |t| format!("before \"{t}\""))
    };
    match *ev {
        AnnounceEvent::PickedUp { draggable } => format!("Picked up \"{}\".", title(draggable.0)),
        AnnounceEvent::MovedOver { draggable, over } => {
            format!("\"{}\" moved to {}.", title(draggable.0), target(over.map(|o| o.0)))
        }
        AnnounceEvent::Dropped { draggable, over } => {
            format!("Dropped \"{}\" at {}.", title(draggable.0), target(over.map(|o| o.0)))
        }
        AnnounceEvent::Cancelled { draggable } => {
            format!("Cancelled dragging \"{}\".", title(draggable.0))
        }
    }
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
