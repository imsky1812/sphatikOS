//! `sphatik-motion`: the one spring engine shared by the shell and every app.
//!
//! Every transition in Sphatik is a spring driven by the finger's velocity
//! (design spec, "Motion system and global gestures"). This crate holds the
//! four presets and the solver ported from the prototype's `run()` (see
//! `docs/design/prototype-reference.md`, section 6).
//!
//! Everything here is plain `Copy` data: nothing allocates, so springs are
//! safe to step in per-frame paths.
//!
//! ```
//! use sphatik_motion::{Preset, Spring};
//!
//! let mut unlock = Spring::new(0.0, Preset::Smooth);
//! unlock.set_velocity(1.2); // from the finger, see `progress_velocity`
//! unlock.animate_to(1.0);
//! while !unlock.is_at_rest() {
//!     let progress = unlock.step(1.0 / 60.0);
//!     assert!(progress.is_finite());
//! }
//! assert_eq!(unlock.value(), 1.0);
//! ```

mod config;
mod spring;

pub use config::{Preset, SpringConfig};
pub use spring::{progress_velocity, Spring, MAX_FRAME_DT, REST_DISTANCE, REST_VELOCITY};
