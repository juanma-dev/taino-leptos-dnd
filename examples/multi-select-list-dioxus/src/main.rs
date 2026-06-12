//! Multi-select drag demo for `taino-dnd-dioxus` — the Dioxus twin of
//! `examples/multi-select-list`.
//!
//! The list is the normal sortable-list, plus an **app-managed selection**
//! layered on top. The app writes to `DndContext::selection` in response to
//! plain / Ctrl-Cmd / Shift clicks; the library reads it once at drag start
//! and reports the whole group back at drop time as `DropResult::draggable`
//! + `DropResult::additional`.
//!
//! Selection UX (matches macOS Finder / Windows Explorer):
//!
//! | Gesture                  | Effect                                    |
//! | ------------------------ | ----------------------------------------- |
//! | Click                    | Select only this item                     |
//! | Click on already-selected| Deferred — collapse to this id only *if* no drag happens before pointerup. |
//! | Ctrl-click / Cmd-click   | Toggle this item in the selection         |
//! | Shift-click              | Extend selection from the anchor to here  |
//! | Drag a selected item     | Drag the whole selection as a group       |
//! | Drag an unselected item  | Drag just that one (selection unchanged)  |
//!
//! Keyboard pickup (Space/Enter) works on the focused item, single-drag only —
//! keyboard multi-select isn't part of the demo (rbd doesn't do it either).

#![allow(non_snake_case)]

use std::collections::HashSet;

use dioxus::prelude::*;
use taino_dnd_dioxus::{
    provide_dnd_context, use_dnd_context, use_draggable, use_droppable, DndAnnouncer, DragOverlay,
    DragState, DraggableId, DroppableId,
};

fn main() {
    console_error_panic_hook::set_once();
    launch(App);
}

// Demo `App` deliberately keeps the selection UX inline so it reads
// top-to-bottom; splitting it would hide the click-semantics story.
#[allow(clippy::too_many_lines)]
fn App() -> Element {
    let ctx = provide_dnd_context();

    let mut items =
        use_signal(|| (1..=15_u64).map(|id| (id, format!("Item #{id}"))).collect::<Vec<_>>());

    // Anchor index for Shift-click range selection. Updated on every
    // plain / Ctrl click.
    let mut anchor = use_signal::<Option<usize>>(|| None);

    // "If the next pointer release happens *without* having entered Dragging,
    // collapse the selection to this id." Set when the user plain-clicks an
    // already-selected item (so a drag-the-group gesture isn't interrupted);
    // cleared when a real drag starts or when the collapse is applied.
    let mut pending_collapse = use_signal::<Option<DraggableId>>(|| None);

    // Watch the state machine so we can resolve `pending_collapse`.
    let mut last_state = use_signal(|| DragState::Idle);
    use_effect(move || {
        let now = *ctx.state.read();
        let prev = *last_state.peek();
        last_state.set(now);
        // A real drag started — the user wants to move the whole group, so
        // cancel the deferred collapse.
        if matches!(now, DragState::Dragging { .. }) && pending_collapse.peek().is_some() {
            pending_collapse.set(None);
        }
        // Pressed → Idle without ever reaching Dragging = a click. If we had
        // deferred a collapse for this click, apply it now.
        if matches!(prev, DragState::Pressed { .. }) && matches!(now, DragState::Idle) {
            // Copy the id out before writing — `peek()`'s guard must drop
            // before `set()` or Dioxus panics on a read/write conflict.
            let deferred = *pending_collapse.peek();
            if let Some(id) = deferred {
                let mut selection = ctx.selection;
                selection.set(std::iter::once(id).collect());
                pending_collapse.set(None);
            }
        }
    });

    // Apply a successful drop: move the whole group (primary + additional) to
    // the slot held by `over`, preserving the group's original order.
    use_effect(move || {
        if let Some(drop) = ctx.take_last_drop() {
            if let Some(over) = drop.over {
                let mut group: Vec<u64> = Vec::with_capacity(1 + drop.additional.len());
                group.push(drop.draggable.0);
                group.extend(drop.additional.iter().map(|d| d.0));
                items.with_mut(|v| reorder_group(v, &group, over.0));
            }
        }
    });

    // Handler the rows call on pointerdown — it mutates `selection` *before*
    // the library's own pointerdown logic snapshots it into `dragged_group`.
    let click = move |(id, ctrl, shift): (DraggableId, bool, bool)| {
        let already = ctx.is_selected(id);
        let selection_size = ctx.selection.peek().len();
        let idx = items.peek().iter().position(|(i, _)| *i == id.0).unwrap_or(0);
        let mut selection = ctx.selection;

        if ctrl {
            // Toggle: add or remove this id; remember as anchor.
            selection.with_mut(|s| {
                if !s.insert(id) {
                    s.remove(&id);
                }
            });
            anchor.set(Some(idx));
            pending_collapse.set(None);
            return;
        }
        if shift {
            // Range from anchor (or just this id if no anchor yet).
            match *anchor.peek() {
                Some(a) => {
                    let (lo, hi) = if a <= idx { (a, idx) } else { (idx, a) };
                    let in_range: Vec<DraggableId> = items
                        .peek()
                        .iter()
                        .skip(lo)
                        .take(hi - lo + 1)
                        .map(|(i, _)| DraggableId(*i))
                        .collect();
                    selection.with_mut(|s| s.extend(in_range));
                }
                None => selection.set(std::iter::once(id).collect()),
            }
            anchor.set(Some(idx));
            pending_collapse.set(None);
            return;
        }
        // Plain click.
        if already && selection_size > 1 {
            // Defer: don't clobber the group yet; the user may be about to
            // drag it. If they don't, we collapse on pointerup.
            pending_collapse.set(Some(id));
        } else {
            selection.set(std::iter::once(id).collect());
            pending_collapse.set(None);
        }
        anchor.set(Some(idx));
    };

    // For the toolbar.
    let selected_count = use_memo(move || ctx.selection.read().len());
    let clear_selection = move |_| {
        let mut selection = ctx.selection;
        selection.set(HashSet::new());
        anchor.set(None);
        pending_collapse.set(None);
    };

    rsx! {
        DndAnnouncer {}
        h1 { "taino-dnd-dioxus — multi-select list" }
        p { class: "hint",
            "Click an item to select it. "
            kbd { "⌘" }
            " / "
            kbd { "Ctrl" }
            "+click to toggle, "
            kbd { "Shift" }
            "+click to extend the range. Drag any selected item to move the whole group."
        }
        div { class: "toolbar",
            span { "{selected_count} selected" }
            button { disabled: *selected_count.read() == 0, onclick: clear_selection,
                "Clear selection"
            }
        }
        div { class: "list",
            for (id, label) in items.read().iter() {
                Row { key: "{id}", id: *id, label: label.clone(), on_click: click }
            }
        }
        DragOverlay {
            {render_overlay(items)}
        }
        footer { "v0.5 demo · multi-drag (single-item drag still works on unselected rows)" }
    }
}

#[component]
fn Row(id: u64, label: String, on_click: EventHandler<(DraggableId, bool, bool)>) -> Element {
    let ctx = use_dnd_context();
    let d = use_draggable(DraggableId(id));
    let z = use_droppable(DroppableId(id));

    let row_class =
        use_memo(move || if *z.is_over.read() { "row over".to_owned() } else { "row".to_owned() });
    // `selected` lights up for everything in the app's selection set.
    // `dragging` is true only for the primary; `in-group` for the *other*
    // group members, fading them in their original slot while the group moves.
    let item_class = use_memo(move || {
        let mut c = String::from("item");
        if ctx.is_selected(DraggableId(id)) {
            c.push_str(" selected");
        }
        if *d.is_dragging.read() {
            c.push_str(" dragging");
        } else if ctx.is_being_dragged(DraggableId(id)) {
            c.push_str(" in-group");
        }
        c
    });
    let is_selected = use_memo(move || ctx.is_selected(DraggableId(id)));

    let label_for_aria = label.clone();
    rsx! {
        div {
            class: "{row_class}",
            onmounted: move |e| z.on_mounted(e),
            style: "{z.drop_preview_style()}",
            div {
                class: "{item_class}",
                onmounted: move |e| d.on_mounted(e),
                onpointerdown: move |e| {
                    // Update the *selection* before letting the library handle
                    // the drag-start. `begin_drag_group` runs inside the
                    // library's own `on_pointer_down`, immediately after we
                    // return — so by then the selection reflects this click.
                    let mods = e.modifiers();
                    on_click.call((DraggableId(id), mods.ctrl() || mods.meta(), mods.shift()));
                    d.on_pointer_down(e);
                },
                onpointermove: move |e| d.on_pointer_move(e),
                onpointerup: move |e| d.on_pointer_up(e),
                onpointercancel: move |e| d.on_pointer_cancel(e),
                onkeydown: move |e| d.on_key_down(e),
                tabindex: "0",
                role: "button",
                "aria-roledescription": "draggable item",
                "aria-pressed": "{is_selected}",
                "aria-label": "{label_for_aria}",
                style: "{d.style_pinned()}",
                span { class: "check", "aria-hidden": "true" }
                "{label}"
            }
        }
    }
}

/// Render the visual preview inside the `DragOverlay` — the primary's card
/// plus a `+N more` badge when a group rides along.
fn render_overlay(items: Signal<Vec<(u64, String)>>) -> Element {
    let ctx = use_dnd_context();
    let DragState::Dragging { id, .. } = *ctx.state.read() else {
        return rsx! {};
    };
    let label = items.read().iter().find(|(i, _)| *i == id.0).map(|(_, l)| l.clone());
    let Some(label) = label else {
        return rsx! {};
    };
    let group_size = ctx.dragged_group.read().len();
    rsx! {
        div { class: "overlay-card",
            "{label}"
            if group_size > 1 {
                span { class: "badge", "+{group_size - 1} more" }
            }
        }
    }
}

/// Move a *group* of items so they end up contiguous, at the target's slot,
/// mirroring the single-drag semantic from `examples/sortable-list-dioxus`.
///
/// `group` is the dragged group as returned by the binding: `[primary,
/// ...additional]`. The items are reinserted **in their current vec order**
/// (not group order — `additional` is sorted by id, not list position), so
/// the relative ordering of the group stays stable across moves.
///
/// If `target` is itself in the group, the move is a no-op.
///
/// # Where the group lands
///
/// We mirror the sortable's "drop A onto B inserts A at B's *original* slot
/// after `remove`" trick, generalised to N removals:
///
/// - **Forward / mixed move** (at least one group member was *before* the
///   target in the vec): the group lands *just past* the target — the
///   target shifted up by `removed_before` to make room, so we insert at
///   `(original_target_idx - removed_before) + 1`.
/// - **Backward move** (every group member was *after* the target): the
///   group lands *just before* the target — `removed_before` is `0`, target
///   stays put, we insert at `original_target_idx`.
fn reorder_group(v: &mut Vec<(u64, String)>, group: &[u64], target: u64) {
    let group_set: HashSet<u64> = group.iter().copied().collect();
    if group_set.contains(&target) {
        return;
    }
    let Some(original_target_idx) = v.iter().position(|(i, _)| *i == target) else {
        return;
    };
    // Group members BEFORE the target will shift it leftwards by this many.
    let removed_before =
        v.iter().take(original_target_idx).filter(|(i, _)| group_set.contains(i)).count();
    // Snapshot the group in vec order so the relative ordering stays stable.
    let dragged: Vec<(u64, String)> =
        v.iter().filter(|(i, _)| group_set.contains(i)).cloned().collect();
    if dragged.is_empty() {
        return;
    }
    v.retain(|(i, _)| !group_set.contains(i));
    let target_idx_after = original_target_idx - removed_before;
    let insert_pos = target_idx_after + usize::from(removed_before > 0);
    for (offset, it) in dragged.into_iter().enumerate() {
        v.insert(insert_pos + offset, it);
    }
}
