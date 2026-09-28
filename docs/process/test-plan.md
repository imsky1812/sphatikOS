# Test plan

## Test layers

| Layer | Covers | Runs | Owner |
| --- | --- | --- | --- |
| Unit | Logic in every crate | Every PR, laptop and CI | Author of the change |
| Component (golden images) | Each UI component in light, dark, 100% and 310% text | Every PR, software rendering | UI toolkit maintainers |
| Compositor (headless) | Gestures, panels, switcher, glass regions, protocol conformance | Every PR | Compositor maintainers |
| UI flows | Unlock, open app, send message, start timer, add widget, customise lock screen | Every PR (headless), nightly (device) | Shell maintainers |
| Device | Hardware checklist, suspend/resume, boot and rollback | Nightly on the device runner | Device maintainers |
| Soak | 8 h standby, 1 h interaction loop, memory growth | Weekly | Release manager |
| Android layer | Container start, WhatsApp install via Aurora, notification bridge, freeze/thaw | Nightly once Phase 6 starts | Android maintainers |

## Performance budgets (Redmi Note 9 Pro)

| Metric | Budget |
| --- | --- |
| Dropped frames during shell transitions | 0 at 60 Hz |
| Touch to first pixel | < 20 ms |
| Glass cost per frame | < 3 ms GPU |
| Cold boot to lock screen | < 15 s |
| Unlock to usable home | < 300 ms |
| First-party app cold launch | < 500 ms |
| Idle system RAM | ≤ 750 MB |
| Standby drain | < 1% per hour |

## Device hardware checklist

Run on every nightly image and before every release. Record results in `device/xiaomi-curtana/STATUS.md`.

| # | Check | Pass criterion |
| --- | --- | --- |
| 1 | Display | 60 Hz, correct orientation, brightness control |
| 2 | Touch | 10 points, no ghost touches, edges reachable |
| 3 | GPU | `glmark2-es2` completes; compositor at 60 fps |
| 4 | Wi-Fi | Scan, connect WPA2/WPA3, 30 min stable |
| 5 | Mobile data | 4G data with SIM 1 and SIM 2 |
| 6 | Calls | Outgoing and incoming, audio both ways, earpiece and speaker |
| 7 | SMS | Send and receive, including OTP |
| 8 | Audio | Speaker, headphones, microphone, Bluetooth headset |
| 9 | Sensors | Accelerometer (rotation, tilt light), proximity during calls |
| 10 | Camera | Preview and capture (once the patch is merged) |
| 11 | Battery | Correct percentage after 10 s, charging detected |
| 12 | Suspend | Screen off suspends; wakes for call, alarm, power button |
| 13 | Boot | Cold boot under 15 s; rollback after three failed boots |

## Bug severity

| Severity | Examples | Response |
| --- | --- | --- |
| S1 critical | Data loss, cannot call emergency numbers, boot loop, security hole | Stop the release; fix immediately |
| S2 major | Calls drop, notifications lost, app crash on launch | Fix before the next release |
| S3 minor | Visual glitch, wrong string, slow animation | Scheduled |
| S4 cosmetic | Spacing off by a few points | When convenient |

## Release exit criteria

No open S1 or S2 bugs, device checklist fully passed, all budgets met on two consecutive nightly runs, 8-hour soak passed.
