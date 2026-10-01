//! Wallpaper engine checks against the prototype (WP 2.3).
//!
//! The references are the prototype's own wallpaper SVGs
//! (`tests/wallpapers/*.svg`, exported by `extract.mjs`) rendered with
//! `resvg`. The GPU renders run headless like `golden.rs`.

use std::path::{Path, PathBuf};

fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// Renders a prototype wallpaper SVG to straight RGBA8 with resvg.
fn reference(key: &str, width: u32, height: u32) -> Vec<u8> {
    let svg = std::fs::read(repo_path(&format!("tests/wallpapers/{key}.svg"))).expect("svg");
    let tree = resvg::usvg::Tree::from_data(&svg, &resvg::usvg::Options::default()).expect("parse");
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).expect("pixmap");
    let scale = width as f32 / 393.0;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.data().to_vec()
}

#[test]
fn prototype_references_render() {
    for key in ["aurora", "obsidian", "dawn", "night"] {
        let px = reference(key, 393, 852);
        assert_eq!(px.len(), 393 * 852 * 4);
        let out = repo_path(&format!("../target/golden-out/ref-{key}.png"));
        std::fs::create_dir_all(out.parent().expect("dir")).expect("mkdir");
        let file = std::io::BufWriter::new(std::fs::File::create(&out).expect("create"));
        let mut enc = png::Encoder::new(file, 393, 852);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()
            .and_then(|mut w| w.write_image_data(&px))
            .expect("png");
    }
}
