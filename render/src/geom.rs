//! Rectangles in logical points.

/// An axis-aligned rectangle: top-left corner plus size, in points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width (never negative for a valid rectangle).
    pub w: f32,
    /// Height (never negative for a valid rectangle).
    pub h: f32,
}

impl Rect {
    /// A rectangle from its top-left corner and size.
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// Right edge.
    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    /// Bottom edge.
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    /// True if the rectangle covers no area.
    pub fn is_empty(&self) -> bool {
        !(self.w > 0.0 && self.h > 0.0)
    }

    /// The smallest rectangle containing both.
    pub fn union(&self, other: &Rect) -> Rect {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        Rect::new(
            x,
            y,
            self.right().max(other.right()) - x,
            self.bottom().max(other.bottom()) - y,
        )
    }

    /// The overlap of both, or `None` if they don't overlap.
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let r = Rect::new(
            x,
            y,
            self.right().min(other.right()) - x,
            self.bottom().min(other.bottom()) - y,
        );
        (!r.is_empty()).then_some(r)
    }

    /// Grows the rectangle by `by` on every side.
    pub fn outset(&self, by: f32) -> Rect {
        Rect::new(
            self.x - by,
            self.y - by,
            self.w + 2.0 * by,
            self.h + 2.0 * by,
        )
    }

    /// The whole-pixel rectangle `(x, y, w, h)` covering this one at `scale`
    /// pixels per point, clipped to a `width` x `height` pixel target.
    /// Returns `None` if nothing of it is on the target.
    pub fn to_pixels(&self, scale: f32, width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
        if self.is_empty() || !scale.is_finite() || scale <= 0.0 {
            return None;
        }
        let x0 = (self.x * scale).floor().max(0.0);
        let y0 = (self.y * scale).floor().max(0.0);
        let x1 = (self.right() * scale).ceil().min(width as f32);
        let y1 = (self.bottom() * scale).ceil().min(height as f32);
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        Some((x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_covers_both_and_ignores_empty() {
        let a = Rect::new(10.0, 10.0, 10.0, 10.0);
        let b = Rect::new(30.0, 5.0, 5.0, 5.0);
        assert_eq!(a.union(&b), Rect::new(10.0, 5.0, 25.0, 15.0));
        assert_eq!(a.union(&Rect::default()), a);
        assert_eq!(Rect::default().union(&b), b);
    }

    #[test]
    fn intersection_is_the_overlap() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert_eq!(
            a.intersection(&Rect::new(5.0, 5.0, 10.0, 10.0)),
            Some(Rect::new(5.0, 5.0, 5.0, 5.0))
        );
        assert_eq!(a.intersection(&Rect::new(10.0, 0.0, 5.0, 5.0)), None);
    }

    #[test]
    fn to_pixels_rounds_outwards_and_clips() {
        let r = Rect::new(1.2, 2.5, 10.0, 10.0);
        assert_eq!(r.to_pixels(2.0, 100, 100), Some((2, 5, 21, 20)));
        assert_eq!(
            Rect::new(-5.0, -5.0, 10.0, 10.0).to_pixels(1.0, 100, 100),
            Some((0, 0, 5, 5))
        );
        assert_eq!(
            Rect::new(200.0, 0.0, 10.0, 10.0).to_pixels(1.0, 100, 100),
            None
        );
        assert_eq!(
            Rect::new(0.0, 0.0, 10.0, 10.0).to_pixels(0.0, 100, 100),
            None
        );
    }
}
