//! Golden-image test for the Glass-on-laptop gate: the lock-screen mock-up —
//! Liquid Aurora wallpaper, one Regular-glass widget over the ribbons, and the
//! clock, date and unlock hint composited on top.
//!
//! This exercises the whole laptop render path (wallpaper, glass, text) in one
//! frame, headless. The text is CPU-rasterised so it is identical on every GPU;
//! the wallpaper and glass agree within a couple of levels across GPUs, as in
//! the other golden tests. Layout mirrors `sphatik-preview`'s lock screen, the
//! source of truth. Skips without EGL unless `SPHATIK_REQUIRE_GPU_TESTS=1`;
//! `SPHATIK_UPDATE_GOLDEN=1` rewrites the reference.

mod common;

use common::*;
use sphatik_render::glow::HasContext;
use sphatik_render::text::{rasterize, OUTFIT_EXTRALIGHT, OUTFIT_LIGHT};
use sphatik_render::wallpaper::Wall;
use sphatik_render::{Color, GlassPanel, GlassRenderer, Rect, TextRenderer, WallpaperRenderer};

const W: u32 = 393;
const H: u32 = 852;
const TOLERANCE: u8 = 10;
const MAX_SHARE: f64 = 0.01;

#[test]
fn lock_screen_mockup() {
    let (headless, gl) = match Headless::new() {
        Ok(h) => h,
        Err(why) if !env_flag("SPHATIK_REQUIRE_GPU_TESTS") => {
            eprintln!("skipping lock-screen golden test: {why}");
            return;
        }
        Err(why) => panic!("lock-screen golden test needs EGL: {why}"),
    };
    let mut wallpaper = WallpaperRenderer::new(headless.glow()).expect("wallpaper");
    let mut glass = GlassRenderer::new(gl).expect("glass");
    let mut clock = TextRenderer::new(headless.glow()).expect("clock");
    let mut date = TextRenderer::new(headless.glow()).expect("date");
    let mut hint = TextRenderer::new(headless.glow()).expect("hint");

    wallpaper
        .render(&Wall::Aurora.build(), W as f32, H as f32, W, H)
        .expect("wallpaper render");
    let panel = GlassPanel::regular(Rect::new(18.0, 556.0, 357.0, 118.0), 30.0);
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

    // Compose the text onto the glass scene, as the preview does.
    clock.set_bitmap(&rasterize(OUTFIT_EXTRALIGHT, "9:41", 118.0, -3.5, true));
    date.set_bitmap(&rasterize(
        OUTFIT_LIGHT,
        "Tuesday, 6 October",
        17.0,
        0.2,
        false,
    ));
    hint.set_bitmap(&rasterize(
        OUTFIT_LIGHT,
        "Swipe up to open",
        13.0,
        0.4,
        false,
    ));
    let centre = |tw: u32| (W as f32 - tw as f32) * 0.5;
    let gl = headless.glow();
    let fbo = glass.scene_framebuffer().expect("scene framebuffer");
    // SAFETY: the context is current; `fbo` is the live scene framebuffer.
    unsafe {
        gl.bind_framebuffer(sphatik_render::glow::FRAMEBUFFER, Some(fbo));
        gl.viewport(0, 0, W as i32, H as i32);
    }
    date.draw(
        W,
        H,
        centre(date.width()),
        70.0,
        Color::rgba(1.0, 1.0, 1.0, 0.88),
    );
    clock.draw(
        W,
        H,
        centre(clock.width()),
        104.0,
        Color::rgba(1.0, 1.0, 1.0, 0.93),
    );
    hint.draw(
        W,
        H,
        centre(hint.width()),
        H as f32 - hint.height() as f32 - 20.0,
        Color::rgba(1.0, 1.0, 1.0, 0.7),
    );
    // SAFETY: as above.
    unsafe { gl.bind_framebuffer(sphatik_render::glow::FRAMEBUFFER, None) };

    let actual = glass.read_pixels().expect("pixels");
    clock.destroy();
    date.destroy();
    hint.destroy();
    glass.destroy();
    wallpaper.destroy();
    drop(headless);

    let reference = repo_path("tests/golden/lock-screen.png");
    if env_flag("SPHATIK_UPDATE_GOLDEN") {
        write_png(&reference, W, H, &actual);
        return;
    }
    let out = repo_path("../target/golden-out/lock-screen.png");
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
