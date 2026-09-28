# Development setup

Get from a fresh Linux machine to the Sphatik compositor running in a window in about 30 minutes.

## 1. Requirements

- Linux on x86_64 (Ubuntu 24.04 or Arch tested), 16 GB RAM recommended, 100 GB free disk.
- For device work: the Redmi Note 9 Pro (curtana) with an unlocked bootloader and a good USB cable.

## 2. System packages (Ubuntu 24.04)

```bash
sudo apt update
sudo apt install -y build-essential pkg-config git curl clang \
  libudev-dev libinput-dev libseat-dev libxkbcommon-dev libdrm-dev libgbm-dev \
  libegl1-mesa-dev libgles2-mesa-dev libwayland-dev libdbus-1-dev \
  adb fastboot qemu-user-static binfmt-support pipx
```

On Arch, install the equivalent packages (`base-devel libinput seatd libxkbcommon mesa wayland dbus android-tools qemu-user-static`).

## 3. Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
rustup target add aarch64-unknown-linux-musl
rustup component add clippy rustfmt
cargo install cargo-zigbuild sccache
pipx install ziglang        # linker used by cargo-zigbuild
```

Add `export RUSTC_WRAPPER=sccache` to your shell profile to cache builds.

## 4. Clone and run

```bash
git clone https://github.com/<your-username>/sphatikOS && cd sphatikOS
cargo run -p sphatik-comp -- --windowed
```

A window opens with the SKY boot splash, then the lock screen. Mouse drags act as touch; `Esc` goes home; `F12` toggles the debug overlay.

## 5. Cross-compile for the phone

```bash
cargo zigbuild --release --target aarch64-unknown-linux-musl -p sphatik-comp
# or, once the sp tool exists:
sp build --target curtana
```

## 6. Device tools

- Install `pmbootstrap` from your distribution or its git repository, following the postmarketOS wiki, and run `pmbootstrap init` choosing vendor `xiaomi`, device `miatoll`.
- Check the phone is visible in fastboot mode: hold Volume Down + Power, then `fastboot devices`.
- Device procedures (unlocking, flashing, recovery) are in [install-guide.md](install-guide.md) and the engineering handbook.

## 7. Editor

Any editor with rust-analyzer. Recommended settings: format on save, clippy as the check command.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| `failed to open EGL display` in windowed mode | Install Mesa EGL packages; on NVIDIA use a recent driver with GBM support |
| Linker errors when cross-compiling | Use `cargo zigbuild`, not `cargo build`, for the aarch64 target |
| `fastboot devices` shows nothing | Try another USB port or cable; add the udev rules from the `android-sdk-platform-tools-common` package |
