//! The spring solver.
//!
//! Behaviour follows the prototype's `run()` (`prototype/index.html`,
//! `/* Springs */`): one spring per animated value, a 34 ms cap on a frame,
//! retargeting that keeps velocity, and the same rest thresholds. The prototype
//! integrates with 4 semi-implicit Euler substeps, which drifts from the true
//! motion (about 0.02 on Snappy's first frame, and Bouncy overshoots 3.3%
//! instead of 3.8%). Build plan 2.6 requires the spec's values, so each frame
//! here advances by the exact closed-form solution of the damped spring. It is
//! also frame-rate independent, which keeps the 120 Hz path honest.

use crate::SpringConfig;

/// Longest frame the solver advances in one step, in seconds. Longer gaps
/// (a dropped frame, a debugger pause) are treated as this long, so a hitch
/// never makes a spring jump.
pub const MAX_FRAME_DT: f32 = 0.034;

/// Damping ratios this close to 1.0 use the critically damped solution.
const CRITICAL_EPSILON: f64 = 1e-4;

/// A spring comes to rest once its speed is below this (units per second)…
pub const REST_VELOCITY: f32 = 0.02;

/// …and it is closer than this to its target.
pub const REST_DISTANCE: f32 = 0.002;

/// An interruptible spring animating one value, usually progress from 0 to 1.
///
/// Retargeting keeps the current value and velocity, so any animation can be
/// caught mid-flight and reversed with no jump. The rest thresholds are tuned
/// for progress units, as in the prototype.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    value: f32,
    velocity: f32,
    target: f32,
    config: SpringConfig,
    at_rest: bool,
}

impl Spring {
    /// A spring resting at `value`.
    pub fn new(value: f32, config: impl Into<SpringConfig>) -> Self {
        Self {
            value,
            velocity: 0.0,
            target: value,
            config: config.into(),
            at_rest: true,
        }
    }

    /// Starts (or redirects) the animation towards `target`, keeping the
    /// current value and velocity.
    pub fn animate_to(&mut self, target: f32) {
        self.target = target;
        self.at_rest = false;
    }

    /// Hands over a velocity, for example from a finger that just lifted.
    /// See [`progress_velocity`] to convert a gesture speed.
    pub fn set_velocity(&mut self, velocity: f32) {
        self.velocity = velocity;
        self.at_rest = false;
    }

    /// Changes the spring constants without disturbing the motion.
    pub fn set_config(&mut self, config: impl Into<SpringConfig>) {
        self.config = config.into();
    }

    /// Moves straight to `target` and stops (Reduce Motion, or a finger
    /// taking direct control).
    pub fn jump_to(&mut self, target: f32) {
        self.value = target;
        self.target = target;
        self.velocity = 0.0;
        self.at_rest = true;
    }

    /// Advances the spring by one frame of `dt` seconds and returns the new
    /// value. Frames longer than [`MAX_FRAME_DT`] are clamped; zero, negative
    /// or non-finite `dt` leaves the spring unchanged.
    pub fn step(&mut self, dt: f32) -> f32 {
        if self.at_rest || !dt.is_finite() || dt <= 0.0 {
            return self.value;
        }
        match advance(
            self.config,
            f64::from(self.value - self.target),
            f64::from(self.velocity),
            f64::from(dt.min(MAX_FRAME_DT)),
        ) {
            Some((offset, velocity)) => {
                self.value = self.target + offset as f32;
                self.velocity = velocity as f32;
            }
            // A spring with no stiffness can never arrive; finish instead.
            None => self.jump_to(self.target),
        }
        if self.velocity.abs() < REST_VELOCITY && (self.value - self.target).abs() < REST_DISTANCE {
            self.jump_to(self.target);
        }
        self.value
    }

    /// Current value.
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Current velocity, in value units per second.
    pub fn velocity(&self) -> f32 {
        self.velocity
    }

    /// Value the spring is heading to.
    pub fn target(&self) -> f32 {
        self.target
    }

    /// Spring constants in use.
    pub fn config(&self) -> SpringConfig {
        self.config
    }

    /// True once the spring has settled on its target.
    pub fn is_at_rest(&self) -> bool {
        self.at_rest
    }
}

/// Exact state of a unit-mass damped spring `t` seconds on, given its offset
/// from the target `e0` and velocity `v0`. Returns `(offset, velocity)`, or
/// `None` for constants that do not describe a returning spring.
fn advance(config: SpringConfig, e0: f64, v0: f64, t: f64) -> Option<(f64, f64)> {
    let k = f64::from(config.stiffness);
    let c = f64::from(config.damping);
    if !(k.is_finite() && c.is_finite()) || k <= 0.0 || c < 0.0 {
        return None;
    }
    let omega = k.sqrt();
    let zeta = c / (2.0 * omega);

    let state = if (zeta - 1.0).abs() < CRITICAL_EPSILON {
        // e(t) = (A + B t) exp(-w t)
        let b = v0 + omega * e0;
        let decay = (-omega * t).exp();
        ((e0 + b * t) * decay, (v0 - omega * b * t) * decay)
    } else if zeta < 1.0 {
        // e(t) = exp(-z w t) (A cos(wd t) + B sin(wd t))
        let wd = omega * (1.0 - zeta * zeta).sqrt();
        let a = e0;
        let b = (v0 + zeta * omega * a) / wd;
        let decay = (-zeta * omega * t).exp();
        let (sin, cos) = (wd * t).sin_cos();
        (
            decay * (a * cos + b * sin),
            decay * ((b * wd - zeta * omega * a) * cos - (a * wd + zeta * omega * b) * sin),
        )
    } else {
        // e(t) = C1 exp(r1 t) + C2 exp(r2 t)
        let root = omega * (zeta * zeta - 1.0).sqrt();
        let (r1, r2) = (-zeta * omega + root, -zeta * omega - root);
        let c1 = (v0 - r2 * e0) / (r1 - r2);
        let c2 = e0 - c1;
        let (x1, x2) = ((r1 * t).exp(), (r2 * t).exp());
        (c1 * x1 + c2 * x2, c1 * r1 * x1 + c2 * r2 * x2)
    };
    Some(state)
}

/// Converts a finger speed in points per millisecond into progress units per
/// second, for a gesture where `travel` points of movement equal a progress of
/// 1.0. Returns 0 for a non-positive or non-finite `travel`.
pub fn progress_velocity(points_per_ms: f32, travel: f32) -> f32 {
    if travel > 0.0 && travel.is_finite() {
        points_per_ms * 1000.0 / travel
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Preset;

    const FRAME: f32 = 1.0 / 60.0;

    /// Reference position of a unit-mass spring released at `x0` with `v0`,
    /// heading to `target`, after `t` seconds. Integrates the equation of
    /// motion directly with fine RK4 steps in f64, so it shares no code or
    /// formulas with the closed-form solver under test.
    fn analytic(cfg: SpringConfig, x0: f64, v0: f64, target: f64, t: f64) -> f64 {
        let (k, c) = (f64::from(cfg.stiffness), f64::from(cfg.damping));
        let accel = |x: f64, v: f64| -k * (x - target) - c * v;
        let steps = (t / 1e-5).ceil().max(1.0) as u64;
        let h = t / steps as f64;
        let (mut x, mut v) = (x0, v0);
        for _ in 0..steps {
            let (k1x, k1v) = (v, accel(x, v));
            let (k2x, k2v) = (
                v + 0.5 * h * k1v,
                accel(x + 0.5 * h * k1x, v + 0.5 * h * k1v),
            );
            let (k3x, k3v) = (
                v + 0.5 * h * k2v,
                accel(x + 0.5 * h * k2x, v + 0.5 * h * k2v),
            );
            let (k4x, k4v) = (v + h * k3v, accel(x + h * k3x, v + h * k3v));
            x += h / 6.0 * (k1x + 2.0 * k2x + 2.0 * k3x + k4x);
            v += h / 6.0 * (k1v + 2.0 * k2v + 2.0 * k3v + k4v);
        }
        x
    }

    /// Runs a spring at 60 Hz until rest (or 5 s) and returns every frame.
    fn simulate(preset: Preset, from: f32, to: f32, v0: f32) -> Vec<f32> {
        let mut s = Spring::new(from, preset);
        s.set_velocity(v0);
        s.animate_to(to);
        let mut frames = Vec::new();
        while !s.is_at_rest() && frames.len() < 300 {
            frames.push(s.step(FRAME));
        }
        frames
    }

    #[test]
    fn tracks_the_exact_solution_at_60_hz() {
        for preset in Preset::ALL {
            for v0 in [0.0_f32, 3.0, -2.0] {
                let cfg = preset.config();
                let frames = simulate(preset, 0.0, 1.0, v0);
                let last = frames.len() - 1;
                for (i, &x) in frames.iter().enumerate() {
                    let t = f64::from(FRAME) * (i as f64 + 1.0);
                    let exact = analytic(cfg, 0.0, f64::from(v0), 1.0, t);
                    // The final frame snaps to the target once inside the
                    // rest thresholds, exactly as the prototype does.
                    let tol = if i == last {
                        f64::from(REST_DISTANCE)
                    } else {
                        1e-4
                    };
                    assert!(
                        (f64::from(x) - exact).abs() < tol,
                        "{preset:?} v0={v0} frame {i}: {x} vs {exact}"
                    );
                }
            }
        }
    }

    #[test]
    fn critically_damped_presets_never_overshoot() {
        for preset in [Preset::Smooth, Preset::Gentle] {
            let peak = simulate(preset, 0.0, 1.0, 0.0)
                .into_iter()
                .fold(f32::MIN, f32::max);
            assert!(peak <= 1.0 + 1e-4, "{preset:?} overshot to {peak}");
        }
    }

    #[test]
    fn bouncy_overshoots_by_the_theoretical_amount() {
        // Peak overshoot of an underdamped spring: exp(-pi * z / sqrt(1 - z^2)).
        let z = f64::from(Preset::Bouncy.damping_ratio());
        let expected = (-core::f64::consts::PI * z / (1.0 - z * z).sqrt()).exp();
        let peak = simulate(Preset::Bouncy, 0.0, 1.0, 0.0)
            .into_iter()
            .fold(f32::MIN, f32::max);
        let overshoot = f64::from(peak) - 1.0;
        assert!(
            (overshoot - expected).abs() < 0.002,
            "overshoot {overshoot} vs {expected}"
        );
    }

    #[test]
    fn every_preset_settles_exactly_on_target_in_time() {
        let limits = [
            (Preset::Snappy, 0.5),
            (Preset::Smooth, 0.8),
            (Preset::Bouncy, 1.0),
            (Preset::Gentle, 1.2),
        ];
        for (preset, max_seconds) in limits {
            let frames = simulate(preset, 0.0, 1.0, 0.0);
            let seconds = frames.len() as f32 * FRAME;
            assert!(seconds <= max_seconds, "{preset:?} took {seconds} s");
            assert_eq!(frames.last().copied(), Some(1.0), "{preset:?} did not snap");
        }
    }

    #[test]
    fn a_settled_spring_stays_put() {
        let mut s = Spring::new(0.0, Preset::Snappy);
        s.animate_to(1.0);
        while !s.is_at_rest() {
            s.step(FRAME);
        }
        assert_eq!((s.value(), s.velocity()), (1.0, 0.0));
        assert_eq!(s.step(FRAME), 1.0);
        assert!(s.is_at_rest());
    }

    #[test]
    fn retargeting_mid_flight_is_continuous() {
        let mut s = Spring::new(0.0, Preset::Smooth);
        s.animate_to(1.0);
        for _ in 0..10 {
            s.step(FRAME);
        }
        let (x, v) = (s.value(), s.velocity());
        assert!(v > 0.5, "should be moving towards 1");

        s.animate_to(0.0);
        assert_eq!((s.value(), s.velocity()), (x, v), "retarget must not jump");

        let next = s.step(FRAME);
        assert!((next - x).abs() <= v * FRAME + 0.01, "{x} -> {next}");
        assert!(next > x, "momentum carries it on briefly before reversing");
    }

    #[test]
    fn motion_is_the_same_at_60_and_120_hz() {
        let mut at60 = Spring::new(0.0, Preset::Bouncy);
        let mut at120 = at60;
        at60.set_velocity(2.0);
        at120.set_velocity(2.0);
        at60.animate_to(1.0);
        at120.animate_to(1.0);
        for _ in 0..30 {
            at60.step(FRAME);
            at120.step(FRAME / 2.0);
            at120.step(FRAME / 2.0);
            assert!((at60.value() - at120.value()).abs() < 1e-4);
            assert!((at60.velocity() - at120.velocity()).abs() < 1e-3);
        }
    }

    #[test]
    fn overdamped_springs_track_the_exact_solution() {
        let cfg = SpringConfig::from_response(1.6, 0.4);
        let mut s = Spring::new(0.0, cfg);
        s.animate_to(1.0);
        for i in 1..=30 {
            let x = s.step(FRAME);
            let exact = analytic(cfg, 0.0, 0.0, 1.0, f64::from(FRAME) * f64::from(i));
            assert!(
                (f64::from(x) - exact).abs() < 1e-4,
                "frame {i}: {x} vs {exact}"
            );
        }
    }

    #[test]
    fn broken_constants_finish_instead_of_diverging() {
        for cfg in [
            SpringConfig::new(0.0, 10.0),
            SpringConfig::new(-5.0, 10.0),
            SpringConfig::new(100.0, -1.0),
            SpringConfig::new(f32::NAN, 10.0),
        ] {
            let mut s = Spring::new(0.0, cfg);
            s.animate_to(1.0);
            assert_eq!(s.step(FRAME), 1.0, "{cfg:?}");
            assert!(s.is_at_rest());
        }
    }

    #[test]
    fn long_frames_are_clamped() {
        let mut a = Spring::new(0.0, Preset::Bouncy);
        let mut b = a;
        a.animate_to(1.0);
        b.animate_to(1.0);
        assert_eq!(a.step(1.0), b.step(MAX_FRAME_DT));
    }

    #[test]
    fn invalid_frame_times_change_nothing() {
        let mut s = Spring::new(0.0, Preset::Smooth);
        s.animate_to(1.0);
        let before = s;
        for dt in [0.0, -0.016, f32::NAN, f32::INFINITY] {
            s.step(dt);
            assert_eq!(s, before, "dt = {dt}");
        }
    }

    #[test]
    fn jump_to_stops_immediately() {
        let mut s = Spring::new(0.0, Preset::Bouncy);
        s.set_velocity(5.0);
        s.animate_to(1.0);
        s.step(FRAME);
        s.jump_to(0.25);
        assert_eq!((s.value(), s.velocity(), s.target()), (0.25, 0.0, 0.25));
        assert!(s.is_at_rest());
    }

    #[test]
    fn handed_over_velocity_moves_a_resting_spring() {
        let mut s = Spring::new(0.5, Preset::Smooth);
        s.set_velocity(2.0);
        assert!(s.step(FRAME) > 0.5);
    }

    #[test]
    fn progress_velocity_converts_finger_speed() {
        // Unlock: 0.5 pt/ms upward over half the 852 pt canvas.
        assert!((progress_velocity(0.5, 426.0) - 1.1737).abs() < 1e-3);
        assert_eq!(progress_velocity(1.0, 0.0), 0.0);
        assert_eq!(progress_velocity(1.0, -5.0), 0.0);
        assert_eq!(progress_velocity(1.0, f32::NAN), 0.0);
    }
}
