# Landing Report — vb-jpq7 wave1 proof

Date: 2026-05-23

## VCS Diagnosis

- Requested workspace: `/home/lewis/src/vb-jpq7-wave1-proof`.
- `jj status` there failed with `No working copy` / missing working-copy commit.
- The shared JJ repo metadata at `/home/lewis/src/velvet-ballistics` was intact and showed the approved work on `main`.
- Landing proceeded from `/home/lewis/src/velvet-ballistics`, the source-of-truth repository where beads and remotes are available.

## Included Changes

- Existing approved commit: `e81c38e94` (`chore: final sweep - remaining test artifacts, TLA+ specs, dependency updates, verification ledger`).
- Added this landing report.
- Excluded generated Apalache `_apalache-out` run files and ignored future `_apalache-out` directories.

## Gate Evidence

- Prior final gate evidence supplied for this landing: `moon ci` PASS, 29 tasks completed, 11531/11531 tests passed.
- Proof review: PASS.
- Bridge review: PASS.
- Evidence packaging: PASS.

## Artifact Exclusions

- Excluded generated/cache artifacts: `target/`, `.moon/cache/`, `.beads/dolt/`, `.beads/backup/`, `.beads/embeddeddolt/`, and `verification/tla/**/_apalache-out/`.

## Push Evidence

- See final assistant response for exact push, beads, and final-status command outcomes.
