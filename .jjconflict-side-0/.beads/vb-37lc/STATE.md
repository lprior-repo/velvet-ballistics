# Femdation State 1: Contract

- Bead: `vb-37lc`
- Scope: canonical spelling scan contract and verification planning only.
- Workspace: `/home/lewis/src/vb-37lc`
- Artifact directory: `.beads/vb-37lc/`
- State status: APPROVED after State 1 repair pass.

## Written Artifacts
- `contract.md`
- `lean-contract.md`
- `verification-layers.md`
- `proof-obligations.jsonl`
- `traceability-matrix.jsonl`
- `martin-fowler-tests.md`
- `STATE.md`

## Guardrails Preserved
- No production code written.
- No test code written.
- No proof code or harness code written.
- No bead status changed.
- No commit or push performed.
- Runtime-core exclusions retained: no YAML, JSON, or HTTP in runtime core.
- Rust rules retained: no unsafe, unwrap, expect, panic, todo, unimplemented, dbg, unchecked indexing/slicing/casts/arithmetic in downstream implementation.

## Required Next Gate
- Independent reviewer must write `contract-verification-review.md` with `STATUS: APPROVED` before downstream test planning, test writing, implementation, or formal proof work consumes these artifacts.

## Repair Notes
- Added direct proof-obligation coverage for PRE-002, PRE-003, and PRE-004.
- Added traceability rows for ERR-001 through ERR-007.
- Rewrote Lean and verification waivers with clause IDs, waived layers, reasons, compensating evidence, owners, and follow-up/complete conditions.
- Revalidated JSONL syntax after repair.
