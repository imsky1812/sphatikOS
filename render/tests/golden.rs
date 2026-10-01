//! Golden-image test for the WP 2.2 demo scene: render it headless and
//! compare with a checked-in PNG (ADR 0008).
//!
//! Runs on Mesa's software GLES driver (llvmpipe) via a surfaceless EGL
//! display (CI). Skips where EGL is missing (Windows) unless
//! `SPHATIK_REQUIRE_GPU_TESTS=1`. `SPHATIK_UPDATE_GOLDEN=1` rewrites the
//! reference. A mismatch writes the rendered image to `target/golden-out/`.

mod common;

use common::*;
use sphatik_render::{demo, Rect, Renderer};

/// Largest per-channel difference that still counts as equal.
const TOLERANCE: u8 = 2;
/// Share of pixels allowed to exceed the tolerance.
const MAX_SHARE: f64 = 0.001;

#[test]
fn aurora_backdrop_with_panel() {
    let (w, h) = (393u32, 852u32);
    let (headless, gl) = match Headless::new() {
        Ok(h) => h,
        Err(why) if !env_flag("SPHATIK_REQUIRE_GPU_TESTS") => {
            eprintln!("skipping golden test: {why}");
            return;
        }
        Err(why) => panic!("golden test needs EGL: {why}"),
    };
    let mut renderer = Renderer::new(gl).expect("renderer");
    renderer.resize(w as f32, h as f32, w, h).expect("target");
    let canvas = Rect::new(0.0, 0.0, w as f32, h as f32);
    let stats = renderer.render(
        &demo::backdrop_with_panel(canvas),
        Some(canvas),
        demo::CLEAR,
    );
    assert!(stats.drawn);
    let actual = renderer.read_pixels().expect("pixels");
    renderer.destroy();
    drop(headless);

    let reference = repo_path("tests/golden/aurora-backdrop-panel.png");
    if env_flag("SPHATIK_UPDATE_GOLDEN") {
        write_png(&reference, w, h, &actual);
        return;
    }
    let out = repo_path("../target/golden-out/aurora-backdrop-panel.png");
    let Some((rw, rh, expected)) = read_png(&reference) else {
        write_png(&out, w, h, &actual);
        panic!(
            "no reference {}; wrote {}",
            reference.display(),
            out.display()
        );
    };
    assert_eq!((rw, rh), (w, h));
    let share = difference_share(&actual, &expected, TOLERANCE);
    if share > MAX_SHARE {
        write_png(&out, w, h, &actual);
        panic!(
            "differs in {:.3}% of pixels; wrote {}",
            share * 100.0,
            out.display()
        );
    }
}
