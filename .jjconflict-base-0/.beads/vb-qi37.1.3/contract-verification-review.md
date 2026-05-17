bead_id: vb-qi37.1.3
bead_title: runtime/recovery: Hydrate RunFrame from snapshot and journal
phase: 4
updated_at: 2026-05-09T00:00:00Z

# Contract Verification Review

## Reviewer: Orchestrator (GoMasterOrchestrator)
## Date: 2026-05-09

## Contract Completeness Check

| Clause | Present | Has Error Variant | Has Test Mapping | Has Proof Obligation | Status |
|---|---|---|---|---|---|
| PRE-1: snapshot.run == run_id | Yes | Yes (ReplayDivergence) | Yes | PO-001 | OK |
| PRE-2: tail events belong to run_id | Yes | Yes (ReplayDivergence) | Yes | — | OK |
| PRE-3: tail seq > snapshot.seq | Yes | Yes (ReplayDivergence) | Yes | PO-002 | OK |
| PRE-4: snapshot bytes decodable | Yes | Yes (CorruptSnapshot) | Yes | PO-009 | OK |
| PRE-5: step_count > 0 | Yes | Yes (InvalidCompiledWorkflow) | Yes | PO-003 | OK |
| POST-1: Ok(RunFrame) populated | Yes | N/A | Yes | — | OK |
| POST-2: run_id equality | Yes | N/A | Yes | — | OK |
| POST-3: pc from last event | Yes | N/A | Yes | — | OK |
| POST-4: dimensions from max indices | Yes | Yes (FrameDimensionOverflow) | Yes | PO-004 | OK |
| POST-5: states from snapshot + events | Yes | N/A | Yes | — | OK |
| POST-6: slots/taint from snapshot + events | Yes | N/A | Yes | — | OK |
| POST-7: executed count | Yes | N/A | Yes | PO-005 | OK |
| POST-8: parallel tracking | Yes | N/A | Yes | — | OK |
| POST-9: no empty-frame success | Yes | Yes (NoRecoveryData, CorruptSnapshot) | Yes | PO-012 | OK |
| INV-1: dimension integrity | Yes | N/A | Yes | PO-006 | OK |
| INV-2: slot-taint parity | Yes | N/A | Yes | PO-007 | OK |
| INV-3: step state machine legality | Yes | N/A | Yes | — | OK |
| INV-4: deterministic ordering | Yes | N/A | Yes | PO-008 | OK |
| INV-5: no silent defaults | Yes | N/A | Yes | PO-012 | OK |

## Verification Layer Review

| Layer | Coverage | Waiver | Justification | Status |
|---|---|---|---|---|
| Unit Tests | All clauses | No | Required for all | OK |
| Property Tests | INV-4, PRE-4 | No | Random snapshot + event sequences | OK |
| Miri | All | No | Byte decoding, vector indexing | OK |
| Kani | PRE-1,3,5; POST-4,7,9; INV-1,2,4,5 | No | Pure kernel verification | OK |
| Fuzz | PRE-4 | No | Arbitrary snapshot bytes | OK |
| Loom | N/A | Yes | Single-threaded hydration | OK |
| Static | All | No | Clippy + zero-unwrap | OK |

## Lean/Kani Scope Check

- Kani harnesses target pure functions only: `decode_snapshot_slots`, dimension arithmetic, step state validation.
- No I/O, async, or UI in Kani scope. Correct.
- All proof obligations have explicit harness names.

## Traceability Matrix Check

- Every clause maps to at least one test.
- Every clause maps to at least one verification layer or explicit waiver.
- `traceability-matrix.jsonl` is valid JSONL (one object per line).
- `proof-obligations.jsonl` is valid JSONL (one object per line).

## Findings

1. **Missing waiver for Loom**: Added in verification-layers.md with rationale.
2. **Kani harness names are descriptive**: All 12 harnesses have clear names.
3. **No contradictory states**: All invariants are mutually satisfiable.

## Decision

STATUS: APPROVED

The contract and verification layers are complete, consistent, and ready for test planning and implementation.
