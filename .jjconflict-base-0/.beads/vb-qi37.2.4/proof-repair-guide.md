# Proof Repair Guide: vb-qi37.2.4

STATUS: APPROVED_HANDOFF

## Required Downstream Handoff
- No State 5 proof-artifact repair remains open after PR-004.
- State 12, rerun_from 12: repair `scripts/rust-verification-gauntlet.sh` so `moon run :verify-proof` executes instead of failing on Rust `//!` doc-comment syntax. Rerun `moon run :verify-proof` and preserve raw output.
- State 7, rerun_from 7: add or wire Kani coverage for `KANI-BUD-001` against concrete nested budget sum/product overflow and rejection behavior in `crates/vb_core/src/budget.rs`. Rerun the proof lane or targeted Kani command and preserve raw output.
- State 7, rerun_from 7: add generated property coverage for `PROP-BUD-001` covering bounded accepted nested `collect`/`reduce`/`repeat`/`together` workflows under `ResourceContract` and `BoundednessPolicy`.
- State 7, rerun_from 7: add generated property coverage for `PROP-DIAG-001` proving rejected nested growth diagnostics include resource, primitive, node, structural path, actual/computed value when known, and limit.
- State 5, rerun_from 5: RESOLVED for `PR-004`; `VERUS-AGG-001` and `VERUS-DIAG-001` now have executable rows plus traceability. Do not reopen unless a later reviewer finds the rows stale or non-executable.

## Accepted Evidence To Preserve
- `verus verification/verus/budget_bounded.rs` currently passes with `15 verified, 0 errors`.
- `tlc -config specs/tla/BoundedAdmission.cfg specs/tla/BoundedAdmission.tla` currently passes with no errors and complete bounded state graph depth `9`.
- Keep the classification that `GATE-BUD-001` is `BLOCKED_TOOLING` with owner_state `12`, rerun_from `12` until the rollup command executes.
- Keep the classification that `KANI-BUD-001`, `PROP-BUD-001`, and `PROP-DIAG-001` are `BLOCKED_SCOPE` with owner_state `7`, rerun_from `7` until concrete harness/property evidence exists.

## Rerun Targets
- `jq -c . .beads/vb-qi37.2.4/proof-obligations.jsonl >/dev/null`
- `jq -c . .beads/vb-qi37.2.4/traceability-matrix.jsonl >/dev/null`
- `verus verification/verus/budget_bounded.rs`
- `tlc -config specs/tla/BoundedAdmission.cfg specs/tla/BoundedAdmission.tla`
- `moon run :verify-proof`
- `moon run :verify-deep` after State 7 proptest/fuzz/Miri/mutation artifacts exist

## Approval Bar
- State 6 proof review is approved for direct State 5 proof artifacts because required TLA+/Verus evidence passed and all non-State-5 obligations carry explicit `owner_state`/`rerun_from` handoff.
- Downstream states may not treat this as a waiver: State 7 must satisfy Kani/proptest obligations, and State 12 must repair/rerun proof/deep/standard rollups before final acceptance.
