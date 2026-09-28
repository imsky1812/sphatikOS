//! `sphatik-preview`: runs the Sphatik shell and renderer in a desktop
//! window, so the UI can be seen and felt on Windows (ADR 0008).
//!
//! ```text
//! cargo run -p sphatik-preview                              # open the window
//! cargo run -p sphatik-preview -- --screenshot out.png      # save one frame
//! ```
//!
//! The mouse acts as a finger. Frames are drawn only when something changed;
//! the title bar counts drawn frames so an idle window can be seen to cost
//! nothing.

mod gl;

use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use clap::Parser;
use sphatik_render::{demo, Damage, Rect, Renderer, Shape};
use sphatik_shell::gesture::{self, Canvas, Hit, Point, ShellContext, TouchTracker};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

/// Command-line options.
#[derive(Debug, Parser)]
#[command(name = "sphatik-preview", version, about)]
struct Options {
    /// Render one frame to this PNG file and exit.
    #[arg(long, value_name = "PNG")]
    screenshot: Option<PathBuf>,
    /// Pixels per point for --screenshot (2.75 matches the phone).
    #[arg(long, default_value_t = 1.0)]
    scale: f32,
}

/// The canvas the preview shows: the prototype's 393 x 852 pt screen.
const CANVAS: Canvas = Canvas::PROTOTYPE;

struct App {
    options: Options,
    state: Option<(gl::GlWindowState, Renderer)>,
    shapes: [Shape; 2],
    damage: Damage,
    frames_drawn: u64,
    last_frame_ms: f64,
    started: Instant,
    touch: Option<TouchTracker>,
    cursor: Point,
    result: Result<(), String>,
}

impl App {
    fn new(options: Options) -> Self {
        Self {
            options,
            state: None,
            shapes: demo::backdrop_with_panel(canvas_rect()),
            damage: Damage::new(),
            frames_drawn: 0,
            last_frame_ms: 0.0,
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

    /// Opens the window and GL context; sizes the render target.
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
        let renderer = Renderer::new(glow_ctx)?;
        self.state = Some((window_state, renderer));
        if screenshot {
            let (w, h) = self.screenshot_size();
            self.resize_target(w, h)?;
        } else {
            let size = self.state.as_ref().map(|(s, _)| s.window.inner_size());
            if let Some(size) = size {
                self.resize_target(size.width, size.height)?;
            }
        }
        Ok(())
    }

    fn screenshot_size(&self) -> (u32, u32) {
        let s = self.options.scale.clamp(0.25, 8.0);
        (
            (CANVAS.width * s).round() as u32,
            (CANVAS.height * s).round() as u32,
        )
    }

    fn resize_target(&mut self, width: u32, height: u32) -> Result<(), Box<dyn Error>> {
        if let Some((_, renderer)) = self.state.as_mut() {
            renderer.resize(CANVAS.width, CANVAS.height, width.max(1), height.max(1))?;
            self.damage.add_all(canvas_rect());
        }
        Ok(())
    }

    fn redraw(&mut self) -> Result<(), Box<dyn Error>> {
        let Some((window_state, renderer)) = self.state.as_mut() else {
            return Ok(());
        };
        let started = Instant::now();
        let stats = renderer.render(&self.shapes, self.damage.take(), demo::CLEAR);
        if !stats.drawn {
            return Ok(());
        }
        let size = window_state.window.inner_size();
        renderer.present(size.width, size.height);
        window_state.swap()?;
        self.frames_drawn += 1;
        self.last_frame_ms = started.elapsed().as_secs_f64() * 1000.0;
        window_state.window.set_title(&format!(
            "Sphatik preview · frames drawn: {} · last frame {:.2} ms CPU",
            self.frames_drawn, self.last_frame_ms
        ));
        Ok(())
    }

    fn save_screenshot(&mut self) -> Result<PathBuf, Box<dyn Error>> {
        let path = self
            .options
            .screenshot
            .clone()
            .ok_or("no screenshot path")?;
        let (_, renderer) = self.state.as_mut().ok_or("no renderer")?;
        renderer.render(&self.shapes, Some(canvas_rect()), demo::CLEAR);
        let (w, h) = renderer.target_size().ok_or("no render target")?;
        let pixels = renderer.read_pixels().ok_or("could not read pixels")?;
        write_png(&path, w, h, &pixels)?;
        Ok(path)
    }

    /// Mouse-as-touch: feeds the gesture recogniser and reports what it saw.
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
                let v = t.velocity();
                eprintln!(
                    "touch: from ({:.0}, {:.0}) moved ({:.0}, {:.0}), v = ({:.2}, {:.2}) pt/ms -> {:?}",
                    t.start().x,
                    t.start().y,
                    t.delta().x,
                    t.delta().y,
                    v.x,
                    v.y,
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
        if let Some((s, _)) = self.state.as_ref() {
            s.window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                if let Some((s, _)) = self.state.as_ref() {
                    s.resize(width, height);
                }
                if let Err(e) = self.resize_target(width, height) {
                    self.fail(event_loop, format!("resize failed: {e}"));
                    return;
                }
                if let Some((s, _)) = self.state.as_ref() {
                    s.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.redraw() {
                    self.fail(event_loop, format!("draw failed: {e}"));
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some((s, _)) = self.state.as_ref() {
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

fn canvas_rect() -> Rect {
    Rect::new(0.0, 0.0, CANVAS.width, CANVAS.height)
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
    if let Some((s, renderer)) = app.state.take() {
        renderer.destroy();
        drop(s);
    }
    match app.result {
        Ok(()) => {
            if app.frames_drawn > 0 {
                println!(
                    "frames drawn: {} (last {:.2} ms CPU)",
                    app.frames_drawn, app.last_frame_ms
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("sphatik-preview: {e}");
            ExitCode::FAILURE
        }
    }
}
