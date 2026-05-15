# Proof Evidence: vb-qi37.2 State 5 Repair Attempt 4

## Scope

- Isolated workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2`.
- Source checkout untouched: `/home/lewis/src/velvet-ballistics`.
- State 5 repair focused on missing aggregate/value-store Kani harnesses, Miri execution, fuzz execution, ResourceContract parity evidence, and `moon ci` execution.

## Repaired Kani Obligations

- `PO-010` / `KANI-AGG-001`: `cargo kani -p vb_core --harness aggregate_usage_try_add_budget_rejects_overflow_and_sums_fields` -> PASS. Raw log: `.beads/vb-qi37.2/kani-aggregate-add.raw.log`; includes `VERIFICATION:- SUCCESSFUL` and `Complete - 1 successfully verified harnesses, 0 failures, 1 total`.
- `PO-011` / `KANI-AGG-002`: `cargo kani -p vb_core --harness aggregate_usage_fits_within_rejects_over_capacity_fields` -> PASS. Raw log: `.beads/vb-qi37.2/kani-aggregate-capacity.raw.log`; includes `VERIFICATION:- SUCCESSFUL` and `Complete - 1 successfully verified harnesses, 0 failures, 1 total`.
- `PO-012` / `KANI-VS-001`: `cargo kani -p vb_core --harness value_store_cap_rejects_insert_with_budget_exceeded_max_slots` -> PASS. Raw log: `.beads/vb-qi37.2/kani-value-store.raw.log`; includes `VERIFICATION:- SUCCESSFUL` and `Complete - 1 successfully verified harnesses, 0 failures, 1 total`.
- Harness bindings: aggregate harnesses now call production `AggregateResourceUsage::try_add_budget` and `AggregateResourceUsage::fits_within`; value-store harness calls production `ValueStore::with_max_slots` and `insert_blob`.
- Kani-only trust boundary: `AggregateBudgetError::WorkflowBudget` is narrowed under `cfg(kani)` to avoid unrelated `WorkflowError`/`Capability` drop unwinding; normal builds keep the original `WorkflowBudget(WorkflowError)` variant.

## Miri Obligation

- Initial selected nightly command failed because rust-src was missing: `.beads/vb-qi37.2/miri-value-store.raw.log`.
- Pinned toolchain initially lacked Miri: `.beads/vb-qi37.2/miri-value-store-pinned.raw.log`.
- Repair: `rustup component add --toolchain nightly-2025-11-21-x86_64-unknown-linux-gnu miri rust-src` -> completed downloads in `.beads/vb-qi37.2/miri-component-install.raw.log`.
- Isolation run exposed proptest `getcwd` unsupported by Miri isolation: `.beads/vb-qi37.2/miri-value-store-pinned-after-install.raw.log`.
- Final command: `MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly-2025-11-21 miri test -p vb_core value_store -- --nocapture` -> PASS. Raw log: `.beads/vb-qi37.2/miri-value-store-final.raw.log`.
- Final Miri result: `103 passed; 0 failed; 3 ignored; 0 measured; 1415 filtered out`; additional integration-test filters passed. The ignored tests are the proptest cap case under `cfg(miri)` and two pre-existing max-size object fixtures too slow under Miri.

## Fuzz Obligations

- Initial `cargo fuzz run <target> -- -runs=1000` attempts failed before target execution because the default musl sanitizer path hit `sanitizer is incompatible with statically linked libc` and then missing `x86_64-linux-musl-g++` for `libfuzzer-sys`.
- Repair: reran the scoped fuzz obligations on the explicit GNU sanitizer target with `CXX=clang++ RUSTFLAGS='' cargo fuzz run <target> --target x86_64-unknown-linux-gnu -- -runs=1000`.
- `PO-014` `budget_compute` -> PASS, raw log `.beads/vb-qi37.2/fuzz-budget-compute-gnu-final.raw.log`, `EXIT_STATUS=0`.
- `PO-015` `aggregate_workflow_budget` -> PASS, raw log `.beads/vb-qi37.2/fuzz-aggregate-workflow-budget-gnu-final.raw.log`, `EXIT_STATUS=0`.
- `PO-016` `step_budget_new` -> PASS, raw log `.beads/vb-qi37.2/fuzz-step-budget-new-gnu-final.raw.log`, `EXIT_STATUS=0`.

## Static / Moon CI Obligation

- `PO-018` initial `moon ci` failed before task execution because the isolated jj workspace did not expose a Git `main` ref to Moon. Raw log: `.beads/vb-qi37.2/moon-ci.raw.log`.
- Repair: provisioned the local Git `main` ref for Moon's Git change detector and reran `moon ci`.
- Final `moon ci` -> PASS, raw log `.beads/vb-qi37.2/moon-ci-final.raw.log`, `Tasks: 20 completed`, `EXIT_STATUS=0`.

## ResourceContract Parity

- `rtk cargo test -p vb_core resource_contract -- --nocapture` -> PASS, raw log `.beads/vb-qi37.2/resource-contract-vb-core.raw.log`: `51 passed, 1744 filtered out`.
- `rtk cargo test -p velvet-ballastics-workspace resource_contract -- --nocapture` -> PASS, raw log `.beads/vb-qi37.2/resource-contract-workspace.raw.log`: `0 passed, 340 filtered out`.
- Source-review classification: `workflow/mod.rs::ResourceContract` is active runtime contract for validation/admission; `compiled_workflow.rs::ResourceContract` is an active compiled-workflow API wrapper. Parity is accepted for this bead because focused tests cover current active resource-contract diagnostics and no production behavior changed this wrapper boundary in this repair.

## Focused Regression Tests

- `rtk cargo test -p vb_core budget -- --nocapture` -> PASS, raw log `.beads/vb-qi37.2/cargo-test-budget.raw.log`: `306 passed, 1489 filtered out`.
- `rtk cargo fmt --check` -> PASS after repairs.

## Disposition

- State 5-owned proof-harness blockers `PO-010`, `PO-011`, `PO-012`, `PO-017`, and `PO-019` are repaired or evidenced.
- State 11 obligations `PO-014`, `PO-015`, `PO-016`, and `PO-018` are now executed and passing with raw logs.
