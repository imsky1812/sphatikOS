//! The GLES renderer: shapes into a persistent scene target, then present.

use std::fmt;

use glow::HasContext;

use crate::shaders;
use crate::{Color, Paint, Rect, Shape, MAX_STOPS};

/// Why the renderer could not be created or resized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// A GL object could not be created.
    Create(String),
    /// A shader failed to compile; carries the driver's log.
    Compile(String),
    /// The shader program failed to link; carries the driver's log.
    Link(String),
    /// The off-screen target is incomplete (GL status code).
    Framebuffer(u32),
    /// A size of zero or beyond the GL limit was requested.
    Size,
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Create(e) => write!(f, "could not create a GL object: {e}"),
            Self::Compile(log) => write!(f, "shader compile failed: {log}"),
            Self::Link(log) => write!(f, "shader link failed: {log}"),
            Self::Framebuffer(status) => write!(f, "framebuffer incomplete: 0x{status:04X}"),
            Self::Size => write!(f, "invalid target size"),
        }
    }
}

impl std::error::Error for RenderError {}

/// What one call to [`Renderer::render`] did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameStats {
    /// False when there was no damage and nothing was drawn.
    pub drawn: bool,
    /// Shapes that touched the damaged region and were drawn.
    pub shapes_drawn: u32,
}

struct Uniforms {
    canvas: Option<glow::UniformLocation>,
    rect: Option<glow::UniformLocation>,
    radius: Option<glow::UniformLocation>,
    scale: Option<glow::UniformLocation>,
    kind: Option<glow::UniformLocation>,
    frame: Option<glow::UniformLocation>,
    geom: Option<glow::UniformLocation>,
    count: Option<glow::UniformLocation>,
    premul: Option<glow::UniformLocation>,
    colors: Option<glow::UniformLocation>,
    offsets: Option<glow::UniformLocation>,
    opacity: Option<glow::UniformLocation>,
}

struct Target {
    framebuffer: glow::Framebuffer,
    texture: glow::Texture,
    width: u32,
    height: u32,
}

/// Draws [`Shape`]s with OpenGL ES 3 (or desktop GL 3.3) through `glow`.
///
/// Create it with a context that is current, and keep that context current
/// whenever calling it.
pub struct Renderer {
    gl: glow::Context,
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    uniforms: Uniforms,
    target: Option<Target>,
    canvas: (f32, f32),
}

impl Renderer {
    /// Compiles the shaders and sets up geometry. The target is created by
    /// the first [`Renderer::resize`].
    pub fn new(gl: glow::Context) -> Result<Self, RenderError> {
        let header = if gl.version().is_embedded {
            shaders::HEADER_ES
        } else {
            shaders::HEADER_DESKTOP
        };
        let program = link_program(&gl, header)?;
        let unit_quad: [f32; 8] = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let bytes: Vec<u8> = unit_quad.iter().flat_map(|v| v.to_ne_bytes()).collect();

        // SAFETY: the caller made `gl`'s context current (documented
        // precondition); every object used here was just created on it.
        let (vao, vbo, uniforms) = unsafe {
            let vao = gl.create_vertex_array().map_err(RenderError::Create)?;
            let vbo = gl.create_buffer().map_err(RenderError::Create)?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STATIC_DRAW);
            let loc = gl.get_attrib_location(program, "a_unit").unwrap_or(0);
            gl.enable_vertex_attrib_array(loc);
            gl.vertex_attrib_pointer_f32(loc, 2, glow::FLOAT, false, 8, 0);
            gl.bind_vertex_array(None);
            let u = |name: &str| gl.get_uniform_location(program, name);
            let uniforms = Uniforms {
                canvas: u("u_canvas"),
                rect: u("u_rect"),
                radius: u("u_radius"),
                scale: u("u_scale"),
                kind: u("u_kind"),
                frame: u("u_frame"),
                geom: u("u_geom"),
                count: u("u_count"),
                premul: u("u_premul"),
                colors: u("u_colors"),
                offsets: u("u_offsets"),
                opacity: u("u_opacity"),
            };
            (vao, vbo, uniforms)
        };
        Ok(Self {
            gl,
            program,
            vao,
            vbo,
            uniforms,
            target: None,
            canvas: (1.0, 1.0),
        })
    }

    /// The GL context, for hosts that need it (screenshots, overlays).
    pub fn gl(&self) -> &glow::Context {
        &self.gl
    }

    /// Target size in pixels, if a target exists.
    pub fn target_size(&self) -> Option<(u32, u32)> {
        self.target.as_ref().map(|t| (t.width, t.height))
    }

    /// Sets the logical canvas (points) and the pixel size of the scene
    /// target, recreating the target if the pixel size changed. The caller
    /// must then damage the whole canvas.
    pub fn resize(
        &mut self,
        canvas_w: f32,
        canvas_h: f32,
        width: u32,
        height: u32,
    ) -> Result<(), RenderError> {
        if width == 0 || height == 0 || width > i32::MAX as u32 || height > i32::MAX as u32 {
            return Err(RenderError::Size);
        }
        if !(canvas_w.is_finite() && canvas_h.is_finite()) || canvas_w <= 0.0 || canvas_h <= 0.0 {
            return Err(RenderError::Size);
        }
        self.canvas = (canvas_w, canvas_h);
        if self.target_size() == Some((width, height)) {
            return Ok(());
        }
        if let Some(old) = self.target.take() {
            delete_target(&self.gl, old);
        }
        self.target = Some(create_target(&self.gl, width, height)?);
        Ok(())
    }

    /// Redraws the damaged region of the scene target: clears it to
    /// `background` and draws every shape that touches it, in order. With no
    /// damage (or no target) nothing is drawn. Doesn't allocate.
    pub fn render(
        &mut self,
        shapes: &[Shape],
        damage: Option<Rect>,
        background: Color,
    ) -> FrameStats {
        let (Some(target), Some(damage)) = (self.target.as_ref(), damage) else {
            return FrameStats::default();
        };
        let (cw, ch) = self.canvas;
        let scale = target.width as f32 / cw;
        let Some((sx, sy, sw, sh)) = damage.to_pixels(scale, target.width, target.height) else {
            return FrameStats::default();
        };
        let gl = &self.gl;
        let u = &self.uniforms;
        let mut drawn = 0_u32;

        // SAFETY: the context is current (documented precondition), and the
        // program, vertex array and framebuffer were created on it and are
        // still alive (only `destroy` and `resize` delete them).
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(target.framebuffer));
            gl.viewport(0, 0, target.width as i32, target.height as i32);
            gl.enable(glow::SCISSOR_TEST);
            // GL scissor rectangles start at the bottom-left.
            gl.scissor(
                sx as i32,
                (target.height - sy - sh) as i32,
                sw as i32,
                sh as i32,
            );
            let bg = premultiplied(background);
            gl.clear_color(bg[0], bg[1], bg[2], bg[3]);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vao));
            gl.uniform_2_f32(u.canvas.as_ref(), cw, ch);
            gl.uniform_1_f32(u.scale.as_ref(), scale);

            for shape in shapes {
                if shape.bounds().intersection(&damage).is_none() {
                    continue;
                }
                let r = shape.rect;
                gl.uniform_4_f32(u.rect.as_ref(), r.x, r.y, r.w, r.h);
                gl.uniform_1_f32(u.radius.as_ref(), shape.radius.max(0.0));
                gl.uniform_1_f32(u.opacity.as_ref(), shape.opacity.clamp(0.0, 1.0));
                set_paint(gl, u, &shape.paint);
                gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                drawn += 1;
            }

            gl.bind_vertex_array(None);
            gl.disable(glow::SCISSOR_TEST);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
        FrameStats {
            drawn: true,
            shapes_drawn: drawn,
        }
    }

    /// Copies the scene target to the default framebuffer (the window),
    /// scaling it to `width` x `height` pixels. The host then swaps buffers.
    pub fn present(&self, width: u32, height: u32) {
        let Some(t) = self.target.as_ref() else {
            return;
        };
        // SAFETY: the context is current; the target framebuffer is alive.
        unsafe {
            self.gl
                .bind_framebuffer(glow::READ_FRAMEBUFFER, Some(t.framebuffer));
            self.gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, None);
            self.gl.blit_framebuffer(
                0,
                0,
                t.width as i32,
                t.height as i32,
                0,
                0,
                width as i32,
                height as i32,
                glow::COLOR_BUFFER_BIT,
                glow::LINEAR,
            );
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
    }

    /// Reads the scene target back as tightly packed RGBA8 rows, top row
    /// first. Returns `None` without a target. Allocates; for screenshots and
    /// tests, not per frame.
    pub fn read_pixels(&self) -> Option<Vec<u8>> {
        let t = self.target.as_ref()?;
        let row = t.width as usize * 4;
        let mut pixels = vec![0_u8; row * t.height as usize];
        // SAFETY: the context is current; the buffer holds exactly
        // width * height RGBA8 pixels, which is what is requested.
        unsafe {
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(t.framebuffer));
            self.gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            self.gl.read_pixels(
                0,
                0,
                t.width as i32,
                t.height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
        // GL rows start at the bottom; flip to top-first.
        let mut flipped = vec![0_u8; pixels.len()];
        for (dst, src) in flipped
            .chunks_exact_mut(row)
            .zip(pixels.chunks_exact(row).rev())
        {
            dst.copy_from_slice(src);
        }
        Some(flipped)
    }

    /// Deletes every GL object. The context must be current.
    pub fn destroy(mut self) {
        if let Some(t) = self.target.take() {
            delete_target(&self.gl, t);
        }
        // SAFETY: the context is current; these objects belong to it and are
        // not used again (`self` is consumed).
        unsafe {
            self.gl.delete_vertex_array(self.vao);
            self.gl.delete_buffer(self.vbo);
            self.gl.delete_program(self.program);
        }
    }
}

fn premultiplied(c: Color) -> [f32; 4] {
    [c.r * c.a, c.g * c.a, c.b * c.a, c.a]
}

/// Sets the paint uniforms. Arrays are built on the stack: no allocation.
fn set_paint(gl: &glow::Context, u: &Uniforms, paint: &Paint) {
    let mut colors = [0.0_f32; MAX_STOPS * 4];
    let mut offsets = [0.0_f32; MAX_STOPS];
    let (kind, frame, geom, count, premul) = match *paint {
        Paint::Solid(c) => {
            colors[..4].copy_from_slice(&c.to_array());
            (0, Rect::new(0.0, 0.0, 1.0, 1.0), [0.0; 4], 1, false)
        }
        Paint::Linear {
            frame,
            from,
            to,
            stops,
        } => {
            fill_stops(&stops, &mut colors, &mut offsets);
            let g = [from[0], from[1], to[0], to[1]];
            (1, frame, g, stops.len(), stops.premultiplied())
        }
        Paint::Radial {
            frame,
            center,
            radius,
            stops,
        } => {
            fill_stops(&stops, &mut colors, &mut offsets);
            let g = [
                center[0],
                center[1],
                radius[0].max(1e-6),
                radius[1].max(1e-6),
            ];
            (2, frame, g, stops.len(), stops.premultiplied())
        }
    };
    let frame_w = if frame.w.abs() > 1e-6 { frame.w } else { 1.0 };
    let frame_h = if frame.h.abs() > 1e-6 { frame.h } else { 1.0 };
    // SAFETY: called from `render` with the context current and the shape
    // program bound; the locations belong to that program.
    unsafe {
        gl.uniform_1_i32(u.kind.as_ref(), kind);
        gl.uniform_4_f32(u.frame.as_ref(), frame.x, frame.y, frame_w, frame_h);
        gl.uniform_4_f32(u.geom.as_ref(), geom[0], geom[1], geom[2], geom[3]);
        gl.uniform_1_i32(u.count.as_ref(), count.max(1) as i32);
        gl.uniform_1_i32(u.premul.as_ref(), i32::from(premul));
        gl.uniform_4_f32_slice(u.colors.as_ref(), &colors);
        gl.uniform_1_f32_slice(u.offsets.as_ref(), &offsets);
    }
}

fn fill_stops(
    stops: &crate::Stops,
    colors: &mut [f32; MAX_STOPS * 4],
    offsets: &mut [f32; MAX_STOPS],
) {
    for (i, (c, o)) in stops
        .colors()
        .iter()
        .zip(stops.offsets())
        .take(stops.len())
        .enumerate()
    {
        colors[i * 4..i * 4 + 4].copy_from_slice(&c.to_array());
        offsets[i] = *o;
    }
}

fn compile(
    gl: &glow::Context,
    kind: u32,
    header: &str,
    body: &str,
) -> Result<glow::Shader, RenderError> {
    let source = format!("{header}{body}");
    // SAFETY: the context is current (precondition of `Renderer::new`).
    unsafe {
        let shader = gl.create_shader(kind).map_err(RenderError::Create)?;
        gl.shader_source(shader, &source);
        gl.compile_shader(shader);
        if gl.get_shader_compile_status(shader) {
            Ok(shader)
        } else {
            let log = gl.get_shader_info_log(shader);
            gl.delete_shader(shader);
            Err(RenderError::Compile(log))
        }
    }
}

fn link_program(gl: &glow::Context, header: &str) -> Result<glow::Program, RenderError> {
    let vs = compile(gl, glow::VERTEX_SHADER, header, shaders::SHAPE_VERTEX)?;
    let fs = match compile(gl, glow::FRAGMENT_SHADER, header, shaders::SHAPE_FRAGMENT) {
        Ok(fs) => fs,
        Err(e) => {
            // SAFETY: `vs` was created on this current context.
            unsafe { gl.delete_shader(vs) };
            return Err(e);
        }
    };
    // SAFETY: the context is current; `vs` and `fs` were created on it.
    unsafe {
        let program = gl.create_program().map_err(RenderError::Create)?;
        gl.attach_shader(program, vs);
        gl.attach_shader(program, fs);
        gl.link_program(program);
        gl.detach_shader(program, vs);
        gl.detach_shader(program, fs);
        gl.delete_shader(vs);
        gl.delete_shader(fs);
        if gl.get_program_link_status(program) {
            Ok(program)
        } else {
            let log = gl.get_program_info_log(program);
            gl.delete_program(program);
            Err(RenderError::Link(log))
        }
    }
}

fn create_target(gl: &glow::Context, width: u32, height: u32) -> Result<Target, RenderError> {
    // SAFETY: the context is current; sizes were checked to fit in i32.
    unsafe {
        let texture = gl.create_texture().map_err(RenderError::Create)?;
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            width as i32,
            height as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(None),
        );
        for (param, value) in [
            (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
            (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
            (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
            (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
        ] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, param, value as i32);
        }
        gl.bind_texture(glow::TEXTURE_2D, None);

        let framebuffer = match gl.create_framebuffer() {
            Ok(fb) => fb,
            Err(e) => {
                gl.delete_texture(texture);
                return Err(RenderError::Create(e));
            }
        };
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(texture),
            0,
        );
        let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        let target = Target {
            framebuffer,
            texture,
            width,
            height,
        };
        if status == glow::FRAMEBUFFER_COMPLETE {
            Ok(target)
        } else {
            delete_target(gl, target);
            Err(RenderError::Framebuffer(status))
        }
    }
}

fn delete_target(gl: &glow::Context, t: Target) {
    // SAFETY: the context is current; the objects belong to it and `t` is
    // consumed, so they are not used again.
    unsafe {
        gl.delete_framebuffer(t.framebuffer);
        gl.delete_texture(t.texture);
    }
}
