//! Sortable-list demo for taino-leptos-dnd.
//!
//! Each list row is a droppable. Inside each row, a draggable handle covers
//! the row's content. Dragging item A onto row B inserts A at B's slot.
//!
//! Keyboard: focus a row (Tab), press Space or Enter to pick up, arrow keys
//! to move, Space/Enter to drop, Escape to cancel.

use leptos::prelude::*;
use taino_dnd_core::{DraggableId, DroppableId};
use taino_dnd_leptos::{
    provide_dnd_context, use_dnd_context, use_draggable_with, use_droppable, DndAnnouncer,
    DragOverlay,
};

#[derive(Clone)]
struct Item {
    id: u64,
    label: String,
}

#[component]
fn App() -> impl IntoView {
    let ctx = provide_dnd_context();

    let items = RwSignal::new(
        (1..=20_u64).map(|id| Item { id, label: format!("Item #{id}") }).collect::<Vec<_>>(),
    );

    Effect::new(move |_| {
        if let Some(drop) = ctx.take_last_drop() {
            if let Some(over) = drop.over {
                if drop.draggable.0 != over.0 {
                    items.update(|v| reorder(v, drop.draggable.0, over.0));
                }
            }
        }
    });

    let items_for_overlay = items;

    view! {
        <DndAnnouncer/>
        <h1>"taino-leptos-dnd — sortable list"</h1>
        <p class="hint">
            "Drag items to reorder. Mouse, touch, and keyboard all work — focus a row with Tab, \
             then Space to pick up, arrows to move, Space again to drop, Esc to cancel."
        </p>
        <div class="list">
            <For
                each=move || items.get()
                key=|item| item.id
                children=move |item| view! { <Row item /> }
            />
        </div>
        <DragOverlay>
            {move || {
                let ctx = use_dnd_context();
                ctx.state.get().dragged_id().and_then(|id| {
                    items_for_overlay
                        .with(|v| v.iter().find(|i| i.id == id.0).map(|i| i.label.clone()))
                        .map(|label| view! { <div class="overlay-card">{label}</div> })
                })
            }}
        </DragOverlay>
        <footer>"Stage 2 demo · v0.0.1"</footer>
    }
}

#[component]
fn Row(item: Item) -> impl IntoView {
    let id = item.id;
    // Item #1 is locked (disabled) to demonstrate conditional dragging.
    let locked = id == 1;
    let d = use_draggable_with(DraggableId(id), Signal::derive(move || locked));
    let z = use_droppable(DroppableId(id));
    let label = item.label.clone();

    view! {
        <div
            class="row"
            node_ref=z.node_ref
            class:over=move || z.is_over.get()
            style=move || z.drop_preview_style()
        >
            <div
                class="item"
                class:dragging=move || d.is_dragging.get()
                class:locked=move || d.disabled.get()
                node_ref=d.node_ref
                tabindex="0"
                role="button"
                aria-roledescription="draggable item"
                aria-disabled=move || d.disabled.get().to_string()
                aria-label=label
                on:pointerdown=move |e| d.on_pointer_down(&e)
                on:pointermove=move |e| d.on_pointer_move(&e)
                on:pointerup=move |e| d.on_pointer_up(&e)
                on:pointercancel=move |e| d.on_pointer_cancel(&e)
                on:keydown=move |e| d.on_key_down(&e)
                style=move || d.style_pinned()
            >
                {item.label}
            </div>
        </div>
    }
}

/// Move the item with id `from` so that it occupies the slot currently
/// held by `to`. Other items shift accordingly.
///
/// Both indices are looked up **before** the removal — looking up `to`
/// after the remove returns the wrong slot for forward moves because the
/// target has already shifted down by one.
fn reorder(v: &mut Vec<Item>, from: u64, to: u64) {
    let Some(from_idx) = v.iter().position(|i| i.id == from) else {
        return;
    };
    let Some(to_idx) = v.iter().position(|i| i.id == to) else {
        return;
    };
    if from_idx == to_idx {
        return;
    }
    let item = v.remove(from_idx);
    // For forward moves (from_idx < to_idx): after `remove`, the target
    // sits at `to_idx - 1` in the new vec. Inserting at the *original*
    // to_idx places `item` just past the target — matching the visual
    // preview where the target shifted up to make room.
    // For backward moves (from_idx > to_idx): the target wasn't shifted
    // by the remove, so inserting at to_idx places `item` just before
    // the target.
    v.insert(to_idx, item);
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
