# Hazard Analysis — vb-pcu4h

- bead_id: vb-pcu4h
- artifact_owner: rust-contract
- scope: hazards relevant to the recovery-reducer test-edit lane; no production-code hazard.

## Hazard catalog

| ID | Hazard | Risk tag | Severity | Mitigation |
|----|--------|----------|----------|------------|
| HZ-1 | Vec length drift in `pending_actions` | len-drift | High (audit P1) | `assert_eq!(recovered.pending_actions, vec![RecoveredPendingAction { step, action }])` — the Vec `PartialEq` covers length and per-element equality in a single assertion. |
| HZ-2 | Phantom-duplicate entry | len-drift, mutation-strength | High | Same Vec-equality assertion: a vec with two entries panics on length-1 vs length-2 mismatch. |
| HZ-3 | Field drift (`step` or `action` wrong) | mutation-strength | High | Vec-equality triggers per-element `RecoveredPendingAction::eq` which compares both `step` and `action` via derived `PartialEq`. |
| HZ-4 | Silent `Err(_)` pass via `matches!(Ok(_) if …)` | silent-err-passes | Medium | Replace `assert!(matches!(seed, Ok(_) if …))` with `let recovered = seed.expect("…")` so any `Err(_)` panics with a named message. |
| HZ-5 | Unsupported-flag derivation drop | derived-state-proxy | Medium (Test A only) | Preserve `assert!(recovered.unsupported.pending_actions)` alongside the new Vec-equality assertion; the boolean exercises a separate accumulator-driven derivation. |
| HZ-6 | `cargo-mutants` swap-order or delete-field mutation passes | mutation-strength | Medium | The Vec-equality assertion's element-wise compare covers the deletion-of-fields mutation: removing either `step` or `action` from the struct breaks compilation in all call sites, including the test literal; a swap-order mutation breaks `PartialEq` for sorted vec and is caught. |
| HZ-7 | Verus mirror drift | api-drift | Medium | Drift gate (`scripts/check-production-inner-drift.sh`) and binding gate (`scripts/check-verus-production-binding.sh`) run as closure gates. The bead does not edit the mirror. |
| HZ-8 | Drift between production `RecoveredPendingAction` and the test-side struct literal | type-contract-drift | Low | Both sides reference the same `crates/vb_storage/src/recovery/types.rs:644-650` definition via the `use crate::recovery::replay::summary::*;` import; if production adds a field, the test literal fails to compile. |
| HZ-9 | SECONDARY test scope creep | secondary-scope | Low | SECONDARY targets in `crates/vb_runtime/tests/recovery_hydration_tests.rs` (lines 1899-1905, 2031-2037) are RECOMMENDED for uplift but marked `optional-modify` in delivery-scope; ownership rests with test-planner. |
| HZ-10 | Audit phrase ambiguity ("steps_started count") | audit-phrase-ambiguity | Low | `domain-model.md` rejects the literal reading: no test currently asserts only `summary.steps_started`; the phrase maps onto "counter/boolean/fuzzy-only" assertions, all of which the Vec-equality fix replaces. |
| HZ-11 | Recovery reducer's own correctness regression (production bug introduced) | production-regression | Low | The bead does not edit production code; the reducer is invoked unchanged. If the test fails post-fix, the failure is a real production regression, not a test artifact. |
| HZ-12 | Sort-order dependency in Vec equality | sort-canonicality | Low | `derive.rs:294` sorts ascending by `(step, action)`; constructed literal vec is single-element so sort order is canonical; multi-element tests (none in this bead) would need sorted-construction. |
| HZ-13 | `Debug` formatter regression breaking `assert_eq!` diagnostics | debug-regression | Low | `RecoveredPendingAction` derives `Debug`; if production ever removed `Debug`, the test would still pass (Vec equality uses `PartialEq`, not `Debug`) but the panic message would be opaque. Out of scope for this bead. |
| HZ-14 | `cargo fmt` style drift on the test edit | lint-fmt | Low | `moon run :lint-src` is a closure gate; `cargo fmt --all -- --check` validates style. The replacement uses standard Rust syntax with no exotic formatting. |
| HZ-15 | Compiler-error message noise if `RecoveredPendingAction` gains a required field | compile-time-fail | Low | The drift gate fires before compile; if it does not, the test literal fails to compile with a clear "missing field `step`" / "missing field `action`" error. |

## Temporal hazards

None. The recovery reducer is synchronous and idempotent. The test edit introduces no concurrency, no cancellation, no timer, and no async boundary.

## Concurrency hazards

None. No threads, no atomics, no shared state.

## Unsafe / provenance hazards

None. The bead does not introduce or modify any `unsafe`, raw pointer, or FFI surface. `RecoveredPendingAction` derives `Copy`, which is safe under the standard layout rules.

## Parser / codec / hostile-input hazards

None. Test fixtures use Rust struct literals (`JournalEvent::ActionScheduled { … }`); no string parsing, no bytes, no YAML, no JSON. Hostile-input fuzzing is out of scope for this bead.

## Performance / release hazards

None. The test edit does not change runtime cost (Vec equality on length-1 is O(1) per element; the prior `.any(...)` was also O(n) per element). No benchmark regression possible.

## API / release hazards

- API-HZ-1 — Adding fields to `RecoveredPendingAction` requires:
  1. Updating production struct (`types.rs:644-650`).
  2. Updating Verus mirror (`verification/verus/production_inner/replay_invariants_production.rs:253-256`).
  3. Updating production binding gate (no change needed; `#[path]` already binds).
  4. Re-running `check-production-inner-drift.sh` and `check-verus-production-binding.sh`.
  5. Updating test literals in `tests.rs:437-454, 621-672, 743-809` to include the new field.
- The bead does not trigger any of these changes; the struct is unchanged.

## Risk-tag mapping (delivery-scope ↔ hazard)

| delivery-scope risk_tag | Hazard ID |
|--------------------------|-----------|
| derived-state-proxy | HZ-5 |
| len-drift | HZ-1, HZ-2 |
| silent-err-passes | HZ-4 |
| mutation-strength | HZ-3, HZ-6 |
| secondary-scope | HZ-9 |
| audit-phrase-ambiguity | HZ-10 |

## Hazard closure

All hazards are closed by the test edit alone. No production-code hazard is introduced. The bead is a pure mutation-strength uplift; the only externally observable change is the test failure mode (panic message becomes more diagnostic).

## Out-of-scope hazards (forwarded)

- Production reducer drift (if a future change to `accumulator.rs` or `derive.rs` introduces a bug) — out of scope for this bead; covered by the existing recovery test suite.
- Verus model drift — out of scope; drift gate covers it.
- Runtime hydration correctness for `pending_actions` — covered by `crates/vb_runtime/tests/recovery_hydration_tests.rs`; the SECONDARY uplift (HZ-9) is the recommended strengthening.