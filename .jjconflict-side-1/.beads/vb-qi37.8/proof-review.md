# Proof Review: vb-qi37.8

## Bead Information
- **bead_id**: vb-qi37.8
- **title**: validate/compile: Prove and complete shared validation pipeline
- **state**: 6 (Proof Review)
- **reviewer**: proof-reviewer (general)

## Review Summary

| Aspect | Status | Finding |
|--------|--------|---------|
| Vacuity | PASS | All 9 gates (G7-G15) documented with implementations |
| Assumption | PASS | A1-A8 documented with rationale |
| Bound | PASS | G7 (≤64), G8 (symbols_count), G9 (u16), G10 (14 variants) bounded |
| Harness | PASS | Kani (16), Miri (9), Proptest (2) plans defined |
| Model | PASS | TLA+ spec and Lean contract exist; temporal PO deferred appropriately |
| Evidence | PASS | Obligation ledger complete; 3 DEFERRED_GLOBAL correctly applied |

## Obligation Analysis

| Lane | Obligations | Status | Assessment |
|------|-------------|--------|------------|
| Miri | 9 (PO-002,004,007,012,015,021,023,027,029) | PLAN_ONLY | Correct: UB fast-fail first |
| Proptest | 2 (PO-018,028) | PLAN_ONLY | Correct: property-based bijection/determinism |
| Kani | 16 (PO-001,003,005,006,008-011,013,014,016,017,019,022,024,030) | PLAN_ONLY | Correct: bounded model checking primary |
| TLA+ | 2 deferred (PO-020,025) | DEFERRED_GLOBAL | Correct: requires prior Kani completion |
| Lean | 1 deferred (PO-026) | DEFERRED_GLOBAL | Correct: requires prior TLA+ completion |
| Integration | 6 (PO-031-036) | PLAN_ONLY | Correct: call site verification |
| Fuzz | 1 (PO-036) | PLAN_ONLY | Correct: continuous fuzzing |

## Deferred Obligation Chain

```
PO-019 (Kani G13 acyclic) → PO-020 (TLA+ G13_NoCycle) → PO-025 (TLA+ G15_Separated) → PO-026 (Lean NDNodesSeparated)
```

Chain is correctly ordered with Kani as prerequisite for TLA+ temporal proofs.

## Risk Assessment

| Risk Level | Gates | Mitigation | Adequate |
|------------|-------|------------|----------|
| LOW | G7, G8, G9 | Kani bounded | ✓ |
| MEDIUM | G10, G11, G12, G13, G14 | Kani + Miri dual | ✓ |
| HIGH | G15 | Kani + TLA+ + Lean | ✓ |

## Engineering Rules Compliance

| Rule | Status | Evidence |
|------|--------|----------|
| No unsafe | COMPLIANT | #![forbid(unsafe_code)] in vb_validate/lib.rs |
| No unwrap/expect | COMPLIANT | All ValidationResult propagated via ? |
| No panic | COMPLIANT | Error paths return Err variants |
| Checked arithmetic | COMPLIANT | checked_sub/checked_add in G7 (gates.rs:72-84) |
| Bounds checking | COMPLIANT | Array::get() used with explicit error |

## Findings

1. **Adequacy**: Proof strategy correctly maps 36 obligations to appropriate verifier lanes
2. **Execution order**: Cheap-first (Miri → Proptest → Kani → TLA+ → Lean) minimizes fast-fail time
3. **Deferred obligations**: Temporal proofs correctly deferred with proper dependency chain
4. **Evidence ledger**: Complete obligation tracking with status matrix

## Recommendations

1. Execute Miri first (fast UB detection) before Kani runs
2. Verify slot_count bound in Kani unwind configuration matches u16 constraint
3. Ensure TLA+ TLC model checking uses same bounds as Kani (slot_count)

---

**STATUS: APPROVED**

The proof strategy is sound, well-structured, and correctly applies DEFERRED_GLOBAL for temporal obligations that require prior bounded proof completion. No repair guide required.
