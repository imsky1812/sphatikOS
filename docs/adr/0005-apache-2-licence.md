# ADR 0005: Sphatik code is licensed Apache-2.0

- **Status:** Accepted
- **Date:** 2026-09-27

## Context
We want app developers and companies to adopt the SDK freely, and to protect contributors against patent claims.

## Options considered
1. **GPL-3.0:** strong copyleft; discourages proprietary apps that link the SDK.
2. **MIT:** simple; no explicit patent grant.
3. **Apache-2.0:** permissive with a patent grant; fine to ship alongside GPL components.

## Decision
Apache-2.0 for all Sphatik code.

## Consequences
- We still meet GPL obligations for the kernel and other GPL components we ship (source offer per release).
