# Contract: vb-9ucmq

**Bead:** vb-9ucmq - Repair actual canonical supply-chain advisories with patched compatible dependencies
**Parent:** vb-2lyiw
**Status:** IN_PROGRESS

## Problem

Canonical CI `cargo audit` fails with 2 current advisories:
- RUSTSEC-2026-0204: crossbeam-epoch 0.9.18 (patched >=0.9.20)
- RUSTSEC-2026-0274: rtrb 0.3.4 (patched >=0.3.5)

## Scope

Minimal compatible dependency updates only:
- crossbeam-epoch: 0.9.18 → 0.9.20 (locked via cargo update)
- rtrb: 0.3.4 → 0.3.5 (locked via cargo update)
- No unrelated dependency updates
- No advisory ignores/allowance weakening

## Constraints

- Workspace root: `/home/lewis/src/isoloated/velvet-ballistics-vb-9ucmq-local3090-deps`
- rtrb constraint `0.3` in workspace Cargo.toml allows 0.3.5
- crossbeam-epoch is transitively required (not direct workspace dependency)
- Must preserve existing locked versions of all other dependencies

## Verification

- `cargo audit` reports 0 vulnerabilities (warnings allowed)
- Scoped consumer tests pass (rtrb: vb_runtime tests; crossbeam-epoch: vb_storage/runtime)
- Lockfile diff shows only targeted version changes
- No source feature changes

## Acceptance

- Cargo.lock updated with patched versions
- No new advisories introduced
- Behavior tests pass for affected consumers
- Evidence captured in .beads/vb-9ucmq/evidence/