# Xiaomi Redmi Note 9 Pro (curtana) hardware status

> **Device note (2026-11-08):** the owner now physically has a **Redmi 9A
> (`dandelion`)**, not the Note 9 Pro. The 9A stays the *phone on hand*, not the
> architecture's design target — curtana remains the target. Reason: the 9A's
> **PowerVR GE8320** GPU has no working open GLES driver on mainline Linux
> (Mesa's open PowerVR effort targets newer parts; this Rogue-era GPU needs the
> proprietary blob on an Android vendor kernel), so the mainline + 60 fps glass
> pipeline cannot run on it. It is also weaker (Helio G25, 2–3 GB RAM,
> 720×1600) and its postmarketOS port is early WIP. On the 9A the glass UI would
> mean Halium/Android-kernel or a reduced software-rendered build — a separate
> track. The device choice is revisited at **Stage 3** (graphics on the phone);
> until then nothing on-device is blocked, since Stages 1–2 are
> platform-independent (ADR 0008). See the ADR to be written if we pivot.

Tested image: `________`  ·  Kernel: `________`  ·  Date: `____-__-__`  ·  Tester: `________`

Legend: ✅ works · ⚠️ partial · ❌ broken · ⬜ not tested

| # | Component | Status | Notes |
| --- | --- | --- | --- |
| 1 | Display (60 Hz, brightness) | ⬜ | |
| 2 | Touch (10 points) | ⬜ | |
| 3 | GPU (Adreno 618, Freedreno) | ⬜ | glmark2-es2 score: |
| 4 | Wi-Fi | ⬜ | |
| 5 | Bluetooth | ⬜ | |
| 6 | Mobile data (SIM 1 / SIM 2) | ⬜ | |
| 7 | Voice calls (audio both ways) | ⬜ | |
| 8 | SMS | ⬜ | |
| 9 | Speaker / earpiece / headphones / mic | ⬜ | |
| 10 | Accelerometer, gyroscope | ⬜ | |
| 11 | Proximity, ambient light | ⬜ | |
| 12 | Rear camera / front camera | ⬜ | Expected to need the community patch |
| 13 | Battery gauge, charging | ⬜ | Ignore first ~10 s readings |
| 14 | Suspend / resume | ⬜ | |
| 15 | Fingerprint (side key) | ⬜ | Expected ❌ on mainline |
| 16 | NFC | ⬜ | Expected ❌ |
| 17 | Hardware video decode | ⬜ | Expected ❌ |
| 18 | GPS | ⬜ | |
| 19 | USB networking (SSH) | ⬜ | |
| 20 | Vibration motor | ⬜ | |

## Baselines

| Metric | Value |
| --- | --- |
| Cold boot to UI | |
| Idle RAM (base image) | |
| Standby drain per hour | |
