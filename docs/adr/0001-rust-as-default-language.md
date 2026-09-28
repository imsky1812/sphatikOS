# ADR 0001: Rust is the default language for Sphatik code

- **Status:** Accepted
- **Date:** 2026-09-27

## Context
Sphatik writes a compositor, system services, a UI toolkit and apps. A small team cannot afford memory-safety bugs in privileged code, and needs good performance on a Snapdragon 720G.

## Options considered
1. **C/C++:** mature graphics ecosystem; high risk of memory bugs; slower iteration.
2. **Rust:** memory safety without a garbage collector; strong crates for Wayland (Smithay), D-Bus (zbus), layout (taffy) and text (cosmic-text).
3. **A managed language (Kotlin, Dart):** fast UI iteration; garbage-collector pauses and higher RAM use on 4 GB.

## Decision
Rust for everything Sphatik writes. C only where an upstream API forces it, wrapped behind safe interfaces.

## Consequences
- Longer compile times; mitigated by a workspace, incremental builds and `sccache`.
- App developers need Rust for native apps; Android apps remain available for everything else.
