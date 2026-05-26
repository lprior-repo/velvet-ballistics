# Machine Gate Report: vb-qi37.2 State 11

STATUS: PASS

## Passing Gates

- `cargo kani -p vb_core --harness aggregate_usage_try_add_budget_rejects_overflow_and_sums_fields` -> PASS.
- `cargo kani -p vb_core --harness aggregate_usage_fits_within_rejects_over_capacity_fields` -> PASS.
- `cargo kani -p vb_core --harness value_store_cap_rejects_insert_with_budget_exceeded_max_slots` -> PASS.
- `MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly-2025-11-21 miri test -p vb_core value_store -- --nocapture` -> PASS with 3 reported ignores.
- `rtk cargo test -p vb_core budget -- --nocapture` -> PASS.
- `rtk cargo test -p vb_core resource_contract -- --nocapture` -> PASS.
- `rtk cargo test -p velvet-ballistics-workspace resource_contract -- --nocapture` -> PASS, no matching tests.
- `CXX=clang++ RUSTFLAGS='' cargo fuzz run budget_compute --target x86_64-unknown-linux-gnu -- -runs=1000` -> PASS, `.beads/vb-qi37.2/fuzz-budget-compute-gnu-final.raw.log`, `EXIT_STATUS=0`.
- `CXX=clang++ RUSTFLAGS='' cargo fuzz run aggregate_workflow_budget --target x86_64-unknown-linux-gnu -- -runs=1000` -> PASS, `.beads/vb-qi37.2/fuzz-aggregate-workflow-budget-gnu-final.raw.log`, `EXIT_STATUS=0`.
- `CXX=clang++ RUSTFLAGS='' cargo fuzz run step_budget_new --target x86_64-unknown-linux-gnu -- -runs=1000` -> PASS, `.beads/vb-qi37.2/fuzz-step-budget-new-gnu-final.raw.log`, `EXIT_STATUS=0`.
- `moon ci` -> PASS, `.beads/vb-qi37.2/moon-ci-final.raw.log`, `Tasks: 20 completed`, `EXIT_STATUS=0`.

## Resolved Blockers

- Initial fuzz attempts failed on the default musl sanitizer path. Explicit GNU sanitizer target executes all three scoped fuzz obligations successfully.
- Initial `moon ci` failed because the isolated jj workspace lacked a Git `main` ref visible to Moon. A local `main` Git ref was provisioned for the gate and the canonical command passed.

## Decision

State 11 is approved. Proceed to State 12/13 evidence review.
