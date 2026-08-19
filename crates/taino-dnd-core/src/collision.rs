//! Collision-detection strategies.
//!
//! - [`pointer_within`] picks the droppable whose rect contains the pointer.
//!   This is the default for pointer-driven drags: it scopes activation to
//!   the area the user is actually pointing at, which matters for layouts
//!   with multiple zones where neighboring zones must not "steal" the drop
//!   target.
//! - [`closest_center`] picks the droppable whose center is nearest to the
//!   pointer, regardless of containment. Available as a building block for
//!   custom strategies; not used by the default pointer path because in a
//!   multi-zone layout it activates the wrong target as soon as the pointer
//!   crosses the midpoint between two zones.
//! - [`spatial_neighbor`] picks the next droppable in a given direction
//!   relative to a starting droppable. Used by the keyboard sensor to handle
//!   arrow keys.

use std::{fmt::Debug, hash::Hash};

use crate::{
    geometry::{Point, Rect},
    state::DroppableId,
};

/// A cardinal direction used by [`spatial_neighbor`] for keyboard navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Toward smaller `y`.
    Up,
    /// Toward larger `y`.
    Down,
    /// Toward smaller `x`.
    Left,
    /// Toward larger `x`.
    Right,
}

/// Pick the [`DroppableId`] whose rectangle contains `pointer`.
///
/// Returns `None` when the pointer is not inside any droppable. When two
/// rects overlap and both contain the pointer (e.g. nested zones), the tie
/// is resolved by picking the droppable whose center is closest to the
/// pointer.
///
/// This is the default `update_over` policy in the framework bindings.
/// Compared to [`closest_center`], it scopes activation to the area the
/// user is actually pointing at, which is what multi-zone layouts need:
/// without containment, the pointer hovering in the *gap* between zone A
/// and zone B would prematurely activate zone B's nearest card, opening a
/// drop slot before the user has even entered zone B.
///
/// # Examples
///
/// ```
/// use taino_dnd_core::{collision::pointer_within, DroppableId, Point, Rect};
///
/// let zones = [
///     (DroppableId(1), Rect::new(0.0, 0.0, 100.0, 100.0)),
///     (DroppableId(2), Rect::new(200.0, 0.0, 100.0, 100.0)),
/// ];
///
/// // Inside zone 1 → zone 1.
/// assert_eq!(pointer_within(Point::new(50.0, 50.0), zones), Some(DroppableId(1)));
/// // In the gap between zone 1 and zone 2 → no activation.
/// assert_eq!(pointer_within(Point::new(150.0, 50.0), zones), None);
/// // Inside zone 2 → zone 2.
/// assert_eq!(pointer_within(Point::new(250.0, 50.0), zones), Some(DroppableId(2)));
/// ```
pub fn pointer_within<T, I>(pointer: Point, droppables: I) -> Option<DroppableId<T>>
where
    T: Debug + Clone + Copy + PartialEq + Eq + Hash + PartialOrd + Ord,
    I: IntoIterator<Item = (DroppableId<T>, Rect)>,
{
    droppables
        .into_iter()
        .filter(|(_, rect)| rect.contains(pointer))
        .map(|(id, rect)| (id, rect.center().distance_squared(pointer)))
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(id, _)| id)
}

/// Pick the [`DroppableId`] whose rectangle's center is closest to `pointer`.
///
/// Returns `None` when `droppables` is empty. Ties (identical distances) resolve
/// to whichever candidate appears first in the iterator.
///
/// The pointer does **not** need to be inside any rect. This matches what users
/// expect: dragging "near" a target should still highlight it.
///
/// # Examples
///
/// ```
/// use taino_dnd_core::{collision::closest_center, DroppableId, Point, Rect};
///
/// let zones = [
///     (DroppableId(1), Rect::new(0.0, 0.0, 100.0, 100.0)),
///     (DroppableId(2), Rect::new(200.0, 0.0, 100.0, 100.0)),
/// ];
///
/// let near_first = closest_center(Point::new(10.0, 10.0), zones.iter().copied());
/// assert_eq!(near_first, Some(DroppableId(1)));
///
/// let near_second = closest_center(Point::new(240.0, 40.0), zones.iter().copied());
/// assert_eq!(near_second, Some(DroppableId(2)));
/// ```
pub fn closest_center<T, I>(pointer: Point, droppables: I) -> Option<DroppableId<T>>
where
    T: Debug + Clone + Copy + PartialEq + Eq + Hash + PartialOrd + Ord,
    I: IntoIterator<Item = (DroppableId<T>, Rect)>,
{
    droppables
        .into_iter()
        .map(|(id, rect)| (id, rect.center().distance_squared(pointer)))
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(id, _)| id)
}

/// Pick the next droppable in `direction` relative to `from`.
///
/// Candidates eligible to be selected are those whose center lies
/// "predominantly" in `direction` relative to `from`'s center — that is, the
/// component of the displacement along `direction`'s axis exceeds the
/// orthogonal component. Among eligible candidates the closest (by Euclidean
/// distance) wins.
///
/// `from` itself is never returned.
///
/// Returns `None` when no candidate qualifies (e.g. the pointer is already at
/// the edge of the layout in that direction).
///
/// # Examples
///
/// ```
/// use taino_dnd_core::{
///     collision::{spatial_neighbor, Direction},
///     DroppableId, Rect,
/// };
///
/// let zones = [
///     (DroppableId(1), Rect::new(0.0,   0.0, 80.0, 40.0)),
///     (DroppableId(2), Rect::new(0.0,  50.0, 80.0, 40.0)),
///     (DroppableId(3), Rect::new(0.0, 100.0, 80.0, 40.0)),
/// ];
/// assert_eq!(
///     spatial_neighbor(DroppableId(1), Direction::Down, zones.iter().copied()),
///     Some(DroppableId(2)),
/// );
/// assert_eq!(
///     spatial_neighbor(DroppableId(3), Direction::Down, zones.iter().copied()),
///     None,
/// );
/// ```
pub fn spatial_neighbor<T, I>(
    from: DroppableId<T>,
    direction: Direction,
    droppables: I,
) -> Option<DroppableId<T>>
where
    T: Debug + Clone + Copy + PartialEq + Eq + Hash + PartialOrd + Ord,
    I: IntoIterator<Item = (DroppableId<T>, Rect)>,
{
    let zones: Vec<(DroppableId<T>, Rect)> = droppables.into_iter().collect();
    let origin = zones.iter().find(|(id, _)| *id == from).map(|(_, r)| r.center())?;

    zones
        .iter()
        .filter(|(id, _)| *id != from)
        .filter_map(|(id, rect)| {
            let c = rect.center();
            let dx = c.x - origin.x;
            let dy = c.y - origin.y;
            let in_direction = match direction {
                Direction::Up => -dy > dx.abs(),
                Direction::Down => dy > dx.abs(),
                Direction::Left => -dx > dy.abs(),
                Direction::Right => dx > dy.abs(),
            };
            if in_direction {
                Some((*id, dx.mul_add(dx, dy * dy)))
            } else {
                None
            }
        })
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone(id: u64, x: f64, y: f64, w: f64, h: f64) -> (DroppableId<u64>, Rect) {
        (DroppableId(id), Rect::new(x, y, w, h))
    }

    #[test]
    fn pointer_within_empty_returns_none() {
        assert!(pointer_within::<u64, _>(Point::new(0.0, 0.0), std::iter::empty()).is_none());
    }

    #[test]
    fn pointer_within_inside_rect_returns_it() {
        let zones = [zone(1, 0.0, 0.0, 100.0, 100.0), zone(2, 200.0, 0.0, 100.0, 100.0)];
        assert_eq!(pointer_within(Point::new(50.0, 50.0), zones), Some(DroppableId(1)));
        assert_eq!(pointer_within(Point::new(250.0, 50.0), zones), Some(DroppableId(2)));
    }

    #[test]
    fn pointer_within_outside_all_returns_none() {
        // Cursor sits in the gap between the two zones — no activation.
        let zones = [zone(1, 0.0, 0.0, 100.0, 100.0), zone(2, 200.0, 0.0, 100.0, 100.0)];
        assert!(pointer_within(Point::new(150.0, 50.0), zones).is_none());
        // Way off in space.
        assert!(pointer_within(Point::new(1000.0, 1000.0), zones).is_none());
    }

    #[test]
    fn pointer_within_overlap_picks_closest_center() {
        // Two overlapping zones. Pointer is inside both. Tie breaks to the
        // zone whose center is closer.
        let zones = [zone(1, 0.0, 0.0, 100.0, 100.0), zone(2, 50.0, 50.0, 100.0, 100.0)];
        // Point (60, 60): zone 1 center (50, 50) → d²=200; zone 2 center
        // (100, 100) → d²=3200. Zone 1 wins.
        assert_eq!(pointer_within(Point::new(60.0, 60.0), zones), Some(DroppableId(1)));
    }

    #[test]
    fn pointer_within_multi_zone_does_not_leak_between_zones() {
        // Two stacked vertical zones separated by a gap.
        let zones = [
            zone(1, 0.0, 0.0, 200.0, 100.0),   // Zone A
            zone(2, 0.0, 150.0, 200.0, 100.0), // Zone B (50 px gap below A)
        ];
        // Cursor in the gap: neither zone activates. This is the property
        // that prevents premature drop-preview shifts in multi-zone layouts.
        assert!(pointer_within(Point::new(100.0, 120.0), zones).is_none());
        assert!(pointer_within(Point::new(100.0, 130.0), zones).is_none());
    }

    #[test]
    fn closest_center_empty_returns_none() {
        assert!(closest_center::<u64, _>(Point::new(0.0, 0.0), std::iter::empty()).is_none());
    }

    #[test]
    fn closest_center_picks_nearest() {
        let zones = [zone(1, 0.0, 0.0, 10.0, 10.0), zone(2, 100.0, 0.0, 10.0, 10.0)];
        assert_eq!(closest_center(Point::new(4.0, 4.0), zones), Some(DroppableId(1)));
        assert_eq!(closest_center(Point::new(108.0, 4.0), zones), Some(DroppableId(2)));
    }

    #[test]
    fn closest_center_works_when_pointer_outside_all_rects() {
        let zones = [zone(1, 0.0, 0.0, 10.0, 10.0), zone(2, 100.0, 0.0, 10.0, 10.0)];
        assert_eq!(closest_center(Point::new(1000.0, 1000.0), zones), Some(DroppableId(2)));
    }

    #[test]
    fn closest_center_tie_resolves_to_first() {
        // Both centers are equidistant from (0, 0).
        let zones = [zone(1, -5.0, -5.0, 10.0, 10.0), zone(2, -5.0, -5.0, 10.0, 10.0)];
        assert_eq!(closest_center(Point::new(0.0, 0.0), zones), Some(DroppableId(1)));
    }

    fn vertical_list() -> [(DroppableId<u64>, Rect); 3] {
        [
            zone(1, 0.0, 0.0, 80.0, 40.0),
            zone(2, 0.0, 50.0, 80.0, 40.0),
            zone(3, 0.0, 100.0, 80.0, 40.0),
        ]
    }

    #[test]
    fn spatial_neighbor_down_in_vertical_list() {
        assert_eq!(
            spatial_neighbor(DroppableId(1), Direction::Down, vertical_list()),
            Some(DroppableId(2))
        );
        assert_eq!(
            spatial_neighbor(DroppableId(2), Direction::Down, vertical_list()),
            Some(DroppableId(3))
        );
        assert_eq!(spatial_neighbor(DroppableId(3), Direction::Down, vertical_list()), None);
    }

    #[test]
    fn spatial_neighbor_up_in_vertical_list() {
        assert_eq!(
            spatial_neighbor(DroppableId(3), Direction::Up, vertical_list()),
            Some(DroppableId(2))
        );
        assert_eq!(spatial_neighbor(DroppableId(1), Direction::Up, vertical_list()), None);
    }

    #[test]
    fn spatial_neighbor_horizontal_in_vertical_list_returns_none() {
        assert_eq!(spatial_neighbor(DroppableId(1), Direction::Right, vertical_list()), None);
        assert_eq!(spatial_neighbor(DroppableId(2), Direction::Left, vertical_list()), None);
    }

    #[test]
    fn spatial_neighbor_unknown_origin_returns_none() {
        assert_eq!(spatial_neighbor(DroppableId(999), Direction::Down, vertical_list()), None);
    }

    #[test]
    fn spatial_neighbor_grid_picks_closest_in_direction() {
        // 2x2 grid; from top-left, Right => top-right, Down => bottom-left.
        let zones = [
            zone(1, 0.0, 0.0, 50.0, 50.0),
            zone(2, 100.0, 0.0, 50.0, 50.0),
            zone(3, 0.0, 100.0, 50.0, 50.0),
            zone(4, 100.0, 100.0, 50.0, 50.0),
        ];
        assert_eq!(spatial_neighbor(DroppableId(1), Direction::Right, zones), Some(DroppableId(2)));
        assert_eq!(spatial_neighbor(DroppableId(1), Direction::Down, zones), Some(DroppableId(3)));
        assert_eq!(spatial_neighbor(DroppableId(2), Direction::Left, zones), Some(DroppableId(1)));
        assert_eq!(spatial_neighbor(DroppableId(4), Direction::Up, zones), Some(DroppableId(2)));
    }
}
