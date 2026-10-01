//! Shared helpers for the golden-image tests: a headless GLES context and
//! image comparison (ADR 0008).

#![allow(dead_code)]

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use khronos_egl as egl;
use sphatik_render::glow;

/// `EGL_PLATFORM_SURFACELESS_MESA`: an EGL display with no window system.
const PLATFORM_SURFACELESS_MESA: egl::Enum = 0x31DD;

/// A current GLES 3 context on a surfaceless display (Mesa llvmpipe in CI).
pub struct Headless {
    egl: egl::DynamicInstance<egl::EGL1_5>,
    display: egl::Display,
    context: egl::Context,
}

impl Headless {
    /// Sets up the context, or returns why it could not.
    pub fn new() -> Result<(Self, glow::Context), String> {
        // SAFETY: loads the system's libEGL; nothing else can be checked.
        let egl = unsafe { egl::DynamicInstance::<egl::EGL1_5>::load_required() }
            .map_err(|e| format!("no EGL 1.5 library: {e}"))?;
        // SAFETY: surfaceless displays take no native display; the attribute
        // list is terminated.
        let display = unsafe {
            egl.get_platform_display(
                PLATFORM_SURFACELESS_MESA,
                egl::DEFAULT_DISPLAY,
                &[egl::ATTRIB_NONE],
            )
        }
        .map_err(|e| format!("no surfaceless display: {e}"))?;
        egl.initialize(display)
            .map_err(|e| format!("EGL initialise failed: {e}"))?;
        let config = egl
            .choose_first_config(
                display,
                &[
                    // The default is window-capable configs, which a
                    // surfaceless display has none of.
                    egl::SURFACE_TYPE,
                    egl::PBUFFER_BIT,
                    egl::RENDERABLE_TYPE,
                    egl::OPENGL_ES3_BIT,
                    egl::RED_SIZE,
                    8,
                    egl::GREEN_SIZE,
                    8,
                    egl::BLUE_SIZE,
                    8,
                    egl::ALPHA_SIZE,
                    8,
                    egl::NONE,
                ],
            )
            .map_err(|e| format!("choose config failed: {e}"))?
            .ok_or("no GLES 3 config")?;
        egl.bind_api(egl::OPENGL_ES_API)
            .map_err(|e| format!("bind GLES failed: {e}"))?;
        let context = egl
            .create_context(
                display,
                config,
                None,
                &[egl::CONTEXT_MAJOR_VERSION, 3, egl::NONE],
            )
            .map_err(|e| format!("create context failed: {e}"))?;
        egl.make_current(display, None, None, Some(context))
            .map_err(|e| format!("make current failed: {e}"))?;
        // SAFETY: the context is current on this thread; the loader returns
        // its function pointers.
        let gl = unsafe {
            glow::Context::from_loader_function(|name| {
                egl.get_proc_address(name)
                    .map_or(std::ptr::null(), |f| f as *const _)
            })
        };
        Ok((
            Self {
                egl,
                display,
                context,
            },
            gl,
        ))
    }
}

impl Drop for Headless {
    fn drop(&mut self) {
        let _ = self.egl.make_current(self.display, None, None, None);
        let _ = self.egl.destroy_context(self.display, self.context);
        let _ = self.egl.terminate(self.display);
    }
}

/// True if an environment flag is set to `1`.
pub fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v == "1")
}

/// A path relative to the `sphatik-render` crate root.
pub fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// Writes RGBA8 to a PNG, creating parent directories.
pub fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).expect("create output directory");
    }
    let file = BufWriter::new(File::create(path).expect("create PNG"));
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut w| w.write_image_data(rgba))
        .expect("write PNG");
}

/// Reads an 8-bit RGBA PNG.
pub fn read_png(path: &Path) -> Option<(u32, u32, Vec<u8>)> {
    let decoder = png::Decoder::new(std::io::BufReader::new(File::open(path).ok()?));
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    (info.color_type == png::ColorType::Rgba && info.bit_depth == png::BitDepth::Eight)
        .then(|| (info.width, info.height, buf[..info.buffer_size()].to_vec()))
}

/// Compares two equal-size RGBA8 buffers: a pixel counts as different if any
/// of R, G, B differ by more than `tolerance`. Returns the share (0.0–1.0).
pub fn difference_share(actual: &[u8], expected: &[u8], tolerance: u8) -> f64 {
    let (a, _) = actual.as_chunks::<4>();
    let (e, _) = expected.as_chunks::<4>();
    let different = a
        .iter()
        .zip(e)
        .filter(|(p, q)| {
            p[..3]
                .iter()
                .zip(&q[..3])
                .any(|(x, y)| x.abs_diff(*y) > tolerance)
        })
        .count();
    different as f64 / a.len().max(1) as f64
}
