# Contributing to Sphatik OS

Thanks for helping build Sphatik. This guide covers how work flows from an idea to `main`.

## Before you start

1. Read the [engineering handbook](docs/README.md) sections for the area you want to touch.
2. For anything larger than a bug fix, open an issue first and describe the change. Architectural changes need an ADR (see `docs/adr/0000-template.md`).
3. Set up your machine with [docs/guides/dev-setup.md](docs/guides/dev-setup.md).

## Workflow

1. Fork, then create a short-lived branch: `feat/halo-nav-activity`, `fix/lock-clock-weight`.
2. Keep pull requests small and focused. One logical change per PR.
3. Make sure these pass locally:
   ```bash
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
4. Open the PR using the template. Include screenshots or a screen recording for any UI change.
5. A maintainer reviews within a few days. Two approvals are needed for changes to `compositor/`, `system/` or `image/`.

## Commit messages

Conventional Commits:

```
feat(shell): add navigation live activity to the Halo
fix(comp): drop frames no longer stall glass cache
docs(adr): record choice of EROFS for the system image
```

Types: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `ci`, `chore`.

## Code standards

- **Rust stable**, edition 2021, no `unsafe` without a `// SAFETY:` comment explaining why it is sound.
- No panics in system services; return errors and log them.
- No heap allocation in per-frame paths of the compositor or UI.
- Public items documented with `///` comments.
- User-facing strings go through Fluent files, never hard-coded.
- UI must follow the [Human Interface Guidelines](docs/design/hig.md) and meet 4.5:1 text contrast.

## Performance budgets

A change that breaks any budget in [docs/process/test-plan.md](docs/process/test-plan.md) on the nightly device run is reverted or fixed the same day.

## Device work

Never push changes that flash or erase `persist`, `modem` or other calibration partitions. Hardware results go in `device/xiaomi-curtana/STATUS.md`.

## Licence

By contributing you agree your work is licensed under Apache-2.0.
