//! The Linux DRM/KMS backend for `sphatik-comp` — the phone display path
//! (WP 3.1): DRM/KMS + GBM + EGL + libinput with a libseat session.
//!
//! This compiles in CI (`--features drm-backend`) but has no DRM device to run
//! against there; its first live run is on the phone (Stage 3 gate). The
//! platform-independent mode selection lives in [`crate::mode`] and is
//! unit-tested. Linux + `drm-backend` only (ADR 0008).
//!
//! Increment 1 (this file): open the primary GPU through libseat, read the
//! connected display's modes, pick one, and find a CRTC for it. Increment 2
//! adds GBM + EGL + the GLES renderer and a `DrmCompositor` that scans out the
//! wallpaper, page-flip paced.

use std::error::Error;

use smithay::backend::drm::{DrmDevice, DrmDeviceFd};
use smithay::backend::session::libseat::LibSeatSession;
use smithay::backend::session::Session;
use smithay::backend::udev::primary_gpu;
use smithay::reexports::drm::control::{connector, crtc, Device as ControlDevice, ModeTypeFlags};
use smithay::reexports::rustix::fs::OFlags;
use smithay::utils::DeviceFd;

use crate::mode::{pick_mode, ModeInfo};

/// Opens the primary GPU and reports the display it would drive. The render
/// path (increment 2) runs on the phone.
pub fn run() -> Result<(), Box<dyn Error>> {
    let (mut session, _notifier) =
        LibSeatSession::new().map_err(|e| format!("libseat session: {e}"))?;
    let seat = session.seat();

    let path = primary_gpu(&seat)
        .map_err(|e| format!("finding the primary GPU: {e}"))?
        .ok_or("no DRM GPU found")?;

    let fd = session
        .open(
            &path,
            OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK,
        )
        .map_err(|e| format!("opening {}: {e}", path.display()))?;
    let fd = DrmDeviceFd::new(DeviceFd::from(fd));
    let (drm, _drm_notifier) = DrmDevice::new(fd, true).map_err(|e| format!("DRM device: {e}"))?;

    let resources = drm
        .resource_handles()
        .map_err(|e| format!("reading DRM resources: {e}"))?;

    for conn_handle in resources.connectors() {
        let Ok(conn) = drm.get_connector(*conn_handle, false) else {
            continue;
        };
        if conn.state() != connector::State::Connected {
            continue;
        }
        let modes: Vec<ModeInfo> = conn
            .modes()
            .iter()
            .map(|m| ModeInfo {
                width: m.size().0,
                height: m.size().1,
                refresh_mhz: m.vrefresh() * 1000,
                preferred: m.mode_type().contains(ModeTypeFlags::PREFERRED),
            })
            .collect();
        let Some(index) = pick_mode(&modes) else {
            continue;
        };
        let mode = conn.modes()[index];
        let crtc = pick_crtc(&drm, &resources, &conn).ok_or("no CRTC available for the display")?;

        println!(
            "drm: {:?} {}x{} @ {} Hz on CRTC {:?}",
            conn.interface(),
            mode.size().0,
            mode.size().1,
            mode.vrefresh(),
            crtc,
        );
        return Ok(());
    }

    Err("no connected display found".into())
}

/// Finds a CRTC that can drive `conn`, via its encoders' possible-CRTC masks.
fn pick_crtc(
    drm: &DrmDevice,
    resources: &smithay::reexports::drm::control::ResourceHandles,
    conn: &connector::Info,
) -> Option<crtc::Handle> {
    for enc_handle in conn.encoders() {
        let Ok(encoder) = drm.get_encoder(*enc_handle) else {
            continue;
        };
        if let Some(crtc) = resources.filter_crtcs(encoder.possible_crtcs()).first() {
            return Some(*crtc);
        }
    }
    None
}
