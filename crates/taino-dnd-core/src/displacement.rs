//! Live drop-preview displacements.
//!
//! While a drag is active, neighbors of the hovered slot should shift to
//! *show* where the dragged item would land — the "transition during
//! drag" feel that `react-beautiful-dnd` and `dnd-kit` made the user
//! expectation. FLIP only animates the post-drop settle; this module
//! computes the live, mid-drag offsets.
//!
//! The computation is a pure function over registered droppable rects.
//! Bindings call [`live_displacements`] each time the active drag state
//! changes and apply the resulting per-item `Vector` as an inline
//! `transform: translate(...)` on each droppable wrapper.
//!
//! # Algorithm
//!
//! Items are arranged in a 1D list (the [`Axis`]). The dragged item
//! and the `over` slot define a range:
//!
//! * **Forward move** (`over_idx > dragged_idx`): items in
//!   `(dragged_idx, over_idx]` shift backward by one step along the
//!   axis, opening a gap where the dragged item would land.
//! * **Backward move** (`over_idx < dragged_idx`): items in
//!   `[over_idx, dragged_idx)` shift forward.
//! * Otherwise: every item's displacement is zero.
//!
//! `step` is the size of the *dragged* item along the axis — i.e.
//! `dragged_rect.height` for [`Axis::Y`] or `dragged_rect.width` for
//! [`Axis::X`]. Using the dragged item's size (not each neighbor's)
//! matches what the user sees when the item finally drops into the
//! freed slot.
//!
//! # Scope
//!
//! This computes a single-axis, single-lane layout. Multi-column
//! kanban boards work correctly for *drop result* purposes (the state
//! machine doesn't change), but the *visual preview* only makes sense
//! within one lane at a time. Callers wanting kanban-style behavior
//! should call [`live_displacements`] per column with the column's
//! own item slice.

use std::{fmt::Debug, hash::Hash};

use crate::{geometry::Rect, modifier::Vector, Axis, DroppableId};

/// Compute the visual displacement each item should apply while a drag
/// is hovering over `over`.
///
/// `items` is the list of registered droppables **in render order**.
/// The caller is responsible for sorting — typically by top edge for
/// vertical lists, by left edge for horizontal.
///
/// `dragged` is the droppable that corresponds to the dragged item. In
/// the typical sortable-list pattern, draggable and droppable share an
/// id (so `DroppableId(item.id) ↔ DraggableId(item.id)`).
///
/// Returns a `Vec` aligned with `items` (same order, same length).
/// Items with no displacement carry the zero vector.
///
/// # Examples
///
/// ```
/// use taino_dnd_core::{
///     displacement::live_displacements,
///     Axis, DroppableId, Rect, Vector,
/// };
///
/// // Four 40-px-tall rows stacked vertically.
/// let items = [
///     (DroppableId(1), Rect::new(0.0,   0.0, 100.0, 40.0)),
///     (DroppableId(2), Rect::new(0.0,  50.0, 100.0, 40.0)),
///     (DroppableId(3), Rect::new(0.0, 100.0, 100.0, 40.0)),
///     (DroppableId(4), Rect::new(0.0, 150.0, 100.0, 40.0)),
/// ];
///
/// // Drag item 2 over item 4 (forward move). Items 3 and 4 shift up.
/// let d = live_displacements(DroppableId(2), Some(DroppableId(4)), &items, Axis::Y);
/// assert_eq!(d[0].1, Vector::default());                  // item 1
/// assert_eq!(d[1].1, Vector::default());                  // item 2 (dragged)
/// assert_eq!(d[2].1, Vector::new(0.0, -40.0));            // item 3
/// assert_eq!(d[3].1, Vector::new(0.0, -40.0));            // item 4
/// ```
pub fn live_displacements<T>(
    dragged: DroppableId<T>,
    over: Option<DroppableId<T>>,
    items: &[(DroppableId<T>, Rect)],
    axis: Axis,
) -> Vec<(DroppableId<T>, Vector)>
where
    T: Debug + Clone + Copy + PartialEq + Eq + Hash + PartialOrd + Ord,
{
    let mut out: Vec<(DroppableId<T>, Vector)> =
        items.iter().map(|(id, _)| (*id, Vector::default())).collect();

    let Some(over) = over else {
        return out;
    };
    if over == dragged {
        return out;
    }

    let dragged_idx = items.iter().position(|(id, _)| *id == dragged);
    let over_idx = items.iter().position(|(id, _)| *id == over);
    let (Some(dragged_idx), Some(over_idx)) = (dragged_idx, over_idx) else {
        return out;
    };

    let step = match axis {
        Axis::X => items[dragged_idx].1.width,
        Axis::Y => items[dragged_idx].1.height,
    };

    let (start, end, sign) = if over_idx > dragged_idx {
        // Forward: items in (dragged, over] shift backward.
        (dragged_idx + 1, over_idx, -1.0)
    } else {
        // Backward: items in [over, dragged) shift forward.
        (over_idx, dragged_idx - 1, 1.0)
    };

    let delta = sign * step;
    let vec = match axis {
        Axis::X => Vector::new(delta, 0.0),
        Axis::Y => Vector::new(0.0, delta),
    };

    for slot in out.iter_mut().take(end + 1).skip(start) {
        slot.1 = vec;
    }

    out
}

/// Best-guess axis for a slice of rects.
///
/// Returns [`Axis::Y`] when the rects' top edges span a larger range
/// than their left edges, and [`Axis::X`] otherwise. For zero or one
/// item, defaults to [`Axis::Y`] — vertical is the most common
/// sortable layout.
pub fn detect_axis<T>(items: &[(DroppableId<T>, Rect)]) -> Axis
where
    T: Debug + Clone + Copy + PartialEq + Eq + Hash + PartialOrd + Ord,
{
    if items.len() < 2 {
        return Axis::Y;
    }
    let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    for (_, r) in items {
        min_x = min_x.min(r.x);
        max_x = max_x.max(r.x);
        min_y = min_y.min(r.y);
        max_y = max_y.max(r.y);
    }
    if (max_y - min_y) >= (max_x - min_x) {
        Axis::Y
    } else {
        Axis::X
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertical_list() -> [(DroppableId<u64>, Rect); 4] {
        [
            (DroppableId(1), Rect::new(0.0, 0.0, 100.0, 40.0)),
            (DroppableId(2), Rect::new(0.0, 50.0, 100.0, 40.0)),
            (DroppableId(3), Rect::new(0.0, 100.0, 100.0, 40.0)),
            (DroppableId(4), Rect::new(0.0, 150.0, 100.0, 40.0)),
        ]
    }

    fn horizontal_list() -> [(DroppableId<u64>, Rect); 4] {
        [
            (DroppableId(1), Rect::new(0.0, 0.0, 80.0, 40.0)),
            (DroppableId(2), Rect::new(100.0, 0.0, 80.0, 40.0)),
            (DroppableId(3), Rect::new(200.0, 0.0, 80.0, 40.0)),
            (DroppableId(4), Rect::new(300.0, 0.0, 80.0, 40.0)),
        ]
    }

    #[test]
    fn no_over_no_displacement() {
        let items = vertical_list();
        let d = live_displacements(DroppableId(2), None, &items, Axis::Y);
        assert!(d.iter().all(|(_, v)| *v == Vector::default()));
    }

    #[test]
    fn over_equals_dragged_no_displacement() {
        let items = vertical_list();
        let d = live_displacements(DroppableId(2), Some(DroppableId(2)), &items, Axis::Y);
        assert!(d.iter().all(|(_, v)| *v == Vector::default()));
    }

    #[test]
    fn forward_move_vertical() {
        let items = vertical_list();
        let d = live_displacements(DroppableId(2), Some(DroppableId(4)), &items, Axis::Y);
        assert_eq!(d[0].1, Vector::default()); // item 1: untouched
        assert_eq!(d[1].1, Vector::default()); // item 2: dragged
        assert_eq!(d[2].1, Vector::new(0.0, -40.0)); // item 3: shifted up
        assert_eq!(d[3].1, Vector::new(0.0, -40.0)); // item 4: shifted up
    }

    #[test]
    fn backward_move_vertical() {
        let items = vertical_list();
        let d = live_displacements(DroppableId(4), Some(DroppableId(2)), &items, Axis::Y);
        assert_eq!(d[0].1, Vector::default()); // item 1
        assert_eq!(d[1].1, Vector::new(0.0, 40.0)); // item 2 shifted down
        assert_eq!(d[2].1, Vector::new(0.0, 40.0)); // item 3 shifted down
        assert_eq!(d[3].1, Vector::default()); // item 4 dragged
    }

    #[test]
    fn single_step_forward() {
        let items = vertical_list();
        // Drag 2 over 3 — only item 3 shifts.
        let d = live_displacements(DroppableId(2), Some(DroppableId(3)), &items, Axis::Y);
        assert_eq!(d[0].1, Vector::default());
        assert_eq!(d[1].1, Vector::default());
        assert_eq!(d[2].1, Vector::new(0.0, -40.0));
        assert_eq!(d[3].1, Vector::default());
    }

    #[test]
    fn single_step_backward() {
        let items = vertical_list();
        // Drag 3 over 2 — only item 2 shifts.
        let d = live_displacements(DroppableId(3), Some(DroppableId(2)), &items, Axis::Y);
        assert_eq!(d[0].1, Vector::default());
        assert_eq!(d[1].1, Vector::new(0.0, 40.0));
        assert_eq!(d[2].1, Vector::default());
        assert_eq!(d[3].1, Vector::default());
    }

    #[test]
    fn horizontal_axis_uses_width() {
        let items = horizontal_list();
        // Drag 1 over 3 — items 2 and 3 shift left by 80 (dragged width).
        let d = live_displacements(DroppableId(1), Some(DroppableId(3)), &items, Axis::X);
        assert_eq!(d[0].1, Vector::default()); // dragged
        assert_eq!(d[1].1, Vector::new(-80.0, 0.0));
        assert_eq!(d[2].1, Vector::new(-80.0, 0.0));
        assert_eq!(d[3].1, Vector::default());
    }

    #[test]
    fn unknown_dragged_or_over_no_displacement() {
        let items = vertical_list();
        let d = live_displacements(DroppableId(999), Some(DroppableId(2)), &items, Axis::Y);
        assert!(d.iter().all(|(_, v)| *v == Vector::default()));

        let d = live_displacements(DroppableId(2), Some(DroppableId(999)), &items, Axis::Y);
        assert!(d.iter().all(|(_, v)| *v == Vector::default()));
    }

    #[test]
    fn drag_from_start_to_end() {
        let items = vertical_list();
        let d = live_displacements(DroppableId(1), Some(DroppableId(4)), &items, Axis::Y);
        assert_eq!(d[0].1, Vector::default()); // dragged
        assert_eq!(d[1].1, Vector::new(0.0, -40.0));
        assert_eq!(d[2].1, Vector::new(0.0, -40.0));
        assert_eq!(d[3].1, Vector::new(0.0, -40.0));
    }

    #[test]
    fn drag_from_end_to_start() {
        let items = vertical_list();
        let d = live_displacements(DroppableId(4), Some(DroppableId(1)), &items, Axis::Y);
        assert_eq!(d[0].1, Vector::new(0.0, 40.0));
        assert_eq!(d[1].1, Vector::new(0.0, 40.0));
        assert_eq!(d[2].1, Vector::new(0.0, 40.0));
        assert_eq!(d[3].1, Vector::default()); // dragged
    }

    #[test]
    fn detect_axis_vertical_list_returns_y() {
        assert_eq!(detect_axis(&vertical_list()), Axis::Y);
    }

    #[test]
    fn detect_axis_horizontal_list_returns_x() {
        assert_eq!(detect_axis(&horizontal_list()), Axis::X);
    }

    #[test]
    fn detect_axis_empty_or_single_defaults_to_y() {
        assert_eq!(detect_axis::<u64>(&[]), Axis::Y);
        assert_eq!(detect_axis(&[(DroppableId(1), Rect::new(0.0, 0.0, 10.0, 10.0))]), Axis::Y);
    }

    #[test]
    fn detect_axis_ties_break_to_y() {
        // Square-ish grid → equal extents → prefer Y.
        let items = [
            (DroppableId(1), Rect::new(0.0, 0.0, 10.0, 10.0)),
            (DroppableId(2), Rect::new(100.0, 100.0, 10.0, 10.0)),
        ];
        assert_eq!(detect_axis(&items), Axis::Y);
    }
}
