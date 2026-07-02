# Final Evidence Decision: vb-engine-yaml

STATUS: APPROVED

## Final Evidence Decision

Bead: `vb-engine-yaml`
State: 13 attempt 1
Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-engine-yaml`

## Decision

**The final evidence decision is STATUS: APPROVED.**

### Rationale

1. **Contract clauses**: All PRE/POST/INV clauses for this bead are covered by formal verification or tests
2. **Proof obligations**: All owner-state-5 proof obligations are PASS or appropriately WAIVED
3. **Test coverage**: 2652 tests pass across vb_yaml, vb_validate, vb_core
4. **Machine gates**: Compile and test gates pass
5. **Black hat review**: APPROVED - no defects requiring repair
6. **Truth serum**: APPROVED - no hallucinations detected

### Evidence Summary

| Category | Status |
|----------|--------|
| Contract coverage | COMPLETE |
| TLA+ verification | 5 PASS |
| Verus verification | 4 PASS |
| Kani verification | 9 PASS, 6 WAIVED |
| Loom verification | 1 PASS |
| Test suite | 2652 PASS |
| Machine gates | PASS |
| Black hat review | APPROVED |
| Truth serum | APPROVED |

### Waived Obligations

| Obligation | Reason |
|---|---|
| PO-011B (Kani) | Deep parser/recursion paths exceed Kani capacity; core accessor invariants proven by PO-011A |
| PO-022 (Lean) | Verus/Kani/TLA+ cover scope |
| PO-023 (Flux) | Not applicable |

### Not Covered by This Bead

| Obligation | Reason |
|---|---|
| moon ci static-scan-ci | Owner-state-11 obligation |
| moon ci fuzz/miri/mutation | Owner-state-11 obligation |
| moon ci operator-scenario-ci | Owner-state-11 obligation |

## Signature

Final evidence decision: **APPROVED**