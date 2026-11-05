//! Interim text rendering for the lock-screen clock (Glass-on-laptop gate).
//!
//! A pure-Rust glyph rasteriser (`ab_glyph`) turns a short string into a
//! single-channel coverage bitmap on the CPU; [`TextRenderer`] uploads that as
//! an R8 texture and draws it as one tinted quad. This is deliberately minimal
//! — enough for the clock, date and hint — and is replaced by the full
//! cosmic-text stack in the Stage 4 toolkit.

use ab_glyph::{point, Font, FontRef, Glyph, PxScale, ScaleFont};
use glow::HasContext;

use crate::{shaders, Color, RenderError};

/// Outfit ExtraLight (OFL), the prototype's default lock-clock weight (~200).
pub const OUTFIT_EXTRALIGHT: &[u8] = include_bytes!("../assets/fonts/Outfit-ExtraLight.ttf");
/// Outfit Light (OFL), for secondary lock-screen text (date, hint).
pub const OUTFIT_LIGHT: &[u8] = include_bytes!("../assets/fonts/Outfit-Light.ttf");

/// A CPU-rasterised line of text: 8-bit coverage (alpha), one byte per pixel.
pub struct TextBitmap {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Row-major coverage, `width * height` bytes.
    pub coverage: Vec<u8>,
}

/// Rasterises `text` in `font_bytes` at `px` pixels, adding `tracking_px`
/// between glyphs. With `tabular`, every glyph sits in a fixed cell the width
/// of `0`, so clock digits do not shift as the time ticks.
pub fn rasterize(
    font_bytes: &[u8],
    text: &str,
    px: f32,
    tracking_px: f32,
    tabular: bool,
) -> TextBitmap {
    let font = FontRef::try_from_slice(font_bytes).expect("bundled font parses");
    let scale = PxScale::from(px);
    let scaled = font.as_scaled(scale);
    let ascent = scaled.ascent();
    let height = (ascent - scaled.descent()).ceil().max(1.0) as u32;
    let cell = scaled.h_advance(font.glyph_id('0'));

    // Lay the glyphs out left to right and record where each one sits.
    let mut glyphs: Vec<Glyph> = Vec::new();
    let mut pen = 0.0f32;
    for c in text.chars() {
        let gid = font.glyph_id(c);
        let glyph_w = scaled.h_advance(gid);
        let advance = if tabular { cell } else { glyph_w };
        // Centre a glyph in its cell when the run is tabular.
        let offset = if tabular { (cell - glyph_w) * 0.5 } else { 0.0 };
        glyphs.push(gid.with_scale_and_position(scale, point(pen + offset, ascent)));
        pen += advance + tracking_px;
    }
    let width = (pen - tracking_px).ceil().max(1.0) as u32;

    let mut coverage = vec![0u8; (width * height) as usize];
    for glyph in glyphs {
        if let Some(outline) = font.outline_glyph(glyph) {
            let bounds = outline.px_bounds();
            let (ox, oy) = (bounds.min.x.round() as i32, bounds.min.y.round() as i32);
            outline.draw(|gx, gy, c| {
                let x = ox + gx as i32;
                let y = oy + gy as i32;
                if x >= 0 && y >= 0 && (x as u32) < width && (y as u32) < height {
                    let idx = (y as u32 * width + x as u32) as usize;
                    let v = (c * 255.0 + 0.5) as u8;
                    if v > coverage[idx] {
                        coverage[idx] = v;
                    }
                }
            });
        }
    }
    TextBitmap {
        width,
        height,
        coverage,
    }
}

/// Draws a [`TextBitmap`] as a tinted quad. Create with a current context and
/// keep it current.
pub struct TextRenderer {
    gl: glow::Context,
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    tex: glow::Texture,
    u_size: Option<glow::UniformLocation>,
    u_color: Option<glow::UniformLocation>,
    u_tex: Option<glow::UniformLocation>,
    width: u32,
    height: u32,
}

impl TextRenderer {
    /// Compiles the text shader and allocates the quad buffer and glyph texture.
    pub fn new(gl: glow::Context) -> Result<Self, RenderError> {
        let header = if gl.version().is_embedded {
            shaders::HEADER_ES
        } else {
            shaders::HEADER_DESKTOP
        };
        let program =
            crate::wallpaper_gl::link(&gl, header, shaders::TEXT_VERTEX, shaders::TEXT_FRAGMENT)?;
        // SAFETY: the caller made the context current; the objects are created
        // on it here and the attribute layout matches the vertices in `draw`.
        let (vao, vbo, tex) = unsafe {
            let vao = gl.create_vertex_array().map_err(RenderError::Create)?;
            let vbo = gl.create_buffer().map_err(RenderError::Create)?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let stride = 4 * 4;
            let pos = gl.get_attrib_location(program, "a_pos").unwrap_or(0);
            gl.enable_vertex_attrib_array(pos);
            gl.vertex_attrib_pointer_f32(pos, 2, glow::FLOAT, false, stride, 0);
            let uv = gl.get_attrib_location(program, "a_uv").unwrap_or(1);
            gl.enable_vertex_attrib_array(uv);
            gl.vertex_attrib_pointer_f32(uv, 2, glow::FLOAT, false, stride, 8);
            gl.bind_vertex_array(None);
            let tex = gl.create_texture().map_err(RenderError::Create)?;
            (vao, vbo, tex)
        };
        let (u_size, u_color, u_tex) = {
            // SAFETY: the program is linked on the current context.
            unsafe {
                (
                    gl.get_uniform_location(program, "u_size"),
                    gl.get_uniform_location(program, "u_color"),
                    gl.get_uniform_location(program, "u_tex"),
                )
            }
        };
        Ok(Self {
            gl,
            program,
            vao,
            vbo,
            tex,
            u_size,
            u_color,
            u_tex,
            width: 0,
            height: 0,
        })
    }

    /// Uploads a coverage bitmap into the glyph texture.
    pub fn set_bitmap(&mut self, bitmap: &TextBitmap) {
        self.width = bitmap.width;
        self.height = bitmap.height;
        let gl = &self.gl;
        // SAFETY: the context is current; the texture is alive and the data
        // length matches the given dimensions (one byte per pixel).
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(self.tex));
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::R8 as i32,
                bitmap.width as i32,
                bitmap.height as i32,
                0,
                glow::RED,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&bitmap.coverage)),
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.bind_texture(glow::TEXTURE_2D, None);
        }
    }

    /// The uploaded bitmap width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The uploaded bitmap height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Draws the uploaded bitmap with its top-left at (`x`, `y`) pixels on a
    /// `canvas_w` x `canvas_h` target, tinted `color` (straight alpha).
    pub fn draw(&self, canvas_w: u32, canvas_h: u32, x: f32, y: f32, color: Color) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        let (w, h) = (self.width as f32, self.height as f32);
        let (x1, y1) = (x + w, y + h);
        // interleaved x, y, u, v for two triangles
        let verts: [f32; 24] = [
            x, y, 0.0, 0.0, //
            x1, y, 1.0, 0.0, //
            x, y1, 0.0, 1.0, //
            x1, y, 1.0, 0.0, //
            x1, y1, 1.0, 1.0, //
            x, y1, 0.0, 1.0, //
        ];
        let gl = &self.gl;
        // SAFETY: the context is current; the program, VAO, VBO and texture are
        // alive, and the vertex data matches the attribute layout set in `new`.
        unsafe {
            gl.enable(glow::BLEND);
            gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
            gl.use_program(Some(self.program));
            gl.uniform_2_f32(self.u_size.as_ref(), canvas_w as f32, canvas_h as f32);
            gl.uniform_4_f32(self.u_color.as_ref(), color.r, color.g, color.b, color.a);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.tex));
            gl.uniform_1_i32(self.u_tex.as_ref(), 0);
            gl.bind_vertex_array(Some(self.vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            let bytes: &[u8] = core::slice::from_raw_parts(
                verts.as_ptr() as *const u8,
                std::mem::size_of_val(&verts),
            );
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STREAM_DRAW);
            gl.draw_arrays(glow::TRIANGLES, 0, 6);
            gl.bind_vertex_array(None);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.disable(glow::BLEND);
        }
    }

    /// Deletes the GL objects. The context must be current.
    pub fn destroy(self) {
        // SAFETY: the context is current; these objects are not used again.
        unsafe {
            self.gl.delete_vertex_array(self.vao);
            self.gl.delete_buffer(self.vbo);
            self.gl.delete_texture(self.tex);
            self.gl.delete_program(self.program);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_digits_rasterise_with_ink() {
        let bmp = rasterize(OUTFIT_EXTRALIGHT, "12:56", 118.0, -3.0, true);
        assert!(bmp.width > 0 && bmp.height > 0);
        assert_eq!(bmp.coverage.len(), (bmp.width * bmp.height) as usize);
        let inked = bmp.coverage.iter().filter(|&&c| c > 0).count();
        assert!(inked > 0, "clock rasterised to an empty bitmap");
    }

    #[test]
    fn tabular_digits_share_one_cell_width() {
        // Two different digit pairs of the same length get the same width when
        // laid out tabularly.
        let a = rasterize(OUTFIT_EXTRALIGHT, "11", 100.0, 0.0, true);
        let b = rasterize(OUTFIT_EXTRALIGHT, "08", 100.0, 0.0, true);
        assert_eq!(a.width, b.width);
    }
}
