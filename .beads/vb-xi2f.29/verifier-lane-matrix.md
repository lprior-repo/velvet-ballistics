# Verifier Lane Matrix: vb-xi2f.29

## Risk → Lane Classification

| Risk Tag | TLA+ | Verus | Kani | Flux | Loom | Miri | Proptest | Fuzz |
|---|---|---|---|---|---|---|---|---|
| CANONICAL_NAME_BUG (C-01) | — | — | ✅ REQUIRED | — | — | — | — | — |
| DIGEST_INSENSITIVITY (C-02,C-03,C-05) | — | — | ✅ REQUIRED | — | — | — | ✅ REQUIRED | — |
| NESTED_STEP_BLINDNESS (C-04) | — | — | ✅ REQUIRED | — | — | — | ✅ REQUIRED | — |
| RECURSION (C-04 bounded) | — | — | ✅ REQUIRED | — | — | — | — | — |
| REGRESSION (C-06,C-07) | — | — | — | — | — | — | ✅ REQUIRED | — |
| EXHAUSTIVENESS (C-01) | — | — | ✅ REQUIRED | — | — | — | — | — |
| EDGE_CASE empty branches (C-06) | — | — | — | — | — | — | — | — |
| NESTED_TOGETHER (C-04) | — | — | — | — | — | — | — | — |

✅ REQUIRED = Obligation created
— = Not applicable (see lane-decisions for evidence)

## Lane Decision Summary

| Verifier | Total Decisions | Required | Not Applicable | Blocked |
|---|---|---|---|---|
| tla-plus | 12 | 0 | 12 | 0 |
| verus | 12 | 0 | 12 | 0 |
| kani | 12 | 5 | 7 | 0 |
| flux-rs | 12 | 0 | 12 | 0 |
| loom | 12 | 0 | 12 | 0 |
| miri | 12 | 0 | 12 | 0 |
| proptest | 12 | 6 | 6 | 0 |
| cargo-fuzz | 12 | 0 | 12 | 0 |

## Proof Seed to Obligation Mapping

| Proof Seed | Required Obligations |
|---|---|
| PS-xi2f29-001 | PO-xi2f29-001 (kani), PO-xi2f29-008 (kani), PO-xi2f29-015 (unit) |
| PS-xi2f29-002 | PO-xi2f29-002 (proptest), PO-xi2f29-010 (kani), PO-xi2f29-014 (unit) |
| PS-xi2f29-003 | PO-xi2f29-003 (proptest), PO-xi2f29-014 (unit) |
| PS-xi2f29-004 | PO-xi2f29-004 (proptest), PO-xi2f29-009 (kani), PO-xi2f29-010 (kani), PO-xi2f29-014 (unit), PO-xi2f29-012 (unit) |
| PS-xi2f29-005 | PO-xi2f29-005 (proptest) |
| PS-xi2f29-006 | PO-xi2f29-006 (proptest), PO-xi2f29-011 (unit), PO-xi2f29-013 (unit) |
| PS-xi2f29-007 | PO-xi2f29-007 (proptest) |
| PS-xi2f29-008 | PO-xi2f29-001 (kani) |
| PS-xi2f29-009 | PO-xi2f29-009 (kani) |
| PS-xi2f29-010 | PO-xi2f29-008 (kani) |
| PS-xi2f29-011 | PO-xi2f29-011 (unit) |
| PS-xi2f29-012 | PO-xi2f29-012 (unit) |

## Waiver Candidates

None. All behavior-affecting obligations are covered by proof or test lanes. No behavior-affecting waiver candidates are proposed.
