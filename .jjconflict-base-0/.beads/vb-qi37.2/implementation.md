# Implementation Report: vb-qi37.2 State 10

STATUS: COMPLETE

## Files Changed

- `crates/vb_core/src/budget.rs`
- `crates/vb_core/src/value_store.rs`

## Clause Mapping

- `PO-010`, `POST-002`, `INV-002`, `ERR-003`: production-bound Kani harness for `AggregateResourceUsage::try_add_budget` exact sum and overflow behavior.
- `PO-011`, `POST-002`, `INV-002`, `ERR-003`: production-bound Kani harness for `AggregateResourceUsage::fits_within` exact capacity error fields.
- `PO-012`, `POST-004`, `INV-005`, `ERR-004`: production-bound Kani harness for `ValueStore::with_max_slots` cap rejection.
- `PO-017`: scoped Miri path made executable by ignoring the proptest-only cap property under Miri; normal proptest remains active outside Miri.

## Non-Changes

- No runtime behavior was changed for non-Kani/non-Miri builds.
- No fuzz or Moon configuration was changed; failures are global environment/tooling blockers.
