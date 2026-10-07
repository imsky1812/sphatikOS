//! The Linux winit backend for `sphatik-comp` (WP 2.1).
//!
//! Opens a window through Smithay's winit backend, creates a GLES renderer on
//! it, and drives a render loop paced by [`crate::frame::FrameClock`] so it
//! holds 60 fps. In CI there is no display server, so the window is opened
//! against Xvfb and the loop runs in [`selftest`] for a fixed number of frames
//! before reporting the measured cadence and exiting.
//!
//! This module is compiled only on Linux with the `winit-backend` feature
//! (ADR 0008). It draws a plain tinted clear for now; compositing real
//! surfaces arrives in WP 2.8 and the shell in the Glass-on-laptop gate.

use std::error::Error;
use std::time::{Duration, Instant};

use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::{Color32F, Frame, Renderer};
use smithay::backend::winit::{self, WinitEvent};
use smithay::reexports::winit::platform::pump_events::PumpStatus;
use smithay::utils::{Rectangle, Transform};

use crate::frame::FrameClock;

/// How long to drive the loop.
#[derive(Clone, Copy)]
enum Limit {
    /// Run until the window is closed.
    UntilClosed,
    /// Stop after this many rendered frames (CI self-test).
    Frames(u64),
}

/// Runs the compositor window until it is closed.
pub fn run() -> Result<(), Box<dyn Error>> {
    drive(Limit::UntilClosed, 60.0).map(|_| ())
}

/// Runs `frames` frames of the real windowed loop and returns the measured
/// frame rate and dropped-frame count.
pub fn selftest(frames: u64, fps: f64) -> Result<(f64, u64), Box<dyn Error>> {
    drive(Limit::Frames(frames), fps)
}

/// Frames rendered before measurement starts, so first-frame window and GL
/// setup costs do not count as dropped frames in the self-test.
const WARMUP_FRAMES: u64 = 24;

/// Opens the winit window, warms up, then runs the paced render loop and
/// returns the steady-state cadence.
fn drive(limit: Limit, fps: f64) -> Result<(f64, u64), Box<dyn Error>> {
    let (mut backend, mut winit_loop) =
        winit::init::<GlesRenderer>().map_err(|e| format!("winit backend init failed: {e}"))?;

    // Warm up only for a bounded self-test; `run` keeps going regardless.
    if let Limit::Frames(_) = limit {
        render_loop(
            &mut backend,
            &mut winit_loop,
            Limit::Frames(WARMUP_FRAMES),
            fps,
        )?;
    }
    render_loop(&mut backend, &mut winit_loop, limit, fps)
}

/// Pumps winit events and draws paced frames until the limit is reached or the
/// window closes. Each call measures from a fresh [`FrameClock`].
fn render_loop(
    backend: &mut winit::WinitGraphicsBackend<GlesRenderer>,
    winit_loop: &mut winit::WinitEventLoop,
    limit: Limit,
    fps: f64,
) -> Result<(f64, u64), Box<dyn Error>> {
    let mut clock = FrameClock::new(fps);
    let start = Instant::now();
    let mut tint = 0.0f32;
    let mut running = true;

    while running {
        let mut close = false;
        let status = winit_loop.dispatch_new_events(|event| {
            if let WinitEvent::CloseRequested = event {
                close = true;
            }
        });
        if let PumpStatus::Exit(_) = status {
            break;
        }
        if close {
            break;
        }

        let now = start.elapsed().as_secs_f64();
        let tick = clock.tick(now);
        if tick.render {
            render_frame(backend, tint)?;
            tint = (tint + 0.01) % 1.0;
            if let Limit::Frames(n) = limit {
                if clock.frames() >= n {
                    running = false;
                }
            }
        } else {
            let wait = clock.time_until_next(start.elapsed().as_secs_f64());
            if wait > 0.0 {
                std::thread::sleep(Duration::from_secs_f64(wait.min(clock.target_dt())));
            }
        }
    }

    Ok((clock.fps(), clock.dropped()))
}

/// Clears the window to a dark tinted colour, proving the GLES renderer draws
/// a fresh frame each tick.
fn render_frame(
    backend: &mut winit::WinitGraphicsBackend<GlesRenderer>,
    tint: f32,
) -> Result<(), Box<dyn Error>> {
    let size = backend.window_size();
    let damage = [Rectangle::from_size(size)];
    {
        // `bind` hands back the renderer and a framebuffer that both borrow the
        // backend; this block drops them before `submit` reborrows it.
        let (renderer, mut fb) = backend.bind()?;
        let mut frame = renderer.render(&mut fb, size, Transform::Flipped180)?;
        frame.clear(Color32F::new(0.02, 0.02, 0.06 + tint * 0.02, 1.0), &damage)?;
        let _ = frame.finish()?;
    }
    backend.submit(Some(&damage))?;
    Ok(())
}
