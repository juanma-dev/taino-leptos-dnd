//! Multi-select drag demo for taino-leptos-dnd.
//!
//! The list is `taino-leptos-dnd`'s normal sortable-list, plus an
//! **app-managed selection** layered on top. The app writes to
//! `DndContext::selection` in response to plain / Ctrl-Cmd / Shift clicks;
//! the library reads it once at drag start and reports the whole group back
//! at drop time as `DropResult::draggable` + `DropResult::additional`.
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

use std::collections::HashSet;

use leptos::prelude::*;
use taino_dnd_core::{DragState, DraggableId, DroppableId};
use taino_dnd_leptos::{
    provide_dnd_context, use_dnd_context, use_draggable, use_droppable, DndAnnouncer, DragOverlay,
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
        (1..=15_u64).map(|id| Item { id, label: format!("Item #{id}") }).collect::<Vec<_>>(),
    );

    // Anchor index for Shift-click range selection. Updated on every
    // plain / Ctrl click.
    let anchor: RwSignal<Option<usize>> = RwSignal::new(None);

    // "If the next pointer release happens *without* having entered Dragging,
    // collapse the selection to this id." Set when the user plain-clicks an
    // already-selected item (so a drag-the-group gesture isn't interrupted);
    // cleared when a real drag starts or when the collapse is applied.
    let pending_collapse: RwSignal<Option<DraggableId>> = RwSignal::new(None);

    // Watch the state machine so we can resolve `pending_collapse`.
    let last_state = RwSignal::new(DragState::Idle);
    Effect::new(move |_| {
        let now = ctx.state.get();
        let prev = last_state.get_untracked();
        last_state.set(now);
        // A real drag started — the user wants to move the whole group, so
        // cancel the deferred collapse.
        if matches!(now, DragState::Dragging { .. }) {
            pending_collapse.set(None);
        }
        // Pressed → Idle without ever reaching Dragging = a click. If we had
        // deferred a collapse for this click, apply it now.
        if matches!(prev, DragState::Pressed { .. }) && matches!(now, DragState::Idle) {
            if let Some(id) = pending_collapse.get_untracked() {
                ctx.selection.set(std::iter::once(id).collect());
                pending_collapse.set(None);
            }
        }
    });

    // Apply a successful drop: move the whole group (primary + additional) to
    // the slot held by `over`, preserving the group's original order.
    Effect::new(move |_| {
        if let Some(drop) = ctx.take_last_drop() {
            if let Some(over) = drop.over {
                let mut group: Vec<u64> = Vec::with_capacity(1 + drop.additional.len());
                group.push(drop.draggable.0);
                group.extend(drop.additional.iter().map(|d| d.0));
                items.update(|v| reorder_group(v, &group, over.0));
            }
        }
    });

    // Handler the rows call on pointerdown — it mutates `selection` *before*
    // the library's own pointerdown logic snapshots it into `dragged_group`.
    // `idx` is looked up from the items signal so the rows don't have to
    // carry their position as a prop (and the `For` iteration keeps the
    // simpler `move |item| view!{...}` form without tuple destructuring).
    let click = move |id: DraggableId, ctrl: bool, shift: bool| {
        let already = ctx.is_selected(id);
        let selection_size = ctx.selection.with_untracked(HashSet::len);
        let idx = items.with_untracked(|v| v.iter().position(|i| i.id == id.0)).unwrap_or(0);

        if ctrl {
            // Toggle: add or remove this id; remember as anchor.
            ctx.selection.update(|s| {
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
            match anchor.get_untracked() {
                Some(a) => {
                    let (lo, hi) = if a <= idx { (a, idx) } else { (idx, a) };
                    let in_range: Vec<DraggableId> = items.with_untracked(|v| {
                        v.iter().skip(lo).take(hi - lo + 1).map(|i| DraggableId(i.id)).collect()
                    });
                    ctx.selection.update(|s| s.extend(in_range));
                }
                None => ctx.selection.set(std::iter::once(id).collect()),
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
            ctx.selection.set(std::iter::once(id).collect());
            pending_collapse.set(None);
        }
        anchor.set(Some(idx));
    };

    // For the toolbar.
    let selected_count = Signal::derive(move || ctx.selection.with(HashSet::len));
    let clear_selection = move |_| {
        ctx.selection.set(HashSet::new());
        anchor.set(None);
        pending_collapse.set(None);
    };

    view! {
        <DndAnnouncer/>
        <h1>"taino-leptos-dnd — multi-select list"</h1>
        <p class="hint">
            "Click an item to select it. "
            <kbd>"⌘"</kbd>" / "<kbd>"Ctrl"</kbd>"+click to toggle, "
            <kbd>"Shift"</kbd>"+click to extend the range. Drag any selected item to move the whole group."
        </p>
        <div class="toolbar">
            <span>{move || format!("{} selected", selected_count.get())}</span>
            <button on:click=clear_selection prop:disabled=move || selected_count.get() == 0>
                "Clear selection"
            </button>
        </div>
        <div class="list">
            <For
                each=move || items.get()
                key=|item| item.id
                children=move |item| view! { <Row item=item on_click=click/> }
            />
        </div>
        <DragOverlay>
            {move || {
                let ctx = use_dnd_context();
                let primary = ctx.state.get().dragged_id()?;
                let label = items
                    .with(|v| v.iter().find(|i| i.id == primary.0).map(|i| i.label.clone()))?;
                let group_size = ctx.dragged_group.with(Vec::len);
                Some(view! {
                    <div class="overlay-card">
                        {label}
                        {(group_size > 1).then(|| view! { <span class="badge">{format!("+{} more", group_size - 1)}</span> })}
                    </div>
                })
            }}
        </DragOverlay>
        <footer>"v0.5 demo · multi-drag (single-item drag still works on unselected rows)"</footer>
    }
}

#[component]
fn Row(item: Item, on_click: impl Fn(DraggableId, bool, bool) + 'static + Copy) -> impl IntoView {
    let id = item.id;
    let ctx = use_dnd_context();
    let d = use_draggable(DraggableId(id));
    let z = use_droppable(DroppableId(id));
    let label = item.label.clone();

    // `is_selected` re-runs whenever the app's selection signal changes.
    let is_selected = Signal::derive(move || ctx.is_selected(DraggableId(id)));
    // Fade *non-primary* group members in their original slot while the
    // group moves. `in_drag_group` is true for everyone in the group;
    // `is_dragging` is true only for the primary — so this CSS class lights
    // up exactly for the "other" ones.
    let in_group_not_primary =
        Signal::derive(move || ctx.is_being_dragged(DraggableId(id)) && !d.is_dragging.get());

    view! {
        <div
            class="row"
            node_ref=z.node_ref
            class:over=move || z.is_over.get()
            style=move || z.drop_preview_style()
        >
            <div
                class="item"
                class:selected=move || is_selected.get()
                class:dragging=move || d.is_dragging.get()
                class:in-group=move || in_group_not_primary.get()
                node_ref=d.node_ref
                tabindex="0"
                role="button"
                aria-roledescription="draggable item"
                aria-pressed=move || is_selected.get().to_string()
                aria-label=label
                on:pointerdown=move |e| {
                    // Update the *selection* before letting the library handle
                    // the drag-start. `begin_drag_group` runs inside the library's
                    // own `on_pointer_down`, immediately after we return — so by
                    // then the selection signal already reflects this click.
                    on_click(DraggableId(id), e.ctrl_key() || e.meta_key(), e.shift_key());
                    d.on_pointer_down(&e);
                }
                on:pointermove=move |e| d.on_pointer_move(&e)
                on:pointerup=move |e| d.on_pointer_up(&e)
                on:pointercancel=move |e| d.on_pointer_cancel(&e)
                on:keydown=move |e| d.on_key_down(&e)
                style=move || d.style_pinned()
            >
                <span class="check" aria-hidden="true"></span>
                {item.label}
            </div>
        </div>
    }
}

/// Move a *group* of items so they end up contiguous, just before `target`.
///
/// `group` is the dragged group as returned by the binding: `[primary,
/// ...additional]`, in selection iteration order. The items are removed from
/// their current positions and reinserted **in their current vec order** at
/// the target slot, which makes the result feel stable regardless of how
/// scattered the selection was.
///
/// If `target` is itself in the group (e.g. the user dropped onto one of the
/// items being dragged), the move is a no-op — there's nowhere for the group
/// to go that's different from where it already is.
fn reorder_group(v: &mut Vec<Item>, group: &[u64], target: u64) {
    let group_set: HashSet<u64> = group.iter().copied().collect();
    if group_set.contains(&target) {
        return;
    }
    // Snapshot dragged items in their *current vec order* (not selection
    // order). Keeps the relative ordering of the group stable across moves.
    let dragged: Vec<Item> = v.iter().filter(|i| group_set.contains(&i.id)).cloned().collect();
    if dragged.is_empty() {
        return;
    }
    v.retain(|i| !group_set.contains(&i.id));
    let target_idx = v.iter().position(|i| i.id == target).unwrap_or(v.len());
    for (offset, it) in dragged.into_iter().enumerate() {
        v.insert(target_idx + offset, it);
    }
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
