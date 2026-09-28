# Security Policy

## Reporting a vulnerability

Email **security@sphatik.dev** (placeholder; set before launch) with a description, affected version and steps to reproduce. Please do not open a public issue.

- We acknowledge reports within **72 hours**.
- Critical issues are fixed in an out-of-band update within **14 days**.
- We credit reporters in release notes unless you prefer otherwise.

## Supported versions

| Channel | Security fixes |
| --- | --- |
| Stable (latest month) | Yes |
| Beta | Yes |
| Nightly | Best effort |

## Known platform limitations

These are inherent to running on the Redmi Note 9 Pro with a mainline kernel and are documented for users:

- The bootloader stays unlocked; a physical attacker could replace the boot image.
- No hardware-backed key storage; PIN attempts are rate-limited in software.
- No NFC payments or secure element.
- Fingerprint unlock unavailable until the mainline driver works.

## Scope

In scope: everything in this repository and official Sphatik images. Out of scope: third-party Android apps, Aurora Store, F-Droid, microG upstream (report to their projects).
