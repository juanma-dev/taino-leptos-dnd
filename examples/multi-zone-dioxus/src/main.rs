//! Multi-zone demo for `taino-dnd-dioxus`.
//!
//! Same scope as the Leptos multi-zone demo:
//!
//! * **Zone A** and **Zone B** are vertical lists stacked so that Zone B
//!   sits below the viewport fold — crossing between them drives the
//!   viewport auto-scroll loop.
//! * **Bar C** and **Bar D** are narrow horizontal task bars side by
//!   side, each with three small cards, driving the X-axis path for
//!   spatial keyboard navigation and drop targets.
//!
//! All four zones share one [`DndContext`](taino_dnd_dioxus::provide_dnd_context).
//! Cards carry unique integer ids; each zone has a "tail" droppable so
//! empty zones (and "drop at end" intent) remain reachable.
//!
//! Live drop-preview displacement is computed **per zone** by
//! [`zone_displacements`]:
//!
//! * **Within-zone** (dragged and over both in this zone): defers to
//!   [`taino_dnd_core::live_displacements`] — neighbors part exactly
//!   like the `sortable-list-dioxus` demo, along the zone's axis.
//! * **Cross-zone destination** (over is in this zone, dragged is
//!   from elsewhere): items at and after `over` shift forward by
//!   `over`'s own size to open a landing slot. The user sees zone B
//!   part to make room while dragging a card from zone A.
//! * **Source-only or unrelated**: no shift here.

#![allow(non_snake_case)]

use std::collections::HashMap;

use dioxus::prelude::*;
use taino_dnd_core::{live_displacements, Axis, Rect, Vector};
use taino_dnd_dioxus::{
    provide_dnd_context, use_dnd_context, use_draggable, use_droppable, DndAnnouncer, DragOverlay,
    DragState, DraggableId, DroppableId,
};

const TAIL_BASE: u64 = 10_000;

const fn zone_tail_id(idx: usize) -> DroppableId {
    DroppableId(TAIL_BASE + idx as u64)
}

fn zone_idx_from_tail(id: DroppableId) -> Option<usize> {
    id.0.checked_sub(TAIL_BASE).and_then(|i| usize::try_from(i).ok())
}

#[derive(Clone, PartialEq, Eq)]
struct Card {
    id: u64,
    label: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ZoneLayout {
    Vertical,
    Horizontal,
}

#[derive(Clone, PartialEq, Eq)]
struct Zone {
    name: &'static str,
    layout: ZoneLayout,
    cards: Vec<Card>,
}

fn card(id: u64, label: &str) -> Card {
    Card { id, label: label.to_owned() }
}

/// Per-zone displacement that handles both within-zone reorders and
/// cross-zone destination shifts. See the module-level docs for the
/// behavior matrix.
fn zone_displacements(
    dragged: DroppableId,
    over: Option<DroppableId>,
    items: &[(DroppableId, Rect)],
    axis: Axis,
) -> Vec<(DroppableId, Vector)> {
    let dragged_idx = items.iter().position(|(id, _)| *id == dragged);
    let over_idx = over.and_then(|o| items.iter().position(|(id, _)| *id == o));

    match (dragged_idx, over_idx) {
        (Some(_), Some(_)) => live_displacements(dragged, over, items, axis),
        (None, Some(o)) => {
            let mut out: Vec<(DroppableId, Vector)> =
                items.iter().map(|(id, _)| (*id, Vector::default())).collect();
            // Use `over`'s own size for the shift step so the gap that
            // opens matches a "typical" card slot in this zone — see
            // the Leptos demo's doc for the full rationale.
            let step = match axis {
                Axis::X => items[o].1.width,
                Axis::Y => items[o].1.height,
            };
            let vec = match axis {
                Axis::X => Vector::new(step, 0.0),
                Axis::Y => Vector::new(0.0, step),
            };
            for slot in out.iter_mut().skip(o) {
                slot.1 = vec;
            }
            out
        }
        _ => items.iter().map(|(id, _)| (*id, Vector::default())).collect(),
    }
}

fn main() {
    console_error_panic_hook::set_once();
    launch(App);
}

fn App() -> Element {
    let ctx = provide_dnd_context();

    let mut zones = use_signal(|| {
        vec![
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
        ]
    });

    use_effect(move || {
        if let Some(drop) = ctx.take_last_drop() {
            if let Some(target) = drop.over {
                if drop.draggable.0 != target.0 {
                    zones.with_mut(|zs| move_card(zs, drop.draggable.0, target));
                }
            }
        }
    });

    rsx! {
        DndAnnouncer {}
        h1 { "taino-dnd-dioxus — multi-zone" }
        p { class: "hint",
            "Two vertical lists stacked (the second sits below the fold — drag a card down \
             to trigger viewport auto-scroll), plus two narrow horizontal task bars below. \
             Cards can be moved within a zone or to any other zone. Mouse, touch, and \
             keyboard all work; arrows step between neighbors, Space or Enter drops, Esc \
             cancels."
        }
        div { class: "layout",
            ZoneView { idx: 0, zones }
            ZoneView { idx: 1, zones }
            div { class: "hzone-row",
                ZoneView { idx: 2, zones }
                ZoneView { idx: 3, zones }
            }
        }
        DragOverlay { {render_overlay(zones)} }
        footer { "Stage 3 demo · v0.0.1" }
    }
}

#[component]
fn ZoneView(idx: usize, zones: Signal<Vec<Zone>>) -> Element {
    let ctx = use_dnd_context();
    let tail = use_droppable(zone_tail_id(idx));
    let cards =
        use_memo(move || zones.read().get(idx).map(|z| z.cards.clone()).unwrap_or_default());
    let layout = use_memo(move || zones.read().get(idx).map_or(ZoneLayout::Vertical, |z| z.layout));
    let name = use_memo(move || zones.read().get(idx).map_or("", |z| z.name).to_owned());
    let is_empty = use_memo(move || cards.read().is_empty());

    // Per-zone live drop-preview displacements. Subscribes to
    // `dragged_droppable` (deduped, drag start/end only) and `over`
    // (hover changes). Peeks droppable rects because displacements are
    // scroll-invariant — all rects shift by the same delta during
    // scroll, preserving relative order and step sizes.
    let displacements: Memo<HashMap<u64, (f64, f64)>> = use_memo(move || {
        let Some(dragged) = *ctx.dragged_droppable.read() else {
            return HashMap::new();
        };
        let over = *ctx.over.read();
        let zone_axis = match *layout.read() {
            ZoneLayout::Vertical => Axis::Y,
            ZoneLayout::Horizontal => Axis::X,
        };
        let card_ids: Vec<u64> = cards.read().iter().map(|c| c.id).collect();
        let mut items: Vec<(DroppableId, Rect)> = ctx.peek_droppables(|map| {
            card_ids
                .iter()
                .filter_map(|id| map.get(&DroppableId(*id)).map(|r| (DroppableId(*id), *r)))
                .collect()
        });
        items.sort_by(|a, b| {
            let (sa, sb) = match zone_axis {
                Axis::Y => (a.1.y, b.1.y),
                Axis::X => (a.1.x, b.1.x),
            };
            sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
        });
        zone_displacements(dragged, over, &items, zone_axis)
            .into_iter()
            .map(|(d, v)| (d.0, (v.x, v.y)))
            .collect()
    });

    let section_class = use_memo(move || match *layout.read() {
        ZoneLayout::Vertical => "zone vzone".to_owned(),
        ZoneLayout::Horizontal => "zone hzone".to_owned(),
    });
    let cards_class = use_memo(move || match *layout.read() {
        ZoneLayout::Vertical => "cards vcards".to_owned(),
        ZoneLayout::Horizontal => "cards hcards".to_owned(),
    });
    let tail_class = use_memo(move || {
        let mut c = String::from("zone-tail");
        if *tail.is_over.read() {
            c.push_str(" over");
        }
        if *is_empty.read() {
            c.push_str(" empty");
        }
        c
    });
    let tail_label = use_memo(move || match (*is_empty.read(), *layout.read()) {
        (true, _) => "Drop a card here",
        (false, ZoneLayout::Vertical) => "Drop at end",
        (false, ZoneLayout::Horizontal) => "Drop at right",
    });

    rsx! {
        section {
            class: "{section_class}",
            "aria-label": "{name}",
            header { h2 { "{name}" } }
            div { class: "{cards_class}",
                for c in cards.read().iter() {
                    CardView { key: "{c.id}", card: c.clone(), displacements }
                }
                div {
                    class: "{tail_class}",
                    onmounted: move |e| tail.on_mounted(e),
                    "aria-hidden": "true",
                    span { "{tail_label}" }
                }
            }
        }
    }
}

#[component]
fn CardView(card: Card, displacements: Memo<HashMap<u64, (f64, f64)>>) -> Element {
    let id = card.id;
    let d = use_draggable(DraggableId(id));
    let z = use_droppable(DroppableId(id));

    let slot_class = use_memo(move || {
        if *z.is_over.read() {
            "card-slot over".to_owned()
        } else {
            "card-slot".to_owned()
        }
    });
    let item_class = use_memo(move || {
        if *d.is_dragging.read() {
            "card dragging".to_owned()
        } else {
            "card".to_owned()
        }
    });
    let preview_style = use_memo(move || {
        let (dx, dy) = displacements.read().get(&id).copied().unwrap_or((0.0, 0.0));
        let z_idx = if dx.abs() > 0.001 || dy.abs() > 0.001 { "z-index: 1;" } else { "" };
        format!(
            "transform: translate({dx}px, {dy}px); \
             transition: transform 220ms cubic-bezier(0.2, 0, 0, 1); \
             {z_idx}"
        )
    });
    let label = card.label;
    let label_for_aria = label.clone();

    rsx! {
        div {
            class: "{slot_class}",
            onmounted: move |e| z.on_mounted(e),
            style: "{preview_style}",
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
                "aria-roledescription": "draggable card",
                "aria-label": "{label_for_aria}",
                style: "{d.style_pinned()}",
                "{label}"
            }
        }
    }
}

/// Render the visual preview inside the `DragOverlay`.
fn render_overlay(zones: Signal<Vec<Zone>>) -> Element {
    let ctx = use_dnd_context();
    let DragState::Dragging { id, .. } = *ctx.state.read() else {
        return rsx! {};
    };
    let label = zones
        .read()
        .iter()
        .flat_map(|z| z.cards.iter())
        .find(|c| c.id == id.0)
        .map(|c| c.label.clone());
    let Some(label) = label else {
        return rsx! {};
    };
    rsx! {
        div { class: "overlay-card", "{label}" }
    }
}

/// Move the card identified by `from` to the slot described by `to`.
///
/// `to` may be either a card-slot droppable (insert *before* that card,
/// possibly in another zone) or a zone-tail droppable (append at the
/// end of that zone). Source and destination zones can differ.
fn move_card(zones: &mut [Zone], from: u64, to: DroppableId) {
    let Some((src_zone, src_idx)) = locate(zones, from) else {
        return;
    };

    if let Some(dest_zone) = zone_idx_from_tail(to) {
        if dest_zone >= zones.len() {
            return;
        }
        let card = zones[src_zone].cards.remove(src_idx);
        zones[dest_zone].cards.push(card);
        return;
    }

    let Some((dest_zone, dest_idx)) = locate(zones, to.0) else {
        return;
    };
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
