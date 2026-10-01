//! GPU playback of a [`Wallpaper`] display list: meshes, Gaussian-blurred
//! groups and screen/normal compositing, into a cached texture.
//!
//! Antialiasing is by 2x supersampling (render big, downsample); blurred
//! groups are blurred at quarter of that resolution, as the design spec's
//! Balanced tier does, which keeps the large glow blurs cheap.

use glow::HasContext;

use crate::shaders;
use crate::wallpaper::{Blend, Op, Wallpaper};
use crate::{Paint, RenderError, MAX_STOPS};

/// Supersampling factor for mesh antialiasing.
const SS: u32 = 2;
/// Blur groups are downsampled by this factor before blurring.
const BLUR_DOWNSCALE: u32 = 4;
/// Must match `RMAX` in the blur shader.
const BLUR_RMAX: i32 = 72;

pub(crate) struct Fbo {
    pub(crate) framebuffer: glow::Framebuffer,
    pub(crate) texture: glow::Texture,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

struct MeshUniforms {
    canvas: Option<glow::UniformLocation>,
    kind: Option<glow::UniformLocation>,
    frame: Option<glow::UniformLocation>,
    geom: Option<glow::UniformLocation>,
    count: Option<glow::UniformLocation>,
    colors: Option<glow::UniformLocation>,
    offsets: Option<glow::UniformLocation>,
    opacity: Option<glow::UniformLocation>,
}

struct BlurUniforms {
    tex: Option<glow::UniformLocation>,
    dir: Option<glow::UniformLocation>,
    sigma: Option<glow::UniformLocation>,
    radius: Option<glow::UniformLocation>,
}

struct CompositeUniforms {
    tex: Option<glow::UniformLocation>,
    opacity: Option<glow::UniformLocation>,
}

/// Renders wallpapers to a texture. Create with a current context and keep it
/// current for every call.
pub struct WallpaperRenderer {
    gl: glow::Context,
    mesh_program: glow::Program,
    blur_program: glow::Program,
    composite_program: glow::Program,
    mesh_uniforms: MeshUniforms,
    blur_uniforms: BlurUniforms,
    composite_uniforms: CompositeUniforms,
    mesh_vao: glow::VertexArray,
    mesh_vbo: glow::Buffer,
    mesh_ebo: glow::Buffer,
    quad_vao: glow::VertexArray,
    quad_vbo: glow::Buffer,
    hi: Option<Fbo>,
    layer: Option<Fbo>,
    blur_a: Option<Fbo>,
    blur_b: Option<Fbo>,
    out: Option<Fbo>,
    canvas: (f32, f32),
}

impl WallpaperRenderer {
    /// Compiles the three programs and sets up geometry.
    pub fn new(gl: glow::Context) -> Result<Self, RenderError> {
        let header = if gl.version().is_embedded {
            shaders::HEADER_ES
        } else {
            shaders::HEADER_DESKTOP
        };
        let mesh_program = link(&gl, header, shaders::MESH_VERTEX, shaders::MESH_FRAGMENT)?;
        let blur_program = link(&gl, header, shaders::QUAD_VERTEX, shaders::BLUR_FRAGMENT)?;
        let composite_program = link(
            &gl,
            header,
            shaders::QUAD_VERTEX,
            shaders::COMPOSITE_FRAGMENT,
        )?;

        // SAFETY: the caller made the context current; all objects below were
        // just created on it.
        let (mesh_vao, mesh_vbo, mesh_ebo, quad_vao, quad_vbo) = unsafe {
            let mesh_vao = gl.create_vertex_array().map_err(RenderError::Create)?;
            let mesh_vbo = gl.create_buffer().map_err(RenderError::Create)?;
            let mesh_ebo = gl.create_buffer().map_err(RenderError::Create)?;
            gl.bind_vertex_array(Some(mesh_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(mesh_vbo));
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(mesh_ebo));
            let loc = gl.get_attrib_location(mesh_program, "a_pos").unwrap_or(0);
            gl.enable_vertex_attrib_array(loc);
            gl.vertex_attrib_pointer_f32(loc, 2, glow::FLOAT, false, 8, 0);

            let quad_vao = gl.create_vertex_array().map_err(RenderError::Create)?;
            let quad_vbo = gl.create_buffer().map_err(RenderError::Create)?;
            gl.bind_vertex_array(Some(quad_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(quad_vbo));
            let quad: [f32; 8] = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];
            let bytes: Vec<u8> = quad.iter().flat_map(|v| v.to_ne_bytes()).collect();
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STATIC_DRAW);
            let qloc = gl.get_attrib_location(blur_program, "a_unit").unwrap_or(0);
            gl.enable_vertex_attrib_array(qloc);
            gl.vertex_attrib_pointer_f32(qloc, 2, glow::FLOAT, false, 8, 0);
            gl.bind_vertex_array(None);
            (mesh_vao, mesh_vbo, mesh_ebo, quad_vao, quad_vbo)
        };

        let mu = |n: &str| {
            // SAFETY: the program is linked on the current context.
            unsafe { gl.get_uniform_location(mesh_program, n) }
        };
        let bu = |n: &str| {
            // SAFETY: the program is linked on the current context.
            unsafe { gl.get_uniform_location(blur_program, n) }
        };
        let cu = |n: &str| {
            // SAFETY: the program is linked on the current context.
            unsafe { gl.get_uniform_location(composite_program, n) }
        };

        Ok(Self {
            mesh_uniforms: MeshUniforms {
                canvas: mu("u_canvas"),
                kind: mu("u_kind"),
                frame: mu("u_frame"),
                geom: mu("u_geom"),
                count: mu("u_count"),
                colors: mu("u_colors"),
                offsets: mu("u_offsets"),
                opacity: mu("u_opacity"),
            },
            blur_uniforms: BlurUniforms {
                tex: bu("u_tex"),
                dir: bu("u_dir"),
                sigma: bu("u_sigma"),
                radius: bu("u_radius"),
            },
            composite_uniforms: CompositeUniforms {
                tex: cu("u_tex"),
                opacity: cu("u_opacity"),
            },
            gl,
            mesh_program,
            blur_program,
            composite_program,
            mesh_vao,
            mesh_vbo,
            mesh_ebo,
            quad_vao,
            quad_vbo,
            hi: None,
            layer: None,
            blur_a: None,
            blur_b: None,
            out: None,
            canvas: (393.0, 852.0),
        })
    }

    /// The finished wallpaper texture, if one has been rendered.
    pub fn texture(&self) -> Option<glow::Texture> {
        self.out.as_ref().map(|f| f.texture)
    }

    /// Final output size in pixels, if rendered.
    pub fn size(&self) -> Option<(u32, u32)> {
        self.out.as_ref().map(|f| (f.width, f.height))
    }

    /// Renders `wallpaper` into a `width` x `height` pixel texture over a
    /// `canvas_w` x `canvas_h` point canvas.
    pub fn render(
        &mut self,
        wallpaper: &Wallpaper,
        canvas_w: f32,
        canvas_h: f32,
        width: u32,
        height: u32,
    ) -> Result<(), RenderError> {
        if width == 0 || height == 0 || !(canvas_w > 0.0 && canvas_h > 0.0) {
            return Err(RenderError::Size);
        }
        self.canvas = (canvas_w, canvas_h);
        let (hi_w, hi_h) = (width * SS, height * SS);
        let (q_w, q_h) = (hi_w / BLUR_DOWNSCALE, hi_h / BLUR_DOWNSCALE);
        self.ensure(Role::Hi, hi_w, hi_h)?;
        self.ensure(Role::Layer, hi_w, hi_h)?;
        self.ensure(Role::BlurA, q_w.max(1), q_h.max(1))?;
        self.ensure(Role::BlurB, q_w.max(1), q_h.max(1))?;
        self.ensure(Role::Out, width, height)?;

        let gl = &self.gl;
        // SAFETY: the context is current; every framebuffer and program used
        // below is alive (created in `new`/`ensure`, dropped only in `destroy`).
        unsafe {
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::DEPTH_TEST);
            self.bind(Role::Hi);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            for op in &wallpaper.ops {
                match op {
                    Op::Fill(fill) => {
                        self.bind(Role::Hi);
                        normal_blend(gl);
                        self.draw_mesh(
                            &fill.mesh.vertices,
                            &fill.mesh.indices,
                            &fill.paint,
                            fill.opacity,
                        );
                    }
                    Op::Group(group) => {
                        self.bind(Role::Layer);
                        gl.clear_color(0.0, 0.0, 0.0, 0.0);
                        gl.clear(glow::COLOR_BUFFER_BIT);
                        normal_blend(gl);
                        for fill in &group.fills {
                            self.draw_mesh(
                                &fill.mesh.vertices,
                                &fill.mesh.indices,
                                &fill.paint,
                                fill.opacity,
                            );
                        }
                        let source = if group.blur > 0.0 {
                            self.blur_group(group.blur);
                            Role::BlurA
                        } else {
                            Role::Layer
                        };
                        self.bind(Role::Hi);
                        match group.blend {
                            Blend::Normal => normal_blend(gl),
                            Blend::Screen => screen_blend(gl),
                        }
                        self.composite(source, group.opacity);
                    }
                }
            }

            // Downsample the supersampled result to the output texture.
            let hi = self.hi.as_ref().expect("hi");
            let out = self.out.as_ref().expect("out");
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(hi.framebuffer));
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(out.framebuffer));
            gl.blit_framebuffer(
                0,
                0,
                hi.width as i32,
                hi.height as i32,
                0,
                0,
                out.width as i32,
                out.height as i32,
                glow::COLOR_BUFFER_BIT,
                glow::LINEAR,
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.disable(glow::BLEND);
        }
        Ok(())
    }

    /// Reads the output texture back as top-first RGBA8. Allocates.
    pub fn read_pixels(&self) -> Option<Vec<u8>> {
        let out = self.out.as_ref()?;
        let row = out.width as usize * 4;
        let mut pixels = vec![0u8; row * out.height as usize];
        // SAFETY: the context is current; the buffer is exactly the right size.
        unsafe {
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(out.framebuffer));
            self.gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            self.gl.read_pixels(
                0,
                0,
                out.width as i32,
                out.height as i32,
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

    /// Blits the output texture to the default framebuffer at `width` x
    /// `height` pixels (for the preview window).
    pub fn present(&self, width: u32, height: u32) {
        let Some(out) = self.out.as_ref() else {
            return;
        };
        // SAFETY: the context is current; the output framebuffer is alive.
        unsafe {
            self.gl
                .bind_framebuffer(glow::READ_FRAMEBUFFER, Some(out.framebuffer));
            self.gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, None);
            self.gl.blit_framebuffer(
                0,
                0,
                out.width as i32,
                out.height as i32,
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
        for role in [Role::Hi, Role::Layer, Role::BlurA, Role::BlurB, Role::Out] {
            if let Some(f) = self.take(role) {
                delete_fbo(&self.gl, f);
            }
        }
        // SAFETY: the context is current; these objects are not used again.
        unsafe {
            self.gl.delete_vertex_array(self.mesh_vao);
            self.gl.delete_buffer(self.mesh_vbo);
            self.gl.delete_buffer(self.mesh_ebo);
            self.gl.delete_vertex_array(self.quad_vao);
            self.gl.delete_buffer(self.quad_vbo);
            self.gl.delete_program(self.mesh_program);
            self.gl.delete_program(self.blur_program);
            self.gl.delete_program(self.composite_program);
        }
    }

    // --- internals ---

    fn slot(&mut self, role: Role) -> &mut Option<Fbo> {
        match role {
            Role::Hi => &mut self.hi,
            Role::Layer => &mut self.layer,
            Role::BlurA => &mut self.blur_a,
            Role::BlurB => &mut self.blur_b,
            Role::Out => &mut self.out,
        }
    }

    fn take(&mut self, role: Role) -> Option<Fbo> {
        self.slot(role).take()
    }

    fn ensure(&mut self, role: Role, width: u32, height: u32) -> Result<(), RenderError> {
        let current = self.slot(role).as_ref().map(|f| (f.width, f.height));
        if current == Some((width, height)) {
            return Ok(());
        }
        if let Some(old) = self.take(role) {
            delete_fbo(&self.gl, old);
        }
        let fbo = create_fbo(&self.gl, width, height)?;
        *self.slot(role) = Some(fbo);
        Ok(())
    }

    fn fbo(&self, role: Role) -> &Fbo {
        let slot = match role {
            Role::Hi => &self.hi,
            Role::Layer => &self.layer,
            Role::BlurA => &self.blur_a,
            Role::BlurB => &self.blur_b,
            Role::Out => &self.out,
        };
        slot.as_ref().expect("target created by render()")
    }

    /// Binds a target as the draw framebuffer and sets the viewport.
    ///
    /// SAFETY: the context is current and the target is alive.
    unsafe fn bind(&self, role: Role) {
        let f = self.fbo(role);
        self.gl
            .bind_framebuffer(glow::FRAMEBUFFER, Some(f.framebuffer));
        self.gl.viewport(0, 0, f.width as i32, f.height as i32);
    }

    /// Uploads a mesh and draws it with a paint into the bound framebuffer.
    ///
    /// SAFETY: the context is current, a target is bound, and the blend mode
    /// is set by the caller.
    unsafe fn draw_mesh(
        &self,
        vertices: &[[f32; 2]],
        indices: &[u32],
        paint: &Paint,
        opacity: f32,
    ) {
        if indices.is_empty() {
            return;
        }
        let gl = &self.gl;
        gl.use_program(Some(self.mesh_program));
        gl.bind_vertex_array(Some(self.mesh_vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.mesh_vbo));
        let vbytes: &[u8] =
            core::slice::from_raw_parts(vertices.as_ptr() as *const u8, vertices.len() * 8);
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, vbytes, glow::STREAM_DRAW);
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.mesh_ebo));
        let ibytes: &[u8] =
            core::slice::from_raw_parts(indices.as_ptr() as *const u8, indices.len() * 4);
        gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, ibytes, glow::STREAM_DRAW);

        let u = &self.mesh_uniforms;
        gl.uniform_2_f32(u.canvas.as_ref(), self.canvas.0, self.canvas.1);
        gl.uniform_1_f32(u.opacity.as_ref(), opacity.clamp(0.0, 1.0));
        self.set_paint(paint);
        gl.draw_elements(glow::TRIANGLES, indices.len() as i32, glow::UNSIGNED_INT, 0);
    }

    /// Sets the mesh gradient uniforms. No allocation.
    ///
    /// SAFETY: the mesh program is in use on the current context.
    unsafe fn set_paint(&self, paint: &Paint) {
        let u = &self.mesh_uniforms;
        let gl = &self.gl;
        let mut colors = [0.0f32; MAX_STOPS * 4];
        let mut offsets = [0.0f32; MAX_STOPS];
        let (kind, frame, geom, count) = match *paint {
            Paint::Solid(c) => {
                colors[..4].copy_from_slice(&c.to_array());
                (0, crate::Rect::new(0.0, 0.0, 1.0, 1.0), [0.0; 4], 1)
            }
            Paint::Linear {
                frame,
                from,
                to,
                stops,
            } => {
                fill_stops(&stops, &mut colors, &mut offsets);
                (1, frame, [from[0], from[1], to[0], to[1]], stops.len())
            }
            Paint::Radial {
                frame,
                center,
                radius,
                stops,
            } => {
                fill_stops(&stops, &mut colors, &mut offsets);
                (
                    2,
                    frame,
                    [
                        center[0],
                        center[1],
                        radius[0].max(1e-6),
                        radius[1].max(1e-6),
                    ],
                    stops.len(),
                )
            }
        };
        let fw = if frame.w.abs() > 1e-6 { frame.w } else { 1.0 };
        let fh = if frame.h.abs() > 1e-6 { frame.h } else { 1.0 };
        gl.uniform_1_i32(u.kind.as_ref(), kind);
        gl.uniform_4_f32(u.frame.as_ref(), frame.x, frame.y, fw, fh);
        gl.uniform_4_f32(u.geom.as_ref(), geom[0], geom[1], geom[2], geom[3]);
        gl.uniform_1_i32(u.count.as_ref(), count.max(1) as i32);
        gl.uniform_4_f32_slice(u.colors.as_ref(), &colors);
        gl.uniform_1_f32_slice(u.offsets.as_ref(), &offsets);
    }

    /// Blurs the current layer at quarter resolution into `BlurA`.
    ///
    /// SAFETY: the context is current; all targets and the blur program are
    /// alive.
    unsafe fn blur_group(&self, sigma_points: f32) {
        let gl = &self.gl;
        let (qw, qh) = {
            let a = self.fbo(Role::BlurA);
            (a.width, a.height)
        };
        // Downsample the layer into BlurA.
        let layer = self.fbo(Role::Layer);
        let a = self.fbo(Role::BlurA);
        gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(layer.framebuffer));
        gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(a.framebuffer));
        gl.blit_framebuffer(
            0,
            0,
            layer.width as i32,
            layer.height as i32,
            0,
            0,
            qw as i32,
            qh as i32,
            glow::COLOR_BUFFER_BIT,
            glow::LINEAR,
        );
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);

        // sigma in quarter-resolution texels.
        let sigma = (sigma_points * (qw as f32) / self.canvas.0).max(0.5);
        let radius = ((sigma * 3.0).ceil() as i32).clamp(1, BLUR_RMAX);
        gl.disable(glow::BLEND);
        gl.use_program(Some(self.blur_program));
        gl.bind_vertex_array(Some(self.quad_vao));
        gl.uniform_1_i32(self.blur_uniforms.tex.as_ref(), 0);
        gl.uniform_1_f32(self.blur_uniforms.sigma.as_ref(), sigma);
        gl.uniform_1_i32(self.blur_uniforms.radius.as_ref(), radius);
        gl.active_texture(glow::TEXTURE0);

        // Horizontal: BlurA -> BlurB.
        let b = self.fbo(Role::BlurB);
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(b.framebuffer));
        gl.viewport(0, 0, qw as i32, qh as i32);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.fbo(Role::BlurA).texture));
        gl.uniform_2_f32(self.blur_uniforms.dir.as_ref(), 1.0 / qw as f32, 0.0);
        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);

        // Vertical: BlurB -> BlurA.
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo(Role::BlurA).framebuffer));
        gl.viewport(0, 0, qw as i32, qh as i32);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.fbo(Role::BlurB).texture));
        gl.uniform_2_f32(self.blur_uniforms.dir.as_ref(), 0.0, 1.0 / qh as f32);
        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        gl.bind_texture(glow::TEXTURE_2D, None);
    }

    /// Composites a source target over the bound framebuffer with opacity.
    ///
    /// SAFETY: the context is current, a target is bound, the blend mode is
    /// set, and `source` is alive.
    unsafe fn composite(&self, source: Role, opacity: f32) {
        let gl = &self.gl;
        gl.use_program(Some(self.composite_program));
        gl.bind_vertex_array(Some(self.quad_vao));
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.fbo(source).texture));
        gl.uniform_1_i32(self.composite_uniforms.tex.as_ref(), 0);
        gl.uniform_1_f32(
            self.composite_uniforms.opacity.as_ref(),
            opacity.clamp(0.0, 1.0),
        );
        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        gl.bind_texture(glow::TEXTURE_2D, None);
    }
}

#[derive(Clone, Copy)]
enum Role {
    Hi,
    Layer,
    BlurA,
    BlurB,
    Out,
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

/// Premultiplied source-over.
///
/// SAFETY: the context is current.
unsafe fn normal_blend(gl: &glow::Context) {
    gl.enable(glow::BLEND);
    gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
}

/// Screen blend for premultiplied sources: src + dst(1 - src).
///
/// SAFETY: the context is current.
unsafe fn screen_blend(gl: &glow::Context) {
    gl.enable(glow::BLEND);
    gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_COLOR);
}

pub(crate) fn create_fbo(gl: &glow::Context, width: u32, height: u32) -> Result<Fbo, RenderError> {
    if width == 0 || height == 0 || width > i32::MAX as u32 || height > i32::MAX as u32 {
        return Err(RenderError::Size);
    }
    // SAFETY: the context is current; sizes fit in i32.
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
        for (p, v) in [
            (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
            (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
            (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
            (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
        ] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, p, v as i32);
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
        let fbo = Fbo {
            framebuffer,
            texture,
            width,
            height,
        };
        if status == glow::FRAMEBUFFER_COMPLETE {
            Ok(fbo)
        } else {
            delete_fbo(gl, fbo);
            Err(RenderError::Framebuffer(status))
        }
    }
}

pub(crate) fn delete_fbo(gl: &glow::Context, f: Fbo) {
    // SAFETY: the context is current; `f` is consumed.
    unsafe {
        gl.delete_framebuffer(f.framebuffer);
        gl.delete_texture(f.texture);
    }
}

pub(crate) fn link(
    gl: &glow::Context,
    header: &str,
    vertex: &str,
    fragment: &str,
) -> Result<glow::Program, RenderError> {
    let compile = |kind: u32, body: &str| -> Result<glow::Shader, RenderError> {
        // SAFETY: the context is current.
        unsafe {
            let s = gl.create_shader(kind).map_err(RenderError::Create)?;
            gl.shader_source(s, &format!("{header}{body}"));
            gl.compile_shader(s);
            if gl.get_shader_compile_status(s) {
                Ok(s)
            } else {
                let log = gl.get_shader_info_log(s);
                gl.delete_shader(s);
                Err(RenderError::Compile(log))
            }
        }
    };
    let vs = compile(glow::VERTEX_SHADER, vertex)?;
    let fs = match compile(glow::FRAGMENT_SHADER, fragment) {
        Ok(fs) => fs,
        Err(e) => {
            // SAFETY: vs was created on the current context.
            unsafe { gl.delete_shader(vs) };
            return Err(e);
        }
    };
    // SAFETY: the context is current; vs and fs are valid shaders on it.
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
