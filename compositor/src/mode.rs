//! Display-mode selection for the DRM backend (WP 3.1).
//!
//! The picking logic is pure and platform-independent so it is unit-tested
//! everywhere; the Linux DRM backend feeds it the connector's modes.
// Used by the Linux `drm` backend and the tests, so it reads as unused on the
// Windows stub and the winit-only build.
#![allow(dead_code)]

/// A display mode, as read from a DRM connector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModeInfo {
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// Refresh rate in millihertz (e.g. 60000 for 60 Hz).
    pub refresh_mhz: u32,
    /// The connector marks this as its preferred mode.
    pub preferred: bool,
}

impl ModeInfo {
    /// Pixel area, for comparing resolutions.
    fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }
}

/// Picks the best mode for a panel: the connector's preferred mode if it has
/// one, else the largest resolution, breaking ties by the higher refresh.
/// Returns the index into `modes`, or `None` if there are none. The phone's
/// panel reports a single fixed mode, so this almost always returns the
/// preferred one; the fallback matters for external displays later.
pub fn pick_mode(modes: &[ModeInfo]) -> Option<usize> {
    if modes.is_empty() {
        return None;
    }
    if let Some(i) = modes.iter().position(|m| m.preferred) {
        return Some(i);
    }
    let mut best = 0;
    for (i, m) in modes.iter().enumerate().skip(1) {
        let b = &modes[best];
        if m.area() > b.area() || (m.area() == b.area() && m.refresh_mhz > b.refresh_mhz) {
            best = i;
        }
    }
    Some(best)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode(w: u16, h: u16, hz: u32, pref: bool) -> ModeInfo {
        ModeInfo {
            width: w,
            height: h,
            refresh_mhz: hz * 1000,
            preferred: pref,
        }
    }

    #[test]
    fn empty_has_no_mode() {
        assert_eq!(pick_mode(&[]), None);
    }

    #[test]
    fn a_preferred_mode_wins_even_if_smaller() {
        let modes = [mode(1920, 1080, 60, false), mode(1080, 2400, 60, true)];
        assert_eq!(pick_mode(&modes), Some(1));
    }

    #[test]
    fn without_a_preferred_the_largest_wins() {
        let modes = [mode(720, 1600, 60, false), mode(1080, 2400, 60, false)];
        assert_eq!(pick_mode(&modes), Some(1));
    }

    #[test]
    fn equal_area_breaks_on_refresh() {
        let modes = [mode(1080, 2400, 60, false), mode(1080, 2400, 90, false)];
        assert_eq!(pick_mode(&modes), Some(1));
    }

    #[test]
    fn the_curtana_panel_mode_is_chosen() {
        // The Redmi Note 9 Pro reports one preferred 1080x2400 60 Hz mode.
        let modes = [mode(1080, 2400, 60, true)];
        assert_eq!(pick_mode(&modes), Some(0));
    }
}
