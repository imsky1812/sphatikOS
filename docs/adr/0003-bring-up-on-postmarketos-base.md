# ADR 0003: Bring up curtana on the postmarketOS base, then replace userspace

- **Status:** Accepted
- **Date:** 2026-09-27

## Context
Booting a mainline kernel on a Qualcomm phone needs device trees, firmware handling and Qualcomm userspace helpers. A community port for the miatoll family already does this.

## Options considered
1. **From scratch:** months of work before the first pixel.
2. **Start from the postmarketOS port:** hardware works on day one; Alpine/musl userspace underneath.

## Decision
Use the postmarketOS miatoll port for kernel, firmware and base packages. Replace the user-facing stack layer by layer: compositor, then sphatikd, then init.

## Consequences
- Fast start; bugs fixed upstream benefit everyone.
- The final image recipe is decided later (open decision in the handbook).
