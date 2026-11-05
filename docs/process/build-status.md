# Sphatik OS — Build Status

A living map of the build plan: what is complete, what is in progress, and what
is left. The authoritative per-work-package detail lives in
[`CLAUDE.md`](../../CLAUDE.md) ("Current status"); this page is the high-level
view. Update it at the end of every work package.

**Last updated:** 2026-11-05

## Legend

| Mark | Meaning |
| --- | --- |
| ✅ | Done and merged to `main` |
| 🔄 | In progress |
| ⬜ | Not started |
| 🚩 | Gate (hard checkpoint) |
| ⏸️ | Out of current agreed scope (planned, not yet scheduled) |

## Agreed scope

Work so far is deliberately scoped to **Stage 1 WPs 1.1–1.3** and **Stage 2 WPs
2.1–2.9** — the foundations plus the laptop graphics core. Everything beyond
that (phone bring-up, the toolkit, services, the shell, apps, Android, beta) is
planned but **not yet scheduled**, and is marked ⏸️ below.

All development happens on **Windows**; Linux-only code (compositor, Wayland) is
compiled and verified in CI, and will first run on the phone in Stage 3
(ADR 0008).

## Overall progress

Within the agreed scope (Stage 1: 1.1–1.3, Stage 2: 2.1–2.9) — **12 work
packages**:

- ✅ Complete: **11** (1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.9)
- 🔄 In progress: **1** (2.8)
- 🚩 Remaining gate: **Glass on laptop** (assemble the lock-screen mock-up)

That is **~92%** of the agreed scope done, with WP 2.8 and the end-of-stage gate
to finish.

Plus one side deliverable outside the build plan: the **interactive resume
showcase**, deployed at <https://imsky1812.github.io/sphatikOS/> (PR #12).

---

## Stage 1 — Foundations

Gate: 🚩 **Bring-up done**.

| WP | Deliverable | Status | Where |
| --- | --- | --- | --- |
| 1.1 | Cargo workspace with the five crates | ✅ | `36f7853` |
| 1.2 | CI: fmt, clippy, tests, aarch64 cross-build | ✅ | PR #1 |
| 1.3 | `sp` CLI skeleton (`build`, `logs`, `flash`) | ✅ | PR #2 |
| 1.4 | Kernel/device-tree bring-up (curtana) | ⏸️ | owner / on-device |
| 1.5 | Initramfs + EROFS image, A/B layout | ⏸️ | owner / on-device |
| 1.6 | First boot to a shell on the phone | ⏸️ | owner / on-device |
| 1.7 | On-device logging (`sp logs`) | ⏸️ | owner / on-device |

WPs 1.4–1.7 are device bring-up that the owner runs on the phone; they are
outside the agreed laptop scope and do not gate the Stage 2 laptop work.

---

## Stage 2 — Graphics core on the laptop

Gate: 🚩 **Glass on laptop** — a lock-screen mock-up (wallpaper + clock + one
glass panel) at 60 fps in a window, with swipes that feel like the prototype,
judged in `sphatik-preview` plus a green headless compositor test.

| WP | Deliverable | Status | Where |
| --- | --- | --- | --- |
| 2.1 | Smithay winit backend, event loop, 60 fps frame clock | ✅ | PR #13 |
| 2.2 | GLES renderer: quads, rounded rects, gradients, damage | ✅ | PR #6 |
| 2.3 | Wallpaper engine (Aurora, Obsidian, Dawn, Night) | ✅ | PR #7 |
| 2.4 | Glass v0: backdrop copy, quarter-res dual Kawase | ✅ | PR #8 |
| 2.5 | Glass v1: refraction, tint, rim, specular, dispersion | ✅ | PR #9 |
| 2.6 | Spring engine: 4 presets + velocity hand-off | ✅ | PR #3 |
| 2.7 | Input + gesture recogniser; draggable panel springs back | ✅ | PR #5, #11 |
| 2.8 | Wayland clients: xdg-shell + linux-dmabuf; app full screen | 🔄 | PR #14 |
| 2.9 | Debug overlay: FPS, frame graph, glass cost (F12) | ✅ | PR #10 |

**WP 2.8 (in progress, PR #14):** a minimal Wayland server in `sphatik-comp`
(`wl_compositor`, `wl_shm`, `xdg-shell`, `wl_seat`, `wl_output`,
`zwp_linux_dmabuf_v1`) that composites an external client's window full-screen;
verified by an in-repo Wayland client test that captures the composited frame,
run headless under Xvfb in CI. Live dmabuf is deferred to the phone (Stage 3).

**After 2.8:** assemble the **Glass on laptop** gate — compose wallpaper +
clock + a glass panel into a lock-screen mock-up running at 60 fps in
`sphatik-preview`.

---

## Stages 3–8 — not yet scheduled ⏸️

Planned, outside the current agreed scope. Each dependent stage waits on its
predecessor's gate (gates are hard).

| Stage | Scope | Gate |
| --- | --- | --- |
| 3 | Graphics on the phone (3.1–3.6) | 🚩 Glass on phone |
| 4 | Toolkit (4.1–4.7) + services (4.8–4.13) | — |
| 5 | Shell + phone basics (5.1–5.12) | 🚩 Usable phone |
| 6 | Platform, privacy v1, stock apps, signature v1, keyboard (6.1–6.8) | — |
| 7 | Android layer (7.1–7.7) + security/updates (7.8–7.12) | 🚩 Android apps |
| 8 | Beta + release (8.1–8.6) | 🚩 Public beta |

---

## What's left (near term)

1. **Finish WP 2.8** — get PR #14 green (server compiles and the compositing
   test passes under Xvfb in CI), then merge.
2. **Glass on laptop gate** — build the lock-screen mock-up (wallpaper + clock +
   glass panel, 60 fps, prototype-feel swipes) in `sphatik-preview`, with a
   green headless compositor test. This closes Stage 2.

After the gate, the next agreement with the owner decides whether to continue
into Stage 3 (phone graphics) or pause.
