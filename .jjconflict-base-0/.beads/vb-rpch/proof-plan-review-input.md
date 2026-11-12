# Proof Plan Review Input — vb-rpch

## Bead
vb-rpch — BDD: Durability and recovery acceptance scenarios (Fjall journal replay, snapshot-plus-tail, typed errors)

## Input Contracts Reviewed
- `contract.md` — 127 lines, 10 preconditions, 10 postconditions, 6 invariants, 12 error variants, Verus/TLA+/Theorem clause splits
- `tla-spec.md` — 124 lines, 6 TLA-owned clauses (TLA-001 through TLA-006)
- `lean-contract.md` — 36 lines, no theorem-owned clauses, Verus sufficient
- `verification-layers.md` — 139 lines, full layer assignment table, Verus scope with 7 obligations, TLA+ scope
- `proof-obligations.jsonl` — 36 obligation rows
- `traceability-matrix.jsonl` — 34 contract clauses mapped to proofs and tests

## Discovery Results

### TLA+ Spec
- `specs/tla/RecoveryReplay.tla` EXISTS (92 lines, idempotency theorems only)
- `specs/tla/RecoveryReplay.cfg` EXISTS but INCOMPLETE (constants only, no SPECIFICATION/INIT/INVARIANTS)
- **GAP**: Existing spec covers `NoDuplicateNonIdempotent` and `ReplaySafe` only
- **GAP**: TLA-001/002/003/005/006 NOT modelled; TLA-004 (NoResolvedReExecution) is `ReplaySafe`
- **GAP**: `recovery_replay_full.tla` and `recovery_replay_full.cfg` do NOT exist

### Verus
- **GAP**: ZERO Verus annotations exist in `types.rs`, `replay/core.rs`, or `hydrate.rs`
- All 7 Verus obligations are MISSING (need proof-writer to add spec/proof fns first)

### Kani
- **GAP**: Zero Kani harnesses exist for recovery (no `hydrate_run_frame_precond_kani`, etc.)
- 3 Kani obligations MISSING

### BDD
- `crates/vb_storage/tests/recovery_bdd_tests.rs` EXISTS (1919 lines)
- 22 BDD tests + 1 durability gate test planned
- Actual pass/fail status UNKNOWN until execution

### Build
- `cargo check -p vb_storage` PASSES with 0 errors (1 warning: unused `_expected_action_abi_digests`)

## TLA+ Obligations Status

| ID | Clause | Claim | Status |
|---|---|---|---|
| TLA-REPLAY-001 | INV-TLA-001 | ReplaySeqOrder: seq ascending, steps monotonic | NOT MODELLED |
| TLA-INCOMPLETE-001 | INV-TLA-003 | OnlyIncompleteRuns | NOT MODELLED |
| TLA-NONIDEM-001 | INV-TLA-004 | NoResolvedReExecution | PARTIAL (ReplaySafe) |

## Verus Obligations Status

| ID | Clause | Claim | Status |
|---|---|---|---|
| VERUS-INV-002 | INV-002 | union algebraic properties | MISSING |
| VERUS-INV-004 | INV-004 | tracker monotonic | MISSING |
| VERUS-INV-005 | INV-005 | DigestCheck hierarchy | MISSING |
| VERUS-PRE-001 | PRE-001 | hydrate_run_frame preconditions | MISSING |
| VERUS-PRE-002 | PRE-002 | hydrate_run_frame_from_events preconditions | MISSING |
| VERUS-POST-009 | POST-009 | replay_events attempt filtering | MISSING |
| VERUS-INV-003 | INV-003 | seed dimension invariants | MISSING |

## Kani Obligations Status

| ID | Clause | Claim | Status |
|---|---|---|---|
| KANI-PRE-001 | PRE-001 | preconditions never panic | MISSING |
| KANI-PRE-002 | PRE-002 | preconditions never panic | MISSING |
| KANI-POST-009 | POST-009 | replay_events no panic | MISSING |

## Waiver Obligations

| ID | Clause | Reason | Status |
|---|---|---|---|
| WAIVER-GAP3-ABI | GAP-3-ActionAbiMismatch | Lookup not implemented | ACCEPTED |
| WAIVER-GAP3-POL | GAP-3-PolicyDigestMismatch | Lookup not implemented | ACCEPTED |
| WAIVER-TERM-MISMATCH | ERR-TerminalStateMismatch | No public API parameter | ACCEPTED |

## Reviewer Questions

1. **TLA SPLIT vs EXTEND**: The existing spec covers idempotency narrowly. Should proof-writer SPLIT into `RecoveryReplayIdempotency.tla` (preserve theorems) + `RecoveryReplayFull.tla` (full pipeline), or EXTEND the existing spec?

2. **Verus proof order**: Verus annotations must be written before Verus can verify. Is the proof-plan adequate as a pre-verification plan, or should proof-writer add stub annotations first?

3. **Kani harness design**: Kani obligations reference harness names that don't exist. Should proof-writer design the harnesses in the proof-obligations.planned.jsonl, or is that implementation detail?

4. **BDD uncertainty**: The BDD tests exist but their actual pass/fail status is unknown. Is it acceptable to mark them as UNKNOWN in the proof plan and update after execution?
