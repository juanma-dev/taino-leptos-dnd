//! Modifiers that transform a drag displacement before it's rendered.
//!
//! A modifier is a function `(Vector, &ModifierContext) → Vector` that runs on
//! every pointermove while a drag is active. The state machine itself sees
//! raw pointer positions (so the click-vs-drag threshold isn't broken by an
//! axis lock), but the visual transform and the collision-detection point
//! both go through the modifier chain.
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
//! use taino_dnd_core::{apply_chain, Axis, Modifier, ModifierContext, Vector};
//!
//! let ms = [
//!     Modifier::RestrictToAxis(Axis::Y),
//!     Modifier::SnapToGrid { x: 8.0, y: 8.0 },
//! ];
//! assert_eq!(
//!     apply_chain(&ms, Vector { x: 14.0, y: 19.0 }, &ModifierContext::default()),
//!     Vector { x: 0.0, y: 16.0 },
//! );
//! ```

use crate::geometry::Rect;

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

/// Read-only inputs available to [`Modifier::apply`] beyond the displacement
/// itself.
///
/// Modifiers that don't need either rect ignore the context (e.g.
/// `RestrictToAxis`, `SnapToGrid`). [`Modifier::RestrictToParent`] needs both;
/// if either is `None` it returns the displacement unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ModifierContext {
    /// The bounding rect of the container the dragged element must stay
    /// within, in viewport (CSS pixel) coordinates.
    pub container: Option<Rect>,
    /// The bounding rect of the dragged element at the start of the drag,
    /// in viewport (CSS pixel) coordinates.
    pub element: Option<Rect>,
}

/// Built-in modifiers.
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
    /// Keep the dragged element inside [`ModifierContext::container`].
    ///
    /// Requires both `container` and `element` rects in the context. When
    /// either is `None`, this modifier is a no-op.
    RestrictToParent,
}

impl Modifier {
    /// Apply this single modifier to `displacement`.
    pub fn apply(self, displacement: Vector, ctx: &ModifierContext) -> Vector {
        match self {
            Self::RestrictToAxis(Axis::X) => Vector::new(displacement.x, 0.0),
            Self::RestrictToAxis(Axis::Y) => Vector::new(0.0, displacement.y),
            Self::SnapToGrid { x, y } => {
                Vector::new(snap(displacement.x, x), snap(displacement.y, y))
            }
            Self::RestrictToParent => restrict_to_parent(displacement, ctx),
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

fn restrict_to_parent(displacement: Vector, ctx: &ModifierContext) -> Vector {
    let (Some(container), Some(element)) = (ctx.container, ctx.element) else {
        return displacement;
    };
    // The element's left edge after applying displacement.x is element.x + dx.
    // For the element to stay inside `container` on the X axis:
    //   container.x <= element.x + dx
    //   element.x + element.width + dx <= container.x + container.width
    // Therefore:
    //   x_min = container.x - element.x
    //   x_max = container.x + container.width - (element.x + element.width)
    let x_min = container.x - element.x;
    let x_max = (container.x + container.width) - (element.x + element.width);
    let y_min = container.y - element.y;
    let y_max = (container.y + container.height) - (element.y + element.height);
    Vector::new(clamp(displacement.x, x_min, x_max), clamp(displacement.y, y_min, y_max))
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    // If the element is *larger* than the container on one axis, min > max.
    // In that case, freeze the element at its current position on that axis.
    if min > max {
        0.0
    } else if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Apply a chain of modifiers in order. The output of each modifier feeds
/// into the next.
pub fn apply_chain(modifiers: &[Modifier], displacement: Vector, ctx: &ModifierContext) -> Vector {
    modifiers.iter().copied().fold(displacement, |v, m| m.apply(v, ctx))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ModifierContext {
        ModifierContext::default()
    }

    #[test]
    fn restrict_to_axis_x_zeroes_y() {
        assert_eq!(
            Modifier::RestrictToAxis(Axis::X).apply(Vector::new(7.0, 11.0), &ctx()),
            Vector::new(7.0, 0.0),
        );
    }

    #[test]
    fn restrict_to_axis_y_zeroes_x() {
        assert_eq!(
            Modifier::RestrictToAxis(Axis::Y).apply(Vector::new(7.0, 11.0), &ctx()),
            Vector::new(0.0, 11.0),
        );
    }

    #[test]
    fn snap_to_grid_rounds_to_nearest_step() {
        let m = Modifier::SnapToGrid { x: 10.0, y: 10.0 };
        assert_eq!(m.apply(Vector::new(4.0, 7.0), &ctx()), Vector::new(0.0, 10.0));
        assert_eq!(m.apply(Vector::new(15.0, 24.0), &ctx()), Vector::new(20.0, 20.0));
        assert_eq!(m.apply(Vector::new(-3.0, -9.0), &ctx()), Vector::new(0.0, -10.0));
    }

    #[test]
    fn snap_to_grid_with_zero_step_passes_through() {
        let m = Modifier::SnapToGrid { x: 0.0, y: 10.0 };
        assert_eq!(m.apply(Vector::new(13.7, 7.0), &ctx()), Vector::new(13.7, 10.0));
    }

    #[test]
    fn snap_to_grid_with_negative_step_passes_through() {
        let m = Modifier::SnapToGrid { x: -5.0, y: -5.0 };
        assert_eq!(m.apply(Vector::new(3.0, 4.0), &ctx()), Vector::new(3.0, 4.0));
    }

    #[test]
    fn empty_chain_is_identity() {
        assert_eq!(apply_chain(&[], Vector::new(3.0, 4.0), &ctx()), Vector::new(3.0, 4.0));
    }

    #[test]
    fn chain_composes_left_to_right() {
        let chain = [Modifier::RestrictToAxis(Axis::Y), Modifier::SnapToGrid { x: 10.0, y: 10.0 }];
        assert_eq!(apply_chain(&chain, Vector::new(14.0, 19.0), &ctx()), Vector::new(0.0, 20.0));
    }

    #[test]
    fn restrict_to_parent_is_no_op_without_rects() {
        let v = Vector::new(50.0, 50.0);
        assert_eq!(Modifier::RestrictToParent.apply(v, &ModifierContext::default()), v);
    }

    #[test]
    fn restrict_to_parent_with_partial_rects_is_no_op() {
        let v = Vector::new(50.0, 50.0);
        let c1 =
            ModifierContext { container: Some(Rect::new(0.0, 0.0, 100.0, 100.0)), element: None };
        let c2 =
            ModifierContext { container: None, element: Some(Rect::new(0.0, 0.0, 10.0, 10.0)) };
        assert_eq!(Modifier::RestrictToParent.apply(v, &c1), v);
        assert_eq!(Modifier::RestrictToParent.apply(v, &c2), v);
    }

    #[test]
    fn restrict_to_parent_passes_through_unchanged_when_in_bounds() {
        let ctx = ModifierContext {
            container: Some(Rect::new(0.0, 0.0, 200.0, 200.0)),
            element: Some(Rect::new(50.0, 50.0, 20.0, 20.0)),
        };
        // The element is at x=50, width=20, so right edge at 70. Container right at 200.
        // It can move dx up to 200 - 70 = 130, and dx_min = 0 - 50 = -50.
        let v = Vector::new(30.0, 40.0);
        assert_eq!(Modifier::RestrictToParent.apply(v, &ctx), v);
    }

    #[test]
    fn restrict_to_parent_clamps_positive_displacement() {
        let ctx = ModifierContext {
            container: Some(Rect::new(0.0, 0.0, 100.0, 100.0)),
            element: Some(Rect::new(50.0, 50.0, 20.0, 20.0)),
        };
        // dx_max = 100 - 70 = 30; dy_max = 100 - 70 = 30
        let v = Vector::new(80.0, 90.0);
        assert_eq!(Modifier::RestrictToParent.apply(v, &ctx), Vector::new(30.0, 30.0));
    }

    #[test]
    fn restrict_to_parent_clamps_negative_displacement() {
        let ctx = ModifierContext {
            container: Some(Rect::new(0.0, 0.0, 100.0, 100.0)),
            element: Some(Rect::new(50.0, 50.0, 20.0, 20.0)),
        };
        // dx_min = 0 - 50 = -50
        let v = Vector::new(-100.0, -100.0);
        assert_eq!(Modifier::RestrictToParent.apply(v, &ctx), Vector::new(-50.0, -50.0));
    }

    #[test]
    fn restrict_to_parent_freezes_axis_when_element_larger_than_container() {
        // Element wider than container — can't move on X without something
        // sticking out. We freeze X movement to 0 in that case.
        let ctx = ModifierContext {
            container: Some(Rect::new(0.0, 0.0, 100.0, 200.0)),
            element: Some(Rect::new(-10.0, 50.0, 150.0, 20.0)),
        };
        let v = Vector::new(30.0, 10.0);
        assert_eq!(Modifier::RestrictToParent.apply(v, &ctx), Vector::new(0.0, 10.0));
    }

    #[test]
    fn restrict_to_parent_chains_with_axis_lock() {
        let ctx = ModifierContext {
            container: Some(Rect::new(0.0, 0.0, 100.0, 100.0)),
            element: Some(Rect::new(50.0, 50.0, 20.0, 20.0)),
        };
        let chain = [Modifier::RestrictToAxis(Axis::Y), Modifier::RestrictToParent];
        let v = Vector::new(999.0, 999.0);
        // Axis lock first → (0, 999); RestrictToParent then clamps y to dy_max = 30.
        assert_eq!(apply_chain(&chain, v, &ctx), Vector::new(0.0, 30.0));
    }
}
