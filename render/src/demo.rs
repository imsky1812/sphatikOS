//! Reference scenes shared by `sphatik-preview` and the golden-image test.

use crate::{Color, Paint, Rect, Shape, Stops};

/// The WP 2.2 scene: the Liquid Aurora background gradient and one panel
/// at the first home widget's position, filled like the prototype's
/// Regular glass (without blur, which arrives in WP 2.4).
///
/// Values from `docs/design/prototype-reference.md`: `bg` stops at sections
/// 13.1 (`#0B0A1C`, `#06050E` at 0.6, `#15113A`, from (0, 0) to (0.3, 1) of
/// the screen) and the `.glass` fill at section 3; widget frame at section 9.
pub fn backdrop_with_panel(canvas: Rect) -> [Shape; 2] {
    let background = Shape::new(
        canvas,
        0.0,
        Paint::Linear {
            frame: canvas,
            from: [0.0, 0.0],
            to: [0.3, 1.0],
            stops: Stops::svg(&[
                (0.0, Color::hex(0x0B0A1C)),
                (0.6, Color::hex(0x06050E)),
                (1.0, Color::hex(0x15113A)),
            ]),
        },
    );
    let panel_rect = Rect::new(18.0, 66.0, 171.5, 162.0);
    let panel = Shape::new(
        panel_rect,
        30.0,
        Paint::css_linear(
            panel_rect,
            165.0,
            Stops::css(&[
                (0.0, Color::WHITE.with_alpha(0.13)),
                (0.42, Color::WHITE.with_alpha(0.03)),
                (1.0, Color::WHITE.with_alpha(0.07)),
            ]),
        ),
    );
    [background, panel]
}

/// Opaque black, the clear colour behind every scene.
pub const CLEAR: Color = Color::rgba(0.0, 0.0, 0.0, 1.0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_covers_the_canvas_and_panel_sits_on_it() {
        let canvas = Rect::new(0.0, 0.0, 393.0, 852.0);
        let [bg, panel] = backdrop_with_panel(canvas);
        assert_eq!(bg.rect, canvas);
        assert!(panel.rect.intersection(&canvas) == Some(panel.rect));
    }
}
