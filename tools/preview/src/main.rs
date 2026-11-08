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
use sphatik_motion::{Preset, Spring};
use sphatik_render::text::{self, OUTFIT_EXTRALIGHT, OUTFIT_LIGHT};
use sphatik_render::wallpaper::Wall;
use sphatik_render::{
    Color, GlassPanel, GlassRenderer, Overlay, PerfGraph, Rect, TextRenderer, WallpaperRenderer,
};
use sphatik_shell::gesture::{
    self, Canvas, Gesture, Hit, LiftOff, Point, ShellContext, TouchTracker,
};
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
    /// Start the lock screen at this unlock progress (0 locked, 1 unlocked);
    /// mainly for screenshots of the mid-unlock state.
    #[arg(long, default_value_t = 0.0)]
    unlock: f32,
}

/// The canvas the preview shows: the prototype's 393 x 852 pt screen.
const CANVAS: Canvas = Canvas::PROTOTYPE;

/// The lock-screen widget: one Regular-glass card over the aurora, where the
/// blur and refraction read against the colour. `rise` lifts it with the lock
/// layer during unlock.
fn lock_panel(rise: f32) -> GlassPanel {
    GlassPanel::regular(Rect::new(18.0, 556.0 - rise, 357.0, 118.0), 30.0)
}

/// The lock screen's text: a big clock, a date kicker and the unlock hint,
/// drawn over the wallpaper and glass. Rasterised at the window's scale so the
/// numerals stay crisp (interim text; the Stage 4 toolkit owns real layout).
struct LockScreen {
    clock: TextRenderer,
    date: TextRenderer,
    hint: TextRenderer,
    scale: f32,
}

impl LockScreen {
    fn new(gl: &dyn Fn() -> sphatik_render::glow::Context) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            clock: TextRenderer::new(gl())?,
            date: TextRenderer::new(gl())?,
            hint: TextRenderer::new(gl())?,
            scale: 0.0,
        })
    }

    /// Rasterises the text for the current scale (points -> pixels). A fixed
    /// demo time keeps the composition deterministic for the golden test.
    fn layout(&mut self, canvas_px_w: u32) {
        let scale = canvas_px_w as f32 / CANVAS.width;
        if (scale - self.scale).abs() < 0.001 {
            return;
        }
        self.scale = scale;
        self.clock.set_bitmap(&text::rasterize(
            OUTFIT_EXTRALIGHT,
            "9:41",
            118.0 * scale,
            -3.5 * scale,
            true,
        ));
        self.date.set_bitmap(&text::rasterize(
            OUTFIT_LIGHT,
            "Tuesday, 6 October",
            17.0 * scale,
            0.2 * scale,
            false,
        ));
        self.hint.set_bitmap(&text::rasterize(
            OUTFIT_LIGHT,
            "Swipe up to open",
            13.0 * scale,
            0.4 * scale,
            false,
        ));
    }

    /// Draws the text onto the currently bound framebuffer, sized `w` x `h`.
    /// `rise` lifts the lock layer and `opacity` fades it during unlock.
    fn draw(&self, w: u32, h: u32, rise: f32, opacity: f32) {
        if opacity <= 0.0 {
            return;
        }
        let scale = w as f32 / CANVAS.width;
        let dy = rise * scale;
        let centre = |tw: u32| (w as f32 - tw as f32) * 0.5;
        let white = |a: f32| Color::rgba(1.0, 1.0, 1.0, a * opacity);
        self.date.draw(
            w,
            h,
            centre(self.date.width()),
            70.0 * scale - dy,
            white(0.88),
        );
        self.clock.draw(
            w,
            h,
            centre(self.clock.width()),
            104.0 * scale - dy,
            white(0.93),
        );
        self.hint.draw(
            w,
            h,
            centre(self.hint.width()),
            h as f32 - self.hint.height() as f32 - 20.0 * scale - dy,
            white(0.7),
        );
    }

    fn destroy(self) {
        self.clock.destroy();
        self.date.destroy();
        self.hint.destroy();
    }
}

fn wall_from_name(name: &str) -> Option<Wall> {
    Wall::ALL.into_iter().find(|w| w.key() == name)
}

struct App {
    options: Options,
    state: Option<(gl::GlWindowState, WallpaperRenderer, GlassRenderer, Overlay)>,
    lock: Option<LockScreen>,
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
    unlock: Spring,
    unlocking: bool,
    animating: bool,
    last_frame: Option<Instant>,
    wallpaper_valid: bool,
    result: Result<(), String>,
}

impl App {
    fn new(options: Options) -> Self {
        let wall = wall_from_name(&options.wallpaper).unwrap_or(Wall::Aurora);
        let options_overlay = options.overlay;
        Self {
            options,
            state: None,
            lock: None,
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
            unlock: Spring::new(0.0, Preset::Smooth),
            unlocking: false,
            animating: false,
            last_frame: None,
            wallpaper_valid: false,
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
        self.lock = Some(LockScreen::new(&|| window_state.make_glow())?);
        self.util_gl = Some(window_state.make_glow());
        let wallpaper = WallpaperRenderer::new(glow_ctx)?;
        self.state = Some((window_state, wallpaper, glass, overlay));
        self.unlock.jump_to(self.options.unlock.clamp(0.0, 1.0));
        self.render_frame()?;
        Ok(())
    }

    /// (Re)renders the wallpaper, and the glass panel over it, at the window
    /// size (or the screenshot size). The wallpaper is re-rendered unless
    /// reused (during the unlock animation, to hold 60 fps over a cached
    /// wallpaper).
    fn render_frame(&mut self) -> Result<(), Box<dyn Error>> {
        self.render_frame_inner(false)
    }

    fn render_frame_inner(&mut self, reuse_wallpaper: bool) -> Result<(), Box<dyn Error>> {
        let glass_on = self.glass_on;
        let wall = self.wall;
        let (w, h) = self.target_size();
        let util = self.util_gl.as_ref();
        // Unlock progress drives a translate-up and fade of the lock layer
        // (reference section 8): translateY(-p * H * 0.55), opacity 1 - 1.5p.
        let p = self.unlock.value();
        let rise = p * 0.55 * CANVAS.height;
        let opacity = (1.0 - 1.5 * p).clamp(0.0, 1.0);
        let mut glass_ms = None;
        let need_wallpaper = !reuse_wallpaper || !self.wallpaper_valid;
        let Some((_, wallpaper, glass, _)) = self.state.as_mut() else {
            return Ok(());
        };
        if need_wallpaper {
            wallpaper.render(&wall.build(), CANVAS.width, CANVAS.height, w, h)?;
            self.wallpaper_valid = true;
        }
        if glass_on {
            let texture = wallpaper.texture().ok_or("no wallpaper texture")?;
            // The glass widget rides up with the lock layer and is dropped once
            // the layer has mostly faded (the glass has no partial alpha).
            let widget = [lock_panel(rise)];
            let panels: &[GlassPanel] = if opacity > 0.25 { &widget } else { &[] };
            // Time the glass pass with a blocking finish (laptop-indicative;
            // the real budget is measured on the phone).
            if let Some(gl) = util {
                finish(gl);
                let start = Instant::now();
                glass.render(texture, CANVAS.width, CANVAS.height, w, h, panels)?;
                finish(gl);
                glass_ms = Some(start.elapsed().as_secs_f64() as f32 * 1000.0);
            } else {
                glass.render(texture, CANVAS.width, CANVAS.height, w, h, panels)?;
            }
            // Draw the lock-screen text onto the glass scene, so it shows in
            // the window and in screenshots.
            if let (Some(lock), Some(gl), Some(fbo)) =
                (self.lock.as_mut(), util, glass.scene_framebuffer())
            {
                use sphatik_render::glow::HasContext;
                lock.layout(w);
                // SAFETY: the context is current; `fbo` is the live scene FBO.
                unsafe {
                    gl.bind_framebuffer(sphatik_render::glow::FRAMEBUFFER, Some(fbo));
                    gl.viewport(0, 0, w as i32, h as i32);
                }
                lock.draw(w, h, rise, opacity);
                // SAFETY: as above.
                unsafe { gl.bind_framebuffer(sphatik_render::glow::FRAMEBUFFER, None) };
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
        // Advance the unlock spring, re-rendering over the cached wallpaper to
        // hold 60 fps.
        if self.animating {
            let now = Instant::now();
            let dt = self
                .last_frame
                .map_or(1.0 / 60.0, |t| now.duration_since(t).as_secs_f32());
            self.last_frame = Some(now);
            self.unlock.step(dt.min(0.034));
            if self.unlock.is_at_rest() {
                self.animating = false;
                // Re-arm the demo: settling unlocked snaps back to locked.
                if self.unlock.value() > 0.5 {
                    self.unlock.jump_to(0.0);
                }
            }
            let _ = self.render_frame_inner(true);
        }

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
        // Keep redrawing while the overlay is up (live graph) or the unlock
        // spring is still settling.
        if overlay_on || self.animating {
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

    /// Asks the window to redraw, if there is one.
    fn request_redraw(&self) {
        if let Some((s, _, _, _)) = self.state.as_ref() {
            s.window.request_redraw();
        }
    }

    /// Feeds a mouse event to the swipe-up-to-unlock gesture. Returns true if
    /// the window should redraw.
    fn pointer(&mut self, pressed: Option<bool>) -> bool {
        let now = self.now_ms();
        match pressed {
            Some(true) => {
                // A press interrupts any settling spring and starts a drag.
                self.animating = false;
                self.unlocking = false;
                self.touch = Some(TouchTracker::begin(self.cursor, now));
                false
            }
            None => {
                let progress = {
                    let Some(t) = self.touch.as_mut() else {
                        return false;
                    };
                    t.move_to(self.cursor, now);
                    if !self.unlocking
                        && t.past_slop()
                        && matches!(
                            gesture::classify(
                                &ShellContext::default(),
                                CANVAS,
                                t.start(),
                                t.delta(),
                                Hit::Other,
                            ),
                            Some(Gesture::Unlock)
                        )
                    {
                        self.unlocking = true;
                    }
                    if self.unlocking {
                        gesture::progress(Gesture::Unlock, CANVAS, t.delta(), 0.0).unwrap_or(0.0)
                    } else {
                        return false;
                    }
                };
                self.unlock.jump_to(progress);
                let _ = self.render_frame_inner(true);
                true
            }
            Some(false) => {
                let Some(mut t) = self.touch.take() else {
                    return false;
                };
                t.move_to(self.cursor, now);
                if !self.unlocking {
                    // Log other gestures, as before, for feel-checking.
                    if t.past_slop() {
                        let kind = gesture::classify(
                            &ShellContext::default(),
                            CANVAS,
                            t.start(),
                            t.delta(),
                            Hit::Other,
                        );
                        eprintln!(
                            "touch: from ({:.0}, {:.0}) moved ({:.0}, {:.0}) -> {:?}",
                            t.start().x,
                            t.start().y,
                            t.delta().x,
                            t.delta().y,
                            kind
                        );
                    }
                    return false;
                }
                self.unlocking = false;
                let lift = LiftOff {
                    delta: t.delta(),
                    velocity: t.velocity(),
                    paused: t.is_paused(now),
                    progress: gesture::progress(Gesture::Unlock, CANVAS, t.delta(), 0.0)
                        .unwrap_or(0.0),
                    has_recents: false,
                    canvas: CANVAS,
                };
                if let gesture::Release::Spring {
                    to,
                    velocity,
                    preset,
                } = gesture::release(Gesture::Unlock, lift)
                {
                    self.unlock.set_config(preset);
                    self.unlock.set_velocity(velocity);
                    self.unlock.animate_to(to);
                    self.animating = true;
                    self.last_frame = None;
                    return true;
                }
                false
            }
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
                if self.touch.is_some() && self.pointer(None) {
                    self.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                let redraw = self.pointer(Some(state == ElementState::Pressed));
                if redraw {
                    self.request_redraw();
                }
            }
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
    if let Some(lock) = app.lock.take() {
        lock.destroy();
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
