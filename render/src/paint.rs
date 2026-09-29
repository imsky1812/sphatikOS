//! Colours, gradient stops and paints.

use crate::Rect;

/// A straight-alpha sRGB colour, each channel 0.0–1.0.
///
/// Blending happens in sRGB space, like the browser the prototype runs in,
/// so colours match the prototype value for value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color {
    /// Red.
    pub r: f32,
    /// Green.
    pub g: f32,
    /// Blue.
    pub b: f32,
    /// Opacity.
    pub a: f32,
}

impl Color {
    /// Opaque white.
    pub const WHITE: Color = Color::rgba(1.0, 1.0, 1.0, 1.0);
    /// Fully transparent.
    pub const TRANSPARENT: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);

    /// A colour from channel values.
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// An opaque colour from `0xRRGGBB`, as written in the prototype's CSS.
    pub const fn hex(rgb: u32) -> Self {
        Self::rgba(
            ((rgb >> 16) & 0xFF) as f32 / 255.0,
            ((rgb >> 8) & 0xFF) as f32 / 255.0,
            (rgb & 0xFF) as f32 / 255.0,
            1.0,
        )
    }

    /// The same colour with opacity `a`.
    pub const fn with_alpha(self, a: f32) -> Self {
        Self::rgba(self.r, self.g, self.b, a)
    }

    /// Channels as an array, for shader uniforms.
    pub const fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// Maximum colour stops in one gradient.
pub const MAX_STOPS: usize = 8;

/// Gradient colour stops. Fixed capacity, so building one never allocates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stops {
    colors: [Color; MAX_STOPS],
    offsets: [f32; MAX_STOPS],
    len: usize,
    premultiplied: bool,
}

impl Stops {
    /// Stops interpolated like SVG gradients (straight alpha), as the
    /// prototype's wallpapers use. Offsets must be ascending in 0.0–1.0; at
    /// most [`MAX_STOPS`] are kept.
    pub fn svg(stops: &[(f32, Color)]) -> Self {
        Self::build(stops, false)
    }

    /// Stops interpolated like CSS gradients (premultiplied alpha), as the
    /// prototype's glass fills use.
    pub fn css(stops: &[(f32, Color)]) -> Self {
        Self::build(stops, true)
    }

    fn build(stops: &[(f32, Color)], premultiplied: bool) -> Self {
        let mut out = Self {
            colors: [Color::TRANSPARENT; MAX_STOPS],
            offsets: [0.0; MAX_STOPS],
            len: 0,
            premultiplied,
        };
        let mut last = 0.0_f32;
        for &(offset, color) in stops.iter().take(MAX_STOPS) {
            // CSS and SVG clamp each offset to at least the previous one.
            last = offset.clamp(last, 1.0);
            out.offsets[out.len] = last;
            out.colors[out.len] = color;
            out.len += 1;
        }
        out
    }

    /// Number of stops.
    pub fn len(&self) -> usize {
        self.len
    }

    /// True if there are no stops.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Stop colours (unused slots are transparent).
    pub fn colors(&self) -> &[Color; MAX_STOPS] {
        &self.colors
    }

    /// Stop offsets (unused slots are 0).
    pub fn offsets(&self) -> &[f32; MAX_STOPS] {
        &self.offsets
    }

    /// True for CSS-style premultiplied interpolation.
    pub fn premultiplied(&self) -> bool {
        self.premultiplied
    }

    /// The colour at `t` (CPU reference of the shader's ramp, used in tests
    /// and for picking representative colours).
    pub fn sample(&self, t: f32) -> Color {
        if self.len == 0 {
            return Color::TRANSPARENT;
        }
        let t = t.clamp(0.0, 1.0);
        if t <= self.offsets[0] {
            return self.colors[0];
        }
        for i in 1..self.len {
            if t <= self.offsets[i] {
                let span = (self.offsets[i] - self.offsets[i - 1]).max(1e-6);
                let f = (t - self.offsets[i - 1]) / span;
                return self.mix(self.colors[i - 1], self.colors[i], f);
            }
        }
        self.colors[self.len - 1]
    }

    fn mix(&self, a: Color, b: Color, f: f32) -> Color {
        let lerp = |x: f32, y: f32| x + (y - x) * f;
        if !self.premultiplied {
            return Color::rgba(
                lerp(a.r, b.r),
                lerp(a.g, b.g),
                lerp(a.b, b.b),
                lerp(a.a, b.a),
            );
        }
        let alpha = lerp(a.a, b.a);
        if alpha <= 0.0 {
            return Color::TRANSPARENT;
        }
        let ch = |x: f32, y: f32| lerp(x * a.a, y * b.a) / alpha;
        Color::rgba(ch(a.r, b.r), ch(a.g, b.g), ch(a.b, b.b), alpha)
    }
}

/// How a shape is filled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Paint {
    /// One colour.
    Solid(Color),
    /// A linear gradient in the unit square of `frame` (SVG
    /// `objectBoundingBox` units): from `from` to `to`, both in 0.0–1.0 of
    /// the frame, which is stretched to the frame like SVG does.
    Linear {
        /// The box the gradient coordinates refer to.
        frame: Rect,
        /// Start point in frame units.
        from: [f32; 2],
        /// End point in frame units.
        to: [f32; 2],
        /// Colour stops.
        stops: Stops,
    },
    /// A radial (elliptical) gradient in frame units.
    Radial {
        /// The box the gradient coordinates refer to.
        frame: Rect,
        /// Centre in frame units.
        center: [f32; 2],
        /// Radii in frame units.
        radius: [f32; 2],
        /// Colour stops.
        stops: Stops,
    },
}

impl Paint {
    /// A CSS `linear-gradient(<angle>deg, …)` over `rect`: 0° points up,
    /// 90° right, 180° down, and the gradient line is long enough for the
    /// corners to reach the first and last stops, exactly as CSS defines it.
    /// Coordinates are in points (the frame is the unit square at the
    /// origin), so the colour bands stay perpendicular to the angle.
    pub fn css_linear(rect: Rect, angle_deg: f32, stops: Stops) -> Paint {
        let a = angle_deg.to_radians();
        let (dx, dy) = (a.sin(), -a.cos());
        let half = 0.5 * (rect.w * dx.abs() + rect.h * dy.abs());
        let (cx, cy) = (rect.x + 0.5 * rect.w, rect.y + 0.5 * rect.h);
        Paint::Linear {
            frame: Rect::new(0.0, 0.0, 1.0, 1.0),
            from: [cx - dx * half, cy - dy * half],
            to: [cx + dx * half, cy + dy * half],
            stops,
        }
    }
}

/// One rounded rectangle to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    /// Bounds in points.
    pub rect: Rect,
    /// Corner radius in points (clamped to half the shorter side).
    pub radius: f32,
    /// Fill.
    pub paint: Paint,
    /// Extra opacity, 0.0–1.0.
    pub opacity: f32,
}

impl Shape {
    /// A shape with full opacity.
    pub const fn new(rect: Rect, radius: f32, paint: Paint) -> Self {
        Self {
            rect,
            radius,
            paint,
            opacity: 1.0,
        }
    }

    /// Area the shape can touch, including one point of antialiasing.
    pub fn bounds(&self) -> Rect {
        self.rect.outset(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_matches_css() {
        let c = Color::hex(0x0B0A1C);
        assert_eq!(
            (c.r * 255.0, c.g * 255.0, c.b * 255.0, c.a),
            (11.0, 10.0, 28.0, 1.0)
        );
    }

    #[test]
    fn stops_interpolate_between_neighbours() {
        let s = Stops::svg(&[
            (0.0, Color::hex(0x000000)),
            (0.5, Color::hex(0xFFFFFF)),
            (1.0, Color::hex(0x000000)),
        ]);
        assert_eq!(s.sample(0.25).r, 0.5);
        assert_eq!(s.sample(0.5), Color::WHITE);
        assert_eq!(s.sample(-1.0), Color::hex(0x000000));
        assert_eq!(s.sample(2.0), Color::hex(0x000000));
    }

    #[test]
    fn svg_and_css_interpolation_differ_towards_transparent() {
        let stops = [
            (0.0, Color::WHITE),
            (1.0, Color::hex(0x000000).with_alpha(0.0)),
        ];
        // SVG (straight alpha) drifts towards black; CSS (premultiplied)
        // keeps the colour white while it fades.
        assert_eq!(Stops::svg(&stops).sample(0.5).r, 0.5);
        assert_eq!(Stops::css(&stops).sample(0.5).r, 1.0);
    }

    #[test]
    fn css_angles_follow_the_css_gradient_line() {
        let r = Rect::new(0.0, 0.0, 100.0, 50.0);
        let stops = Stops::css(&[(0.0, Color::WHITE), (1.0, Color::TRANSPARENT)]);
        let ends = |angle: f32| match Paint::css_linear(r, angle, stops) {
            Paint::Linear { from, to, .. } => (from, to),
            other => panic!("unexpected {other:?}"),
        };
        let close =
            |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4;
        // 180deg: top to bottom through the centre.
        let (f, t) = ends(180.0);
        assert!(
            close(f, [50.0, 0.0]) && close(t, [50.0, 50.0]),
            "{f:?} {t:?}"
        );
        // 90deg: left to right.
        let (f, t) = ends(90.0);
        assert!(
            close(f, [0.0, 25.0]) && close(t, [100.0, 25.0]),
            "{f:?} {t:?}"
        );
        // 135deg on a 100 x 50 box: length = (100 + 50) * sin 45.
        let (f, t) = ends(135.0);
        let len = ((t[0] - f[0]).powi(2) + (t[1] - f[1]).powi(2)).sqrt();
        assert!((len - 150.0 * std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-3);
    }

    #[test]
    fn offsets_never_go_backwards_and_extra_stops_are_dropped() {
        let many: Vec<(f32, Color)> = (0..12)
            .map(|i| (1.0 - i as f32 * 0.1, Color::WHITE))
            .collect();
        let s = Stops::css(&many);
        assert_eq!(s.len(), MAX_STOPS);
        assert!(s.offsets().windows(2).all(|w| w[0] <= w[1]));
    }
}
