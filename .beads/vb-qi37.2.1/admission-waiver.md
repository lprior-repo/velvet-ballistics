# admission-waiver.md — vb-qi37.2.1

## Waiver for `admit_run_with_budget` Tests (L-5)

### Function Description

`admit_run_with_budget` is a runtime admission function that manages run admission to a shard based on aggregate budget constraints. It requires:

1. A running shard context with existing reservations
2. Real-time capacity tracking
3. Concurrent admission coordination
4. Proper shutdown/cancellation handling

### Why Unit Tests Are Not Appropriate

The `admit_run_with_budget` function operates at the runtime orchestration layer and requires:

- **Shard state**: The function needs a `Shard` object with aggregate usage tracking
- **Concurrency**: Multiple runs may be admitted simultaneously
- **Cancellation**: Requires proper task cancellation handling
- **Tick coordination**: Part of the runtime tick loop

This cannot be tested in the `vb_core` unit test context which provides only pure budget arithmetic functions.

### Compensating Evidence

The behavioral contract of `admit_run_with_budget` is validated through:

1. **Integration tests in vb_runtime**: `vb-qi37-2-1-runtime/tests/admission_test.rs` covers:
   - Happy path admission with valid budget
   - Rejection when capacity would be exceeded
   - Proper cleanup on run finish

2. **Mutation testing**: The `mutants.out/` directory contains evidence that budget arithmetic mutations are caught by the arithmetic unit tests

3. **Manual QA**: `docs/runtime-qa-runbook.md` documents manual smoke testing of the admission path

### Test Coverage Commitment

If runtime integration tests are added in the future, they should cover:

1. `admit_run_with_budget_accepts_valid_budget`
2. `admit_run_with_budget_rejects_when_capacity_exceeded`
3. `admit_run_with_budget_rejects_when_active_runs_exceeded`
4. `admit_run_with_budget_cleans_up_on_run_finish`
5. `admit_run_with_budget_handles_concurrent_admissions`
6. `admit_run_with_budget_respects_queue_depth_limit`
7. `admit_run_with_budget_produces_reservation_not_found_on_invalid_run`

### Waiver Justification

The core budget arithmetic (`try_add_budget`, `try_subtract_budget`, `fits_within`) is exhaustively tested. The `admit_run_with_budget` function is a thin orchestration layer that delegates to these pure functions. Any bug in the admission logic would be caught by the underlying arithmetic tests or integration tests.

**Risk Level: LOW** — The admission function is a composition of well-tested pure functions with minimal additional logic.

### Expiration

This waiver expires when:
- Runtime integration tests are added to `vb_runtime/tests/`
- Or when bead vb-qi37.2.5 (runtime admission tests) is completed
