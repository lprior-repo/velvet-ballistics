# Proof Writer Report - vb-qi37.2.5 State 5 FUZZ-RESOURCE-001 repair

STATUS: READY_FOR_REVIEW

## Scope

- Bead: `vb-qi37.2.5`.
- Agent role: go-skill State 5 proof-writer repair after State 4 `FUZZ-RESOURCE-001` proof-plan repair.
- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2-5`.
- Source checkout: `/home/lewis/src/velvet-ballistics` was not written.
- Boundary: `.beads/vb-qi37.2.5/` evidence/report/state refresh only.
- Production/source edits: none.
- Test edits: none.
- Proof/model/harness edits: none.
- Dependency/config edits: none.

## Inputs Read

- `.beads/vb-qi37.2.5/proof-obligations.jsonl` after State 3 repair.
- `.beads/vb-qi37.2.5/proof-obligations.planned.jsonl` after State 4 repair.
- `.beads/vb-qi37.2.5/proof-strategy.md`.
- `.beads/vb-qi37.2.5/proof-plan-review-input.md`.
- `.beads/vb-qi37.2.5/proof-evidence.md` prior State 5 evidence.
- `.beads/vb-qi37.2.5/formal-verification-report.md` State 11 rejection that identified the invalid cargo-fuzz evidence lane.

## Artifacts Written Or Repaired

- `.beads/vb-qi37.2.5/proof-writer-report.md`: refreshed for the repaired `FUZZ-RESOURCE-001` lane.
- `.beads/vb-qi37.2.5/proof-evidence.md`: refreshed with exact stdin replay plus companion proptest evidence.
- `.beads/vb-qi37.2.5/STATE.md`: appended State 5 repair transition/completion evidence.
- No production, test, Verus, TLA+, Kani, fuzz harness, dependency, or config file was edited.

## Obligation Coverage

| Obligation | Artifact | State 5 repair result |
| --- | --- | --- |
| `PO-001` / `VERUS-STEP-001` | `verification/verus/step_budget.rs` | Prior State 5/11 PASS evidence remains context; not rerun in this repair. |
| `PO-002` / `VERUS-BUDGET-001` | `verification/verus/resource_budget.rs` | Prior State 5/11 PASS evidence remains context; not rerun in this repair. |
| `PO-003` / `TLA-SLICE-001` | `specs/vb_qi37_2_5/BoundednessSlice.*` | Prior State 5/11 PASS evidence remains context; not rerun in this repair. |
| `PO-004` / `TLA-ADMIT-001` | `specs/vb_qi37_2_5/NestedBoundednessAdmission.*` | Prior State 5/11 PASS evidence remains context; not rerun in this repair. |
| `PO-005` / `KANI-LOOP-001` | `kani/gate_11_loop.rs;kani/gate_12_14_15.rs` | WAIVED row only; no Kani PASS claimed. |
| `PO-009` / `FUZZ-RESOURCE-001` | `fuzz/src/bin/resource_budget.rs`; `crates/vb_core/tests/vb_qi37_2_5_boundedness_adversarial.rs` | PASS for repaired lane: deterministic stdin replay reports `resource_budget stdin replay PASS cases=1000`; companion proptest reports `cargo test: 3 passed, 19 filtered out`. |

## Verification Commands

| Command | Exit | Result |
| --- | ---: | --- |
| `pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2-5" && case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; esac` | 0 | PASS: workspace isolation preserved. |
| `mkdir -p target/tmp && RUSTC_WRAPPER= TMPDIR=target/tmp rtk cargo build --manifest-path fuzz/Cargo.toml --features fuzz --bin resource_budget && python3 -c "...1000 deterministic stdin cases..." && RUSTC_WRAPPER= TMPDIR=target/tmp PROPTEST_CASES=10000 rtk cargo test --package vb_core --test vb_qi37_2_5_boundedness_adversarial proptest -- --nocapture` | 0 | PASS: fuzz binary built, stdin replay printed exact `resource_budget stdin replay PASS cases=1000`, companion adversarial proptest reported `3 passed, 19 filtered out`. |

## FUZZ-RESOURCE-001 Discharge

- Discharged by the repaired State 4 lane: deterministic stdin replay plus companion proptest evidence.
- No PASS is claimed for `cargo fuzz run resource_budget -- -runs=1000`.
- The old cargo-fuzz command remains invalid evidence for the current stdin-once driver because prior State 11 showed it failed before execution under local cargo-fuzz/static-musl sanitizer selection.
- The cargo-fuzz waiver is evidence-command-specific only; `INV-008` hostile-input boundedness remains required and is discharged here by replaying 1000 bounded stdin cases plus 10000-case companion proptest configuration.

## Assumptions And Bounds

- Replay case count is exact: 9 fixed adversarial inputs plus 991 generated bounded byte inputs equals 1000 cases.
- Each stdin replay subprocess uses a 2 second timeout and requires exit code 0.
- Companion proptest uses `PROPTEST_CASES=10000` against `vb_qi37_2_5_boundedness_adversarial` filtered by `proptest`.
- This evidence proves no panic, nonzero exit, timeout, or process kill for the current stdin-once driver and companion property tests; it is not a libFuzzer coverage claim.

## Reviewer Guidance

- Review only the repaired `FUZZ-RESOURCE-001` evidence delta for this State 5 repair.
- Verify that `FUZZ-RESOURCE-001` is no longer discharged by the invalid cargo-fuzz command.
- Verify that no production/test/proof source changes were made in this repair.
