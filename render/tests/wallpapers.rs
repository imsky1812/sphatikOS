//! Golden-image test for the four wallpapers (WP 2.3): render each with the
//! GPU and compare against the prototype's own SVG, rendered with resvg.
//!
//! The reference is produced live from `tests/wallpapers/<key>.svg` (exported
//! from the prototype by `extract.mjs`, grain-free), so there is no committed
//! PNG to drift. Runs on Mesa llvmpipe via surfaceless EGL in CI; skips on
//! Windows unless `SPHATIK_REQUIRE_GPU_TESTS=1`. A mismatch writes both images
//! to `target/golden-out/`.

mod common;

use common::*;
use sphatik_render::wallpaper::Wall;
use sphatik_render::WallpaperRenderer;

const W: u32 = 393;
const H: u32 = 852;

/// A pixel differs if any channel is off by more than this. Wallpapers have
/// large blurs and fine strokes, so the tolerance is looser than the shape
/// scene's; blends and rasterisation differ slightly between resvg and the GPU.
const TOLERANCE: u8 = 16;
/// Share of pixels allowed to differ (worst observed is ~0.3%).
const MAX_SHARE: f64 = 0.008;

/// Renders a prototype wallpaper SVG to straight RGBA8 with resvg.
fn reference(key: &str) -> Vec<u8> {
    let svg = std::fs::read(repo_path(&format!("tests/wallpapers/{key}.svg"))).expect("svg");
    let tree = resvg::usvg::Tree::from_data(&svg, &resvg::usvg::Options::default()).expect("parse");
    let mut pixmap = resvg::tiny_skia::Pixmap::new(W, H).expect("pixmap");
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    pixmap.data().to_vec()
}

#[test]
fn wallpapers_match_the_prototype() {
    let (headless, gl) = match Headless::new() {
        Ok(h) => h,
        Err(why) if !env_flag("SPHATIK_REQUIRE_GPU_TESTS") => {
            eprintln!("skipping wallpaper golden test: {why}");
            return;
        }
        Err(why) => panic!("wallpaper golden test needs EGL: {why}"),
    };
    let mut renderer = WallpaperRenderer::new(gl).expect("wallpaper renderer");
    let mut failures = Vec::new();
    for wall in Wall::ALL {
        let key = wall.key();
        renderer
            .render(&wall.build(), W as f32, H as f32, W, H)
            .expect("render wallpaper");
        let actual = renderer.read_pixels().expect("pixels");
        let expected = reference(key);
        let share = difference_share(&actual, &expected, TOLERANCE);
        if share > MAX_SHARE {
            write_png(
                &repo_path(&format!("../target/golden-out/wp-{key}.png")),
                W,
                H,
                &actual,
            );
            write_png(
                &repo_path(&format!("../target/golden-out/ref-{key}.png")),
                W,
                H,
                &expected,
            );
            failures.push(format!("{key}: {:.3}% of pixels differ", share * 100.0));
        }
    }
    renderer.destroy();
    drop(headless);
    assert!(
        failures.is_empty(),
        "wallpapers differ from the prototype:\n{}",
        failures.join("\n")
    );
}
