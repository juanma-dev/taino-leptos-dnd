//! Multi-zone demo for taino-leptos-dnd.
//!
//! Exercises the binding across challenging configurations in a single
//! [`DndContext`](taino_dnd_leptos::provide_dnd_context):
//!
//! * **Zone A** and **Zone B** are vertical lists stacked so that Zone B
//!   sits below the viewport fold. Crossing from A into B (or vice
//!   versa) drives the viewport auto-scroll loop.
//! * **Bar C** and **Bar D** are narrow horizontal task bars side by
//!   side, each with three small cards, driving the X-axis path for
//!   spatial keyboard navigation and drop targets.
//!
//! Cards carry unique integer ids across all zones. Each zone has a
//! "tail" droppable (id `TAIL_BASE + zone_idx`) used as a "drop at
//! end" target so empty zones remain reachable.
//!
//! Live drop-preview displacement is intentionally **not** applied to
//! card slots in this demo: the global single-axis detection in
//! [`taino_dnd_core::live_displacements`] can't disambiguate the mixed
//! vertical / horizontal layout. The [`DragOverlay`] following the
//! cursor plus `class:over` hover state is enough feedback to make the
//! drop intent visible.

use leptos::prelude::*;
use taino_dnd_core::{DraggableId, DroppableId};
use taino_dnd_leptos::{
    provide_dnd_context, use_dnd_context, use_draggable, use_droppable, DndAnnouncer, DragOverlay,
};

const TAIL_BASE: u64 = 10_000;

const fn zone_tail_id(idx: usize) -> DroppableId {
    DroppableId(TAIL_BASE + idx as u64)
}

fn zone_idx_from_tail(id: DroppableId) -> Option<usize> {
    id.0.checked_sub(TAIL_BASE).and_then(|i| usize::try_from(i).ok())
}

#[derive(Clone)]
struct Card {
    id: u64,
    label: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ZoneLayout {
    Vertical,
    Horizontal,
}

#[derive(Clone)]
struct Zone {
    name: &'static str,
    layout: ZoneLayout,
    cards: Vec<Card>,
}

fn card(id: u64, label: &str) -> Card {
    Card { id, label: label.to_owned() }
}

#[component]
fn App() -> impl IntoView {
    let ctx = provide_dnd_context();

    let zones = RwSignal::new(vec![
        Zone {
            name: "Zone A",
            layout: ZoneLayout::Vertical,
            cards: vec![
                card(1, "A · ship feature"),
                card(2, "A · write tests"),
                card(3, "A · review PR"),
                card(4, "A · update docs"),
            ],
        },
        Zone {
            name: "Zone B",
            layout: ZoneLayout::Vertical,
            cards: vec![
                card(5, "B · plan sprint"),
                card(6, "B · oncall handoff"),
                card(7, "B · refactor sensor"),
                card(8, "B · debug FLIP"),
            ],
        },
        Zone {
            name: "Bar C",
            layout: ZoneLayout::Horizontal,
            cards: vec![card(9, "C1"), card(10, "C2"), card(11, "C3")],
        },
        Zone {
            name: "Bar D",
            layout: ZoneLayout::Horizontal,
            cards: vec![card(12, "D1"), card(13, "D2"), card(14, "D3")],
        },
    ]);

    Effect::new(move |_| {
        if let Some(drop) = ctx.take_last_drop() {
            if let Some(over) = drop.over {
                if drop.draggable.0 != over.0 {
                    zones.update(|zs| move_card(zs, drop.draggable.0, over));
                }
            }
        }
    });

    view! {
        <DndAnnouncer/>
        <h1>"taino-leptos-dnd — multi-zone"</h1>
        <p class="hint">
            "Two vertical lists stacked (the second sits below the fold — drag a card down to \
             trigger viewport auto-scroll), plus two narrow horizontal task bars below. Cards \
             can be moved within a zone or to any other zone. Mouse, touch, and keyboard all \
             work; arrows step between neighbors, Space or Enter drops, Esc cancels."
        </p>
        <div class="layout">
            <ZoneView idx=0 zones=zones />
            <ZoneView idx=1 zones=zones />
            <div class="hzone-row">
                <ZoneView idx=2 zones=zones />
                <ZoneView idx=3 zones=zones />
            </div>
        </div>
        <DragOverlay>
            {move || {
                let ctx = use_dnd_context();
                ctx.state.get().dragged_id().and_then(|id| {
                    zones
                        .with(|zs| {
                            zs.iter()
                                .flat_map(|z| z.cards.iter())
                                .find(|c| c.id == id.0)
                                .map(|c| c.label.clone())
                        })
                        .map(|label| view! { <div class="overlay-card">{label}</div> })
                })
            }}
        </DragOverlay>
        <footer>"Stage 3 demo · v0.0.1"</footer>
    }
}

#[component]
fn ZoneView(idx: usize, zones: RwSignal<Vec<Zone>>) -> impl IntoView {
    let tail = use_droppable(zone_tail_id(idx));
    let name = Signal::derive(move || zones.with(|zs| zs.get(idx).map_or("", |z| z.name)));
    let layout = Signal::derive(move || {
        zones.with(|zs| zs.get(idx).map_or(ZoneLayout::Vertical, |z| z.layout))
    });
    let cards = Signal::derive(move || {
        zones.with(|zs| zs.get(idx).map(|z| z.cards.clone()).unwrap_or_default())
    });
    let is_empty = Signal::derive(move || cards.with(Vec::is_empty));

    let section_class = move || match layout.get() {
        ZoneLayout::Vertical => "zone vzone",
        ZoneLayout::Horizontal => "zone hzone",
    };
    let cards_class = move || match layout.get() {
        ZoneLayout::Vertical => "cards vcards",
        ZoneLayout::Horizontal => "cards hcards",
    };
    let tail_label = move || match (is_empty.get(), layout.get()) {
        (true, _) => "Drop a card here",
        (false, ZoneLayout::Vertical) => "Drop at end",
        (false, ZoneLayout::Horizontal) => "Drop at right",
    };

    view! {
        <section class=section_class aria-label=move || name.get()>
            <header><h2>{move || name.get()}</h2></header>
            <div class=cards_class>
                <For
                    each=move || cards.get()
                    key=|c| c.id
                    children=move |c| view! { <CardView card=c /> }
                />
                <div
                    class="zone-tail"
                    node_ref=tail.node_ref
                    class:over=move || tail.is_over.get()
                    class:empty=move || is_empty.get()
                    aria-hidden="true"
                >
                    <span>{tail_label}</span>
                </div>
            </div>
        </section>
    }
}

#[component]
fn CardView(card: Card) -> impl IntoView {
    let id = card.id;
    let d = use_draggable(DraggableId(id));
    let z = use_droppable(DroppableId(id));
    let label = card.label.clone();

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
                {card.label}
            </div>
        </div>
    }
}

/// Move the card identified by `from` to the slot described by `to`.
///
/// `to` may be either a card-slot droppable (insert *before* that card,
/// possibly in another zone) or a zone-tail droppable (append at the end
/// of that zone). Source and destination zones can differ.
fn move_card(zones: &mut [Zone], from: u64, to: DroppableId) {
    let Some((src_zone, src_idx)) = locate(zones, from) else { return };

    // Resolve the destination **before** removing the source. Locating
    // the target after the remove returns the wrong slot for same-zone
    // forward moves (the target has already shifted by one).
    if let Some(dest_zone) = zone_idx_from_tail(to) {
        if dest_zone >= zones.len() {
            return;
        }
        let card = zones[src_zone].cards.remove(src_idx);
        zones[dest_zone].cards.push(card);
        return;
    }

    let Some((dest_zone, dest_idx)) = locate(zones, to.0) else { return };
    if (src_zone, src_idx) == (dest_zone, dest_idx) {
        return;
    }

    let card = zones[src_zone].cards.remove(src_idx);
    zones[dest_zone].cards.insert(dest_idx, card);
}

fn locate(zones: &[Zone], id: u64) -> Option<(usize, usize)> {
    zones
        .iter()
        .enumerate()
        .find_map(|(zi, z)| z.cards.iter().position(|c| c.id == id).map(|i| (zi, i)))
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
