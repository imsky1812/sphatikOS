//! The F12 debug overlay (WP 2.9): an FPS read-out and a frame-time graph
//! drawn over the window, plus the glass GPU cost when available.
//!
//! The performance maths lives in [`PerfGraph`]; this module draws it with a
//! flat per-vertex-colour shader and a 5x7 bitmap font.

mod font;
mod perf;

pub use perf::{PerfGraph, HISTORY};

use glow::HasContext;

use crate::shaders;
use crate::{Color, RenderError};

const BG: Color = Color::rgba(0.03, 0.03, 0.08, 0.72);
const TEXT: Color = Color::rgba(0.93, 0.95, 1.0, 1.0);
const GOOD: Color = Color::rgba(0.25, 0.86, 0.5, 0.95); // under budget
const BAD: Color = Color::rgba(1.0, 0.3, 0.37, 0.95); // over budget
const BUDGET_LINE: Color = Color::rgba(1.0, 1.0, 1.0, 0.25);

/// Draws the debug overlay. Create with a current context; keep it current.
pub struct Overlay {
    gl: glow::Context,
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    u_size: Option<glow::UniformLocation>,
    verts: Vec<f32>, // interleaved x, y, r, g, b, a
}

impl Overlay {
    /// Compiles the overlay shader and sets up a dynamic vertex buffer.
    pub fn new(gl: glow::Context) -> Result<Self, RenderError> {
        let header = if gl.version().is_embedded {
            shaders::HEADER_ES
        } else {
            shaders::HEADER_DESKTOP
        };
        let program = crate::wallpaper_gl::link(
            &gl,
            header,
            shaders::OVERLAY_VERTEX,
            shaders::OVERLAY_FRAGMENT,
        )?;
        // SAFETY: the caller made the context current; the VAO and VBO are
        // created on it here, and the attribute layout matches `verts`.
        let (vao, vbo) = unsafe {
            let vao = gl.create_vertex_array().map_err(RenderError::Create)?;
            let vbo = gl.create_buffer().map_err(RenderError::Create)?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let stride = 6 * 4;
            let pos = gl.get_attrib_location(program, "a_pos").unwrap_or(0);
            gl.enable_vertex_attrib_array(pos);
            gl.vertex_attrib_pointer_f32(pos, 2, glow::FLOAT, false, stride, 0);
            let col = gl.get_attrib_location(program, "a_color").unwrap_or(1);
            gl.enable_vertex_attrib_array(col);
            gl.vertex_attrib_pointer_f32(col, 4, glow::FLOAT, false, stride, 8);
            gl.bind_vertex_array(None);
            (vao, vbo)
        };
        let u_size = {
            // SAFETY: the program is linked on the current context.
            unsafe { gl.get_uniform_location(program, "u_size") }
        };
        Ok(Self {
            gl,
            program,
            vao,
            vbo,
            u_size,
            verts: Vec::with_capacity(4096),
        })
    }

    /// Draws the overlay onto the currently bound framebuffer (the window's
    /// default framebuffer, or an offscreen for a screenshot), sized `width` x
    /// `height` pixels. `ui` scales the overlay (pixels per font cell);
    /// `glass_ms` is the glass GPU cost if measured. The caller binds the
    /// target first.
    pub fn render(
        &mut self,
        width: u32,
        height: u32,
        ui: f32,
        graph: &PerfGraph,
        glass_ms: Option<f32>,
    ) {
        self.verts.clear();
        let ui = ui.max(1.0);
        let pad = 4.0 * ui;
        let line_h = (font::GLYPH_H as f32 + 2.0) * ui;

        // Panel background.
        let graph_w = HISTORY as f32 * ui;
        let graph_h = 40.0 * ui;
        let panel_w = graph_w + 2.0 * pad;
        let panel_h = pad + 2.0 * line_h + pad + graph_h + pad;
        self.rect(pad, pad, panel_w, panel_h, BG);

        // Text lines.
        let tx = 2.0 * pad;
        let mut ty = 2.0 * pad;
        let fps = graph.fps();
        self.text(
            tx,
            ty,
            ui,
            TEXT,
            &format!(
                "FPS {}  FRAME {}MS",
                round0(fps),
                round1(graph.average_ms())
            ),
        );
        ty += line_h;
        let glass = glass_ms.map_or_else(|| "N/A".to_string(), |ms| format!("{}MS", round2(ms)));
        self.text(
            tx,
            ty,
            ui,
            TEXT,
            &format!("MAX {}MS  GLASS {}", round1(graph.max_ms()), glass),
        );

        // Frame-time graph: one bar per frame, height scaled so the budget
        // sits at 60% of the graph; bars over budget turn red.
        let gx = 2.0 * pad;
        let gy = 2.0 * pad + 2.0 * line_h + pad;
        let full = (graph.budget_ms / 0.6).max(1.0);
        for (i, ms) in graph.samples().enumerate() {
            let frac = (ms / full).clamp(0.0, 1.0);
            let bh = frac * graph_h;
            let color = if ms > graph.budget_ms { BAD } else { GOOD };
            self.rect(
                gx + i as f32 * ui,
                gy + graph_h - bh,
                ui.max(1.0),
                bh,
                color,
            );
        }
        // Budget line.
        let line_y = gy + graph_h * (1.0 - graph.budget_ms / full);
        self.rect(gx, line_y, graph_w, (ui * 0.5).max(1.0), BUDGET_LINE);

        self.flush(width, height);
    }

    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let p = [c.r * c.a, c.g * c.a, c.b * c.a, c.a];
        let v = |vx: f32, vy: f32, out: &mut Vec<f32>| {
            out.extend_from_slice(&[vx, vy, p[0], p[1], p[2], p[3]]);
        };
        let (x1, y1) = (x + w, y + h);
        v(x, y, &mut self.verts);
        v(x1, y, &mut self.verts);
        v(x, y1, &mut self.verts);
        v(x1, y, &mut self.verts);
        v(x1, y1, &mut self.verts);
        v(x, y1, &mut self.verts);
    }

    fn text(&mut self, x: f32, y: f32, scale: f32, c: Color, text: &str) {
        let mut cx = x;
        for ch in text.chars() {
            let rows = font::glyph(ch);
            for (ry, row) in rows.iter().enumerate() {
                for bit in 0..font::GLYPH_W {
                    if row & (1 << (font::GLYPH_W - 1 - bit)) != 0 {
                        self.rect(
                            cx + bit as f32 * scale,
                            y + ry as f32 * scale,
                            scale,
                            scale,
                            c,
                        );
                    }
                }
            }
            cx += (font::GLYPH_W as f32 + 1.0) * scale;
        }
    }

    /// Uploads and draws the accumulated geometry with alpha blending.
    fn flush(&mut self, width: u32, height: u32) {
        if self.verts.is_empty() {
            return;
        }
        let gl = &self.gl;
        // SAFETY: the context is current; the program, VAO and VBO are alive,
        // and the vertex data matches the attribute layout set in `new`.
        unsafe {
            // The caller binds the target framebuffer (the window's default
            // framebuffer, or an offscreen for a screenshot).
            gl.viewport(0, 0, width as i32, height as i32);
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::DEPTH_TEST);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
            gl.use_program(Some(self.program));
            gl.uniform_2_f32(self.u_size.as_ref(), width as f32, height as f32);
            gl.bind_vertex_array(Some(self.vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            let bytes: &[u8] =
                core::slice::from_raw_parts(self.verts.as_ptr() as *const u8, self.verts.len() * 4);
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STREAM_DRAW);
            gl.draw_arrays(glow::TRIANGLES, 0, (self.verts.len() / 6) as i32);
            gl.bind_vertex_array(None);
            gl.disable(glow::BLEND);
        }
    }

    /// Deletes the GL objects. The context must be current.
    pub fn destroy(self) {
        // SAFETY: the context is current; these objects are not used again.
        unsafe {
            self.gl.delete_vertex_array(self.vao);
            self.gl.delete_buffer(self.vbo);
            self.gl.delete_program(self.program);
        }
    }
}

fn round0(v: f32) -> i64 {
    v.round() as i64
}
fn round1(v: f32) -> String {
    format!("{:.1}", v)
}
fn round2(v: f32) -> String {
    format!("{:.2}", v)
}
