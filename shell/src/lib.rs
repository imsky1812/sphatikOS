//! `sphatik-shell`: the Sphatik OS system UI.
//!
//! Lock screen, home, App Library, panels, Halo, switcher and Intent Bar.
//! The shell runs inside `sphatik-comp` (ADR 0007) so glass can read the
//! backdrop directly. Its visual reference is `docs/design/prototype-reference.md`.

pub mod gesture;

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
