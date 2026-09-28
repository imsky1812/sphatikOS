//! Spring constants and the design spec's four presets.

use core::f32::consts::TAU;

/// Physical constants of a damped spring with unit mass.
///
/// The acceleration towards the target is
/// `a = -stiffness * (x - target) - damping * v`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringConfig {
    /// Spring constant `k` (per second squared).
    pub stiffness: f32,
    /// Damping coefficient `c` (per second).
    pub damping: f32,
}

impl SpringConfig {
    /// A spring from raw constants, as the prototype writes them (`{k, c}`).
    pub const fn new(stiffness: f32, damping: f32) -> Self {
        Self { stiffness, damping }
    }

    /// A spring from the design spec's parameters: damping ratio (1.0 is
    /// critically damped, below 1.0 bounces) and response time in seconds
    /// (the period of the undamped oscillation).
    ///
    /// With unit mass: `omega = 2 * pi / response`, `k = omega^2`,
    /// `c = 2 * damping_ratio * omega`.
    pub const fn from_response(damping_ratio: f32, response: f32) -> Self {
        let omega = TAU / response;
        Self {
            stiffness: omega * omega,
            damping: 2.0 * damping_ratio * omega,
        }
    }

    /// Damping ratio `c / (2 * sqrt(k))`.
    pub fn damping_ratio(&self) -> f32 {
        self.damping / (2.0 * self.stiffness.sqrt())
    }

    /// Response time in seconds, `2 * pi / sqrt(k)`.
    pub fn response(&self) -> f32 {
        TAU / self.stiffness.sqrt()
    }
}

/// The four motion presets from the design spec.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Preset {
    /// Buttons, toggles, small chips. Damping 0.90, response 0.25 s.
    Snappy,
    /// Sheets, cards, page pushes. Damping 1.00, response 0.40 s.
    Smooth,
    /// App open and close, Halo expansion, Melt. Damping 0.72, response 0.45 s.
    Bouncy,
    /// Wallpaper parallax, lock-to-home fade. Damping 1.00, response 0.60 s.
    Gentle,
}

impl Preset {
    /// Every preset, in the spec's order.
    pub const ALL: [Preset; 4] = [Self::Snappy, Self::Smooth, Self::Bouncy, Self::Gentle];

    /// Damping ratio from the spec table.
    pub const fn damping_ratio(self) -> f32 {
        match self {
            Self::Snappy => 0.90,
            Self::Smooth | Self::Gentle => 1.00,
            Self::Bouncy => 0.72,
        }
    }

    /// Response time in seconds from the spec table.
    pub const fn response(self) -> f32 {
        match self {
            Self::Snappy => 0.25,
            Self::Smooth => 0.40,
            Self::Bouncy => 0.45,
            Self::Gentle => 0.60,
        }
    }

    /// Spring constants for this preset.
    pub const fn config(self) -> SpringConfig {
        SpringConfig::from_response(self.damping_ratio(), self.response())
    }
}

impl From<Preset> for SpringConfig {
    fn from(preset: Preset) -> Self {
        preset.config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn presets_match_the_design_spec_constants() {
        // k and c derived from the spec's damping ratio and response time;
        // these are the values recorded for conflict C1.
        let expected = [
            (Preset::Snappy, 631.65, 45.24),
            (Preset::Smooth, 246.74, 31.42),
            (Preset::Bouncy, 194.96, 20.11),
            (Preset::Gentle, 109.66, 20.94),
        ];
        for (preset, k, c) in expected {
            let cfg = preset.config();
            assert!(
                close(cfg.stiffness, k, 0.01),
                "{preset:?} k = {}",
                cfg.stiffness
            );
            assert!(
                close(cfg.damping, c, 0.01),
                "{preset:?} c = {}",
                cfg.damping
            );
        }
    }

    #[test]
    fn presets_round_trip_to_the_spec_table() {
        for preset in Preset::ALL {
            let cfg = SpringConfig::from(preset);
            assert!(close(cfg.damping_ratio(), preset.damping_ratio(), 1e-5));
            assert!(close(cfg.response(), preset.response(), 1e-5));
        }
    }

    #[test]
    fn prototype_constants_describe_the_expected_springs() {
        // The prototype's Smooth {k: 210, c: 29} is critically damped with a
        // slightly slower response than the spec (see conflict C1).
        let smooth = SpringConfig::new(210.0, 29.0);
        assert!(close(smooth.damping_ratio(), 1.0, 0.001));
        assert!(close(smooth.response(), 0.4336, 0.0005));
        let bouncy = SpringConfig::new(190.0, 18.0);
        assert!(close(bouncy.damping_ratio(), 0.653, 0.001));
    }

    #[test]
    fn softer_presets_respond_more_slowly() {
        let k = |p: Preset| p.config().stiffness;
        assert!(k(Preset::Snappy) > k(Preset::Smooth));
        assert!(k(Preset::Smooth) > k(Preset::Bouncy));
        assert!(k(Preset::Bouncy) > k(Preset::Gentle));
    }
}
