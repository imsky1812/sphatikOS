//! `sphatik-render`: the Sphatik OS GPU renderer.
//!
//! Draws the shell (wallpapers, shapes, gradients and, from WP 2.4, glass)
//! with OpenGL ES 3 through `glow`. It has no Smithay or platform types in
//! its API (ADR 0008): the compositor hands it the phone's EGL context and
//! `sphatik-preview` hands it a desktop window's context on Windows.
//!
//! The scene is drawn into a persistent off-screen target and only the
//! damaged region is redrawn; a frame with no damage is skipped entirely.
//! All coordinates are logical points; the renderer scales them to pixels.

mod damage;
pub mod demo;
mod geom;
mod glass;
mod overlay;
mod paint;
mod renderer;
mod shaders;
pub mod vector;
pub mod wallpaper;
mod wallpaper_gl;

pub use damage::Damage;
pub use geom::Rect;
pub use glass::{GlassPanel, GlassRenderer};
pub use glow;
pub use overlay::{Overlay, PerfGraph};
pub use paint::{Color, Paint, Shape, Stops, MAX_STOPS};
pub use renderer::{FrameStats, RenderError, Renderer};
pub use wallpaper_gl::WallpaperRenderer;
