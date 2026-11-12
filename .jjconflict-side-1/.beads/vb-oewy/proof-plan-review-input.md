---
bead_id: vb-oewy
bead_title: "bdd: Full suite runner and evidence artifact contract"
phase: 4
updated_at: 2026-05-20T05:15:00Z
attempt: 1
---

# Proof Plan Review Input — vb-oewy

## Proof Obligations Summary

Total obligations: 10
- Verus: 3 (POST-001, POST-003, INV-002)
- Test: 7 (POST-002, POST-004, POST-005, POST-006, INV-001, INV-003, INV-004)
- Waived: 1 (INV-002)

## Risk Classification

- `HIGH`: 1 — POST-001 (aggregation invariant)
- `MEDIUM`: 4 — POST-002, POST-003, POST-004, INV-001
- `LOW`: 4 — POST-005, POST-006, INV-003, INV-004

## TLA+ Coverage

No TLA+ obligations. Non-applicability rationale: the BDD runner is a deterministic sequential function, not a temporal/state-over-time system.

## Verus Coverage

- POST-001: `BddSuiteResult::total >= passed + failed + skipped` invariant proven by Verus
- POST-003: `BddScenarioStatus` enum exhaustive match proven by Verus
- INV-002: `duration_ms` monotonicity — waived as LOW risk

## Test Coverage

All behavioral obligations covered by `cargo test` in `bdd_runner_tests.rs`.

## Open Questions for Review

1. Is `INV-002` waiver justified, or should the Verus proof be required?
2. Is `POST-001` Verus proof sufficient, or does this need Kani bounded checking?
3. Should the evidence bundle schema versioning be tested via a serialization compatibility test?
