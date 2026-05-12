//! Collision-detection strategies.
//!
//! Stage 1 ships a single strategy — *closest center* — which is the default
//! used by `react-beautiful-dnd` and works well for vertical/horizontal lists
//! and grids alike.

use crate::{
    geometry::{Point, Rect},
    state::DroppableId,
};

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
}
