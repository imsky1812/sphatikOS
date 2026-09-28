# CLAUDE.md: Sphatik OS working memory

Read this first in every session. Keep it current: update "Current status" at the end of every work package.

## Project

Sphatik OS (स्फटिक, "crystal") is a phone OS with a liquid-glass UI. It has its own Wayland compositor, shell, toolkit, services and apps, all written in Rust, plus an Android container with microG. Target: **Redmi Note 9 Pro, India model (curtana)**: Snapdragon 720G (2x A76 + 6x A55), Adreno 618 (Freedreno), 4 GB RAM, 64 GB UFS, 1080 x 2400 60 Hz LCD with a centre punch-hole. Mainline Linux via the postmarketOS miatoll port.

The owner runs everything that touches the phone. We work step by step: plan, then OK, then implement.

## Sources of truth (read in this order when in doubt)

1. `docs/files/Sphatik-OS-Design-Specification.pdf`: product and UI design (36 pages)
2. `docs/files/Sphatik-OS-Engineering-Handbook.pdf`: architecture (24 pages)
3. `docs/files/Sphatik-OS-Build-Plan.pdf`: 8 stages, work packages, gates, first 12 weeks (14 pages)
4. `docs/design/prototype-reference.md`: the pixel-exact extraction of the prototype, **the spec the Rust code must match**, with a list of conflicts (C1–C14)
5. `prototype/index.html`: the interactive prototype (same file as the hosted artifact https://claude.ai/artifact/NMySuxaWV3TseE5PTxNc13)
6. `docs/adr/`, `docs/api/` (`sphatikd-dbus.md`, `sphatik-glass-v1.xml`), `docs/design/hig.md`, `docs/design/brand.md`, `docs/process/test-plan.md`, `docs/process/backlog-milestone-1.md`, `device/xiaomi-curtana/STATUS.md`

To read the PDFs as text: `pdftotext -layout <file> -` (Git Bash). For tables, use PyMuPDF (`import fitz`, `page.find_tables()`); it's installed.

## Performance budgets (hard requirements)

| Metric | Budget |
| --- | --- |
| Frame rate | 60 fps locked, 0 dropped frames in shell transitions (16.6 ms frame, composition target < 8 ms) |
| Glass cost | < 3 ms GPU per frame at 1080 x 2400 |
| Touch to first pixel | < 20 ms |
| Cold boot to lock screen | < 15 s |
| Unlock to usable home | < 300 ms |
| First-party app cold launch | < 500 ms |
| Idle system RAM | ≤ 750 MB (kernel and services 350, compositor and shell 250, keyboard, Intent Bar and background 150) |
| Standby drain | < 1% per hour |

Glass degradation order when over budget: drop caustics, then refraction and dispersion, then halve blur, then cached blur snapshot, then solid tint. The default tier is Balanced: quarter-resolution blur, at most 2 live-blur layers, the rest cached.

## Architecture (six layers)

- **Hardware and kernel:** mainline SM7125 (Freedreno DRM, binderfs, Qualcomm remoteprocs, zram).
- **Base system:** `sphatik-init` (PID 1, TOML service files, cgroup per service), D-Bus, udev, seatd, signed read-only EROFS image with dm-verity, A/B slots (ADR 0006).
- **System services:** `sphatikd` on the system bus (`org.sphatik.Daemon`: Settings, Permissions, Notifications, Apps, LiveActivities, Power; see `docs/api/sphatikd-dbus.md`, via zbus). Upstream plumbing: ModemManager, callaudiod, PipeWire, BlueZ, iwd, Geoclue, iio-sensor-proxy. Container manager (LXC).
- **Compositor + shell in one process:** `sphatik-comp` on Smithay (ADR 0002), with the shell inside (ADR 0007). GLES 3.2 renderer (Vulkan later). Backends: winit window on the laptop, DRM/KMS + GBM + EGL + libinput/libseat on the phone. Damage tracking, direct scan-out for video. Private protocols `sphatik_glass_v1` and `sphatik_shell_v1`. Scale 2.75 gives a 393 pt wide logical canvas (heights reflow: 872.7 pt tall on the device). The compositor recognises edge gestures before apps see them.
- **Toolkit:** `sphatik-ui`: signals with a declarative view tree, taffy layout, cosmic-text + rustybuzz, femtovg for app rendering, AccessKit, Fluent strings, one spring engine (Snappy, Smooth, Bouncy, Gentle), tokens from one theme file.
- **Apps:** 16 stock apps + Terminal, each its own crate; `.spk` = tar.zst of `manifest.toml`, `bin/`, `res/`, Ed25519 `SIGNATURE`. Sandbox: per-app user, namespaces, seccomp, Landlock, portals. Android: LineageOS in LXC, microG, sphatik-bridge APK (ADR 0004).

**Glass pipeline per region:** backdrop copy → quarter-resolution downsample → dual Kawase (2–4 passes) → upsample + SDF refraction (up to 6 px) → adaptive tint (legibility guard 4.5:1) → rim light + gyro specular → 0.5 px chromatic split on the rim → cache if the backdrop is unchanged.

## Repository layout (handbook)

```
docs/            spec PDFs (docs/files), ADRs, API specs, guides, design, process
tools/sp/        sp CLI: build, image, flash, logs, perf
device/xiaomi-curtana/   kernel.config fragment, firmware list, UCM, STATUS.md
system/init/     sphatik-init            system/sphatikd/  sphatikd
compositor/      sphatik-comp + glass renderer
shell/           sphatik-shell: boot, lockscreen, home, library, panels, halo, switcher, intent
ui/              sphatik-ui toolkit      apps/  one crate per app
sdk/             SDK crates, spk packager, templates
android/         container image recipe, sphatik-bridge APK
image/           system image, initramfs, A/B layout
tests/           integration and on-device suites
prototype/       index.html reference prototype
```

Crates required by WP 1.1: `sphatik-comp`, `sphatik-shell`, `sphatik-ui`, `sphatikd`, `sp`. Later crates (for example the spring engine for 2.6) get proposed in their work package plan.

## Coding rules (CONTRIBUTING.md + owner rules)

- Rust stable, edition 2021. No `unsafe` without a `// SAFETY:` comment.
- **No heap allocation in per-frame paths** (compositor, shell, UI). **No panics in system services**: return errors and log them.
- `///` docs on public items. User-facing strings go in Fluent files, never hard-coded. Follow the HIG; text contrast ≥ 4.5:1.
- Before every commit: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
- Conventional Commits (`feat(comp): …`, `fix(shell): …`, `docs(adr): …`); types feat, fix, perf, refactor, docs, test, build, ci, chore. Small commits. `main` is always buildable.
- Tests: unit tests everywhere; **golden-image tests for anything visual**, compared against `docs/design/prototype-reference.md`.
- UI performance: text layout cached, images decoded off-thread, lists virtualised past 50 rows, any frame over 12 ms CPU logged in debug builds.
- Two approvals for `compositor/`, `system/`, `image/` (when there are other maintainers).

## Working rules for Claude

1. **Match the prototype.** If Rust and the prototype differ, the prototype wins unless a doc says otherwise. Flag new conflicts to the owner. Settled policy (C1–C14 in `prototype-reference.md`): the prototype wins for visual layout, sizes and gesture feel; the spec wins for spring constants, preset assignment and UX rules the prototype lacks; the handbook wins for the renderer (GLES).
2. **Follow the ADRs.** Propose a new ADR (`docs/adr/0000-template.md`) before changing an architectural decision.
3. **Per work package:** state the plan in a few lines and wait for OK → implement in small conventional commits → add tests → fmt, clippy `-D warnings`, test, fix everything → tell the owner how to see or run it → update "Current status" below.
4. **Never run anything that flashes, erases or modifies the phone** (fastboot, dd to a device, partition tools). Give the exact command and let the owner run it. Never touch `persist`, `modem` or `efs` partitions.
5. Never ask the owner to paste passwords or tokens into the chat.
6. **No v2 features** until after the public beta: Desktop mode, Glass Lens, Offline Nearby, Private Memory, decoy data, domain firewall, Private Space, split screen.
7. When unsure, ask one clear question instead of guessing.
8. Gates are hard: don't start a dependent stage in earnest until its gate passes.

## Build plan: stages and gates

1. Foundations (1.1–1.7), gate **Bring-up done**
2. Graphics core on the laptop (2.1–2.9), gate **Glass on laptop**: lock-screen mock-up with wallpaper, clock and one glass panel at 60 fps in a window; swipes feel like the prototype
3. Graphics on the phone (3.1–3.6), gate **Glass on phone**
4. Toolkit (4.1–4.7) and services (4.8–4.13), no gate
5. Shell and phone basics (5.1–5.12), gate **Usable phone**
6. Platform, privacy v1, stock apps, signature v1, keyboard (6.1–6.8)
7. Android layer (7.1–7.7) and security and updates (7.8–7.12), gate **Android apps**
8. Beta and release (8.1–8.6), gate **Public beta**

Current scope agreed with the owner: **Stage 1 WPs 1.1–1.3 and Stage 2 WPs 2.1–2.9 only.**

| WP | Deliverable | Done when |
| --- | --- | --- |
| 1.1 | Monorepo + Cargo workspace with empty crates `sphatik-comp`, `sphatik-shell`, `sphatik-ui`, `sphatikd`, `sp` | `cargo build --workspace` passes |
| 1.2 | CI: fmt, clippy, tests, aarch64 cross-build | Green check on a PR |
| 1.3 | `sp` skeleton: `build`, `logs`, `flash` | `sp --help` lists the commands |
| 2.1 | Smithay winit backend, event loop, 60 fps frame clock | Steady frame timer in a window |
| 2.2 | GLES renderer: textured quads, rounded rects, gradients, damage tracking | Wallpaper renders; unchanged frames cost almost nothing |
| 2.3 | Wallpaper engine: Liquid Aurora, Obsidian, crystal art as GPU vector shapes | Matches the prototype side by side |
| 2.4 | Glass v0: backdrop copy, quarter-resolution dual Kawase, upsample | Blurred rounded panel over the wallpaper |
| 2.5 | Glass v1: SDF refraction, tint, rim, specular, dispersion, blur cache | Matches prototype glass; < 3 ms on the laptop |
| 2.6 | Spring engine crate: 4 presets + velocity hand-off | Unit tests match the spec values |
| 2.7 | Input and gesture recogniser: edge zones, swipe up, pull down, interrupt | Panel follows the drag and springs back at 60 fps |
| 2.8 | Wayland clients: xdg-shell, linux-dmabuf; external app full screen | A terminal or test app renders inside |
| 2.9 | Debug overlay: FPS, frame graph, damage flashes, glass cost | Toggle with F12 |

## Environment notes

- Owner's machine: Windows 11; WSL has only `docker-desktop`. Smithay needs Linux.
- **Decision (2026-09-28):** the Stage 2 dev environment is **Ubuntu 24.04 installed natively (dual boot)**, as the build plan requires (the compositor must use the real GPU; a VM or WSL can't give valid GPU timings). Until then, WPs 1.1–1.3 are built on Windows with rustup, and CI (GitHub Actions, Ubuntu) is the Linux reference. Crates that need Linux (Smithay, libinput, DRM) must compile only on Linux targets so the workspace stays checkable on Windows.
- GitHub: https://github.com/imsky1812/sphatikOS (`origin`, branch `main`).
- Shells available: PowerShell and Git Bash. `pdftotext` and Python 3.11 with PyMuPDF are installed.

## Current status

- **Stage:** 1 (Foundations): agreed scope 1.1–1.3 done (1.4–1.7 are device work for the owner). **Next:** Stage 2. WP 2.1 needs native Ubuntu 24.04; proposed next is 2.6 (spring engine), which is platform-independent, while the owner sets up Ubuntu. Plan not yet approved.
- Done: Step 0 (repo on GitHub, commit `3dbcc96`); Step 1 (all docs and the prototype read); Step 2 (this file + `docs/design/prototype-reference.md`).
- **WP 1.1 done** (`36f7853`): Cargo workspace (resolver 2, edition 2021, Apache-2.0) with `sphatik-comp` (bin, `compositor/`), `sphatik-shell` (lib, `shell/`), `sphatik-ui` (lib, `ui/`), `sphatikd` (bin, `system/sphatikd/`, denies unwrap/expect/panic outside tests), `sp` (bin, `tools/sp/`). Workspace lints: `missing_docs` warn, `clippy::undocumented_unsafe_blocks` deny. `rust-toolchain.toml` pins stable + clippy + rustfmt; `rustfmt.toml` forces LF; `.gitattributes` normalises to LF. fmt, clippy `-D warnings` and 5 smoke tests pass.
- **WP 1.2 done** (PR #1, `36e2667`): `.github/workflows/ci.yml` on ubuntu-24.04 for every PR and push to `main`: Format, Clippy (`-D warnings`), Test, and a Cross-build (`cargo zigbuild --workspace --release --target aarch64-unknown-linux-musl`, `file` check, artifact `sphatik-aarch64-musl`). The PR flow: branch → PR with the template → wait for green → merge (squash).
- **WP 1.3 done** (PR #2): `sp` with clap. `sp build [--target host|curtana] [--release] [-p CRATE]` runs cargo or cargo-zigbuild and echoes the command. `sp logs` is a stub until WP 1.7 (exit 2). `sp flash` is **print-only** (never executes): it allows only boot, dtbo, vbmeta and userdata, refuses the `PROTECTED` list (persist, modem, modemst1/2, fsg, fsc, efs, dsp, bluetooth, sec, the bootloader chain) in any case and with any `_a`/`_b` suffix. 18 unit tests.
- Local toolchain: Windows, rustup stable `x86_64-pc-windows-gnu` (Rust 1.98.1), installed in `%USERPROFILE%\.cargo` (on the user PATH). GNU binutils (needed by `windows-sys`) come from **WinLibs** (`winget` package `BrechtSanders.WinLibs.POSIX.MSVCRT`, user scope); its `mingw64\bin` is on the user PATH. In Git Bash, run `export PATH="$HOME/.cargo/bin:$PATH"` if `cargo` isn't found.
- Decided: conflicts C1–C14 (see `prototype-reference.md`), dev environment (native Ubuntu 24.04 dual boot for Stage 2).
