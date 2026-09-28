//! Golden-image tests: render reference scenes headless and compare them
//! with checked-in PNGs (ADR 0008).
//!
//! They run on Mesa's software GLES driver (llvmpipe) through a surfaceless
//! EGL display, which CI provides. Where EGL isn't available (Windows) they
//! skip, unless `SPHATIK_REQUIRE_GPU_TESTS=1` demands them.
//!
//! - A mismatch or missing reference writes the rendered image to
//!   `target/golden-out/`, which CI uploads.
//! - `SPHATIK_UPDATE_GOLDEN=1` writes the rendered image as the new reference.

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use khronos_egl as egl;
use sphatik_render::glow;
use sphatik_render::{demo, Rect, Renderer};

/// `EGL_PLATFORM_SURFACELESS_MESA`: an EGL display with no window system.
const PLATFORM_SURFACELESS_MESA: egl::Enum = 0x31DD;

/// Largest per-channel difference that still counts as equal.
const CHANNEL_TOLERANCE: u8 = 2;
/// Share of pixels allowed to exceed [`CHANNEL_TOLERANCE`].
const MAX_DIFFERENT_SHARE: f64 = 0.001;

struct Headless {
    egl: egl::DynamicInstance<egl::EGL1_5>,
    display: egl::Display,
    context: egl::Context,
}

impl Headless {
    /// A current GLES 3 context on a surfaceless display, or why not.
    fn new() -> Result<(Self, glow::Context), String> {
        // SAFETY: loads the system's libEGL; nothing else can be checked.
        let egl = unsafe { egl::DynamicInstance::<egl::EGL1_5>::load_required() }
            .map_err(|e| format!("no EGL 1.5 library: {e}"))?;
        // SAFETY: surfaceless displays take no native display; the
        // attribute list is terminated.
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
        // SAFETY: the context is current on this thread and the loader
        // returns its function pointers.
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

fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v == "1")
}

fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) {
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

fn read_png(path: &Path) -> Option<(u32, u32, Vec<u8>)> {
    let decoder = png::Decoder::new(std::io::BufReader::new(File::open(path).ok()?));
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    (info.color_type == png::ColorType::Rgba && info.bit_depth == png::BitDepth::Eight)
        .then(|| (info.width, info.height, buf[..info.buffer_size()].to_vec()))
}

/// Renders `shapes` at 1 px per point on a `w` x `h` canvas and compares the
/// result with `render/tests/golden/<name>.png`.
fn check_golden(name: &str, w: u32, h: u32, shapes: &[sphatik_render::Shape]) {
    let (headless, gl) = match Headless::new() {
        Ok(h) => h,
        Err(why) if !env_flag("SPHATIK_REQUIRE_GPU_TESTS") => {
            eprintln!("skipping golden test `{name}`: {why}");
            return;
        }
        Err(why) => panic!("golden test `{name}` needs EGL: {why}"),
    };
    let mut renderer = Renderer::new(gl).expect("renderer");
    renderer
        .resize(w as f32, h as f32, w, h)
        .expect("render target");
    let stats = renderer.render(
        shapes,
        Some(Rect::new(0.0, 0.0, w as f32, h as f32)),
        demo::CLEAR,
    );
    assert!(stats.drawn);
    let actual = renderer.read_pixels().expect("pixels");
    renderer.destroy();
    drop(headless);

    let reference = repo_path(&format!("tests/golden/{name}.png"));
    if env_flag("SPHATIK_UPDATE_GOLDEN") {
        write_png(&reference, w, h, &actual);
        eprintln!("updated {}", reference.display());
        return;
    }
    let out = repo_path(&format!("../target/golden-out/{name}.png"));
    let Some((rw, rh, expected)) = read_png(&reference) else {
        write_png(&out, w, h, &actual);
        panic!(
            "no reference {}; rendered image written to {}",
            reference.display(),
            out.display()
        );
    };
    assert_eq!((rw, rh), (w, h), "reference size differs");
    let (actual_px, _) = actual.as_chunks::<4>();
    let (expected_px, _) = expected.as_chunks::<4>();
    let different = actual_px
        .iter()
        .zip(expected_px)
        .filter(|(a, e)| {
            a.iter()
                .zip(e.iter())
                .any(|(x, y)| x.abs_diff(*y) > CHANNEL_TOLERANCE)
        })
        .count();
    let share = different as f64 / f64::from(w * h);
    if share > MAX_DIFFERENT_SHARE {
        write_png(&out, w, h, &actual);
        panic!(
            "`{name}` differs from its reference in {different} pixels ({:.3}%); rendered image written to {}",
            share * 100.0,
            out.display()
        );
    }
}

#[test]
fn aurora_backdrop_with_panel() {
    let canvas = Rect::new(0.0, 0.0, 393.0, 852.0);
    check_golden(
        "aurora-backdrop-panel",
        393,
        852,
        &demo::backdrop_with_panel(canvas),
    );
}
