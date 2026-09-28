# Backlog: Milestone 1 (first four weeks)

Goal: a repository that builds, a compositor that shows the Liquid Aurora wallpaper and one real glass panel in a laptop window, and a Redmi Note 9 Pro running the postmarketOS base with its hardware checklist filled in.

Estimates assume part-time work (about 15 hours a week). 1 point = about 2 hours.

## Track A: Repository and tooling

| ID | Task | Points | Done when |
| --- | --- | --- | --- |
| A1 | Create GitHub organisation and `sphatik` monorepo; add these docs | 1 | Repo public or private, README renders |
| A2 | Cargo workspace with empty crates: `sphatik-comp`, `sphatik-shell`, `sphatik-ui`, `sphatikd`, `sp` | 2 | `cargo build --workspace` passes |
| A3 | CI: fmt, clippy, test, aarch64 cross-build | 3 | Green check on a PR |
| A4 | `sp` CLI skeleton with `build` and `logs` subcommands | 3 | `sp --help` lists commands |
| A5 | Issue and PR templates, labels, milestones | 1 | Templates appear on new issues |

## Track B: Compositor on the laptop

| ID | Task | Points | Done when |
| --- | --- | --- | --- |
| B1 | Smithay winit backend: open a window, clear to a colour at 60 fps | 4 | Window with steady frame timer |
| B2 | Render the Liquid Aurora wallpaper (port the prototype's vector art to a texture) | 4 | Wallpaper matches the prototype |
| B3 | Accept Wayland clients; show a test client (e.g. a terminal) full screen | 5 | External app renders inside |
| B4 | Glass pass v0: copy backdrop, downsample, dual Kawase blur, upsample | 6 | One blurred rounded panel over the wallpaper |
| B5 | Glass pass v1: SDF refraction, tint, rim light | 5 | Panel matches prototype glass side by side |
| B6 | Spring engine crate with the four presets and tests | 3 | Unit tests match the design spec values |
| B7 | Mouse-as-touch input and a swipe-up gesture that animates the panel | 3 | Panel follows the drag and springs back |
| B8 | Frame-time overlay (F12) | 2 | FPS and frame graph visible |

## Track C: Device bring-up (in parallel)

| ID | Task | Points | Done when |
| --- | --- | --- | --- |
| C1 | Back up the phone; flash the latest stock fastboot ROM | 1 | Phone on current MIUI firmware |
| C2 | Request bootloader unlock; wait out the period | 1 | Bootloader unlocked |
| C3 | `pmbootstrap init` for miatoll; build and flash the base image with Phosh | 3 | Phone boots to Phosh |
| C4 | Run the hardware checklist; write `STATUS.md` | 3 | Every row has a result |
| C5 | SSH over USB; measure idle RAM and boot time as a baseline | 2 | Numbers recorded |
| C6 | Run `glmark2-es2` and a simple Wayland client to confirm GPU and display path | 2 | Scores recorded |

**Total:** about 54 points, roughly four weeks part-time.

## Definition of done for the milestone

- [ ] CI green on `main`.
- [ ] Laptop window shows wallpaper plus a glass panel that follows a swipe at 60 fps.
- [ ] Phone runs the base image; `STATUS.md` complete; baseline RAM and boot time recorded.
- [ ] Short demo video recorded for the README.
