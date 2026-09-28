//! `sphatik-motion`: the one spring engine shared by the shell and every app.
//!
//! Every transition in Sphatik is a spring driven by the finger's velocity
//! (design spec, "Motion system and global gestures"). This crate holds the
//! four presets and, from the next commit, the solver ported from the
//! prototype's `run()` (see `docs/design/prototype-reference.md`, section 6).
//!
//! Everything here is plain `Copy` data: nothing allocates, so springs are
//! safe to step in per-frame paths.

mod config;

pub use config::{Preset, SpringConfig};
