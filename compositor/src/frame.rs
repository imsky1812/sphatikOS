//! Frame pacing for the compositor's render loop (WP 2.1).
//!
//! [`FrameClock`] is a pure, allocation-free timer: it decides *when* to draw
//! the next frame to hold a target rate (60 fps), measures the real cadence,
//! and counts frames that fell behind their deadline. It takes the current
//! monotonic time as a plain `f64` of seconds, so it has no platform
//! dependency and is tested the same on every OS; the Linux backend feeds it
//! `Instant` deltas and arms a `calloop` timer from [`FrameClock::time_until_next`].

/// The outcome of one [`FrameClock::tick`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameTick {
    /// Whether a frame is due now and should be drawn.
    pub render: bool,
    /// Seconds since the previous rendered frame (0.0 on the first frame).
    pub dt: f64,
}

/// A render-pacing clock targeting a fixed frame rate.
///
/// Call [`tick`](Self::tick) with the current monotonic time every time the
/// loop wakes. It renders at most once per frame period; between periods it
/// returns `render: false` so a busy loop does no work. When the caller is
/// late, the missed whole periods are counted as dropped frames and the
/// schedule resynchronises instead of trying to catch up in a burst.
#[derive(Clone, Copy, Debug)]
pub struct FrameClock {
    target_dt: f64,
    next_deadline: f64,
    last_render: f64,
    avg_dt: f64,
    frames: u64,
    dropped: u64,
    started: bool,
}

/// How strongly each frame's duration pulls the smoothed average (EMA weight
/// of the newest sample). Small enough to stay steady, large enough to react.
const AVG_SMOOTHING: f64 = 0.1;

impl FrameClock {
    /// A clock pacing to `fps` frames per second (clamped to a sane range).
    pub fn new(fps: f64) -> Self {
        let fps = if fps.is_finite() && fps > 0.0 {
            fps.clamp(1.0, 1000.0)
        } else {
            60.0
        };
        Self {
            target_dt: 1.0 / fps,
            next_deadline: 0.0,
            last_render: 0.0,
            avg_dt: 1.0 / fps,
            frames: 0,
            dropped: 0,
            started: false,
        }
    }

    /// The target seconds-per-frame (e.g. ~0.0166 for 60 fps).
    pub fn target_dt(&self) -> f64 {
        self.target_dt
    }

    /// Advances the clock to `now` (monotonic seconds) and reports whether a
    /// frame is due. The first call always renders and establishes the grid.
    pub fn tick(&mut self, now: f64) -> FrameTick {
        if !self.started {
            self.started = true;
            self.last_render = now;
            self.next_deadline = now + self.target_dt;
            self.frames = 1;
            return FrameTick {
                render: true,
                dt: 0.0,
            };
        }

        // A tiny epsilon so a deadline landing on `now` still fires.
        if now + 1e-9 < self.next_deadline {
            return FrameTick {
                render: false,
                dt: now - self.last_render,
            };
        }

        let dt = now - self.last_render;
        // Whole periods missed beyond the one we are about to draw.
        let behind = ((now - self.next_deadline) / self.target_dt).floor();
        let behind = if behind.is_finite() && behind > 0.0 {
            behind as u64
        } else {
            0
        };
        self.dropped += behind;
        self.last_render = now;
        self.frames += 1;
        // Advance the ideal grid past the periods we skipped; never leave the
        // deadline in the past (which would busy-spin rendering).
        self.next_deadline += self.target_dt * (behind as f64 + 1.0);
        if self.next_deadline <= now {
            self.next_deadline = now + self.target_dt;
        }
        self.avg_dt += AVG_SMOOTHING * (dt - self.avg_dt);
        FrameTick { render: true, dt }
    }

    /// Seconds until the next frame is due (0.0 if already due or not started).
    pub fn time_until_next(&self, now: f64) -> f64 {
        if !self.started {
            return 0.0;
        }
        (self.next_deadline - now).max(0.0)
    }

    /// Frames rendered so far.
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// Frame deadlines missed because the loop was late.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// The measured rate from the smoothed frame time (0.0 before any frame).
    pub fn fps(&self) -> f64 {
        if self.avg_dt > 0.0 {
            1.0 / self.avg_dt
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT60: f64 = 1.0 / 60.0;

    #[test]
    fn first_tick_always_renders_with_zero_dt() {
        let mut c = FrameClock::new(60.0);
        let t = c.tick(123.0);
        assert!(t.render);
        assert_eq!(t.dt, 0.0);
        assert_eq!(c.frames(), 1);
    }

    #[test]
    fn subframe_ticks_do_not_render() {
        let mut c = FrameClock::new(60.0);
        c.tick(0.0);
        let mid = c.tick(DT60 * 0.5);
        assert!(!mid.render);
        assert_eq!(c.frames(), 1);
    }

    #[test]
    fn steady_sixty_is_sixty_with_no_drops() {
        let mut c = FrameClock::new(60.0);
        for k in 0..240 {
            c.tick(k as f64 * DT60);
        }
        assert_eq!(c.frames(), 240);
        assert_eq!(c.dropped(), 0);
        assert!((c.fps() - 60.0).abs() < 0.01, "fps was {}", c.fps());
    }

    #[test]
    fn a_long_stall_counts_dropped_frames() {
        let mut c = FrameClock::new(60.0);
        c.tick(0.0); // frame 1, deadline at DT60
                     // Wake three periods late: render once, two whole periods were missed.
        let t = c.tick(DT60 + 3.0 * DT60);
        assert!(t.render);
        assert_eq!(c.dropped(), 3);
        assert_eq!(c.frames(), 2);
    }

    #[test]
    fn deadline_never_trails_behind_after_a_stall() {
        let mut c = FrameClock::new(60.0);
        c.tick(0.0);
        let now = 10.0; // huge jump
        c.tick(now);
        // The next frame is scheduled in the future, not immediately.
        assert!(c.time_until_next(now) > 0.0);
        assert!(c.time_until_next(now) <= DT60 + 1e-9);
    }

    #[test]
    fn time_until_next_counts_down_within_a_period() {
        let mut c = FrameClock::new(60.0);
        c.tick(0.0);
        let left = c.time_until_next(DT60 * 0.25);
        assert!((left - DT60 * 0.75).abs() < 1e-9, "left was {left}");
    }

    #[test]
    fn invalid_fps_falls_back_to_sixty() {
        for bad in [0.0, -5.0, f64::NAN, f64::INFINITY] {
            let c = FrameClock::new(bad);
            assert!((c.target_dt() - DT60).abs() < 1e-9);
        }
    }
}
