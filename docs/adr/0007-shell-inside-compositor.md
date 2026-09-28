# ADR 0007: The shell runs inside the compositor process

- **Status:** Accepted
- **Date:** 2026-09-27

## Context
Glass panels need the pixels behind them every frame. A separate shell process would need extra buffer copies and cross-process synchronisation.

## Options considered
1. **Shell as a Wayland client:** crash isolation; extra copies for glass and harder frame pacing.
2. **Shell inside the compositor:** direct access to the backdrop; one render pass.

## Decision
Lock screen, home, panels, Halo, switcher and Intent Bar run inside `sphatik-comp`. Apps get glass through the `sphatik_glass_v1` protocol.

## Consequences
- A shell crash restarts the compositor; it must restart in under 1 second and restore the session.
- Shell code is held to compositor-level quality and performance rules.
