//! Damage tracking: which part of the screen must be redrawn.

use crate::Rect;

/// Collects the areas that changed since the last frame. A frame with no
/// damage is skipped entirely, so an idle screen costs nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Damage {
    region: Option<Rect>,
}

impl Damage {
    /// No damage.
    pub const fn new() -> Self {
        Self { region: None }
    }

    /// Marks `rect` as changed. Regions are merged into one bounding box,
    /// which keeps this allocation-free and the scissor a single rectangle.
    pub fn add(&mut self, rect: Rect) {
        if rect.is_empty() {
            return;
        }
        self.region = Some(match self.region {
            Some(r) => r.union(&rect),
            None => rect,
        });
    }

    /// Marks the whole `canvas` as changed (first frame, resize, wallpaper).
    pub fn add_all(&mut self, canvas: Rect) {
        self.add(canvas);
    }

    /// True if something must be redrawn.
    pub fn is_damaged(&self) -> bool {
        self.region.is_some()
    }

    /// Returns the damaged region, if any, and resets to no damage.
    pub fn take(&mut self) -> Option<Rect> {
        self.region.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_clean_and_merges_regions() {
        let mut d = Damage::new();
        assert!(!d.is_damaged());
        d.add(Rect::new(0.0, 0.0, 10.0, 10.0));
        d.add(Rect::new(50.0, 50.0, 10.0, 10.0));
        assert_eq!(d.take(), Some(Rect::new(0.0, 0.0, 60.0, 60.0)));
        assert_eq!(d.take(), None, "take resets");
    }

    #[test]
    fn empty_rectangles_are_ignored() {
        let mut d = Damage::new();
        d.add(Rect::new(5.0, 5.0, 0.0, 10.0));
        assert!(!d.is_damaged());
    }
}
