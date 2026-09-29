//! Window and OpenGL context creation with glutin.

use std::error::Error;
use std::num::NonZeroU32;

use glutin::config::{ConfigTemplateBuilder, GlConfig};
use glutin::context::{
    ContextApi, ContextAttributesBuilder, GlProfile, NotCurrentGlContext, PossiblyCurrentContext,
    Version,
};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::{GlSurface, Surface, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasWindowHandle;
use sphatik_render::glow;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

/// A window with a current GL context and a `glow` handle to it.
pub struct GlWindowState {
    pub window: Window,
    pub surface: Surface<WindowSurface>,
    pub context: PossiblyCurrentContext,
}

/// Opens a window and makes an OpenGL ES 3.0 context current on it, falling
/// back to desktop OpenGL 3.3 core (Windows drivers often only offer that).
/// Vsync is on. Returns the window state and the `glow` context.
pub fn create(
    event_loop: &ActiveEventLoop,
    attributes: WindowAttributes,
) -> Result<(GlWindowState, glow::Context), Box<dyn Error>> {
    let template = ConfigTemplateBuilder::new().with_alpha_size(8);
    let (window, config) = DisplayBuilder::new()
        .with_window_attributes(Some(attributes))
        .build(event_loop, template, |configs| {
            // Our shaders antialias; prefer the config with the fewest samples.
            configs
                .reduce(|best, c| {
                    if c.num_samples() < best.num_samples() {
                        c
                    } else {
                        best
                    }
                })
                .expect("the platform offered no GL configs")
        })?;
    let window = window.ok_or("no window was created")?;
    let raw_handle = window.window_handle()?.as_raw();
    let display = config.display();

    let gles = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::Gles(Some(Version::new(3, 0))))
        .build(Some(raw_handle));
    let desktop = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
        .with_profile(GlProfile::Core)
        .build(Some(raw_handle));
    // SAFETY: `raw_handle` belongs to `window`, which outlives the context
    // (both live in the returned state and are dropped together).
    let not_current = unsafe {
        display
            .create_context(&config, &gles)
            .or_else(|_| display.create_context(&config, &desktop))?
    };

    let surface_attributes = window.build_surface_attributes(Default::default())?;
    // SAFETY: the surface attributes carry `window`'s handle, and the window
    // is kept alive alongside the surface.
    let surface = unsafe { display.create_window_surface(&config, &surface_attributes)? };
    let context = not_current.make_current(&surface)?;
    if let Err(e) = surface.set_swap_interval(&context, SwapInterval::Wait(NonZeroU32::MIN)) {
        eprintln!("sphatik-preview: could not enable vsync: {e}");
    }

    // SAFETY: the context was just made current on this thread, and the
    // loader returns that context's function pointers.
    let gl =
        unsafe { glow::Context::from_loader_function_cstr(|name| display.get_proc_address(name)) };

    Ok((
        GlWindowState {
            window,
            surface,
            context,
        },
        gl,
    ))
}

impl GlWindowState {
    /// Resizes the surface to `width` x `height` pixels (ignored if zero).
    pub fn resize(&self, width: u32, height: u32) {
        if let (Some(w), Some(h)) = (NonZeroU32::new(width), NonZeroU32::new(height)) {
            self.surface.resize(&self.context, w, h);
        }
    }

    /// Shows the frame that was just drawn.
    pub fn swap(&self) -> Result<(), glutin::error::Error> {
        self.surface.swap_buffers(&self.context)
    }
}
