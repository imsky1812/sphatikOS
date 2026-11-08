//! Golden-image test for `Renderer::draw_onto`: compositing shell shapes
//! (tinted rounded tiles, a dot) onto the glass scene, the path the home-screen
//! mock-up uses. Headless; the reference is a committed llvmpipe PNG (the test
//! skips without EGL unless `SPHATIK_REQUIRE_GPU_TESTS=1`;
//! `SPHATIK_UPDATE_GOLDEN=1` rewrites it).

mod common;

use common::*;
use sphatik_render::glow::HasContext;
use sphatik_render::wallpaper::Wall;
use sphatik_render::{
    Color, GlassRenderer, Paint, Rect, Renderer, Shape, Stops, WallpaperRenderer,
};

const W: u32 = 393;
const H: u32 = 852;
const TOLERANCE: u8 = 8;
const MAX_SHARE: f64 = 0.01;

#[test]
fn shapes_composite_onto_the_glass_scene() {
    let (headless, gl) = match Headless::new() {
        Ok(h) => h,
        Err(why) if !env_flag("SPHATIK_REQUIRE_GPU_TESTS") => {
            eprintln!("skipping draw_onto golden test: {why}");
            return;
        }
        Err(why) => panic!("draw_onto golden test needs EGL: {why}"),
    };
    let mut wallpaper = WallpaperRenderer::new(headless.glow()).expect("wallpaper");
    let mut glass = GlassRenderer::new(gl).expect("glass");
    let shapes = Renderer::new(headless.glow()).expect("shapes");

    wallpaper
        .render(&Wall::Aurora.build(), W as f32, H as f32, W, H)
        .expect("wallpaper render");
    glass
        .render(
            wallpaper.texture().expect("wallpaper texture"),
            W as f32,
            H as f32,
            W,
            H,
            &[],
        )
        .expect("glass render");

    let tile = Rect::new(80.0, 300.0, 120.0, 120.0);
    let batch = [
        Shape::new(
            tile,
            24.0,
            Paint::css_linear(
                tile,
                160.0,
                Stops::svg(&[(0.0, Color::hex(0x8AF0F0)), (1.0, Color::hex(0x6546F2))]),
            ),
        ),
        Shape::new(
            Rect::new(240.0, 340.0, 60.0, 60.0),
            30.0,
            Paint::Solid(Color::hex(0xFF8FB7)),
        ),
    ];

    let fbo = glass.scene_framebuffer().expect("scene framebuffer");
    let gl = headless.glow();
    // SAFETY: the context is current; `fbo` is the live scene framebuffer.
    unsafe {
        gl.bind_framebuffer(sphatik_render::glow::FRAMEBUFFER, Some(fbo));
        gl.viewport(0, 0, W as i32, H as i32);
    }
    shapes.draw_onto(&batch, W as f32, H as f32, W);
    // SAFETY: as above.
    unsafe { gl.bind_framebuffer(sphatik_render::glow::FRAMEBUFFER, None) };

    let actual = glass.read_pixels().expect("pixels");
    shapes.destroy();
    glass.destroy();
    wallpaper.destroy();
    drop(headless);

    let reference = repo_path("tests/golden/shapes-onto.png");
    if env_flag("SPHATIK_UPDATE_GOLDEN") {
        write_png(&reference, W, H, &actual);
        return;
    }
    let out = repo_path("../target/golden-out/shapes-onto.png");
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
