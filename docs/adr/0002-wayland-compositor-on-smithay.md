# ADR 0002: The compositor is a Wayland compositor built on Smithay

- **Status:** Accepted
- **Date:** 2026-09-27

## Context
The compositor owns the display, input and glass rendering. Apps and the Android container need a window protocol.

## Options considered
1. **Custom protocol:** total freedom; every app, toolkit and the Android container would need custom support.
2. **Wayland on wlroots (C):** proven; C codebase and bindings friction.
3. **Wayland on Smithay (Rust):** native Rust building blocks for DRM/KMS, libinput and a GLES renderer, used in production by other compositors.

## Decision
Wayland, implemented with Smithay. Sphatik-specific features ship as private protocols (`sphatik_glass_v1`, `sphatik_shell_v1`).

## Consequences
- Android container windows work through standard Wayland.
- We track Smithay releases and contribute fixes upstream.
