# Waiver Candidates — vb-t6hx (Reduced Scope)

## WC-vb-t6hx-001 — Non-Behavior Supply-Chain Attestation

- **Status**: Candidate (carried from prior plan)
- **Requirement**: `REQ-10` / Non-Functional Contract
- **Why candidate**: If State 11 makes no `Cargo.toml`/`Cargo.lock`/dependency-source changes, deep supply-chain attestation beyond canonical `moon ci` is excessive for a CLI test-first bead.
- **Behavior affecting**: No. This waives only deep dependency attestation, not runtime/core boundary behavior, source checks, tests, verifier obligations, or `moon ci`.
- **Boundary**: Valid only if `git diff` for implementation contains no Cargo.toml/Cargo.lock dependency changes AND source checks show doctor storage types/formatting remain outside `vb_core`, `vb_runtime`, `vb_ipc`, and hot storage writer paths.
- **Compensating evidence**: State 12 must still run canonical `moon ci`, targeted source/dependency diff inspection, and boundary source checks.
- **Expiry**: 2026-06-24T00:00:00Z
- **Invalidation trigger**: Any dependency manifest change or any runtime/core boundary drift.

## Behavior-Affecting Obligations

No behavior-affecting proof obligation is waived. All 18 obligations in `proof-obligations.planned.jsonl` must be proven, blocked, or rejected.
