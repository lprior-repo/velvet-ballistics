# CB-002: `From<WorkflowError>` and `From<BudgetTraversalError>` for `BudgetError` swallow original context

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_core/src/budget/validation.rs:203` and `:212`
- **Confidence**: confirmed

## Description

Both blanket `From` impls collapse every underlying error variant into a
single `BudgetError::TotalStepsExceeded { actual: u64::MAX, limit: u64::MAX }`,
discarding the original error. Callers can no longer distinguish
`EntryOutOfBounds`, `StepOutOfBounds`, `JumpCycle`, `StepCountOverflow`, or
`InvalidCompiledWorkflow` from a genuine step overflow — every diagnostic
path that pattern-matches the resulting `BudgetError` reports an identical,
misleading "u64::MAX > u64::MAX" message.

## Evidence

```rust
impl From<WorkflowError> for BudgetError {
    fn from(_err: WorkflowError) -> Self {
        BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        }
    }
}

impl From<BudgetTraversalError> for BudgetError {
    fn from(_err: BudgetTraversalError) -> Self {
        BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        }
    }
}
```

(`crates/vb_core/src/budget/validation.rs:203` and `:212`)

Note the parameter is `_err` — explicitly dropped on the floor.

## Adversarial Check

The simplest counter-argument is "BudgetError already carries
`TotalStepsExceeded` because that's all the caller needs." But
`BudgetError::StepsExecutableExceeded`, `TimerEntriesExceeded`,
`JournalBatchBytesExceeded`, etc. exist precisely to discriminate failure
modes for telemetry. Furthermore, `JumpCycle` is a workflow-author-facing
correctness error that is fundamentally different from a budget overrun; the
`From` impl makes them indistinguishable. The `actual: u64::MAX` sentinel is
not documented anywhere and any operator dashboard that plots `actual` will
see a wildly misleading spike.

## Suggested Fix

Either widen `BudgetError` with a `Traversal(BudgetTraversalError)` /
`Workflow(WorkflowError)` source variant, or `map` each underlying variant
to the most specific existing `BudgetError` (e.g. `EntryOutOfBounds` →
`StepsExecutableExceeded { actual: 0, limit: 0 }` with a `reason`).
At minimum, stop using `u64::MAX` as a sentinel — surface the original
numbers.
