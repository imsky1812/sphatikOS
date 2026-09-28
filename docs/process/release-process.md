# Release process

## Cadence

| Channel | Source | Cadence | Signed with |
| --- | --- | --- | --- |
| Nightly | `main` | Every night | Nightly key (CI secret) |
| Beta | Release branch | Every 2 weeks | Release key |
| Stable | Release branch | Monthly + security fixes | Release key |

Version format: `YEAR.MONTH.PATCH`, for example `2027.03.1`.

## Cutting a release

1. **Branch:** `git switch -c release/2027.03 main` on the first Monday of the month.
2. **Freeze:** only fixes land on the release branch; each fix also lands on `main`.
3. **Beta build:** CI builds the image; the release manager tags `2027.03.0-beta.1`.
4. **Test:** run the full test plan exit criteria (see `test-plan.md`).
5. **Changelog:** update `CHANGELOG.md` with Added, Changed, Fixed and Security sections, written for users.
6. **Sign offline:**
   - Copy the unsigned image and manifest to the offline release machine.
   - Sign with the release key on the hardware security key.
   - Copy the signed artefacts back; verify signatures on a second machine.
7. **Publish:** upload images, delta updates, manifest, checksums, SBOM and the GPL source bundle.
8. **Staged rollout:** 10% of devices for 48 hours; watch update success and crash reports voluntarily shared by testers; then 100%.
9. **Tag:** `git tag -s 2027.03.1` and push.

## Hotfixes

Security or S1 fixes: branch from the latest stable tag, fix, run the device checklist and budgets, sign, and roll out to 100% immediately. Target: within 14 days of a critical report.

## Rollback

If a release causes widespread boot failures, devices roll back automatically after three failed boots. The release manager also pulls the manifest so no further devices update, then publishes a fixed patch release.

## Key custody

- Release and Store keys live only on hardware security keys, with a sealed backup copy stored separately.
- Two maintainers must be present (in person or on a call) for every signing session.
- Key compromise procedure: revoke via a pre-signed revocation update, rotate keys, publish an advisory.
