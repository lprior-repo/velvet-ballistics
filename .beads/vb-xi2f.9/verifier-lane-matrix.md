# Verifier Lane Matrix: YAML Path and Source Span Diagnostics

**Bead:** vb-xi2f.9
**Agent:** proof-planner (State 4)

Summary of lane decisions per proof seed. See `verifier-lane-decisions.jsonl` for machine-readable details.

| Proof Seed | TLA+ | Verus | Kani | Flux | Loom | Miri | proptest | fuzz | cargo-check/test |
|---|---|---|---|---|---|---|---|---|---|
| PS-001 (SPAN-ENRICH) | N/A | N/A | **REQUIRED** | **REQUIRED** | N/A | N/A | **REQUIRED** | N/A | N/A |
| PS-002 (NEVEC) | N/A | N/A | **REQUIRED** | N/A | N/A | N/A | **REQUIRED** | N/A | N/A |
| PS-003 (DIAG-FILE) | N/A | N/A | **REQUIRED** | N/A | N/A | N/A | N/A | N/A | N/A |
| PS-004 (YERR-SPAN) | N/A | N/A | **REQUIRED** | N/A | N/A | N/A | **REQUIRED** | N/A | N/A |
| PS-005 (CANON-SPAN) | N/A | N/A | **REQUIRED** | N/A | N/A | N/A | N/A | N/A | **REQUIRED** |
| PS-006 (VERR-SPAN) | N/A | N/A | **REQUIRED** | N/A | N/A | N/A | **REQUIRED** | N/A | **REQUIRED** |
| PS-007 (SPAN-BRIDGE) | N/A | N/A | **REQUIRED** | N/A | N/A | **REQUIRED** | **REQUIRED** | N/A | N/A |
| PS-008 (TREE-MARK) | N/A | N/A | **REQUIRED** | N/A | N/A | N/A | **REQUIRED** | N/A | N/A |
| PS-009 (RM-SRCMAP) | N/A | N/A | WAIVED | N/A | N/A | N/A | N/A | N/A | **REQUIRED** |
| PS-010 (UNIFY-DIAG) | N/A | N/A | WAIVED | N/A | N/A | N/A | N/A | N/A | **REQUIRED** |
| PS-011 (SEM-MAP-MSG) | N/A | N/A | WAIVED | N/A | N/A | N/A | **REQUIRED** | N/A | N/A |
| PS-012 (BACK-COMPAT) | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | **REQUIRED** |

## Legend

- **REQUIRED** — Obligation created in `proof-obligations.planned.jsonl`
- **WAIVED** — Waiver candidate documented in `waiver-candidates.jsonl`; non-behavior change or covered by cheaper verifier
- **N/A** — Not applicable with concrete evidence in `verifier-lane-decisions.jsonl`

## Summary

| Verifier | Required | Waived | N/A |
|---|---|---|---|
| TLA+ | 0 | 0 | 12 |
| Verus | 0 | 0 | 12 |
| Kani | 8 | 3 | 1 |
| Flux | 1 | 0 | 11 |
| Loom | 0 | 0 | 12 |
| Miri | 1 | 0 | 11 |
| proptest | 7 | 0 | 5 |
| fuzz | 0 | 0 | 12 |
| cargo-check/test/ci | 4 | 0 | 8 |

**Total obligations planned: 25** (8 Kani + 1 Flux + 1 Miri + 7 proptest + 4 static + 4 unit-test embedded in static)
