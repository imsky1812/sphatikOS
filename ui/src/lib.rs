//! `sphatik-ui`: the one toolkit used by the shell, the stock apps and
//! third-party native apps.
//!
//! Reactive state, taffy layout, cosmic-text, the shared spring engine and
//! the design tokens will live here (handbook, "UI toolkit and shell").

/// The shared spring engine (presets, springs, velocity hand-off).
pub use sphatik_motion as motion;

/// Crate version, for diagnostics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_the_workspace() {
        assert_eq!(VERSION, "0.1.0");
    }
}
