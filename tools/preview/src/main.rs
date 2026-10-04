//! `sphatik-preview`: runs the Sphatik renderer in a desktop window, so the
//! UI can be seen and felt on Windows (ADR 0008).
//!
//! ```text
//! cargo run -p sphatik-preview                                 # open the window
//! cargo run -p sphatik-preview -- --wallpaper obsidian         # pick a wallpaper
//! cargo run -p sphatik-preview -- --screenshot out.png         # save one frame
//! ```
//!
//! Keys 1–4 switch between Liquid Aurora, Obsidian, Quartz Dawn and Amethyst.
//! The mouse acts as a finger: each drag is classified and logged. Frames are
//! drawn only when something changes; the title bar counts drawn frames.

mod gl;

use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use clap::Parser;
use sphatik_render::wallpaper::Wall;
use sphatik_render::{GlassPanel, GlassRenderer, Overlay, PerfGraph, Rect, WallpaperRenderer};
use sphatik_shell::gesture::{self, Canvas, Hit, Point, ShellContext, TouchTracker};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

/// Command-line options.
#[derive(Debug, Parser)]
#[command(name = "sphatik-preview", version, about)]
struct Options {
    /// Which wallpaper to show: aurora, obsidian, dawn or night.
    #[arg(long, default_value = "aurora")]
    wallpaper: String,
    /// Start with the F12 debug overlay visible.
    #[arg(long)]
    overlay: bool,
    /// Render one frame to this PNG file and exit.
    #[arg(long, value_name = "PNG")]
    screenshot: Option<PathBuf>,
    /// Pixels per point for --screenshot (2.75 matches the phone).
    #[arg(long, default_value_t = 1.0)]
    scale: f32,
}

/// The canvas the preview shows: the prototype's 393 x 852 pt screen.
const CANVAS: Canvas = Canvas::PROTOTYPE;

/// A Regular-glass panel at the first home-widget position (reference §9).
fn demo_panel() -> GlassPanel {
    GlassPanel::regular(Rect::new(18.0, 66.0, 171.5, 162.0), 30.0)
}

fn wall_from_name(name: &str) -> Option<Wall> {
    Wall::ALL.into_iter().find(|w| w.key() == name)
}

struct App {
    options: Options,
    state: Option<(gl::GlWindowState, WallpaperRenderer, GlassRenderer, Overlay)>,
    util_gl: Option<sphatik_render::glow::Context>,
    wall: Wall,
    glass_on: bool,
    overlay_on: bool,
    graph: PerfGraph,
    glass_ms: Option<f32>,
    last_present: Option<Instant>,
    dirty: bool,
    frames_drawn: u64,
    started: Instant,
    touch: Option<TouchTracker>,
    cursor: Point,
    result: Result<(), String>,
}

impl App {
    fn new(options: Options) -> Self {
        let wall = wall_from_name(&options.wallpaper).unwrap_or(Wall::Aurora);
        let options_overlay = options.overlay;
        Self {
            options,
            state: None,
            util_gl: None,
            wall,
            glass_on: true,
            overlay_on: options_overlay,
            graph: PerfGraph::new(1000.0 / 60.0),
            glass_ms: None,
            last_present: None,
            dirty: true,
            frames_drawn: 0,
            started: Instant::now(),
            touch: None,
            cursor: Point::default(),
            result: Ok(()),
        }
    }

    fn now_ms(&self) -> f64 {
        self.started.elapsed().as_secs_f64() * 1000.0
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, message: String) {
        self.result = Err(message);
        event_loop.exit();
    }

    fn start(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let screenshot = self.options.screenshot.is_some();
        let zoom = if screenshot {
            1.0
        } else {
            fit_zoom(event_loop)
        };
        let attributes = Window::default_attributes()
            .with_title("Sphatik preview")
            .with_inner_size(LogicalSize::new(
                f64::from(CANVAS.width * zoom),
                f64::from(CANVAS.height * zoom),
            ))
            .with_resizable(true)
            .with_visible(!screenshot);
        let (window_state, glow_ctx) = gl::create(event_loop, attributes)?;
        let glass = GlassRenderer::new(window_state.make_glow())?;
        let overlay = Overlay::new(window_state.make_glow())?;
        self.util_gl = Some(window_state.make_glow());
        let wallpaper = WallpaperRenderer::new(glow_ctx)?;
        self.state = Some((window_state, wallpaper, glass, overlay));
        self.render_frame()?;
        Ok(())
    }

    /// (Re)renders the wallpaper, and the glass panel over it, at the window
    /// size (or the screenshot size).
    fn render_frame(&mut self) -> Result<(), Box<dyn Error>> {
        let glass_on = self.glass_on;
        let wall = self.wall;
        let (w, h) = self.target_size();
        let util = self.util_gl.as_ref();
        let mut glass_ms = None;
        let Some((_, wallpaper, glass, _)) = self.state.as_mut() else {
            return Ok(());
        };
        wallpaper.render(&wall.build(), CANVAS.width, CANVAS.height, w, h)?;
        if glass_on {
            let texture = wallpaper.texture().ok_or("no wallpaper texture")?;
            let panels = [demo_panel()];
            // Time the glass pass with a blocking finish (laptop-indicative;
            // the real budget is measured on the phone).
            if let Some(gl) = util {
                finish(gl);
                let start = Instant::now();
                glass.render(texture, CANVAS.width, CANVAS.height, w, h, &panels)?;
                finish(gl);
                glass_ms = Some(start.elapsed().as_secs_f64() as f32 * 1000.0);
            } else {
                glass.render(texture, CANVAS.width, CANVAS.height, w, h, &panels)?;
            }
        }
        self.glass_ms = glass_ms;
        self.dirty = true;
        Ok(())
    }

    fn target_size(&self) -> (u32, u32) {
        if self.options.screenshot.is_some() {
            let s = self.options.scale.clamp(0.25, 8.0);
            (
                (CANVAS.width * s).round() as u32,
                (CANVAS.height * s).round() as u32,
            )
        } else if let Some((window_state, _, _, _)) = self.state.as_ref() {
            let size = window_state.window.inner_size();
            (size.width.max(1), size.height.max(1))
        } else {
            (CANVAS.width as u32, CANVAS.height as u32)
        }
    }

    fn redraw(&mut self) -> Result<(), Box<dyn Error>> {
        let overlay_on = self.overlay_on;
        if !self.dirty && !overlay_on {
            return Ok(());
        }
        // Frame interval (wall clock), for the FPS graph.
        let now = Instant::now();
        if let Some(prev) = self.last_present {
            self.graph
                .push(now.duration_since(prev).as_secs_f64() as f32 * 1000.0);
        }
        self.last_present = Some(now);

        let glass_on = self.glass_on;
        let graph = self.graph;
        let glass_ms = self.glass_ms;
        let Some((window_state, wallpaper, glass, overlay)) = self.state.as_mut() else {
            return Ok(());
        };
        let size = window_state.window.inner_size();
        if glass_on {
            glass.present(size.width, size.height);
        } else {
            wallpaper.present(size.width, size.height);
        }
        if overlay_on {
            overlay.render(size.width, size.height, 2.0, &graph, glass_ms);
        }
        window_state.swap()?;
        self.dirty = false;
        self.frames_drawn += 1;
        window_state.window.set_title(&format!(
            "Sphatik preview · {} · glass {} · F12 overlay {}",
            self.wall.key(),
            if glass_on { "on" } else { "off" },
            if overlay_on { "on" } else { "off" },
        ));
        // Keep redrawing while the overlay is up, for a live graph.
        if overlay_on {
            window_state.window.request_redraw();
        }
        Ok(())
    }

    fn save_screenshot(&mut self) -> Result<PathBuf, Box<dyn Error>> {
        let path = self
            .options
            .screenshot
            .clone()
            .ok_or("no screenshot path")?;
        let glass_on = self.glass_on;
        let overlay_on = self.overlay_on;
        // A demo graph so the overlay shows a populated frame-time plot.
        let mut graph = PerfGraph::new(1000.0 / 60.0);
        for i in 0..90 {
            let base = 15.0 + ((i as f32 * 0.5).sin() * 1.5);
            graph.push(if i % 37 == 0 { 24.0 } else { base });
        }
        let glass_ms = self.glass_ms;
        let (_, wallpaper, glass, overlay) = self.state.as_mut().ok_or("no renderer")?;
        if !glass_on {
            let (w, h) = wallpaper.size().ok_or("no wallpaper rendered")?;
            let pixels = wallpaper.read_pixels().ok_or("could not read pixels")?;
            write_png(&path, w, h, &pixels)?;
            return Ok(path);
        }
        let (w, h) = glass.size().ok_or("no glass rendered")?;
        if overlay_on {
            // Draw the overlay onto the glass scene so the screenshot shows it.
            if let (Some(fbo), Some(gl)) = (glass.scene_framebuffer(), self.util_gl.as_ref()) {
                use sphatik_render::glow::HasContext;
                // SAFETY: the context is current; `fbo` is the live scene FBO.
                unsafe { gl.bind_framebuffer(sphatik_render::glow::FRAMEBUFFER, Some(fbo)) };
                overlay.render(w, h, 2.0, &graph, glass_ms);
                // SAFETY: as above.
                unsafe { gl.bind_framebuffer(sphatik_render::glow::FRAMEBUFFER, None) };
            }
        }
        let pixels = glass.read_pixels().ok_or("could not read pixels")?;
        write_png(&path, w, h, &pixels)?;
        Ok(path)
    }

    fn set_wallpaper(&mut self, wall: Wall) {
        if self.wall == wall {
            return;
        }
        self.wall = wall;
        if let Err(e) = self.render_frame() {
            eprintln!("sphatik-preview: {e}");
        }
        if let Some((s, _, _, _)) = self.state.as_ref() {
            s.window.request_redraw();
        }
    }

    fn pointer(&mut self, pressed: Option<bool>) {
        let now = self.now_ms();
        match (pressed, self.touch.as_mut()) {
            (Some(true), _) => self.touch = Some(TouchTracker::begin(self.cursor, now)),
            (None, Some(t)) => t.move_to(self.cursor, now),
            (Some(false), Some(t)) => {
                t.move_to(self.cursor, now);
                let kind = if t.past_slop() {
                    gesture::classify(
                        &ShellContext::default(),
                        CANVAS,
                        t.start(),
                        t.delta(),
                        Hit::Other,
                    )
                } else {
                    None
                };
                eprintln!(
                    "touch: from ({:.0}, {:.0}) moved ({:.0}, {:.0}) -> {:?}",
                    t.start().x,
                    t.start().y,
                    t.delta().x,
                    t.delta().y,
                    kind
                );
                self.touch = None;
            }
            _ => {}
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        if let Err(e) = self.start(event_loop) {
            self.fail(event_loop, format!("could not start: {e}"));
            return;
        }
        if self.options.screenshot.is_some() {
            match self.save_screenshot() {
                Ok(path) => println!("wrote {}", path.display()),
                Err(e) => self.result = Err(format!("screenshot failed: {e}")),
            }
            event_loop.exit();
            return;
        }
        if let Some((s, _, _, _)) = self.state.as_ref() {
            s.window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                if let Some((s, _, _, _)) = self.state.as_ref() {
                    s.resize(width, height);
                }
                if let Err(e) = self.render_frame() {
                    self.fail(event_loop, format!("resize failed: {e}"));
                    return;
                }
                if let Some((s, _, _, _)) = self.state.as_ref() {
                    s.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.redraw() {
                    self.fail(event_loop, format!("draw failed: {e}"));
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                let pick = match code {
                    KeyCode::Digit1 => Some(Wall::Aurora),
                    KeyCode::Digit2 => Some(Wall::Obsidian),
                    KeyCode::Digit3 => Some(Wall::Dawn),
                    KeyCode::Digit4 => Some(Wall::Night),
                    KeyCode::KeyG => {
                        self.glass_on = !self.glass_on;
                        if let Err(e) = self.render_frame() {
                            eprintln!("sphatik-preview: {e}");
                        }
                        if let Some((s, _, _, _)) = self.state.as_ref() {
                            s.window.request_redraw();
                        }
                        None
                    }
                    KeyCode::F12 => {
                        self.overlay_on = !self.overlay_on;
                        self.dirty = true;
                        if let Some((s, _, _, _)) = self.state.as_ref() {
                            s.window.request_redraw();
                        }
                        None
                    }
                    KeyCode::Escape => {
                        event_loop.exit();
                        None
                    }
                    _ => None,
                };
                if let Some(wall) = pick {
                    self.set_wallpaper(wall);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some((s, _, _, _)) = self.state.as_ref() {
                    let size = s.window.inner_size();
                    let per_point = size.width.max(1) as f64 / f64::from(CANVAS.width);
                    self.cursor = Point::new(
                        (position.x / per_point) as f32,
                        (position.y / per_point) as f32,
                    );
                }
                if self.touch.is_some() {
                    self.pointer(None);
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => self.pointer(Some(state == ElementState::Pressed)),
            _ => {}
        }
    }
}

/// Blocks until the GPU has finished, so the glass pass can be timed.
fn finish(gl: &sphatik_render::glow::Context) {
    use sphatik_render::glow::HasContext;
    // SAFETY: the context is current on this thread.
    unsafe { gl.finish() };
}

/// Zoom so the whole 852 pt tall canvas fits in 85% of the monitor height.
fn fit_zoom(event_loop: &ActiveEventLoop) -> f32 {
    let Some(monitor) = event_loop.primary_monitor() else {
        return 1.0;
    };
    let logical_height = monitor.size().height as f64 / monitor.scale_factor();
    ((logical_height * 0.85) as f32 / CANVAS.height).clamp(0.3, 1.0)
}

fn write_png(path: &PathBuf, width: u32, height: u32, rgba: &[u8]) -> Result<(), Box<dyn Error>> {
    let file = BufWriter::new(File::create(path)?);
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(rgba)?;
    Ok(())
}

fn main() -> ExitCode {
    let options = Options::parse();
    if wall_from_name(&options.wallpaper).is_none() {
        eprintln!(
            "sphatik-preview: unknown wallpaper `{}` (use aurora, obsidian, dawn or night)",
            options.wallpaper
        );
        return ExitCode::FAILURE;
    }
    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            eprintln!("sphatik-preview: no event loop: {e}");
            return ExitCode::FAILURE;
        }
    };
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new(options);
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("sphatik-preview: {e}");
        return ExitCode::FAILURE;
    }
    if let Some((s, wallpaper, glass, overlay)) = app.state.take() {
        overlay.destroy();
        glass.destroy();
        wallpaper.destroy();
        drop(s);
    }
    match app.result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("sphatik-preview: {e}");
            ExitCode::FAILURE
        }
    }
}
