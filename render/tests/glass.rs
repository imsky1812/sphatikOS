//! Golden-image test for glass v0 (WP 2.4): the Liquid Aurora wallpaper with
//! a Regular-glass panel over the ribbons, so the backdrop blur is exercised.
//!
//! The prototype's blur is a CSS `backdrop-filter`, which `resvg` cannot
//! render, so the reference is a committed PNG (like the shape scene). It is
//! produced by Mesa llvmpipe in CI and cross-checked against the owner's GPU;
//! both agree within a couple of levels. Skips where EGL is missing unless
//! `SPHATIK_REQUIRE_GPU_TESTS=1`; `SPHATIK_UPDATE_GOLDEN=1` rewrites it.

mod common;

use common::*;
use sphatik_render::wallpaper::Wall;
use sphatik_render::{GlassPanel, GlassRenderer, Rect, WallpaperRenderer};

const W: u32 = 393;
const H: u32 = 852;
const TOLERANCE: u8 = 3;
const MAX_SHARE: f64 = 0.002;

#[test]
fn aurora_with_glass_panel() {
    let (headless, gl) = match Headless::new() {
        Ok(h) => h,
        Err(why) if !env_flag("SPHATIK_REQUIRE_GPU_TESTS") => {
            eprintln!("skipping glass golden test: {why}");
            return;
        }
        Err(why) => panic!("glass golden test needs EGL: {why}"),
    };
    let mut wallpaper = WallpaperRenderer::new(headless.glow()).expect("wallpaper");
    let mut glass = GlassRenderer::new(gl).expect("glass");
    wallpaper
        .render(&Wall::Aurora.build(), W as f32, H as f32, W, H)
        .expect("wallpaper render");
    let panel = GlassPanel::regular(Rect::new(18.0, 540.0, 171.5, 162.0), 30.0);
    glass
        .render(
            wallpaper.texture().expect("wallpaper texture"),
            W as f32,
            H as f32,
            W,
            H,
            &[panel],
        )
        .expect("glass render");
    let actual = glass.read_pixels().expect("pixels");
    glass.destroy();
    wallpaper.destroy();
    drop(headless);

    let reference = repo_path("tests/golden/glass-aurora-panel.png");
    if env_flag("SPHATIK_UPDATE_GOLDEN") {
        write_png(&reference, W, H, &actual);
        return;
    }
    let out = repo_path("../target/golden-out/glass-aurora-panel.png");
    let Some((rw, rh, expected)) = read_png(&reference) else {
        write_png(&out, W, H, &actual);
        panic!(
            "no reference {}; wrote {}",
            reference.display(),
            out.display()
        );
    };
    assert_eq!((rw, rh), (W, H));
    let share = difference_share(&actual, &expected, TOLERANCE);
    if share > MAX_SHARE {
        write_png(&out, W, H, &actual);
        panic!(
            "differs in {:.3}% of pixels; wrote {}",
            share * 100.0,
            out.display()
        );
    }
}
