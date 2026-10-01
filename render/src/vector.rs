//! Vector paths: a small SVG path parser and tessellation into triangles.
//!
//! Used to build the prototype's wallpapers from its own path strings.
//! Tessellation happens once when a wallpaper is loaded, never per frame.

use lyon::math::{point, Box2D};
use lyon::path::Path;
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, LineJoin, StrokeOptions,
    StrokeTessellator, StrokeVertex, VertexBuffers,
};

use crate::Rect;

/// Why a path string could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathError(pub String);

/// Triangles ready for the GPU, plus the path's bounding box (SVG
/// `objectBoundingBox`, which gradients refer to).
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    /// Vertex positions in points.
    pub vertices: Vec<[f32; 2]>,
    /// Triangle list indices into `vertices`.
    pub indices: Vec<u32>,
    /// Tight bounds of the path geometry (without stroke width).
    pub bounds: Rect,
}

/// An affine transform `x' = a x + c y + e`, `y' = b x + d y + f`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    /// Matrix entries in SVG order.
    pub m: [f32; 6],
}

impl Affine {
    /// No change.
    pub const IDENTITY: Affine = Affine {
        m: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    };

    /// SVG `translate(x y)`.
    pub const fn translate(x: f32, y: f32) -> Affine {
        Affine {
            m: [1.0, 0.0, 0.0, 1.0, x, y],
        }
    }

    /// SVG `scale(sx sy)`.
    pub const fn scale(sx: f32, sy: f32) -> Affine {
        Affine {
            m: [sx, 0.0, 0.0, sy, 0.0, 0.0],
        }
    }

    /// `self` applied after `inner` (SVG `transform="self inner"` order).
    pub fn then(self, inner: Affine) -> Affine {
        let [a, b, c, d, e, f] = self.m;
        let [a2, b2, c2, d2, e2, f2] = inner.m;
        Affine {
            m: [
                a * a2 + c * b2,
                b * a2 + d * b2,
                a * c2 + c * d2,
                b * c2 + d * d2,
                a * e2 + c * f2 + e,
                b * e2 + d * f2 + f,
            ],
        }
    }

    /// Maps a point.
    pub fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        let [a, b, c, d, e, f] = self.m;
        (a * x + c * y + e, b * x + d * y + f)
    }
}

/// Splits SVG path data into commands and numbers (`M0-6` is `M`, `0`, `-6`).
fn tokens(data: &str) -> Result<Vec<Token>, PathError> {
    let mut out = Vec::new();
    let bytes = data.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_whitespace() || c == ',' {
            i += 1;
        } else if c.is_ascii_alphabetic() {
            out.push(Token::Command(c));
            i += 1;
        } else if c == '-' || c == '+' || c == '.' || c.is_ascii_digit() {
            let start = i;
            i += 1;
            let mut seen_dot = c == '.';
            while i < bytes.len() {
                let n = bytes[i] as char;
                if n.is_ascii_digit() {
                    i += 1;
                } else if n == '.' && !seen_dot {
                    seen_dot = true;
                    i += 1;
                } else {
                    break;
                }
            }
            let text = &data[start..i];
            let value = text
                .parse::<f32>()
                .map_err(|_| PathError(format!("bad number `{text}`")))?;
            out.push(Token::Number(value));
        } else {
            return Err(PathError(format!("unexpected `{c}`")));
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Token {
    Command(char),
    Number(f32),
}

/// Parses absolute `M`, `L`, `C` and `Z` path data (the only commands the
/// prototype's wallpapers use; repeated coordinates after `M` or `L` are
/// further line-tos, as in SVG), applying `transform` to every point.
pub fn parse_path(data: &str, transform: Affine) -> Result<Path, PathError> {
    let toks = tokens(data)?;
    let mut builder = Path::builder();
    let mut i = 0;
    let mut command = None;
    let mut open = false;
    let take = |i: &mut usize| -> Result<f32, PathError> {
        match toks.get(*i) {
            Some(Token::Number(v)) => {
                *i += 1;
                Ok(*v)
            }
            other => Err(PathError(format!("expected a number, found {other:?}"))),
        }
    };
    let pt = |x: f32, y: f32| {
        let (x, y) = transform.apply(x, y);
        point(x, y)
    };
    while i < toks.len() {
        if let Token::Command(c) = toks[i] {
            command = Some(c);
            i += 1;
            if c == 'Z' || c == 'z' {
                if open {
                    builder.close();
                    open = false;
                }
                continue;
            }
        }
        match command {
            Some('M') => {
                let (x, y) = (take(&mut i)?, take(&mut i)?);
                if open {
                    builder.end(false);
                }
                builder.begin(pt(x, y));
                open = true;
                command = Some('L');
            }
            Some('L') => {
                let (x, y) = (take(&mut i)?, take(&mut i)?);
                builder.line_to(pt(x, y));
            }
            Some('C') => {
                let (x1, y1) = (take(&mut i)?, take(&mut i)?);
                let (x2, y2) = (take(&mut i)?, take(&mut i)?);
                let (x, y) = (take(&mut i)?, take(&mut i)?);
                builder.cubic_bezier_to(pt(x1, y1), pt(x2, y2), pt(x, y));
            }
            Some(other) => return Err(PathError(format!("unsupported command `{other}`"))),
            None => return Err(PathError("path data must start with a command".into())),
        }
    }
    if open {
        builder.end(false);
    }
    Ok(builder.build())
}

/// A closed polygon through `points` (SVG `<polygon points="…">`).
pub fn polygon(points: &[(f32, f32)], transform: Affine) -> Path {
    let mut builder = Path::builder();
    let mut iter = points.iter();
    if let Some(&(x, y)) = iter.next() {
        let (x, y) = transform.apply(x, y);
        builder.begin(point(x, y));
        for &(x, y) in iter {
            let (x, y) = transform.apply(x, y);
            builder.line_to(point(x, y));
        }
        builder.close();
    }
    builder.build()
}

/// An axis-aligned ellipse (SVG `<ellipse>`), as four cubic arcs.
pub fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32, transform: Affine) -> Path {
    // Control-point distance for a quarter circle.
    const K: f32 = 0.552_284_8;
    let (kx, ky) = (rx * K, ry * K);
    let p = |x: f32, y: f32| {
        let (x, y) = transform.apply(x, y);
        point(x, y)
    };
    let mut b = Path::builder();
    b.begin(p(cx + rx, cy));
    b.cubic_bezier_to(p(cx + rx, cy + ky), p(cx + kx, cy + ry), p(cx, cy + ry));
    b.cubic_bezier_to(p(cx - kx, cy + ry), p(cx - rx, cy + ky), p(cx - rx, cy));
    b.cubic_bezier_to(p(cx - rx, cy - ky), p(cx - kx, cy - ry), p(cx, cy - ry));
    b.cubic_bezier_to(p(cx + kx, cy - ry), p(cx + rx, cy - ky), p(cx + rx, cy));
    b.close();
    b.build()
}

/// Tight bounds of a path's geometry.
pub fn bounds(path: &Path) -> Rect {
    let b: Box2D = lyon::algorithms::aabb::bounding_box(path.iter());
    Rect::new(b.min.x, b.min.y, b.max.x - b.min.x, b.max.y - b.min.y)
}

/// Tessellation accuracy in points; fine enough at the phone's 2.75 scale.
const TOLERANCE: f32 = 0.02;

/// Triangles covering the inside of `path` (non-zero fill, as SVG).
pub fn fill(path: &Path) -> Result<Mesh, PathError> {
    let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
    FillTessellator::new()
        .tessellate_path(
            path,
            &FillOptions::tolerance(TOLERANCE)
                .with_fill_rule(lyon::tessellation::FillRule::NonZero),
            &mut BuffersBuilder::new(&mut buffers, |v: FillVertex| v.position().to_array()),
        )
        .map_err(|e| PathError(format!("fill tessellation failed: {e:?}")))?;
    Ok(Mesh {
        vertices: buffers.vertices,
        indices: buffers.indices,
        bounds: bounds(path),
    })
}

/// Triangles covering a stroke of `width` points along `path`, with round
/// joins if asked (SVG `stroke-linejoin`) and butt caps (the SVG default).
pub fn stroke(path: &Path, width: f32, round_joins: bool) -> Result<Mesh, PathError> {
    let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
    let join = if round_joins {
        LineJoin::Round
    } else {
        LineJoin::Miter
    };
    StrokeTessellator::new()
        .tessellate_path(
            path,
            &StrokeOptions::tolerance(TOLERANCE)
                .with_line_width(width)
                .with_line_join(join),
            &mut BuffersBuilder::new(&mut buffers, |v: StrokeVertex| v.position().to_array()),
        )
        .map_err(|e| PathError(format!("stroke tessellation failed: {e:?}")))?;
    Ok(Mesh {
        vertices: buffers.vertices,
        indices: buffers.indices,
        bounds: bounds(path),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn numbers_split_on_signs_like_svg() {
        let t = tokens("M0-6 1.1-1.1Z").expect("tokens");
        assert_eq!(
            t,
            vec![
                Token::Command('M'),
                Token::Number(0.0),
                Token::Number(-6.0),
                Token::Number(1.1),
                Token::Number(-1.1),
                Token::Command('Z'),
            ]
        );
    }

    #[test]
    fn repeated_coordinates_after_move_are_lines() {
        let p = parse_path(
            "M0-6 1.1-1.1 6 0 1.1 1.1 0 6-1.1 1.1-6 0-1.1-1.1Z",
            Affine::IDENTITY,
        )
        .expect("parse");
        let b = bounds(&p);
        assert!(close(b.x, -6.0) && close(b.y, -6.0) && close(b.w, 12.0) && close(b.h, 12.0));
    }

    #[test]
    fn cubic_bounds_are_tight_not_control_points() {
        // A curve whose control points reach y = 0 but whose curve does not.
        let p = parse_path("M0 100 C 0 0, 100 0, 100 100", Affine::IDENTITY).expect("parse");
        let b = bounds(&p);
        assert!(close(b.y, 25.0), "top of the curve is at 25, got {}", b.y);
    }

    #[test]
    fn transforms_compose_in_svg_order() {
        // translate(393 0) scale(-1 1) mirrors x around 196.5.
        let flip = Affine::translate(393.0, 0.0).then(Affine::scale(-1.0, 1.0));
        assert_eq!(flip.apply(0.0, 5.0), (393.0, 5.0));
        assert_eq!(flip.apply(100.0, 5.0), (293.0, 5.0));
    }

    #[test]
    fn fills_and_strokes_produce_triangles() {
        let p = parse_path(
            "M-40 720 C 90 580, 215 840, 440 540 L 440 640 C 235 930, 80 690, -40 820 Z",
            Affine::IDENTITY,
        )
        .expect("parse");
        let f = fill(&p).expect("fill");
        assert!(!f.indices.is_empty() && f.indices.len().is_multiple_of(3));
        let s = stroke(&p, 1.4, false).expect("stroke");
        assert!(!s.indices.is_empty() && s.indices.len().is_multiple_of(3));
        assert!(close(f.bounds.x, -40.0) && close(f.bounds.right(), 440.0));
    }

    #[test]
    fn ellipse_bounds_match_its_radii() {
        let b = bounds(&ellipse(200.0, 700.0, 280.0, 220.0, Affine::IDENTITY));
        assert!(close(b.x, -80.0) && close(b.y, 480.0) && close(b.w, 560.0) && close(b.h, 440.0));
    }

    #[test]
    fn bad_path_data_is_an_error_not_a_panic() {
        assert!(parse_path("M 1", Affine::IDENTITY).is_err());
        assert!(parse_path("Q 1 2 3 4", Affine::IDENTITY).is_err());
        assert!(parse_path("1 2", Affine::IDENTITY).is_err());
        assert!(parse_path("M 1 2 # 3", Affine::IDENTITY).is_err());
    }
}
