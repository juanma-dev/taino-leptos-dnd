//! Multi-drag group snapshotting.
//!
//! When a drag starts on an item that belongs to a multi-item selection,
//! the whole selection travels as one group. The app owns the selection
//! (click semantics are app UX); the library only needs to decide, at
//! drag start, *which* ids ride along. That decision is a pure function
//! over `(primary, selection)` — this module holds it so both bindings
//! share one implementation and one test suite, instead of each binding
//! re-implementing it against its framework's signal type.

use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::{BuildHasher, Hash};

use crate::state::DraggableId;

/// Build the dragged group for a drag starting on `primary`.
///
/// If `primary` is in `selection` and the selection has more than one
/// member, the group is `[primary, ...rest]` with the remaining members
/// sorted by ascending id — a deterministic order, so repeated drags of
/// the same selection always produce the same group (and the same
/// `DropResult::additional`). Otherwise the drag is single-item and the
/// group is just `[primary]`.
///
/// Note the sorted order is *not* the app's list order — the library
/// doesn't know it. Apps applying a group drop should reinsert the
/// members in their own list order (see `examples/multi-select-list`'s
/// `reorder_group`), treating the group as a set.
///
/// # Examples
///
/// ```
/// use std::collections::HashSet;
/// use taino_dnd_core::{drag_group, DraggableId};
///
/// let selection: HashSet<DraggableId> =
///     [DraggableId(3), DraggableId(1), DraggableId(2)].into_iter().collect();
///
/// // Primary in a multi-selection: primary first, rest sorted by id.
/// assert_eq!(
///     drag_group(DraggableId(2), &selection),
///     vec![DraggableId(2), DraggableId(1), DraggableId(3)],
/// );
///
/// // Primary outside the selection: single-item drag.
/// assert_eq!(drag_group(DraggableId(9), &selection), vec![DraggableId(9)]);
/// ```
pub fn drag_group<T, S>(
    primary: DraggableId<T>,
    selection: &HashSet<DraggableId<T>, S>,
) -> Vec<DraggableId<T>>
where
    T: Debug + Clone + Copy + PartialEq + Eq + Hash + PartialOrd + Ord,
    S: BuildHasher,
{
    if selection.len() > 1 && selection.contains(&primary) {
        let mut g = Vec::with_capacity(selection.len());
        g.push(primary);
        let mut rest: Vec<DraggableId<T>> =
            selection.iter().copied().filter(|id| *id != primary).collect();
        rest.sort_unstable();
        g.extend(rest);
        g
    } else {
        vec![primary]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(ids: &[u64]) -> HashSet<DraggableId<u64>> {
        ids.iter().map(|&id| DraggableId(id)).collect()
    }

    #[test]
    fn empty_selection_is_singleton() {
        assert_eq!(drag_group(DraggableId(7), &set(&[])), vec![DraggableId(7)]);
    }

    #[test]
    fn primary_not_in_selection_is_singleton() {
        // Selection holds 1 and 2, but the drag starts on 99 — an
        // *unselected* item. The selection must be ignored.
        assert_eq!(drag_group(DraggableId(99), &set(&[1, 2])), vec![DraggableId(99)]);
    }

    #[test]
    fn selection_with_only_the_primary_is_singleton() {
        // A single-item selection must not trigger multi-drag.
        assert_eq!(drag_group(DraggableId(4), &set(&[4])), vec![DraggableId(4)]);
    }

    #[test]
    fn multi_selection_carries_the_group_primary_first_rest_sorted() {
        assert_eq!(
            drag_group(DraggableId(2), &set(&[3, 1, 2])),
            vec![DraggableId(2), DraggableId(1), DraggableId(3)],
        );
    }

    #[test]
    fn group_order_is_deterministic_across_calls() {
        let sel = set(&[5, 9, 1, 7, 3]);
        let first = drag_group(DraggableId(7), &sel);
        for _ in 0..10 {
            assert_eq!(drag_group(DraggableId(7), &sel), first);
        }
    }
}
