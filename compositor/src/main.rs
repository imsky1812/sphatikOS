//! `sphatik-comp`: the Sphatik OS Wayland compositor.
//!
//! Owns the display and input, composites every window and draws the shell
//! and all glass in one GPU pass per frame (ADR 0002, ADR 0007).
//!
//! The real compositor runs on Linux only (Smithay, Wayland, GLES) and is
//! built behind the `winit-backend` feature (ADR 0008): on Windows and in the
//! aarch64 cross-build the feature is off and this binary is a stub, so the
//! workspace stays buildable everywhere. The frame pacing in [`frame`] is
//! platform-independent and always compiled.

mod frame;

#[cfg(all(target_os = "linux", feature = "winit-backend"))]
mod backend;

/// One-line identification printed at start-up.
fn banner() -> String {
    format!("sphatik-comp {}", env!("CARGO_PKG_VERSION"))
}

/// What the command line asked the binary to do.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    /// Print the banner and exit (default off-device, and when no window is
    /// available).
    Banner,
    /// Run `frames` frames of the render loop, report the cadence, and exit.
    SelfTest { frames: u64, fps: f64 },
    /// Run the compositor until closed (Linux + `winit-backend` only).
    Run,
}

/// Parses the process arguments into a [`Mode`]. Unknown flags are ignored so
/// the stub and the real backend accept the same command line.
fn parse_mode(args: &[String]) -> Mode {
    let mut iter = args.iter();
    let mut fps = 60.0;
    let mut selftest: Option<u64> = None;
    let mut run = false;
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--selftest" => {
                let frames = iter
                    .clone()
                    .next()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(120);
                // Consume the number if it was one.
                if iter
                    .clone()
                    .next()
                    .is_some_and(|s| s.parse::<u64>().is_ok())
                {
                    iter.next();
                }
                selftest = Some(frames);
            }
            "--fps" => {
                if let Some(v) = iter.next().and_then(|s| s.parse::<f64>().ok()) {
                    fps = v;
                }
            }
            "--run" => run = true,
            _ => {}
        }
    }
    if let Some(frames) = selftest {
        Mode::SelfTest { frames, fps }
    } else if run {
        Mode::Run
    } else {
        Mode::Banner
    }
}

/// Paces `frames` frames through `FrameClock` against the real monotonic clock
/// and returns the measured rate and the dropped-frame count. This exercises
/// the pacing without a GPU; the Linux backend drives the same clock with real
/// draws instead, so this path is only compiled where that backend is not.
#[cfg(not(all(target_os = "linux", feature = "winit-backend")))]
fn run_selftest(frames: u64, fps: f64) -> (f64, u64) {
    use crate::frame::FrameClock;
    use std::time::{Duration, Instant};

    let mut clock = FrameClock::new(fps);
    let start = Instant::now();
    while clock.frames() < frames {
        let now = start.elapsed().as_secs_f64();
        let _ = clock.tick(now);
        let wait = clock.time_until_next(start.elapsed().as_secs_f64());
        if wait > 0.0 {
            // Cap the sleep so a never-ending wait can't wedge the loop.
            std::thread::sleep(Duration::from_secs_f64(wait.min(clock.target_dt())));
        }
    }
    (clock.fps(), clock.dropped())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_mode(&args) {
        Mode::Banner => {
            println!("{}", banner());
            #[cfg(not(all(target_os = "linux", feature = "winit-backend")))]
            println!("(compositor backend not built: this is the portable stub)");
        }
        Mode::SelfTest { frames, fps } => {
            println!("{}", banner());
            // On Linux with the backend, drive a real window (under Xvfb in
            // CI); elsewhere pace the clock without a GPU.
            #[cfg(all(target_os = "linux", feature = "winit-backend"))]
            let result = backend::selftest(frames, fps);
            #[cfg(not(all(target_os = "linux", feature = "winit-backend")))]
            let result: Result<(f64, u64), std::convert::Infallible> =
                Ok(run_selftest(frames, fps));

            match result {
                Ok((measured, dropped)) => {
                    println!(
                        "selftest: {frames} frames at target {fps:.0} fps -> {measured:.1} fps, {dropped} dropped"
                    );
                }
                Err(e) => {
                    eprintln!("sphatik-comp: selftest failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        Mode::Run => {
            println!("{}", banner());
            #[cfg(all(target_os = "linux", feature = "winit-backend"))]
            {
                if let Err(e) = backend::run() {
                    eprintln!("sphatik-comp: {e}");
                    std::process::exit(1);
                }
            }
            #[cfg(not(all(target_os = "linux", feature = "winit-backend")))]
            {
                eprintln!("sphatik-comp: --run needs the winit-backend feature on Linux");
                std::process::exit(2);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_names_the_binary_and_version() {
        assert_eq!(
            banner(),
            format!("sphatik-comp {}", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn no_args_is_banner_mode() {
        assert_eq!(parse_mode(&[]), Mode::Banner);
    }

    #[test]
    fn selftest_flag_parses_frame_count_and_fps() {
        let args = ["--selftest", "90", "--fps", "120"].map(String::from);
        assert_eq!(
            parse_mode(&args),
            Mode::SelfTest {
                frames: 90,
                fps: 120.0
            }
        );
    }

    #[test]
    fn selftest_without_a_number_uses_the_default() {
        let args = ["--selftest".to_string()];
        assert_eq!(
            parse_mode(&args),
            Mode::SelfTest {
                frames: 120,
                fps: 60.0
            }
        );
    }

    #[test]
    fn run_flag_selects_run_mode() {
        assert_eq!(parse_mode(&["--run".to_string()]), Mode::Run);
    }

    #[cfg(not(all(target_os = "linux", feature = "winit-backend")))]
    #[test]
    fn selftest_paces_close_to_target_without_dropping() {
        // A short, fast run: the measured rate should be near target and the
        // loop should not fall behind on an idle machine.
        let (fps, dropped) = run_selftest(12, 60.0);
        assert!(fps > 30.0 && fps < 90.0, "fps was {fps}");
        assert_eq!(dropped, 0);
    }
}
