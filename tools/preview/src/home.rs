//! The home-screen mock-up for `sphatik-preview` (reference §9).
//!
//! Composes the prototype home over the wallpaper: two glass widgets, a 4-wide
//! app grid with **placeholder** tinted-tile icons and labels, page dots, a
//! search pill and a dock. Glass panels go through the `GlassRenderer`; the
//! tiles, dots and sun are `Shape`s drawn with `Renderer::draw_onto`; the text
//! uses the interim `TextRenderer`. Icons are deliberately placeholders — the
//! real icon set is a later pass.

use std::error::Error;

use sphatik_render::glow;
use sphatik_render::text::{self, OUTFIT_EXTRALIGHT, OUTFIT_LIGHT};
use sphatik_render::{Color, GlassPanel, Paint, Rect, Shape, Stops, TextRenderer};

/// Logical canvas width in points (the prototype's 393 pt screen).
const CW: f32 = 393.0;

/// An app on the home grid: a label and a placeholder tile tint.
struct AppIcon {
    label: &'static str,
    tint: u32,
}

/// The twelve home-grid apps, in the prototype's default order.
const APPS: [AppIcon; 12] = [
    AppIcon {
        label: "Calendar",
        tint: 0xFF6B6B,
    },
    AppIcon {
        label: "Clock",
        tint: 0x4DABF7,
    },
    AppIcon {
        label: "Weather",
        tint: 0xFFA94D,
    },
    AppIcon {
        label: "Maps",
        tint: 0x51CF66,
    },
    AppIcon {
        label: "Gallery",
        tint: 0xCC5DE8,
    },
    AppIcon {
        label: "Camera",
        tint: 0x868E96,
    },
    AppIcon {
        label: "Notes",
        tint: 0xFFD43B,
    },
    AppIcon {
        label: "Files",
        tint: 0x3BC9DB,
    },
    AppIcon {
        label: "Calculator",
        tint: 0x748FFC,
    },
    AppIcon {
        label: "Store",
        tint: 0xF783AC,
    },
    AppIcon {
        label: "Music",
        tint: 0xA78BFA,
    },
    AppIcon {
        label: "Mail",
        tint: 0x38D9A9,
    },
];

/// The four dock app tints (phone, messages, browser, settings).
const DOCK: [u32; 4] = [0x51CF66, 0x4DABF7, 0xFFA94D, 0x868E96];

/// A line of text positioned by its centre-x and top-y in points.
struct Text {
    tr: TextRenderer,
    content: String,
    font: &'static [u8],
    size: f32,
    cx: f32,
    top: f32,
    color: Color,
}

/// The home screen: glass panels, shapes and text, laid out once per scale.
pub struct Home {
    texts: Vec<Text>,
    scale: f32,
}

impl Home {
    /// Builds the text renderers and the fixed layout.
    pub fn new(make_glow: &dyn Fn() -> glow::Context) -> Result<Self, Box<dyn Error>> {
        let mut texts = Vec::new();
        let mut push = |content: &str,
                        font: &'static [u8],
                        size: f32,
                        cx: f32,
                        top: f32,
                        color: Color|
         -> Result<(), Box<dyn Error>> {
            texts.push(Text {
                tr: TextRenderer::new(make_glow())?,
                content: content.to_string(),
                font,
                size,
                cx,
                top,
                color,
            });
            Ok(())
        };

        let white = |a: f32| Color::rgba(1.0, 1.0, 1.0, a);

        // Weather widget (18,66,171.5,162): kicker, big temp, condition.
        push("WEATHER", OUTFIT_LIGHT, 11.0, 18.0 + 40.0, 82.0, white(0.6))?;
        push(
            "31°",
            OUTFIT_EXTRALIGHT,
            54.0,
            18.0 + 44.0,
            104.0,
            white(0.94),
        )?;
        push(
            "Mostly sunny",
            OUTFIT_LIGHT,
            13.0,
            18.0 + 70.0,
            196.0,
            white(0.88),
        )?;

        // Up-next widget (203.5,66,171.5,162): kicker + two events.
        let ux = 203.5;
        push("UP NEXT", OUTFIT_LIGHT, 11.0, ux + 50.0, 82.0, white(0.6))?;
        push(
            "Project review",
            OUTFIT_LIGHT,
            13.0,
            ux + 66.0,
            112.0,
            white(0.9),
        )?;
        push("4:00 PM", OUTFIT_LIGHT, 11.0, ux + 44.0, 130.0, white(0.6))?;
        push("Lab viva", OUTFIT_LIGHT, 13.0, ux + 45.0, 160.0, white(0.9))?;
        push("5:30 PM", OUTFIT_LIGHT, 11.0, ux + 44.0, 178.0, white(0.6))?;

        // App-grid labels, under each tile.
        for (i, app) in APPS.iter().enumerate() {
            let (cx, _, label_top) = grid_cell(i);
            push(app.label, OUTFIT_LIGHT, 11.5, cx, label_top, white(0.9))?;
        }

        // Search pill.
        push(
            "Search or ask",
            OUTFIT_LIGHT,
            14.0,
            CW * 0.5,
            662.0,
            white(0.72),
        )?;

        Ok(Self { texts, scale: 0.0 })
    }

    /// The glass panels: two widgets, the search pill and the dock.
    pub fn glass_panels(&self) -> Vec<GlassPanel> {
        vec![
            GlassPanel::regular(Rect::new(18.0, 66.0, 171.5, 162.0), 30.0),
            GlassPanel::regular(Rect::new(203.5, 66.0, 171.5, 162.0), 30.0),
            GlassPanel::regular(Rect::new(106.5, 650.0, 180.0, 40.0), 20.0),
            GlassPanel::regular(Rect::new(16.0, 732.0, 361.0, 94.0), 36.0),
        ]
    }

    /// The solid shapes: app tiles, dock tiles, the sun disc and page dots.
    pub fn shapes(&self) -> Vec<Shape> {
        let mut shapes = Vec::new();

        // The weather widget's sun disc (radial gradient circle).
        let sun = Rect::new(143.5, 108.0, 30.0, 30.0);
        shapes.push(Shape::new(
            sun,
            15.0,
            Paint::Radial {
                frame: sun,
                center: [0.35, 0.35],
                radius: [0.9, 0.9],
                stops: Stops::svg(&[
                    (0.0, Color::hex(0xFFF3C4)),
                    (0.6, Color::hex(0xFFC24D)),
                    (1.0, Color::hex(0xFF9F43)),
                ]),
            },
        ));

        // App-grid tiles.
        for (i, app) in APPS.iter().enumerate() {
            let (cx, icon_top, _) = grid_cell(i);
            let rect = Rect::new(cx - 31.0, icon_top, 62.0, 62.0);
            shapes.push(Shape::new(rect, 15.0, tile_paint(rect, app.tint)));
        }

        // Dock tiles, spaced across the dock glass.
        for (k, tint) in DOCK.iter().enumerate() {
            let centre = 16.0 + (k as f32 + 0.5) * (361.0 / 4.0);
            let rect = Rect::new(centre - 27.0, 752.0, 54.0, 54.0);
            shapes.push(Shape::new(rect, 14.0, tile_paint(rect, *tint)));
        }

        // Page dots (two pages).
        for k in 0..2 {
            let x = CW * 0.5 - 9.5 + k as f32 * 13.0;
            let alpha = if k == 0 { 1.0 } else { 0.4 };
            shapes.push(Shape::new(
                Rect::new(x, 612.0, 6.0, 6.0),
                3.0,
                Paint::Solid(Color::rgba(1.0, 1.0, 1.0, alpha)),
            ));
        }

        shapes
    }

    /// (Re)rasterises the text for the current scale.
    pub fn layout(&mut self, px_width: u32) {
        let scale = px_width as f32 / CW;
        if (scale - self.scale).abs() < 0.001 {
            return;
        }
        self.scale = scale;
        for t in &mut self.texts {
            let bmp = text::rasterize(t.font, &t.content, t.size * scale, 0.2 * scale, false);
            t.tr.set_bitmap(&bmp);
        }
    }

    /// Draws the text onto the bound framebuffer, faded by `opacity`.
    pub fn draw_text(&self, w: u32, h: u32, opacity: f32) {
        if opacity <= 0.0 {
            return;
        }
        let scale = w as f32 / CW;
        for t in &self.texts {
            let x = t.cx * scale - t.tr.width() as f32 * 0.5;
            let y = t.top * scale;
            let c = Color::rgba(t.color.r, t.color.g, t.color.b, t.color.a * opacity);
            t.tr.draw(w, h, x, y, c);
        }
    }

    /// Releases the GL objects.
    pub fn destroy(self) {
        for t in self.texts {
            t.tr.destroy();
        }
    }
}

/// The grid cell for app `i`: (centre-x, icon-top, label-top) in points.
fn grid_cell(i: usize) -> (f32, f32, f32) {
    let col = (i % 4) as f32;
    let row = (i / 4) as f32;
    let cell = (CW - 32.0) / 4.0; // 90.25
    let cx = 16.0 + col * cell + cell * 0.5;
    let icon_top = 246.0 + row * 101.5;
    let label_top = icon_top + 62.0 + 7.0;
    (cx, icon_top, label_top)
}

/// A soft top-to-bottom gradient tile from the tint, as a placeholder icon.
fn tile_paint(rect: Rect, tint: u32) -> Paint {
    let base = Color::hex(tint);
    let lighter = Color::rgba(
        (base.r + 0.18).min(1.0),
        (base.g + 0.18).min(1.0),
        (base.b + 0.18).min(1.0),
        1.0,
    );
    Paint::css_linear(rect, 160.0, Stops::svg(&[(0.0, lighter), (1.0, base)]))
}
