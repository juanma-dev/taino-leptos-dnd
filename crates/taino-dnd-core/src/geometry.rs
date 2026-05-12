//! Geometric primitives used by the state machine and collision detection.
//!
//! Everything is in CSS pixels (the same coordinate system as
//! `Element::getBoundingClientRect` returns).

/// A 2D point in CSS pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: f64,
    /// Vertical coordinate.
    pub y: f64,
}

impl Point {
    /// Construct a new [`Point`].
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Euclidean distance squared. Cheaper than [`Self::distance`] when you only
    /// need to compare magnitudes (e.g. drag-threshold checks).
    pub fn distance_squared(self, other: Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx.mul_add(dx, dy * dy)
    }

    /// Euclidean distance.
    pub fn distance(self, other: Self) -> f64 {
        self.distance_squared(other).sqrt()
    }
}

/// An axis-aligned rectangle in CSS pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    /// Left edge.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// Width. Must be non-negative.
    pub width: f64,
    /// Height. Must be non-negative.
    pub height: f64,
}

impl Rect {
    /// Construct a new [`Rect`].
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }

    /// Geometric center.
    pub fn center(self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// `true` when `point` lies within `self` (edges inclusive).
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }

    /// `true` when `self` and `other` overlap on both axes (edges inclusive).
    pub fn intersects(self, other: Self) -> bool {
        self.x <= other.x + other.width
            && self.x + self.width >= other.x
            && self.y <= other.y + other.height
            && self.y + self.height >= other.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_distance_zero_to_self() {
        let p = Point::new(3.0, 4.0);
        assert!((p.distance(p)).abs() < f64::EPSILON);
    }

    #[test]
    fn point_distance_3_4_5() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(3.0, 4.0);
        assert!((a.distance(b) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn rect_center_is_midpoint() {
        let r = Rect::new(10.0, 20.0, 100.0, 80.0);
        assert_eq!(r.center(), Point::new(60.0, 60.0));
    }

    #[test]
    fn rect_contains_inside_and_edge() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(r.contains(Point::new(5.0, 5.0)));
        assert!(r.contains(Point::new(0.0, 0.0)));
        assert!(r.contains(Point::new(10.0, 10.0)));
        assert!(!r.contains(Point::new(10.1, 5.0)));
    }

    #[test]
    fn rect_intersects_overlap_and_touch() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 5.0, 10.0, 10.0);
        let c = Rect::new(10.0, 0.0, 10.0, 10.0); // edge-touching
        let d = Rect::new(20.0, 20.0, 1.0, 1.0); // disjoint
        assert!(a.intersects(b));
        assert!(a.intersects(c));
        assert!(!a.intersects(d));
    }
}
