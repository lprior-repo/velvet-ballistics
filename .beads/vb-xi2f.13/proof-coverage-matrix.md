# Proof Coverage Matrix: vb-xi2f.13

## Hazard → Proof Obligation Mapping

| Hazard | Severity | Proof Seed | Kani | Verus | Flux | Proptest | Fuzz | Covered? |
|---|---|---|---|---|---|---|---|---|
| H1: Layout/Width Mismatch | CRITICAL | PS-TEMPORAL-001, PS-EMISSION-PARITY | PO-KANI-001, PO-KANI-012 | — | — | PO-PROPTEST-001, PO-PROPTEST-005 | — | ✅ FULL |
| H2: Body Steps Interleaving | HIGH | PS-TEMPORAL-002 | PO-KANI-002 | — | — | PO-PROPTEST-002 | — | ✅ FULL |
| H3: Otherwise Target Amidst Bodies | MEDIUM | PS-TEMPORAL-003 | PO-KANI-003 | — | — | PO-PROPTEST-003 | — | ✅ FULL |
| H4: Body Width Overflow | LOW | PS-ARITH-001 | PO-KANI-004 | — | — | — | — | ✅ FULL |
| H5: SlotIndex Reuse | CRITICAL | PS-INVARIANT-001 | PO-KANI-006 | — | PO-FLUX-001 | — | — | ✅ FULL |
| H6: Condition Slot Overwritten | HIGH | PS-INVARIANT-002 | PO-KANI-007 | — | PO-FLUX-002 | — | — | ✅ FULL |
| H7: StepIdx Overflow | LOW | PS-ARITH-002 | PO-KANI-005 | — | — | — | — | ✅ FULL |
| H8: Branch Fanout Evasion | LOW | PS-FANOUT-001 | PO-KANI-008 | — | — | — | — | ✅ FULL |
| H9: Boolean Slot Type Mismatch | HIGH | PS-TYPE-001 | PO-KANI-009 | PO-VERUS-001 | — | — | — | ✅ FULL |
| H10: All-Branches-False | MEDIUM | PS-LIVENESS-001 | PO-KANI-010 | — | — | — | — | ✅ FULL |
| H11: Slot Race (N/A) | LOW | PS-CONCURRENCY-001 | — | — | — | — | — | ⚪ WAIVED |
| H12: YAML Injection (when) | MEDIUM | PS-INPUT-001 | PO-KANI-011 | — | — | — | PO-FUZZ-001 | ✅ FULL |
| H13: Deep Body Nesting | LOW | PS-INPUT-002 | — | — | — | PO-PROPTEST-004 | PO-FUZZ-002 | ✅ FULL |
| H14: Body Node Bloat | LOW | — | — | — | — | — | — | ⚪ DEFERRED |

## Acceptance Criteria Coverage

| AC | Description | Kani | Unit Test | Integration | Proptest |
|---|---|---|---|---|---|
| AC1 | choose_width returns correct count | PO-KANI-001 | ✅ planned | — | PO-PROPTEST-001 |
| AC2 | choose_width returns 1 for all-empty | PO-KANI-001 | ✅ planned (regression) | — | — |
| AC3 | lower_canonical_choose emits ChooseSlot + body nodes | PO-KANI-012 | — | ✅ planned | PO-PROPTEST-005 |
| AC4 | Each SlotBranch.target points to correct body start | PO-KANI-002 | ✅ planned | — | PO-PROPTEST-002 |
| AC5 | Last body step falls through to next | PO-KANI-002 | ✅ planned | — | PO-PROPTEST-002 |
| AC6 | All condition slots recorded | PO-KANI-006 | ✅ planned | — | — |
| AC7 | IR passes validate() | PO-KANI-012 | — | ✅ planned | — |
| AC8 | Empty-body produces identical IR | — | ✅ planned (regression) | — | — |
| AC9 | No YAML strings in IR | PO-KANI-013 | ✅ planned | — | — |
| AC10 | Fanout/route/label preserved | PO-KANI-008 | ✅ planned (existing) | — | — |

## Contract Clause Coverage

| Clause | Proof Seeds Covered By |
|---|---|
| 2.1 choose_width (MODIFIED) | PS-TEMPORAL-001, PS-ARITH-001, PS-EMISSION-PARITY |
| 2.2 lower_canonical_choose (MODIFIED) | PS-TEMPORAL-002, PS-TEMPORAL-003, PS-ARITH-002, PS-INVARIANT-001, PS-INVARIANT-002, PS-FANOUT-001, PS-LIVENESS-001, PS-YAML-FREE-IR |
| 2.3 Body Step Lowering | PS-TEMPORAL-002, PS-EMISSION-PARITY |
| 2.4 Backward Compatibility | PS-EMISSION-PARITY |
| Domain I1 (boolean slots) | PS-TYPE-001 |
| Domain I2 (fanout ≤64) | PS-FANOUT-001 |
| Domain I4 (targets valid) | PS-TEMPORAL-002, PS-TEMPORAL-003 |

## Proof Obligation Summary

| Obligation Type | Count | Status |
|---|---|---|
| Kani harnesses | 13 | planned |
| Verus specs | 1 | planned |
| Flux refinements | 2 | planned |
| Proptest properties | 5 | planned |
| Fuzz targets | 2 | planned |
| TLA+ specs | 0 | not_applicable |
| Loom models | 0 | not_applicable |
| Miri checks | 0 | not_applicable |
| **Total** | **23** | |

## Coverage Gaps

| Gap | Severity | Mitigation |
|---|---|---|
| Per-branch step count limits | LOW | Deferred (H14). Not in scope for this bead. Future refinement. |
| Compile-time slot type tracking | MEDIUM | PS-TYPE-001 identifies this existing gap. Verus spec models the invariant but does not close the gap. Waiver candidate for completeness. |
| Choose-within-choose (nested choose in body) | MEDIUM | Explicitly out of scope (Non-Goals item 1). The body lowering rejects non-Set/Do primitives, which catches nested choose at compile time. |
