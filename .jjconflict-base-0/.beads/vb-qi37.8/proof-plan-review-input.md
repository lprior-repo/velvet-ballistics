# Proof Plan Review Input: vb-qi37.8

## Bead Information
- **bead_id**: vb-qi37.8
- **state**: 4 (Proof Planning)
- **title**: validate/compile: Prove and complete shared validation pipeline
- **dispatch_manifest**: delegate_agent=general (proof planning), isolated_workdir=/home/lewis/src/vb-qi37-ws

## Proof Obligations Summary

| Verifier | Count | Obligations |
|----------|-------|-------------|
| Kani | 16 | PO-001,003,005,006,008,009,010,011,013,014,016,017,019,022,024,030 |
| Miri | 9 | PO-002,004,007,012,015,021,023,027,029 |
| Proptest | 2 | PO-018, PO-028 |
| TLA+ | 2 | PO-020, PO-025 |
| Lean | 1 | PO-026 |
| Integration | 6 | PO-031,032,033,034,035,036 |

**Total**: 36 proof obligations

## Risk Distribution

| Risk Level | Gates | Count | Strategy |
|------------|-------|-------|----------|
| LOW | G7, G8, G9 | 10 | Kani + Miri dual |
| MEDIUM | G10, G11, G12, G13, G14 | 17 | Kani + Miri + Proptest/TLA+ |
| HIGH | G15 | 4 | Kani + TLA+ + Lean + Miri |
| PIPELINE | Pipeline | 3 | Kani + Proptest + Miri |
| INTEGRATION | Integration | 6 | Integration tests + Fuzz |

## Critical Questions for Proof Reviewer

### Q1: G12 Bijection Proof Adequacy
PO-016 and PO-017 verify surjection and injection via Kani bounded lookup.
PO-018 verifies bijection property via Proptest with 1000 iterations.
**Question**: Is Kani bounded lookup sufficient to prove total bijection, or should Lean be added?

### Q2: G15 Lean Theorem Proving Scope
PO-026 proposes Lean theorem proving for NDNodesSeparated.
**Question**: Is the Lean proof kernel necessary given Kani + TLA+ coverage?
**Risk**: Lean adds significant proof burden; TLA+ temporal invariant may suffice.

### Q3: Miri Coverage Completeness
9 Miri obligations cover UB in slot operations, graph traversal, symbol resolution.
**Question**: Should Miri run on all 36 obligations, or is selective Miri sufficient?

### Q4: TLA+ Model Scope
PO-020 (G13) and PO-025 (G15) use TLA+ for temporal properties.
**Question**: Should TLA+ model include all 9 gates or only G13 and G15?

### Q5: Proptest Iteration Count
PO-018 and PO-028 use 1000 iterations.
**Question**: Is 1000 sufficient for bijection/determinism properties, or increase to 10000?

## Verification Lane Recommendations

### Minimum Viable Proof (MVP)
1. Miri on all UB-sensitive obligations (9 POs)
2. Kani on structural/bounded obligations (16 POs)
3. Proptest on G12 and pipeline (2 POs)
4. Integration tests (6 POs)

### Recommended Proof (FULL)
Add TLA+ for G13 and G15 temporal properties.

### Maximum Proof (AUDIT-READY)
Add Lean for G15 theorem proving.

## Dependency Analysis

- PO-020 (TLA+) depends on PO-019 (Kani) — cycle detection algorithm must be sound
- PO-025 (TLA+) depends on PO-024 (Kani) — suspension point algorithm must be sound
- PO-026 (Lean) depends on PO-025 (TLA+) — temporal property must hold
- Integration tests depend on all gate implementations

## Deferral Criteria

| Obligation | Deferral Condition |
|-----------|-------------------|
| PO-026 (Lean) | If Kani + TLA+ pass, defer Lean as audit-ready not MVP |
| PO-020 (TLA+) | If Kani cycle detection passes, defer TLA+ to FULL |
| PO-025 (TLA+) | If Kani suspension check passes, defer TLA+ to FULL |

## Proof Execution Order

1. Miri (fast fail) — 9 obligations
2. Kani (bounded) — 16 obligations
3. Proptest — 2 obligations
4. TLA+ (temporal) — 2 obligations (deferred to FULL)
5. Lean (theorem) — 1 obligation (deferred to AUDIT-READY)
6. Integration — 6 obligations
7. Fuzz — continuous

## Evidence Required

| Lane | Evidence | Format |
|------|----------|--------|
| Miri | 0 UB reports | .miri.log |
| Kani | 0 failed assertions | .kani.json |
| Proptest | 0 failures | .proptest.log |
| TLA+ | 0 invariant violations | .tlc.out |
| Lean | 0 unproven theorems | .lean.out |
| Integration | all tests pass | cargo test output |
| Fuzz | no crashes/UB | fuzz corpus |

## Reviewer Action

Approve/reject proof strategy or request modifications to:
- Verifier lane assignments
- Obligation deferral criteria
- Execution order
- Bound/timeout configuration
