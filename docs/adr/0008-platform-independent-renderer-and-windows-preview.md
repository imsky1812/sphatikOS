# ADR 0008: The renderer and shell are platform-independent, with a desktop preview host

- **Status:** Accepted
- **Date:** 2026-09-28
- **Deciders:** project owner

## Context

The build plan runs Stage 2 ("Graphics core on the laptop") inside `sphatik-comp` on a Linux laptop, and asks for Linux installed natively so the compositor uses the real GPU. The owner develops on Windows 11 (HP Victus 15: Ryzen 5 5600H, Radeon RX 6500M, 8 GB RAM) and has decided not to install Linux, natively or in WSL.

Smithay, Wayland, DRM/KMS and libinput only exist on Linux. As designed, the shell and the glass renderer live inside the Smithay compositor, so none of Stage 2 could be seen or felt on the owner's machine.

## Options considered

1. **Dual-boot Ubuntu (the build plan).** Real GPU, the plan as written. Rejected by the owner.
2. **Ubuntu in WSL2 with WSLg.** Runs the real compositor nested, GPU through Mesa's D3D12 layer; unreliable GPU timings. Rejected by the owner.
3. **Develop blind on Windows, verify only in CI and on the phone.** No local window at all; every visual change costs a CI round trip; gesture feel can't be judged before Stage 3.
4. **Make the renderer and shell independent of Smithay, and add a small cross-platform preview host.** The same Rust code draws the UI on Windows on the real RX 6500M; only Linux integration code needs CI or the phone.

## Decision

Option 4.

- **`sphatik-render`** (new crate): the GLES 3 renderer (rounded rectangles, gradients, wallpapers, blur, glass) written against `glow`, with no Smithay types in its API. Shaders use the GLSL ES 3.00 subset, so they run on GLES 3.2 (Freedreno), on desktop OpenGL with `ARB_ES3_compatibility` (Windows), and on Mesa's llvmpipe (CI).
- **`sphatik-shell`** and **`sphatik-motion`** stay free of platform code: screens, gesture recognition and springs are plain Rust.
- **`sphatik-preview`** (new crate, developer tool): a winit + glutin window that hosts the real shell and renderer on Windows, macOS or Linux, with mouse-as-touch and the F12 debug overlay. It is not shipped on the phone.
- **`sphatik-comp`** remains the only product host: Smithay on Linux, handing its EGL/GLES context to `sphatik-render` through `glow`. Its Linux-only dependencies compile only on Linux targets.
- ADRs 0002 (Smithay) and 0007 (shell inside the compositor) still hold: on the phone the shell still runs inside the compositor, in one process, with direct access to the backdrop.

### Stage 2 on this basis

| WP | Where it is built and checked |
| --- | --- |
| 2.2 renderer, 2.3 wallpapers, 2.4/2.5 glass, 2.7 gestures, 2.9 overlay | On Windows in `sphatik-preview`; golden images in CI on llvmpipe |
| 2.6 springs | Anywhere (done) |
| 2.1 Smithay winit backend | Written on Windows; built and run headless in CI; first live run on the phone (Stage 3) |
| 2.8 Wayland clients | Written on Windows; headless compositor test with a Wayland test client in CI, screenshot as a golden image |

The **Glass on laptop** gate is judged in `sphatik-preview` (60 fps, prototype-matching glass and swipes), plus a green headless compositor test in CI.

## Consequences

- The owner sees and feels the real UI on Windows, drawn on a real GPU, which gives better laptop timings than WSL.
- The renderer needs a thin abstraction over the GL context; Smithay integration happens through `glow` rather than Smithay's own `GlesRenderer` drawing calls.
- Desktop GL on Windows and GLES on Freedreno can differ: CI renders every golden image on Mesa's GLES driver, and Stage 3 checks on the Adreno 618.
- Linux-only code (Smithay backends, Wayland protocol handling, DRM) can't run on the owner's machine: its bugs surface in CI or on the phone.
- Device work that needs Linux tools (building the postmarketOS base image with `pmbootstrap` in WP 1.5, image assembly later) will run in CI or in a container under Docker Desktop, which the owner already has.
- The engineering handbook's "Desktop first" rule becomes "desktop first, on any OS": every shell component must run in `sphatik-preview` before it runs on the phone.
