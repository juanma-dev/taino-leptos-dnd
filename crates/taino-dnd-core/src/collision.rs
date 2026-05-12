//! Collision-detection strategies.
//!
//! - [`closest_center`] picks the droppable whose center is nearest to the
//!   pointer. Default for pointer-driven drags.
//! - [`spatial_neighbor`] picks the next droppable in a given direction
//!   relative to a starting droppable. Used by the keyboard sensor to handle
//!   arrow keys.

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
pub fn closest_center<I>(pointer: Point, droppables: I) -> Option<DroppableId>
where
    I: IntoIterator<Item = (DroppableId, Rect)>,
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
pub fn spatial_neighbor<I>(
    from: DroppableId,
    direction: Direction,
    droppables: I,
) -> Option<DroppableId>
where
    I: IntoIterator<Item = (DroppableId, Rect)>,
{
    let zones: Vec<(DroppableId, Rect)> = droppables.into_iter().collect();
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

    fn zone(id: u64, x: f64, y: f64, w: f64, h: f64) -> (DroppableId, Rect) {
        (DroppableId(id), Rect::new(x, y, w, h))
    }

    #[test]
    fn closest_center_empty_returns_none() {
        assert!(closest_center(Point::new(0.0, 0.0), std::iter::empty()).is_none());
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

    fn vertical_list() -> [(DroppableId, Rect); 3] {
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
