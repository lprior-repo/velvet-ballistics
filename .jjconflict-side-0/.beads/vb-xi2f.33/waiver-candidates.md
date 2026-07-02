# Waiver Candidates — vb-xi2f.33

**Bead**: `vb-xi2f.33` / P1: digest covers ask semantics
**State**: 4 (proof-planner)

## Waiver Candidate Summary

**No behavior-affecting waivers.** All behavior-affecting requirements (INV-ASK-001 through INV-ASK-007, TC-001, TC-002, TC-007) are covered by required proof obligations (Kani, proptest, cargo-fuzz, unit-test).

## Non-Applicable Verifier Lanes (Not Waivers)

These verifier lanes are `not_applicable` by concrete evidence. They are not waived — the risk they protect against is genuinely absent from the code under verification.

| Verifier | Non-Applicability Reason | Evidence |
|----------|------------------------|----------|
| TLA+ | No temporal, state-machine, or distributed properties | `boundary-map.md` lines 70-83 |
| Verus | P1 scope; 3-line fix; Kani covers bounded invariants proportionately | `delivery-scope.jsonl` (P1), `boundary-map.md` (trusted blake3) |
| Flux | No refinement-type properties | `type-contracts.md` lines 15-17 (no type-level change) |
| Loom | No concurrency (no threads, channels, async in digest path) | `boundary-map.md` lines 69-71 |
| Miri | No unsafe code, FFI, raw pointers, or interior mutability | `boundary-map.md` lines 85-90 |

## Non-Behavior Waivers

| ID | Scope | Reason |
|----|-------|--------|
| — | — | None required. All non-behavior items (code duplication, legacy path, empty prompt validation) are addressed by unit tests or are out of scope. |

## Validation

- `behavior_affecting: false` for all `not_applicable` lane decisions.
- No behavior-affecting waiver candidates exist.
- All behavior-affecting contract clauses have at least one required proof obligation.
