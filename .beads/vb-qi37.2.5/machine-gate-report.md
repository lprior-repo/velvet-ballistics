# Machine Gate Report — vb-qi37.2.5 State 11 (fresh execution)

STATUS: APPROVED

## Mandatory Preflight
- artifact existence + JSONL validation: PASS.
- contract verification status: PASS (`contract-verification-review.md` contains `STATUS: APPROVED`).
- isolated workspace guard: PASS; `pwd -P` returned bead workspace path exactly.

## Command Evidence Summary
| Gate | Command | Result | Evidence |
|---|---|---:|---|
| Verus step | `RUSTC_WRAPPER= TMPDIR=target/tmp verus verification/verus/step_budget.rs` | PASS | `6 verified, 0 errors` |
| Verus budget | `RUSTC_WRAPPER= TMPDIR=target/tmp verus verification/verus/resource_budget.rs` | PASS | `10 verified, 0 errors` |
| TLC slice | `tlc -metadir /tmp/opencode/tlc-vb-qi37-2-5-slice specs/vb_qi37_2_5/BoundednessSlice.tla -config ...` | PASS | no errors; 41 states, 21 distinct |
| TLC nested | `tlc -metadir /tmp/opencode/tlc-vb-qi37-2-5-nested specs/vb_qi37_2_5/NestedBoundednessAdmission.tla -config ...` | PASS | no errors; 301 states, 237 distinct |
| Budget proptests | five exact `budget::tests::*` commands | PASS | each `1 passed, 1520 filtered out` |
| Value proptests | three exact `value_store::tests::*` commands | PASS | each `1 passed, 1520 filtered out` |
| Miri | `RUSTC_WRAPPER= TMPDIR=target/tmp moon run :miri` | PASS | three scoped Miri tests passed; 1m 7s |
| Lint | `RUSTC_WRAPPER= TMPDIR=target/tmp moon run :lint-src` | PASS | `Tasks: 1 completed`; 808ms |
| Focused integration | `rtk cargo test --package vb_core --test vb_qi37_2_5_boundedness_adversarial` | PASS | `22 passed` |
| FUZZ-RESOURCE-001 repaired | stdin replay + proptest from proof-obligations.jsonl | PASS | `resource_budget stdin replay PASS cases=1000`; proptest `3 passed` |
| Old cargo-fuzz (waived) | `cargo fuzz run resource_budget -- -runs=1000` | WAIVED | musl+ASAN incompatibility; waived in proof-obligations.jsonl |
| DEFERRED-GLOBAL-001 | classification only | DEFERRED_GLOBAL | outside bead-local scope |

## Blocking Classification
- No blocking failures.
- `FUZZ-RESOURCE-001`: PASS with repaired stdin replay+proptest command; old cargo-fuzz command explicitly waived in proof-obligations.jsonl `waived_command` field.
- `DEFERRED-GLOBAL-001`: pre-existing workspace issue, not a bead-local failure.

## Decision
- APPROVED: all required/local obligations are PASS or WAIVED; DEFERRED_GLOBAL is unrelated to bead scope.
