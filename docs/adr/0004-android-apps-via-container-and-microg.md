# ADR 0004: Android apps run in an LXC container with LineageOS and microG

- **Status:** Accepted
- **Date:** 2026-09-27

## Context
Users need WhatsApp, Instagram, YouTube and other Play Store apps. Google Play Services and the Play Store are licensed only to certified manufacturers.

## Options considered
1. **No Android apps:** unacceptable for daily use.
2. **Emulation:** too slow on a Snapdragon 720G.
3. **Container sharing the host kernel (Waydroid approach):** native speed; needs binderfs and a Wayland compositor.

## Decision
Run a LineageOS-based Android image in an LXC container, with microG replacing Play Services and Aurora Store (installed by the user) for Play Store apps. Integrate icons, notifications, clipboard and files through a bridge APK.

## Consequences
- Banking and UPI apps that require Play Integrity will likely not run; users are told up front.
- About 800 MB to 1 GB RAM while running; the container is frozen when idle.
- Sphatik never ships Google software.
