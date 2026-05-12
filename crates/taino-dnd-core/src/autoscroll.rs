//! Pure auto-scroll math. Given a pointer position and a container's viewport
//! rect, compute how fast the container should scroll this frame.
//!
//! The binding layer (`taino-dnd-leptos`) wraps this in a `requestAnimationFrame`
//! loop and feeds it back into `scrollBy`.
//!
//! # Curve
//!
//! Speed ramps **linearly** from `0` at the inner edge of the threshold band
//! to `max_speed` at the container edge. Outside the band, speed is `0`.
//! Past the edge (pointer outside the container) the speed clamps to
//! `max_speed`.

use crate::{geometry::Rect, modifier::Vector};

/// Tuning knobs for auto-scroll.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AutoScrollConfig {
    /// When `false`, [`scroll_velocity`] always returns the zero vector.
    pub enabled: bool,
    /// Distance from the container edge, in CSS pixels, at which the linear
    /// speed ramp starts. Default `48.0`.
    pub threshold_px: f64,
    /// Maximum scroll, in CSS pixels per frame, when the pointer is at the
    /// container edge. Default `18.0` (roughly 1000 px/sec at 60 fps).
    pub max_speed_px: f64,
}

impl Default for AutoScrollConfig {
    fn default() -> Self {
        Self { enabled: true, threshold_px: 48.0, max_speed_px: 18.0 }
    }
}

/// Compute the scroll vector (in CSS pixels for this frame) for a container
/// of `rect` when the pointer is at `pointer`. Returns `(0, 0)` outside the
/// threshold band or when `config.enabled` is `false`.
///
/// # Examples
///
/// ```
/// use taino_dnd_core::{autoscroll::{scroll_velocity, AutoScrollConfig}, Point, Rect};
///
/// let cfg = AutoScrollConfig::default();
/// let viewport = Rect::new(0.0, 0.0, 1000.0, 800.0);
///
/// // Pointer in the center: no scrolling.
/// assert_eq!(scroll_velocity(Point::new(500.0, 400.0), viewport, cfg).y, 0.0);
///
/// // Pointer near bottom edge: positive (downward) y velocity.
/// let v = scroll_velocity(Point::new(500.0, 790.0), viewport, cfg);
/// assert!(v.y > 0.0);
/// ```
pub fn scroll_velocity(pointer: crate::Point, rect: Rect, config: AutoScrollConfig) -> Vector {
    if !config.enabled || config.threshold_px <= 0.0 {
        return Vector::default();
    }
    Vector::new(
        axis_velocity(pointer.x, rect.x, rect.x + rect.width, config),
        axis_velocity(pointer.y, rect.y, rect.y + rect.height, config),
    )
}

fn axis_velocity(p: f64, edge_lo: f64, edge_hi: f64, config: AutoScrollConfig) -> f64 {
    let t = config.threshold_px;
    let max = config.max_speed_px;
    if p < edge_lo + t {
        // Near (or past) the low edge → negative velocity.
        let depth = ((edge_lo + t) - p).min(t);
        -(depth / t) * max
    } else if p > edge_hi - t {
        // Near (or past) the high edge → positive velocity.
        let depth = (p - (edge_hi - t)).min(t);
        (depth / t) * max
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Point;

    fn cfg() -> AutoScrollConfig {
        AutoScrollConfig { enabled: true, threshold_px: 50.0, max_speed_px: 10.0 }
    }

    fn vp() -> Rect {
        Rect::new(0.0, 0.0, 1000.0, 800.0)
    }

    #[test]
    fn middle_of_viewport_is_zero() {
        let v = scroll_velocity(Point::new(500.0, 400.0), vp(), cfg());
        assert_eq!(v, Vector::default());
    }

    #[test]
    fn just_inside_threshold_is_zero() {
        // Threshold = 50; pointer at y = 50 → on the inner boundary, velocity 0.
        let v = scroll_velocity(Point::new(500.0, 50.0), vp(), cfg());
        assert!(v.y.abs() < f64::EPSILON);
        let v = scroll_velocity(Point::new(500.0, 750.0), vp(), cfg());
        assert!(v.y.abs() < f64::EPSILON);
    }

    #[test]
    fn at_edge_is_max_speed() {
        let v = scroll_velocity(Point::new(500.0, 0.0), vp(), cfg());
        assert!((v.y - -10.0).abs() < f64::EPSILON);
        let v = scroll_velocity(Point::new(500.0, 800.0), vp(), cfg());
        assert!((v.y - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn linear_ramp_at_midpoint() {
        // At y = 25 (halfway through the bottom of the band on the top side),
        // speed should be half of max in the negative direction.
        let v = scroll_velocity(Point::new(500.0, 25.0), vp(), cfg());
        assert!((v.y - -5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn past_edge_clamps_to_max() {
        // Pointer past the bottom edge → still max speed, doesn't accelerate.
        let v = scroll_velocity(Point::new(500.0, 900.0), vp(), cfg());
        assert!((v.y - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn both_axes_respond_independently() {
        // Pointer near top-right corner: negative y, positive x.
        let v = scroll_velocity(Point::new(990.0, 10.0), vp(), cfg());
        assert!(v.x > 0.0);
        assert!(v.y < 0.0);
    }

    #[test]
    fn disabled_returns_zero() {
        let c = AutoScrollConfig { enabled: false, ..cfg() };
        let v = scroll_velocity(Point::new(500.0, 0.0), vp(), c);
        assert_eq!(v, Vector::default());
    }

    #[test]
    fn zero_threshold_returns_zero() {
        let c = AutoScrollConfig { threshold_px: 0.0, ..cfg() };
        let v = scroll_velocity(Point::new(500.0, 0.0), vp(), c);
        assert_eq!(v, Vector::default());
    }
}
