# Proof Strategy - vb-qi37.2.5 State 4 FUZZ-RESOURCE-001 repair

STATUS: PLANNED

## Boundary
- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2-5`.
- Source checkout writes: forbidden and not used.
- State 4 writes only: this strategy, `proof-plan-review-input.md`, `proof-obligations.planned.jsonl`, and State transition/evidence entries in `STATE.md` / `verification-ledger.jsonl`.
- Replanning reason: State 11 rejected `FUZZ-RESOURCE-001` because the previously approved `cargo fuzz run resource_budget -- -runs=1000` command failed before execution and was invalid evidence for the current stdin-once driver.

## Inputs Read
- `.beads/vb-qi37.2.5/contract.md`
- `.beads/vb-qi37.2.5/verification-layers.md`
- `.beads/vb-qi37.2.5/proof-obligations.jsonl`
- `.beads/vb-qi37.2.5/proof-obligations.planned.jsonl`
- `.beads/vb-qi37.2.5/traceability-matrix.jsonl`
- State 11 blocker evidence: `formal-verification-report.md`, `machine-gate-report.md`, `regression-diff.md`, `verification-ledger.jsonl`, and `STATE.md`.

## Discovery Commands
- `pwd -P`: PASS, returned `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2-5`.
- Isolation guard: PASS, workspace is not `/home/lewis/src/velvet-ballistics` and is not nested under it.
- `test -s` for `contract.md`, `verification-layers.md`, `proof-obligations.jsonl`, `proof-obligations.planned.jsonl`, `traceability-matrix.jsonl`, and State 11 blocker artifacts: PASS.
- `jq -c 'select(.id=="FUZZ-RESOURCE-001")' .beads/vb-qi37.2.5/proof-obligations.jsonl`: PASS; source obligation now uses `stdin replay plus cargo test`.
- `jq -c 'select(.id=="PO-009")' .beads/vb-qi37.2.5/proof-obligations.planned.jsonl`: PASS; planned obligation now targets stdin replay plus property-test surrogate.
- Blocked discovery commands: none.

## Risk Classification
- Temporal execution-slice boundedness: TLA+ with TLC remains required for `INV-002` and `POST-001`.
- Nested admission/rejection lifecycle: TLA+ with TLC remains required for `POST-006`, `INV-006`, and capped store terminal behavior.
- Pure step-budget arithmetic: Verus remains required for `INV-001`.
- Pure resource-budget composition arithmetic: Verus remains required for `INV-006`.
- Cargo-integrated Kani: `status: waived`, not passed, because discovered `kani/` files are standalone and no truthful `cargo kani --package ... --harness ...` command exists without proof-source or manifest edits.
- Budget and value-store generated coverage: exact `cargo test --package vb_core --lib -- ...` commands remain required.
- Value-store UB lane: `moon run :miri` remains required, with raw evidence or blocked_tooling if unavailable.
- `FUZZ-RESOURCE-001`: required obligation remains active, but the executable proof-plan lane is now deterministic stdin replay plus companion property-test evidence. The invalid `cargo fuzz run resource_budget -- -runs=1000` command is waived only as evidence for the current stdin-once driver; no cargo-fuzz PASS is claimed.
- Static no-panic/source governance: `moon run :lint-src` remains required for production/source lint.
- Pre-existing `vb_runtime` missing generated chunk: `status: planned`; deferred/global classification remains a State 11 evidence classification, not a State 4 proof result.

## Planned Obligations
- `PO-001` Verus step-budget arithmetic.
- `PO-002` Verus resource-budget composition.
- `PO-003` TLC execution-slice model.
- `PO-004` TLC nested admission/value-cap model.
- `PO-005` Kani waiver with pending-review details in the waiver object.
- `PO-006` budget proptest/unit exact commands.
- `PO-007` value-store proptest/unit exact commands.
- `PO-008` Miri lane.
- `PO-009` `FUZZ-RESOURCE-001` stdin replay plus property-test surrogate, requiring exact output `resource_budget stdin replay PASS cases=1000` and companion `PROPTEST_CASES=10000` adversarial proptest evidence.
- `PO-010` source lint/static no-panic lane.
- `PO-011` planned State 11 classification row for possible pre-existing global runtime failure.

## FUZZ-RESOURCE-001 Repair
- Replaced cargo-fuzz as the planned executable evidence lane for the current target because `fuzz/src/bin/resource_budget.rs` is a stdin-once binary and `-runs=1000` does not truthfully execute 1000 malformed inputs.
- Required command now builds `fuzz/Cargo.toml --features fuzz --bin resource_budget`, runs exactly 1000 deterministic bounded stdin cases, requires exact output `resource_budget stdin replay PASS cases=1000`, and then runs `PROPTEST_CASES=10000 rtk cargo test --package vb_core --test vb_qi37_2_5_boundedness_adversarial proptest -- --nocapture`.
- Waiver is narrow: `cargo fuzz run resource_budget -- -runs=1000` is invalid evidence for this driver only until a true `libfuzzer_sys::fuzz_target!` harness exists or cargo-fuzz can truthfully execute 1000 malformed-input cases for this target.

## Non-Claims
- No verifier pass result is claimed by State 4.
- Prior State 7/8/9/11 PASS evidence remains context only and must be consumed by the next owning review/execution state after this repaired plan is accepted.
- No production, test, proof, model, harness, dependency, or config files were edited.
