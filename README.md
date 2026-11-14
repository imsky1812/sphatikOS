# Sphatik OS

**स्फटिक (Sphatik), Sanskrit for crystal.** A phone operating system built from the ground up, with a liquid-glass interface, its own compositor and apps written in Rust, and support for Android apps through a compatibility container with microG.

> Status: **pre-alpha.** The glass shell (wallpaper, lock screen and home mock-up) runs at a locked 60 fps on the desktop in a preview window — this milestone is done. Nothing runs on a phone yet. See the roadmap below.

## What makes it different

- **Liquid glass UI.** Every system surface is rendered as refracting glass by our own GPU compositor.
- **The Halo.** Live activities (calls, timers, music, navigation) flow around the camera cutout.
- **Intent Bar.** One box to find apps, settings and answers, and later to act across apps.
- **Open by default.** Real file system, default-app choice, sideloading, no lock-in.
- **Private by default.** No telemetry. Permissions per use, visible indicators, a 7-day access log.
- **Android apps.** WhatsApp, Instagram, YouTube and most Play Store apps run in an isolated Android container with microG. No Google software is shipped.

## Target device

Redmi Note 9 Pro, India model (codename `curtana`, Snapdragon 720G, 4 GB RAM). Built on the mainline Linux kernel via the community postmarketOS port for the miatoll family.

## Repository layout

| Path | Contents |
| --- | --- |
| `docs/` | Handbook pages, ADRs, API specs, guides, design and process docs |
| `tools/sp/` | The `sp` command-line tool: build, image, flash, logs, perf |
| `device/xiaomi-curtana/` | Kernel config fragment, firmware list, audio configs, hardware status |
| `system/` | `sphatik-init` (PID 1) and `sphatikd` (system services) |
| `compositor/` | `sphatik-comp`, the Wayland compositor and glass renderer |
| `shell/` | Lock screen, home, App Library, panels, Halo, switcher, Intent Bar |
| `ui/` | `sphatik-ui`, the app toolkit |
| `apps/` | The 16 stock apps |
| `sdk/` | App SDK and the `.spk` packager |
| `android/` | Android container image recipe and the `sphatik-bridge` APK |
| `image/` | System image, initramfs and A/B layout |

## Quick start (developers)

All desktop development runs through `sphatik-preview`, which hosts the renderer and shell in a window on Windows, macOS or Linux (see [ADR 0008](docs/adr/)).

```bash
git clone https://github.com/imsky1812/sphatikOS && cd sphatikOS
cargo run -p sphatik-preview        # wallpaper + glass shell in a window
```

Swipe up (drag with the mouse) to unlock into the home screen; press `1`–`4` to switch wallpapers, `F12` for the debug overlay. The Linux-only Wayland compositor (`sphatik-comp`) is built and tested headless in CI and runs for real on the phone in Stage 3.

Full setup: [docs/guides/dev-setup.md](docs/guides/dev-setup.md).

## Roadmap

| Phase | Gate | Status |
| --- | --- | --- |
| 0. Toolchain and workspace | Cargo workspace builds, CI green, `sp` tool skeleton | ✅ done |
| 1. Compositor on laptop | Glass shell at 60 fps in a window | ✅ done |
| 2. Compositor on the phone | 60 fps full screen on curtana, touch working | 🔲 in progress (device-blocked) |
| 3. Core shell and sphatikd | Lock, home, panels, Halo, notifications | 🔲 |
| 4. Phone basics | A full week of daily use: calls, SMS, data, audio, camera | 🔲 |
| 5. Stock apps and SDK | 16 apps, `.spk` packaging | 🔲 |
| 6. Android layer and microG | WhatsApp works with notifications | 🔲 |
| 7. Security, updates, own init | Signed A/B updates, encrypted storage | 🔲 |
| 8. Public beta | All performance budgets met | 🔲 |

### Where it stands today

**Built so far**
- Design specification, engineering handbook and ADRs ([`docs/`](docs/))
- Interactive shell prototype you can open in a browser ([`prototype/index.html`](prototype/index.html))
- Rust workspace: compositor (Smithay), renderer, shell, UI toolkit, `sphatikd` and the `sp` build tool
- The glass shell — wallpapers, lock screen and home mock-up — rendering at 60 fps in `sphatik-preview`
- Device config for the Redmi Note 9 Pro ([`device/xiaomi-curtana/`](device/xiaomi-curtana/))

**Not built yet**
- Booting and running on the phone
- Stock apps and the SDK
- The Android compatibility layer

## Documentation

Start at [docs/README.md](docs/README.md). The design spec, engineering handbook and build plan are listed in [docs/claude-docs.md](docs/claude-docs.md).

## Prototype

Open [`prototype/index.html`](prototype/index.html) in any browser for the interactive shell prototype: boot, lock screen, home, Control Center, the Halo, all stock apps and the signature features.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) and the [Code of Conduct](CODE_OF_CONDUCT.md). Security issues: [SECURITY.md](SECURITY.md).

## License

Apache-2.0 for all Sphatik code. Third-party components keep their own licences; see `NOTICE` and the per-release source page.
