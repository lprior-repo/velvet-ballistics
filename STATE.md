# State: vb-qi37.2.1

**Bead:** vb-qi37.2.1
**Title:** runtime: Define aggregate resource budget model
**Phase:** 13 (Evidence Packaging — COMPLETE)
**Isolated workspace:** /home/lewis/src/vb-qi37-2-1
**Source checkout:** /home/lewis/src/Velvet-ballistics
**VERDICT:** APPROVED

## Phase History

| Phase | State | Status |
|-------|-------|--------|
| 1 | Explore | ✅ Complete |
| 2 | Plan | ✅ Complete |
| 4 | Contract review | ✅ APPROVED |
| 5 | Proof writing | ✅ Complete |
| 6 | Tests (State 8) | ✅ Complete |
| 7 | Formal execution | ✅ PASS |
| 8 (sic) | Black-hat review | ✅ APPROVED |
| 9 (sic) | Evidence packaging | ✅ APPROVED |

## State 9 Summary — Test Suite Review

**VERDICT: APPROVED** (0 LETHAL, 0 MAJOR, 3 MINOR)

- Banned pattern scan: 0 hits
- 1745 vb_core tests + 10 vb_runtime admission tests
- Overall line coverage: 90.17% (≥90% target met)
- 3 MINOR findings below 5-item threshold

## State 12 Summary — Black-Hat Review

**black-hat-report.md: STATUS: APPROVED**

PHASE 1 (Contract Parity): PASS — all contract clauses verified
PHASE 2 (Farley Rigor): PASS — no functions >25 lines, 0 unwraps
PHASE 3 (Holzman Rust): PASS — forbid(unsafe_code), checked_add/sub only
PHASE 4 (DDD/Simplicity): PASS — no invalid states representable
PHASE 5 (Bitter Truth): PASS — obvious, readable, no clever tricks

Findings: 0 LETHAL, 0 MAJOR

## State 13 Summary — Evidence Packaging

**final-evidence-decision.md: STATUS: APPROVED**

| Gate | Verdict |
|------|---------|
| Truth Serum | ✅ APPROVED |
| Evidence Packaging | ✅ APPROVED |
| holzman-report.md | ✅ APPROVED |
| test-review.md | ✅ APPROVED |
| formal-verification-report.md | ✅ APPROVED |
| machine-gate-report.md | ✅ PASS |
| black-hat-report.md | ✅ APPROVED |

## Artifacts Produced

| Artifact | Path | Status |
|---|---|---|
| contract.md | .beads/vb-qi37.2.1/contract.md | ✅ |
| domain-model-review.md | .beads/vb-qi37.2.1/domain-model-review.md | ✅ |
| tla-spec.md | .beads/vb-qi37.2.1/tla-spec.md | ✅ |
| lean-contract.md | .beads/vb-qi37.2.1/lean-contract.md | ✅ |
| verification-layers.md | .beads/vb-qi37.2.1/verification-layers.md | ✅ |
| proof-obligations.jsonl | .beads/vb-qi37.2.1/proof-obligations.jsonl | ✅ |
| traceability-matrix.jsonl | .beads/vb-qi37.2.1/traceability-matrix.jsonl | ✅ |
| test-repair-guide.md | .beads/vb-qi37.2.1/test-repair-guide.md | ✅ |
| admission-waiver.md | .beads/vb-qi37.2.1/admission-waiver.md | ✅ |
| test-suite-review.md | .beads/vb-qi37.2.1/test-suite-review.md | ✅ APPROVED |
| black-hat-report.md | .beads/vb-qi37.2.1/black-hat-report.md | ✅ APPROVED |
| assurance-bundle.md | .beads/vb-qi37.2.1/assurance-bundle.md | ✅ |
| truth-serum-report.md | .beads/vb-qi37.2.1/truth-serum-report.md | ✅ |
| final-evidence-decision.md | .beads/vb-qi37.2.1/final-evidence-decision.md | ✅ APPROVED |

## Key Decisions

1. **TLA+ not applicable** — Aggregate resource budget is entirely Rust-local arithmetic; no temporal, workflow, protocol, or concurrent behavior. All properties proven by Verus + Kani + Lean + unit tests.

2. **16-dimension model** — `AggregateResourceBudget` has 16 fields: 14 original dimensions plus `max_step_budget_per_tick` and `max_transitions_per_tick` (both u64). Step ceiling hard limits set at 1_000_000.

3. **BH-BUD findings addressed**:
   - BH-BUD-01 (u32 saturation): `validate_step_ceilings` enforces hard limits; zero rejected.
   - BH-BUD-02 (max_run_time_seconds hardcoded to 0): sourced from `WholeWorkflowBudget.max_run_time_seconds`.
   - BH-BUD-03 (information loss): narrowing uses exact integer conversion with overflow detection.
   - BH-BUD-06 (saturating_add inconsistency): `add_dim` uses `checked_add` only; no `saturating_add` in budget module.
   - BH-BUD-07 (gather_items saturating): `gather_items` uses same `add_dim`/`sub_dim` as all other dimensions.

4. **Lean owns 6 theorems**: AddSafe, SubSafe, FitsWithin, PolicyExact, AddSubRoundtrip, ConvLossless.

5. **Waivers issued**: WAIVER-001 (runtime admission lifecycle) and WAIVER-002 (WholeWorkflowBudget::compute IR traversal) covered by integration/proptest/fuzz.

## Test Summary

| Test File | Tests | Status |
|---|---|---|
| aggregate_budget_vb_qi37_2_1.rs | 1745 | All pass |
| admission_budget_vb_qi37_2_1.rs | 10 | All pass |
| Total vb_core | 1745 | 85.48% regions, 87.66% lines |
| Overall coverage | — | 90.17% lines (≥90% target) |

## Next Steps

- **State 14: Landing** — invoke landing-skill to merge and push to remote
- Blocking: vb-qi37.2.2, vb-qi37.2.3, vb-qi37.2.4

## Blocking Status

This bead BLOCKS:
- `vb-qi37.2.2` — aggregate budget enforcement at tick admission
- `vb-qi37.2.3` — aggregate budget release on finish/fail/cancel
- `vb-qi37.2.4` — aggregate budget audit journal integration
