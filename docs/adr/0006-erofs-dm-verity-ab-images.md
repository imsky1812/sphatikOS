# ADR 0006: Read-only EROFS system images, verified with dm-verity, in A/B slots

- **Status:** Accepted
- **Date:** 2026-09-27

## Context
Updates must be atomic and recoverable, and system files must not be silently modified.

## Options considered
1. **Package-manager updates in place:** flexible; a failed update can leave a broken phone.
2. **OSTree:** atomic; more moving parts.
3. **Whole-image A/B with EROFS and dm-verity:** simple, atomic, verifiable; needs two slots of storage.

## Decision
Two EROFS system images on userdata, verified by the initramfs with dm-verity against Sphatik's signing key, with a boot counter and automatic rollback.

## Consequences
- About 6 GB of the 64 GB storage goes to two system slots.
- Delta updates are needed to keep downloads small.
