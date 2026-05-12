//! Modifiers that transform a drag displacement before it's rendered.
//!
//! A modifier is a function `Vector → Vector` that runs on every pointermove
//! while a drag is active. The state machine itself sees raw pointer positions
//! (so the click-vs-drag threshold isn't broken by an axis lock), but the
//! visual transform and the collision-detection point both go through the
//! modifier chain.
//!
//! # Composition
//!
//! Modifiers compose left-to-right via [`apply_chain`]: the output of one
//! feeds into the next. Order matters — `RestrictToAxis(Y)` followed by
//! `SnapToGrid` will snap the locked Y component; reversing the order snaps
//! the X+Y movement first and then drops the X. Most users want the lock
//! first, then the snap.
//!
//! ```
//! use taino_dnd_core::{apply_chain, Axis, Modifier, Vector};
//!
//! let ms = [
//!     Modifier::RestrictToAxis(Axis::Y),
//!     Modifier::SnapToGrid { x: 8.0, y: 8.0 },
//! ];
//! assert_eq!(
//!     apply_chain(&ms, Vector { x: 14.0, y: 19.0 }),
//!     Vector { x: 0.0, y: 16.0 },
//! );
//! ```

/// A 2D displacement vector in CSS pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector {
    /// Horizontal component.
    pub x: f64,
    /// Vertical component.
    pub y: f64,
}

impl Vector {
    /// Construct a new [`Vector`].
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Cardinal axis used by [`Modifier::RestrictToAxis`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Horizontal axis. Drags are clamped to `y = 0`.
    X,
    /// Vertical axis. Drags are clamped to `x = 0`.
    Y,
}

/// Built-in modifiers for Stage 2.
///
/// Custom modifiers are out of scope until Stage 3's sensor-trait refactor —
/// users for whom these built-ins don't suffice should compose them with a
/// final pass in their own code (e.g. an `Effect` watching the state).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Modifier {
    /// Constrain movement to a single axis.
    RestrictToAxis(Axis),
    /// Snap movement to a grid. Components with `grid <= 0` are passed
    /// through unchanged (so `SnapToGrid { x: 8.0, y: 0.0 }` snaps only X).
    SnapToGrid {
        /// Horizontal grid step in CSS pixels.
        x: f64,
        /// Vertical grid step in CSS pixels.
        y: f64,
    },
}

impl Modifier {
    /// Apply this single modifier to `displacement`.
    pub fn apply(self, displacement: Vector) -> Vector {
        match self {
            Self::RestrictToAxis(Axis::X) => Vector::new(displacement.x, 0.0),
            Self::RestrictToAxis(Axis::Y) => Vector::new(0.0, displacement.y),
            Self::SnapToGrid { x, y } => {
                Vector::new(snap(displacement.x, x), snap(displacement.y, y))
            }
        }
    }
}

fn snap(value: f64, grid: f64) -> f64 {
    if grid > 0.0 {
        (value / grid).round() * grid
    } else {
        value
    }
}

/// Apply a chain of modifiers in order. The output of each modifier feeds
/// into the next.
pub fn apply_chain(modifiers: &[Modifier], displacement: Vector) -> Vector {
    modifiers.iter().copied().fold(displacement, |v, m| m.apply(v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restrict_to_axis_x_zeroes_y() {
        assert_eq!(
            Modifier::RestrictToAxis(Axis::X).apply(Vector::new(7.0, 11.0)),
            Vector::new(7.0, 0.0),
        );
    }

    #[test]
    fn restrict_to_axis_y_zeroes_x() {
        assert_eq!(
            Modifier::RestrictToAxis(Axis::Y).apply(Vector::new(7.0, 11.0)),
            Vector::new(0.0, 11.0),
        );
    }

    #[test]
    fn snap_to_grid_rounds_to_nearest_step() {
        let m = Modifier::SnapToGrid { x: 10.0, y: 10.0 };
        assert_eq!(m.apply(Vector::new(4.0, 7.0)), Vector::new(0.0, 10.0));
        assert_eq!(m.apply(Vector::new(15.0, 24.0)), Vector::new(20.0, 20.0));
        assert_eq!(m.apply(Vector::new(-3.0, -9.0)), Vector::new(0.0, -10.0));
    }

    #[test]
    fn snap_to_grid_with_zero_step_passes_through() {
        let m = Modifier::SnapToGrid { x: 0.0, y: 10.0 };
        assert_eq!(m.apply(Vector::new(13.7, 7.0)), Vector::new(13.7, 10.0));
    }

    #[test]
    fn snap_to_grid_with_negative_step_passes_through() {
        let m = Modifier::SnapToGrid { x: -5.0, y: -5.0 };
        assert_eq!(m.apply(Vector::new(3.0, 4.0)), Vector::new(3.0, 4.0));
    }

    #[test]
    fn empty_chain_is_identity() {
        assert_eq!(apply_chain(&[], Vector::new(3.0, 4.0)), Vector::new(3.0, 4.0));
    }

    #[test]
    fn chain_composes_left_to_right() {
        let chain = [Modifier::RestrictToAxis(Axis::Y), Modifier::SnapToGrid { x: 10.0, y: 10.0 }];
        assert_eq!(apply_chain(&chain, Vector::new(14.0, 19.0)), Vector::new(0.0, 20.0));
    }

    #[test]
    fn chain_order_matters() {
        let lock_then_snap =
            [Modifier::RestrictToAxis(Axis::Y), Modifier::SnapToGrid { x: 10.0, y: 10.0 }];
        let snap_then_lock =
            [Modifier::SnapToGrid { x: 10.0, y: 10.0 }, Modifier::RestrictToAxis(Axis::Y)];
        let v = Vector::new(14.0, 19.0);
        // Same final answer here, but assert by-step that we exercised both orders.
        assert_eq!(apply_chain(&lock_then_snap, v), Vector::new(0.0, 20.0));
        assert_eq!(apply_chain(&snap_then_lock, v), Vector::new(0.0, 20.0));

        // A case where order produces visibly different results: SnapToGrid
        // can move the value past a boundary the next lock would zero.
        let chain_a =
            [Modifier::SnapToGrid { x: 100.0, y: 0.0 }, Modifier::RestrictToAxis(Axis::X)];
        let chain_b =
            [Modifier::RestrictToAxis(Axis::X), Modifier::SnapToGrid { x: 100.0, y: 0.0 }];
        let v2 = Vector::new(40.0, 80.0);
        assert_eq!(apply_chain(&chain_a, v2), Vector::new(0.0, 0.0));
        assert_eq!(apply_chain(&chain_b, v2), Vector::new(0.0, 0.0));
    }
}
