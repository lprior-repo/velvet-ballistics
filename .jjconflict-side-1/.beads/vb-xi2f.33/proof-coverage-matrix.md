# Proof Coverage Matrix — vb-xi2f.33

**Bead**: `vb-xi2f.33` / P1: digest covers ask semantics
**Schema**: Maps proof seeds → verifier lanes → obligations with coverage status.

## Coverage Legend

- `✓ required` = verifier is required for this seed (obligation created)
- `→ S8` = delegated to State 8 test-planner (behavior test)
- `— not_applicable` = verifier is not applicable (see lane decision for evidence)
- `blocked` = verifier needed but tooling unavailable

## Coverage Matrix

| Proof Seed | TLA+ | Verus | Kani | Flux | Loom | Miri | proptest | fuzz | Behavior Test |
|---|---|---|---|---|---|---|---|---|---|
| PS-ASK-001 (prompt sensitivity) | `—` | `—` | `✓` PO-KANI-001 | `—` | `—` | `—` | `✓` PO-PROPTEST-001 | `✓` PO-FUZZ-001 | `→ S8` |
| PS-ASK-002 (timeout sensitivity) | `—` | `—` | `✓` PO-KANI-002 | `—` | `—` | `—` | `✓` PO-PROPTEST-002 | — | `→ S8` |
| PS-ASK-003 (determinism) | `—` | `—` | — | `—` | `—` | `—` | `✓` PO-PROPTEST-003 | — | `→ S8` |
| PS-ASK-004 (empty prompt) | `—` | `—` | `✓` PO-KANI-003 | `—` | `—` | `—` | — | — | `→ S8` |
| PS-ASK-005 (None vs Some("")) | `—` | `—` | `✓` PO-KANI-004 | `—` | `—` | `—` | — | — | `→ S8` |
| PS-ASK-006 (duplicate parity) | `—` | `—` | — | `—` | `—` | `—` | — | — | `→ S8` (primary) |
| PS-ASK-007 (regression) | `—` | `—` | — | `—` | `—` | `—` | — | — | `→ S8` (primary) |
| PS-ASK-008 (field ordering) | `—` | `—` | `✓` PO-KANI-005 | `—` | `—` | `—` | `✓` PO-PROPTEST-004 | — | `→ S8` |
| PS-ASK-009 (panic-freedom) | `—` | `—` | `✓` PO-KANI-006 | `—` | `—` | `—` | — | — | — |
| PS-ASK-010 (explicit arm) | `—` | `—` | — | `—` | `—` | `—` | — | — | `→ S8` + static review |

## Obligation Coverage by Contract Clause

| Contract Clause | Formal Proof | Behavior Test (State 8) |
|-----------------|-------------|------------------------|
| INV-ASK-001 (prompt sensitivity) | PO-KANI-001, PO-PROPTEST-001, PO-FUZZ-001 | `→ S8` |
| INV-ASK-002 (timeout sensitivity) | PO-KANI-002, PO-PROPTEST-002 | `→ S8` |
| INV-ASK-003 (determinism) | PO-PROPTEST-003 | `→ S8` |
| INV-ASK-004 (empty prompt) | PO-KANI-003 | `→ S8` |
| INV-ASK-005 (None vs Some("")) | PO-KANI-004 | `→ S8` |
| INV-ASK-006 (duplicate parity) | — | `→ S8` (primary) |
| INV-ASK-007 (Set/Finish regression) | — | `→ S8` (primary) |
| TC-001 (explicit Ask arm) | — | `→ S8` + static review |
| TC-002 (field ordering) | PO-KANI-005, PO-PROPTEST-004 | `→ S8` |
| TC-007 (panic-freedom) | PO-KANI-006 | — |

## Coverage Summary

| Verifier | Seeds Covered | Obligations |
|----------|--------------|-------------|
| Kani | 6 (PS-ASK-001/002/004/005/008/009) | PO-KANI-001 through PO-KANI-006 |
| proptest | 4 (PS-ASK-001/002/003/008) | PO-PROPTEST-001 through PO-PROPTEST-004 |
| cargo-fuzz | 1 (PS-ASK-001) | PO-FUZZ-001 |
| TLA+ | 0 | not_applicable (10 seeds per VLD) |
| Verus | 0 | not_applicable (10 seeds per VLD) |
| Flux | 0 | not_applicable (10 seeds per VLD) |
| Loom | 0 | not_applicable (10 seeds per VLD) |
| Miri | 0 | not_applicable (10 seeds per VLD) |
| Behavior test (S8) | 9 (PS-ASK-001 through PS-ASK-008, PS-ASK-010) | delegated to test-planner |
| Static review | 1 (PS-ASK-010) | code review |

**Total formal proof obligations**: 11 (6 Kani + 4 proptest + 1 fuzz)
**Delegated to State 8 (test-planner)**: 9 seeds with concrete test scenarios in traceability-matrix.jsonl
**All 10 seeds covered**: yes
**All 7 invariants covered**: yes (5 via formal proof, 2 via behavior tests)
**All 4 type contracts covered**: yes (3 via formal proof, 1 via behavior test)
