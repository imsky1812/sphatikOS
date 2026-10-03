//! Glass v0 (WP 2.4): a rounded panel showing a blurred copy of the
//! wallpaper behind it.
//!
//! The backdrop is downsampled to quarter resolution and blurred with a few
//! dual-Kawase passes (the handbook's Balanced-tier approach), then sampled
//! inside a continuous-curvature rounded rectangle with the material's
//! gradient fill on top. Refraction, rim light, specular and dispersion
//! arrive in WP 2.5.

use glow::HasContext;

use crate::shaders;
use crate::wallpaper_gl::{self, Fbo};
use crate::{Paint, Rect, RenderError, Stops, MAX_STOPS};

/// Kawase pass texel offsets, in blur-resolution texels, applied in order.
/// A smooth progression at half resolution gives a clean, Gaussian-like
/// frosted blur close to the prototype's 16 px backdrop-filter.
const KAWASE_OFFSETS: &[f32] = &[1.0, 2.0, 3.0, 4.0, 5.0, 5.0];
/// The backdrop is blurred at this fraction of the output resolution.
const BLUR_DOWNSCALE: u32 = 2;

/// One rounded glass panel.
#[derive(Clone, Debug, PartialEq)]
pub struct GlassPanel {
    /// Bounds in points.
    pub rect: Rect,
    /// Corner radius in points.
    pub radius: f32,
    /// The material's gradient fill, composited over the blurred backdrop.
    pub fill: Paint,
    /// The specular spot position, 0..1 across the panel (from the pointer or
    /// the gyroscope). The prototype's default is `--lx:30% --ly:0%`.
    pub light: [f32; 2],
}

impl GlassPanel {
    /// A Regular-glass panel (`.glass`): the prototype's 165-degree white
    /// gradient over the blur.
    pub fn regular(rect: Rect, radius: f32) -> Self {
        let fill = Paint::css_linear(
            rect,
            165.0,
            Stops::css(&[
                (0.0, crate::Color::WHITE.with_alpha(0.13)),
                (0.42, crate::Color::WHITE.with_alpha(0.03)),
                (1.0, crate::Color::WHITE.with_alpha(0.07)),
            ]),
        );
        Self {
            rect,
            radius,
            fill,
            light: [0.3, 0.0],
        }
    }

    /// Sets the specular light position, 0..1 across the panel.
    pub fn with_light(mut self, light: [f32; 2]) -> Self {
        self.light = light;
        self
    }
}

struct GlassUniforms {
    canvas: Option<glow::UniformLocation>,
    rect: Option<glow::UniformLocation>,
    pad: Option<glow::UniformLocation>,
    radius: Option<glow::UniformLocation>,
    scale: Option<glow::UniformLocation>,
    light: Option<glow::UniformLocation>,
    frame: Option<glow::UniformLocation>,
    geom: Option<glow::UniformLocation>,
    count: Option<glow::UniformLocation>,
    premul: Option<glow::UniformLocation>,
    colors: Option<glow::UniformLocation>,
    offsets: Option<glow::UniformLocation>,
    backdrop: Option<glow::UniformLocation>,
}

struct ShadowUniforms {
    canvas: Option<glow::UniformLocation>,
    rect: Option<glow::UniformLocation>,
    pad: Option<glow::UniformLocation>,
    radius: Option<glow::UniformLocation>,
    scale: Option<glow::UniformLocation>,
    offset: Option<glow::UniformLocation>,
    spread: Option<glow::UniformLocation>,
    alpha: Option<glow::UniformLocation>,
}

/// Drop-shadow geometry, in points.
const SHADOW_OFFSET: f32 = 14.0;
const SHADOW_SPREAD: f32 = 30.0;
const SHADOW_ALPHA: f32 = 0.30;
const SHADOW_PAD: f32 = SHADOW_OFFSET + SHADOW_SPREAD + 4.0;
/// Glass quad padding (room for 1 px antialiasing).
const GLASS_PAD: f32 = 2.0;

/// Composites glass panels over a backdrop texture into a scene texture.
pub struct GlassRenderer {
    gl: glow::Context,
    copy_program: glow::Program,
    kawase_program: glow::Program,
    glass_program: glow::Program,
    shadow_program: glow::Program,
    copy_tex: Option<glow::UniformLocation>,
    kawase_tex: Option<glow::UniformLocation>,
    kawase_offset: Option<glow::UniformLocation>,
    glass: GlassUniforms,
    shadow: ShadowUniforms,
    quad_vao: glow::VertexArray,
    quad_vbo: glow::Buffer,
    scene: Option<Fbo>,
    blur_a: Option<Fbo>,
    blur_b: Option<Fbo>,
    canvas: (f32, f32),
}

impl GlassRenderer {
    /// Compiles the copy, blur and glass programs.
    pub fn new(gl: glow::Context) -> Result<Self, RenderError> {
        let header = if gl.version().is_embedded {
            shaders::HEADER_ES
        } else {
            shaders::HEADER_DESKTOP
        };
        let copy_program =
            wallpaper_gl::link(&gl, header, shaders::QUAD_VERTEX, shaders::COPY_FRAGMENT)?;
        let kawase_program =
            wallpaper_gl::link(&gl, header, shaders::QUAD_VERTEX, shaders::KAWASE_FRAGMENT)?;
        let glass_program =
            wallpaper_gl::link(&gl, header, shaders::GLASS_VERTEX, shaders::GLASS_FRAGMENT)?;
        let shadow_program =
            wallpaper_gl::link(&gl, header, shaders::GLASS_VERTEX, shaders::SHADOW_FRAGMENT)?;

        // SAFETY: the caller made the context current; the quad buffer and VAO
        // are created on it here.
        let (quad_vao, quad_vbo) = unsafe {
            let vao = gl.create_vertex_array().map_err(RenderError::Create)?;
            let vbo = gl.create_buffer().map_err(RenderError::Create)?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let quad: [f32; 8] = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];
            let bytes: Vec<u8> = quad.iter().flat_map(|v| v.to_ne_bytes()).collect();
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STATIC_DRAW);
            for prog in [copy_program, kawase_program, glass_program, shadow_program] {
                let loc = gl.get_attrib_location(prog, "a_unit").unwrap_or(0);
                gl.enable_vertex_attrib_array(loc);
                gl.vertex_attrib_pointer_f32(loc, 2, glow::FLOAT, false, 8, 0);
            }
            gl.bind_vertex_array(None);
            (vao, vbo)
        };
        let u = |p: glow::Program, n: &str| {
            // SAFETY: the program is linked on the current context.
            unsafe { gl.get_uniform_location(p, n) }
        };
        Ok(Self {
            copy_tex: u(copy_program, "u_tex"),
            kawase_tex: u(kawase_program, "u_tex"),
            kawase_offset: u(kawase_program, "u_offset"),
            glass: GlassUniforms {
                canvas: u(glass_program, "u_canvas"),
                rect: u(glass_program, "u_rect"),
                pad: u(glass_program, "u_pad"),
                radius: u(glass_program, "u_radius"),
                scale: u(glass_program, "u_scale"),
                light: u(glass_program, "u_light"),
                frame: u(glass_program, "u_frame"),
                geom: u(glass_program, "u_geom"),
                count: u(glass_program, "u_count"),
                premul: u(glass_program, "u_premul"),
                colors: u(glass_program, "u_colors"),
                offsets: u(glass_program, "u_offsets"),
                backdrop: u(glass_program, "u_backdrop"),
            },
            shadow: ShadowUniforms {
                canvas: u(shadow_program, "u_canvas"),
                rect: u(shadow_program, "u_rect"),
                pad: u(shadow_program, "u_pad"),
                radius: u(shadow_program, "u_radius"),
                scale: u(shadow_program, "u_scale"),
                offset: u(shadow_program, "u_offset"),
                spread: u(shadow_program, "u_spread"),
                alpha: u(shadow_program, "u_alpha"),
            },
            gl,
            copy_program,
            kawase_program,
            glass_program,
            shadow_program,
            quad_vao,
            quad_vbo,
            scene: None,
            blur_a: None,
            blur_b: None,
            canvas: (393.0, 852.0),
        })
    }

    /// The finished scene texture, if rendered.
    pub fn texture(&self) -> Option<glow::Texture> {
        self.scene.as_ref().map(|f| f.texture)
    }

    /// Output size in pixels, if rendered.
    pub fn size(&self) -> Option<(u32, u32)> {
        self.scene.as_ref().map(|f| (f.width, f.height))
    }

    /// Composites `panels` over `backdrop` (a `width` x `height` pixel texture
    /// covering the whole `canvas`) into the scene texture.
    pub fn render(
        &mut self,
        backdrop: glow::Texture,
        canvas_w: f32,
        canvas_h: f32,
        width: u32,
        height: u32,
        panels: &[GlassPanel],
    ) -> Result<(), RenderError> {
        if width == 0 || height == 0 || !(canvas_w > 0.0 && canvas_h > 0.0) {
            return Err(RenderError::Size);
        }
        self.canvas = (canvas_w, canvas_h);
        let (qw, qh) = (
            (width / BLUR_DOWNSCALE).max(1),
            (height / BLUR_DOWNSCALE).max(1),
        );
        self.scene = Some(ensure(&self.gl, self.scene.take(), width, height)?);
        self.blur_a = Some(ensure(&self.gl, self.blur_a.take(), qw, qh)?);
        self.blur_b = Some(ensure(&self.gl, self.blur_b.take(), qw, qh)?);

        let gl = &self.gl;
        // SAFETY: the context is current; all framebuffers, programs, the VAO
        // and the backdrop texture are alive for this call.
        unsafe {
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::BLEND);
            gl.bind_vertex_array(Some(self.quad_vao));
            gl.active_texture(glow::TEXTURE0);

            // Seed the scene with the sharp backdrop.
            let scene = self.scene.as_ref().expect("scene");
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(scene.framebuffer));
            gl.viewport(0, 0, width as i32, height as i32);
            gl.use_program(Some(self.copy_program));
            gl.uniform_1_i32(self.copy_tex.as_ref(), 0);
            gl.bind_texture(glow::TEXTURE_2D, Some(backdrop));
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);

            // Downsample the backdrop into blur_a.
            let a = self.blur_a.as_ref().expect("blur_a");
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(a.framebuffer));
            gl.viewport(0, 0, qw as i32, qh as i32);
            gl.bind_texture(glow::TEXTURE_2D, Some(backdrop));
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);

            // Dual-Kawase passes, ping-ponging blur_a <-> blur_b.
            gl.use_program(Some(self.kawase_program));
            gl.uniform_1_i32(self.kawase_tex.as_ref(), 0);
            let mut src_is_a = true;
            for &offset in KAWASE_OFFSETS {
                let (src, dst) = if src_is_a {
                    (self.blur_a.as_ref(), self.blur_b.as_ref())
                } else {
                    (self.blur_b.as_ref(), self.blur_a.as_ref())
                };
                let (src, dst) = (src.expect("src"), dst.expect("dst"));
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(dst.framebuffer));
                gl.viewport(0, 0, qw as i32, qh as i32);
                gl.bind_texture(glow::TEXTURE_2D, Some(src.texture));
                gl.uniform_2_f32(
                    self.kawase_offset.as_ref(),
                    offset / qw as f32,
                    offset / qh as f32,
                );
                gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                src_is_a = !src_is_a;
            }
            let blurred = if src_is_a {
                self.blur_a.as_ref()
            } else {
                self.blur_b.as_ref()
            };
            let blurred = blurred.expect("blurred").texture;

            // Draw each panel's drop shadow, then the glass, over the scene.
            let scene = self.scene.as_ref().expect("scene");
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(scene.framebuffer));
            gl.viewport(0, 0, width as i32, height as i32);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
            let scale = width as f32 / canvas_w;

            // Shadows first (no texture needed).
            gl.use_program(Some(self.shadow_program));
            let s = &self.shadow;
            gl.uniform_2_f32(s.canvas.as_ref(), canvas_w, canvas_h);
            gl.uniform_2_f32(s.pad.as_ref(), SHADOW_PAD, SHADOW_PAD);
            gl.uniform_1_f32(s.scale.as_ref(), scale);
            gl.uniform_1_f32(s.offset.as_ref(), SHADOW_OFFSET);
            gl.uniform_1_f32(s.spread.as_ref(), SHADOW_SPREAD);
            gl.uniform_1_f32(s.alpha.as_ref(), SHADOW_ALPHA);
            for panel in panels {
                gl.uniform_4_f32(
                    s.rect.as_ref(),
                    panel.rect.x,
                    panel.rect.y,
                    panel.rect.w,
                    panel.rect.h,
                );
                gl.uniform_1_f32(s.radius.as_ref(), panel.radius.max(0.0));
                gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
            }

            // Then the glass panels.
            gl.use_program(Some(self.glass_program));
            gl.bind_texture(glow::TEXTURE_2D, Some(blurred));
            gl.uniform_1_i32(self.glass.backdrop.as_ref(), 0);
            gl.uniform_2_f32(self.glass.canvas.as_ref(), canvas_w, canvas_h);
            gl.uniform_2_f32(self.glass.pad.as_ref(), GLASS_PAD, GLASS_PAD);
            gl.uniform_1_f32(self.glass.scale.as_ref(), scale);
            for panel in panels {
                self.set_panel(panel);
                gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
            }
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.disable(glow::BLEND);
        }
        Ok(())
    }

    /// Reads the scene back as top-first RGBA8. Allocates; for tests.
    pub fn read_pixels(&self) -> Option<Vec<u8>> {
        let scene = self.scene.as_ref()?;
        let row = scene.width as usize * 4;
        let mut pixels = vec![0u8; row * scene.height as usize];
        // SAFETY: the context is current; the buffer is the right size.
        unsafe {
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(scene.framebuffer));
            self.gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            self.gl.read_pixels(
                0,
                0,
                scene.width as i32,
                scene.height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
        let mut flipped = vec![0u8; pixels.len()];
        for (dst, src) in flipped
            .chunks_exact_mut(row)
            .zip(pixels.chunks_exact(row).rev())
        {
            dst.copy_from_slice(src);
        }
        Some(flipped)
    }

    /// Blits the scene to the default framebuffer at `width` x `height`.
    pub fn present(&self, width: u32, height: u32) {
        let Some(scene) = self.scene.as_ref() else {
            return;
        };
        // SAFETY: the context is current; the scene framebuffer is alive.
        unsafe {
            self.gl
                .bind_framebuffer(glow::READ_FRAMEBUFFER, Some(scene.framebuffer));
            self.gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, None);
            self.gl.blit_framebuffer(
                0,
                0,
                scene.width as i32,
                scene.height as i32,
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

    /// Deletes every GL object. The context must be current.
    pub fn destroy(mut self) {
        for f in [self.scene.take(), self.blur_a.take(), self.blur_b.take()]
            .into_iter()
            .flatten()
        {
            wallpaper_gl::delete_fbo(&self.gl, f);
        }
        // SAFETY: the context is current; these objects are not used again.
        unsafe {
            self.gl.delete_vertex_array(self.quad_vao);
            self.gl.delete_buffer(self.quad_vbo);
            self.gl.delete_program(self.copy_program);
            self.gl.delete_program(self.kawase_program);
            self.gl.delete_program(self.glass_program);
            self.gl.delete_program(self.shadow_program);
        }
    }

    /// Sets the per-panel uniforms. No allocation.
    ///
    /// SAFETY: the glass program is in use on the current context.
    unsafe fn set_panel(&self, panel: &GlassPanel) {
        let gl = &self.gl;
        let g = &self.glass;
        gl.uniform_4_f32(
            g.rect.as_ref(),
            panel.rect.x,
            panel.rect.y,
            panel.rect.w,
            panel.rect.h,
        );
        gl.uniform_1_f32(g.radius.as_ref(), panel.radius.max(0.0));
        gl.uniform_2_f32(g.light.as_ref(), panel.light[0], panel.light[1]);
        let mut colors = [0.0f32; MAX_STOPS * 4];
        let mut offsets = [0.0f32; MAX_STOPS];
        let (frame, geom, count, premul) = match panel.fill {
            Paint::Linear {
                frame,
                from,
                to,
                stops,
            } => {
                fill_stops(&stops, &mut colors, &mut offsets);
                (
                    frame,
                    [from[0], from[1], to[0], to[1]],
                    stops.len(),
                    stops.premultiplied(),
                )
            }
            Paint::Solid(c) => {
                colors[..4].copy_from_slice(&c.to_array());
                (
                    Rect::new(0.0, 0.0, 1.0, 1.0),
                    [0.0, 0.0, 1.0, 0.0],
                    1,
                    false,
                )
            }
            Paint::Radial { frame, stops, .. } => {
                fill_stops(&stops, &mut colors, &mut offsets);
                (
                    frame,
                    [0.0, 0.0, 1.0, 0.0],
                    stops.len(),
                    stops.premultiplied(),
                )
            }
        };
        let fw = if frame.w.abs() > 1e-6 { frame.w } else { 1.0 };
        let fh = if frame.h.abs() > 1e-6 { frame.h } else { 1.0 };
        gl.uniform_4_f32(g.frame.as_ref(), frame.x, frame.y, fw, fh);
        gl.uniform_4_f32(g.geom.as_ref(), geom[0], geom[1], geom[2], geom[3]);
        gl.uniform_1_i32(g.count.as_ref(), count.max(1) as i32);
        gl.uniform_1_i32(g.premul.as_ref(), i32::from(premul));
        gl.uniform_4_f32_slice(g.colors.as_ref(), &colors);
        gl.uniform_1_f32_slice(g.offsets.as_ref(), &offsets);
    }
}

fn fill_stops(stops: &Stops, colors: &mut [f32; MAX_STOPS * 4], offsets: &mut [f32; MAX_STOPS]) {
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

fn ensure(
    gl: &glow::Context,
    existing: Option<Fbo>,
    width: u32,
    height: u32,
) -> Result<Fbo, RenderError> {
    if let Some(f) = existing {
        if f.width == width && f.height == height {
            return Ok(f);
        }
        wallpaper_gl::delete_fbo(gl, f);
    }
    wallpaper_gl::create_fbo(gl, width, height)
}
