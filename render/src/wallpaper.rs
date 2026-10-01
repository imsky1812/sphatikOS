//! The four built-in wallpapers, built as GPU vector shapes.
//!
//! A display list ([`Wallpaper`]) of filled meshes and grouped (blurred or
//! blended) layers, transcribed from the prototype's generators (WALLS,
//! RIBBONS, `ribbonMarkup`, `crystalMarkup`) and
//! `docs/design/prototype-reference.md` section 13. The geometry is
//! tessellated once here; the GPU plays the list back (see the renderer).
//!
//! Grain and the time-of-day tint are applied later (living wallpapers); the
//! references are grain-free so the output can be checked exactly.

use crate::vector::{self, Affine, Mesh};
use crate::{Color, Paint, Rect, Stops};

/// The whole 393 x 852 pt wallpaper canvas.
pub const CANVAS: Rect = Rect::new(0.0, 0.0, 393.0, 852.0);

/// Which built-in wallpaper.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Wall {
    /// Liquid Aurora (default): silky ribbons on deep ink.
    Aurora,
    /// Obsidian: the same ribbons mirrored, in graphite.
    Obsidian,
    /// Quartz Dawn: a lit crystal on a dusk sky.
    Dawn,
    /// Amethyst: the crystal on a violet night sky.
    Night,
}

impl Wall {
    /// Every wallpaper, in the prototype's order.
    pub const ALL: [Wall; 4] = [Wall::Aurora, Wall::Obsidian, Wall::Dawn, Wall::Night];

    /// The storage key the prototype uses (also the SVG file stem).
    pub fn key(self) -> &'static str {
        match self {
            Wall::Aurora => "aurora",
            Wall::Obsidian => "obsidian",
            Wall::Dawn => "dawn",
            Wall::Night => "night",
        }
    }

    /// Builds the display list for this wallpaper.
    pub fn build(self) -> Wallpaper {
        match self {
            Wall::Aurora => ribbon(AURORA, false),
            Wall::Obsidian => ribbon(OBSIDIAN, true),
            Wall::Dawn => crystal(DAWN),
            Wall::Night => crystal(NIGHT),
        }
    }
}

/// How a group composites onto what is below.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blend {
    /// Source-over (the default).
    Normal,
    /// Screen (lightens), for the crystal's inner glow.
    Screen,
}

/// One filled mesh with a paint and an extra opacity.
#[derive(Clone, Debug, PartialEq)]
pub struct Fill {
    /// Triangles to draw.
    pub mesh: Mesh,
    /// How to colour them.
    pub paint: Paint,
    /// Element opacity, 0.0–1.0.
    pub opacity: f32,
}

/// A group rendered to its own layer, then blurred and composited. The
/// prototype never nests these, so a group holds only plain fills.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    /// Fills drawn into the layer, in order.
    pub fills: Vec<Fill>,
    /// Gaussian blur radius in points (0 for none).
    pub blur: f32,
    /// Group opacity applied when compositing.
    pub opacity: f32,
    /// How the layer composites onto what is below.
    pub blend: Blend,
}

/// One step of a wallpaper.
// A `Fill` carries a `Paint` (and its gradient stops), so it is larger than a
// `Group` header. The list is built once at load time, never per frame, so the
// size difference does not matter.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// A plain source-over fill.
    Fill(Fill),
    /// A blurred or blended group.
    Group(Group),
}

/// A wallpaper as an ordered display list, bottom layer first.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Wallpaper {
    /// Draw operations in back-to-front order.
    pub ops: Vec<Op>,
}

// ---------------------------------------------------------------------------
// Palettes (prototype WALLS, reference section 13).
// ---------------------------------------------------------------------------

struct RibbonPalette {
    bg: [u32; 3],
    rib: [[u32; 3]; 4],
    hl: [u32; 3],
    flip: bool,
}

const AURORA: &RibbonPalette = &RibbonPalette {
    bg: [0x0B0A1C, 0x06050E, 0x15113A],
    rib: [
        [0x231C6E, 0x3B2C9E, 0x5B47D6],
        [0x6D5BFF, 0x43C6F5, 0xA6F4E8],
        [0xFF6FA8, 0xFFA982, 0xFFD9A0],
        [0x8BE9FF, 0xC2B2FF, 0xFFFFFF],
    ],
    hl: [0xFFFFFF, 0xBFF6FF, 0xFFC6E6],
    flip: false,
};

const OBSIDIAN: &RibbonPalette = &RibbonPalette {
    bg: [0x0C0C12, 0x040406, 0x15151D],
    rib: [
        [0x0E0E14, 0x1C1C26, 0x2A2A36],
        [0x1A1A24, 0x55556A, 0x22222C],
        [0x24242F, 0x9A9AB2, 0x2C2C38],
        [0x3A3A4A, 0xE4E4F0, 0x5A5A6A],
    ],
    hl: [0xFFFFFF, 0x9FF0F5, 0xD6C6FF],
    flip: true,
};

struct CrystalPalette {
    sky: [u32; 4],
    b: [u32; 3],
}

const DAWN: &CrystalPalette = &CrystalPalette {
    sky: [0x0F1640, 0x34307A, 0xA8739E, 0xF4C2B0],
    b: [0x4FD8D0, 0xFF8FB7, 0x8B6CFF],
};

const NIGHT: &CrystalPalette = &CrystalPalette {
    sky: [0x06061A, 0x1B1450, 0x4A2A8C, 0xA064CF],
    b: [0x6C4BFF, 0xFF6FB5, 0x3FC8FF],
};

// ---------------------------------------------------------------------------
// Shared geometry.
// ---------------------------------------------------------------------------

/// The four ribbon bodies; index 0 has no highlighted top edge.
const RIBBONS: [(&str, Option<&str>); 4] = [
    (
        "M-40 900 L-40 830 C 110 730, 250 905, 440 700 L 440 900 Z",
        None,
    ),
    (
        "M-40 720 C 90 580, 215 840, 440 540 L 440 640 C 235 930, 80 690, -40 820 Z",
        Some("M-40 720 C 90 580, 215 840, 440 540"),
    ),
    (
        "M-40 575 C 120 480, 195 715, 440 415 L 440 470 C 215 780, 110 555, -40 640 Z",
        Some("M-40 575 C 120 480, 195 715, 440 415"),
    ),
    (
        "M-40 485 C 140 395, 205 615, 440 318 L 440 336 C 220 640, 130 432, -40 510 Z",
        Some("M-40 485 C 140 395, 205 615, 440 318"),
    ),
];

/// The five crystal facets: polygon points and which facet gradient fills it.
const FACETS: [(&[(f32, f32)], u8); 5] = [
    (
        &[
            (150.0, 440.0),
            (142.0, 442.0),
            (86.0, 560.0),
            (118.0, 410.0),
        ],
        2,
    ),
    (
        &[
            (142.0, 440.0),
            (212.0, 690.0),
            (136.0, 770.0),
            (88.0, 562.0),
        ],
        4,
    ),
    (
        &[
            (268.0, 420.0),
            (322.0, 560.0),
            (270.0, 735.0),
            (212.0, 690.0),
        ],
        3,
    ),
    (
        &[
            (268.0, 420.0),
            (306.0, 352.0),
            (342.0, 520.0),
            (322.0, 560.0),
        ],
        2,
    ),
    (
        &[
            (196.0, 186.0),
            (268.0, 420.0),
            (212.0, 690.0),
            (142.0, 440.0),
        ],
        1,
    ),
];

/// Sparkles: centre x, y, scale and opacity.
const SPARKLES: [(f32, f32, f32, f32); 6] = [
    (268.0, 420.0, 0.9, 0.9),
    (142.0, 440.0, 0.6, 0.7),
    (306.0, 352.0, 0.5, 0.7),
    (70.0, 150.0, 0.45, 0.5),
    (330.0, 250.0, 0.35, 0.45),
    (212.0, 690.0, 0.7, 0.8),
];

const SPARKLE_PATH: &str = "M0-6 1.1-1.1 6 0 1.1 1.1 0 6-1.1 1.1-6 0-1.1-1.1Z";

// ---------------------------------------------------------------------------
// Builders.
// ---------------------------------------------------------------------------

fn white(a: f32) -> Color {
    Color::WHITE.with_alpha(a)
}

/// A full-canvas quad, for background gradients.
fn canvas_mesh() -> Mesh {
    Mesh {
        vertices: vec![
            [CANVAS.x, CANVAS.y],
            [CANVAS.right(), CANVAS.y],
            [CANVAS.x, CANVAS.bottom()],
            [CANVAS.right(), CANVAS.y],
            [CANVAS.right(), CANVAS.bottom()],
            [CANVAS.x, CANVAS.bottom()],
        ],
        indices: vec![0, 1, 2, 3, 4, 5],
        bounds: CANVAS,
    }
}

fn solid_fill(mesh: Mesh, color: Color, opacity: f32) -> Fill {
    Fill {
        mesh,
        paint: Paint::Solid(color),
        opacity,
    }
}

/// `objectBoundingBox` linear gradient over a mesh's own bounds.
fn linear(mesh: &Mesh, from: [f32; 2], to: [f32; 2], stops: Stops) -> Paint {
    Paint::Linear {
        frame: mesh.bounds,
        from,
        to,
        stops,
    }
}

fn ribbon(p: &RibbonPalette, _flip_only_marker: bool) -> Wallpaper {
    let tf = if p.flip {
        // translate(393 0) scale(-1 1): mirror the body around x = 196.5.
        Affine::translate(393.0, 0.0).then(Affine::scale(-1.0, 1.0))
    } else {
        Affine::IDENTITY
    };
    // SVG paints each objectBoundingBox gradient in the path's local space,
    // then the group transform mirrors the painted result. Baking the mirror
    // into the geometry flips the bounding box, so a gradient's x endpoints
    // must be flipped (u -> 1 - u) to match.
    let fx = |x: f32| if p.flip { 1.0 - x } else { x };
    let mut ops = Vec::new();

    // 1. Background gradient (outside the mirrored body).
    let bg = canvas_mesh();
    let bg_stops = Stops::svg(&[
        (0.0, Color::hex(p.bg[0])),
        (0.6, Color::hex(p.bg[1])),
        (1.0, Color::hex(p.bg[2])),
    ]);
    let bg_paint = linear(&bg, [0.0, 0.0], [0.3, 1.0], bg_stops);
    ops.push(Op::Fill(Fill {
        mesh: bg,
        paint: bg_paint,
        opacity: 1.0,
    }));

    // Ribbon fill gradients: linear (0,0.2) -> (1,0), three stops.
    let rg = |i: usize| {
        Stops::svg(&[
            (0.0, Color::hex(p.rib[i][0])),
            (0.55, Color::hex(p.rib[i][1])),
            (1.0, Color::hex(p.rib[i][2])),
        ])
    };
    // Satin shade overlay: vertical, white at top fading to black at bottom.
    let shade = || {
        Stops::svg(&[
            (0.0, white(0.30)),
            (0.3, white(0.0)),
            (0.6, Color::hex(0x000000).with_alpha(0.0)),
            (1.0, Color::hex(0x000000).with_alpha(0.5)),
        ])
    };
    // Mirror-edge highlight: horizontal.
    let hl_stops = Stops::svg(&[
        (0.0, Color::hex(p.hl[0]).with_alpha(0.0)),
        (0.35, Color::hex(p.hl[0]).with_alpha(0.9)),
        (0.65, Color::hex(p.hl[1]).with_alpha(0.8)),
        (1.0, Color::hex(p.hl[2]).with_alpha(0.2)),
    ]);

    // 2. Blurred background ellipse at rib[1][0], opacity .35.
    let ell = vector::fill(&vector::ellipse(200.0, 700.0, 280.0, 220.0, tf))
        .expect("aurora glow ellipse");
    ops.push(Op::Group(Group {
        fills: vec![solid_fill(ell, Color::hex(p.rib[1][0]), 1.0)],
        blur: 28.0,
        opacity: 0.35,
        blend: Blend::Normal,
    }));

    // 3. Blurred group of ribbons 1 and 2, opacity .55.
    let mut glow_fills = Vec::new();
    for i in [1usize, 2] {
        let mesh = vector::fill(&vector::parse_path(RIBBONS[i].0, tf).expect("ribbon"))
            .expect("ribbon fill");
        let paint = linear(&mesh, [fx(0.0), 0.2], [fx(1.0), 0.0], rg(i));
        glow_fills.push(Fill {
            mesh,
            paint,
            opacity: 1.0,
        });
    }
    ops.push(Op::Group(Group {
        fills: glow_fills,
        blur: 28.0,
        opacity: 0.55,
        blend: Blend::Normal,
    }));

    // 4. Sharp ribbons: fill, shade overlay, and a highlighted top edge.
    for (i, (body, top)) in RIBBONS.iter().enumerate() {
        let path = vector::parse_path(body, tf).expect("ribbon body");
        let mesh = vector::fill(&path).expect("ribbon fill");
        ops.push(Op::Fill(Fill {
            paint: linear(&mesh, [fx(0.0), 0.2], [fx(1.0), 0.0], rg(i)),
            mesh: mesh.clone(),
            opacity: 1.0,
        }));
        ops.push(Op::Fill(Fill {
            paint: linear(&mesh, [fx(0.0), 0.0], [fx(0.0), 1.0], shade()),
            mesh,
            opacity: 1.0,
        }));
        if let Some(top) = top {
            let tp = vector::parse_path(top, tf).expect("ribbon top");
            let width = if i == 3 { 1.0 } else { 1.4 };
            let stroke = vector::stroke(&tp, width, false).expect("ribbon edge");
            ops.push(Op::Fill(Fill {
                paint: linear(&stroke, [fx(0.0), 0.0], [fx(1.0), 0.0], hl_stops),
                mesh: stroke,
                opacity: 0.85,
            }));
        }
    }

    Wallpaper { ops }
}

fn crystal(p: &CrystalPalette) -> Wallpaper {
    let id = Affine::IDENTITY;
    let b = |i: usize| Color::hex(p.b[i]);
    let sky = |i: usize| Color::hex(p.sky[i]);
    let mut ops = Vec::new();

    // Sky background gradient.
    let bg = canvas_mesh();
    let sky_stops = Stops::svg(&[(0.0, sky(0)), (0.4, sky(1)), (0.76, sky(2)), (1.0, sky(3))]);
    let bg_paint = linear(&bg, [0.0, 0.0], [0.25, 1.0], sky_stops);
    ops.push(Op::Fill(Fill {
        mesh: bg,
        paint: bg_paint,
        opacity: 1.0,
    }));

    // Soft blurred colour blobs, group opacity .8.
    let blob = |cx, cy, rx, ry, color, op| {
        solid_fill(
            vector::fill(&vector::ellipse(cx, cy, rx, ry, id)).expect("blob"),
            color,
            op,
        )
    };
    ops.push(Op::Group(Group {
        fills: vec![
            blob(40.0, 250.0, 160.0, 120.0, b(0), 0.7),
            blob(380.0, 590.0, 150.0, 180.0, b(1), 0.6),
            blob(210.0, 430.0, 120.0, 100.0, b(2), 0.55),
        ],
        blur: 42.0,
        opacity: 0.8,
        blend: Blend::Normal,
    }));

    // Inner glow ellipse (radial white).
    let inner = vector::fill(&vector::ellipse(206.0, 420.0, 110.0, 190.0, id)).expect("inner");
    let inner_paint = Paint::Radial {
        frame: inner.bounds,
        center: [0.5, 0.35],
        radius: [0.5, 0.5],
        stops: Stops::svg(&[(0.0, white(0.5)), (1.0, white(0.0))]),
    };
    ops.push(Op::Fill(Fill {
        mesh: inner,
        paint: inner_paint,
        opacity: 1.0,
    }));

    // Facet gradients.
    let facet_paint = |which: u8, mesh: &Mesh| match which {
        1 => linear(
            mesh,
            [0.0, 0.0],
            [1.0, 1.0],
            Stops::svg(&[
                (0.0, white(0.6)),
                (0.5, b(0).with_alpha(0.22)),
                (1.0, white(0.06)),
            ]),
        ),
        2 => linear(
            mesh,
            [1.0, 0.0],
            [0.0, 1.0],
            Stops::svg(&[(0.0, b(2).with_alpha(0.45)), (1.0, sky(1).with_alpha(0.12))]),
        ),
        3 => linear(
            mesh,
            [0.0, 0.0],
            [1.0, 1.0],
            Stops::svg(&[(0.0, b(1).with_alpha(0.45)), (1.0, white(0.05))]),
        ),
        _ => linear(
            mesh,
            [0.0, 1.0],
            [1.0, 0.0],
            Stops::svg(&[(0.0, b(0).with_alpha(0.4)), (1.0, white(0.1))]),
        ),
    };
    // Facets: fill with their gradient, then outline white at .34, width .7.
    for (points, which) in FACETS {
        let path = vector::polygon(points, id);
        let mesh = vector::fill(&path).expect("facet fill");
        ops.push(Op::Fill(Fill {
            paint: facet_paint(which, &mesh),
            mesh,
            opacity: 1.0,
        }));
        let outline = vector::stroke(&path, 0.7, true).expect("facet outline");
        ops.push(Op::Fill(solid_fill(outline, white(0.34), 1.0)));
    }

    // Tip spine line, white at .5, width .6.
    let spine = vector::parse_path("M196 186 L212 690", id).expect("spine");
    ops.push(Op::Fill(solid_fill(
        vector::stroke(&spine, 0.6, false).expect("spine stroke"),
        white(0.5),
        1.0,
    )));

    // Sparkles.
    for (x, y, scale, op) in SPARKLES {
        let tf = Affine::translate(x, y).then(Affine::scale(scale, scale));
        let mesh = vector::fill(&vector::parse_path(SPARKLE_PATH, tf).expect("sparkle"))
            .expect("sparkle fill");
        ops.push(Op::Fill(solid_fill(mesh, Color::WHITE, op)));
    }

    // Battery glow, screen-blended at .22.
    let glow = vector::fill(&vector::ellipse(205.0, 440.0, 70.0, 150.0, id)).expect("glow");
    ops.push(Op::Group(Group {
        fills: vec![solid_fill(glow, Color::WHITE, 1.0)],
        blur: 0.0,
        opacity: 0.22,
        blend: Blend::Screen,
    }));

    Wallpaper { ops }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_fills(w: &Wallpaper) -> usize {
        w.ops
            .iter()
            .map(|op| match op {
                Op::Fill(_) => 1,
                Op::Group(g) => g.fills.len(),
            })
            .sum()
    }

    #[test]
    fn every_wallpaper_builds_with_triangles() {
        for wall in Wall::ALL {
            let wp = wall.build();
            assert!(!wp.ops.is_empty(), "{wall:?} has no ops");
            let mut total = 0;
            let visit = |f: &Fill| {
                assert!(!f.mesh.indices.is_empty(), "{wall:?} empty mesh");
                assert!(f.mesh.indices.len().is_multiple_of(3));
            };
            for op in &wp.ops {
                match op {
                    Op::Fill(f) => {
                        visit(f);
                        total += 1;
                    }
                    Op::Group(g) => {
                        for f in &g.fills {
                            visit(f);
                        }
                        total += g.fills.len();
                    }
                }
            }
            assert!(total > 5, "{wall:?} only {total} fills");
        }
    }

    #[test]
    fn aurora_has_the_expected_structure() {
        let wp = Wall::Aurora.build();
        // bg + 2 groups + 4 ribbons * (fill + shade) + 3 edges = 1+2+8+3 = 14 ops.
        assert_eq!(wp.ops.len(), 14);
        // bg, ellipse glow, ribbon glow group, then 4 fill + 4 shade + 3 edge.
        assert!(matches!(wp.ops[0], Op::Fill(_)));
        assert!(matches!(wp.ops[1], Op::Group(_)));
        assert!(matches!(wp.ops[2], Op::Group(_)));
        // 1 bg + 11 ribbon fills + ellipse(1) + glow group(2) = 15 fills.
        assert_eq!(count_fills(&wp), 15);
    }

    #[test]
    fn obsidian_mirrors_the_ribbons_but_not_the_background() {
        let wp = Wall::Obsidian.build();
        let Op::Fill(bg) = &wp.ops[0] else {
            panic!("bg");
        };
        assert_eq!(bg.mesh.bounds, CANVAS, "background is not mirrored");
        // A ribbon's path reaches x = 440 before mirroring; after mirror its
        // geometry lands at negative x (393 - 440 = -47).
        let Op::Group(glow) = &wp.ops[2] else {
            panic!("glow group");
        };
        assert!(glow.fills[0].mesh.bounds.x < 0.0);
    }

    #[test]
    fn crystal_ends_with_a_screen_blended_glow() {
        let wp = Wall::Dawn.build();
        let last = wp.ops.last().expect("ops");
        let Op::Group(g) = last else {
            panic!("expected a group");
        };
        assert_eq!(g.blend, Blend::Screen);
        assert_eq!(g.blur, 0.0);
        assert!((g.opacity - 0.22).abs() < 1e-6);
    }

    #[test]
    fn crystal_has_five_outlined_facets_and_six_sparkles() {
        let wp = Wall::Night.build();
        let solid_white_34 = wp
            .ops
            .iter()
            .filter(|op| {
                matches!(op, Op::Fill(f) if matches!(f.paint, Paint::Solid(c) if (c.a - 0.34).abs() < 1e-6))
            })
            .count();
        assert_eq!(solid_white_34, 5, "five facet outlines");
    }
}
