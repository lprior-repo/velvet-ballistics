# Proof Writer Report: vb-qi37.2 State 5 Repair Attempt 4

STATUS: COMPLETE_WITH_DEFERRED_GLOBAL_GATES

## Files Changed

- `crates/vb_core/src/budget.rs`: repaired exact aggregate Kani harnesses to call production `AggregateResourceUsage::try_add_budget` and `AggregateResourceUsage::fits_within`; added `cfg(kani)` narrowing for `AggregateBudgetError::WorkflowBudget` to keep unrelated `WorkflowError` drop recursion out of the aggregate proof surface.
- `crates/vb_core/src/value_store.rs`: marked `property_value_store_cap` ignored under `cfg(miri)` after Miri isolation and runtime made the proptest path non-executable in the scoped Miri lane; normal cargo/proptest execution remains active.

## Executed Evidence

- Kani aggregate add: PASS, `.beads/vb-qi37.2/kani-aggregate-add.raw.log`.
- Kani aggregate capacity: PASS, `.beads/vb-qi37.2/kani-aggregate-capacity.raw.log`.
- Kani value-store cap: PASS, `.beads/vb-qi37.2/kani-value-store.raw.log`.
- Miri value-store scoped lane with disabled isolation: PASS, `.beads/vb-qi37.2/miri-value-store-final.raw.log`.
- Focused budget tests: PASS, `.beads/vb-qi37.2/cargo-test-budget.raw.log`.
- ResourceContract focused tests: PASS, `.beads/vb-qi37.2/resource-contract-vb-core.raw.log` and `.beads/vb-qi37.2/resource-contract-workspace.raw.log`.

## Blockers Routed To State 11

- Fuzz targets cannot execute in this environment because cargo-fuzz selects `x86_64-unknown-linux-musl`; ASan is incompatible with static libc, and the non-static retry requires missing global binary `x86_64-linux-musl-g++`.
- `moon ci` cannot execute in this jj workspace because Moon's git base discovery expects `main`, which is absent in the workspace git view.

## Next Routing

- State 6 review may approve State 5 repair because missing harnesses and Miri evidence are repaired.
- State 11 must remain blocked until fuzz toolchain and Moon git-ref topology are repaired or formally waived by project policy.
