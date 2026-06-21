# SR-010: `recover_all_incomplete_runs` aborts the entire batch on a single bad run

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_storage/src/recovery/recover.rs:273`
- **Confidence**: confirmed

## Description

`recover_all_incomplete_runs` iterates every run header, calls
`events_for_run` and `recover_runtime_frame_seed_from_events` per run, and
propagates the first error via `?`. A single corrupt, missing-events, or
divergent run therefore aborts the entire batch, even for runs that would
have hydrated cleanly. The function returns `Vec<RecoveryHydration>` with no
slot for partial failures, so callers cannot recover the remaining runs
without reimplementing the loop.

## Evidence

```rust
pub fn recover_all_incomplete_runs(
    journal: &FjallJournal,
) -> RecoveryResult<Vec<RecoveryHydration>> {
    let headers = journal.run_headers()?;
    let mut recovered = Vec::new();

    for header in headers {
        let events = journal.events_for_run(header.run)?;
        if events.is_empty() {
            return Err(RecoveryError::NoRecoveryData { run: header.run });   // <-- abort
        }
        if crate::recovery::replay::extract_terminal(&events).is_none() {
            let seed =
                crate::recovery::replay::summary::recover_runtime_frame_seed_from_events(&events)?;
            recovered.push(RecoveryHydration::FrameSeed(seed));
        }
    }

    Ok(recovered)
}
```

Failure mode: if run A is corrupt and run B is healthy, the loop hits A
first, returns `Err(RecoveryError::...)`, and never visits B. The operator
sees only the error for A and has no way to recover B through this API.

## Adversarial Check

A counter-argument: "fail closed on the first error is the right behavior
for recovery — partial state is worse than no state." That would be true if
the function were transactional, but it is not: `recovered` accumulates
state in memory and is discarded on the first error, so the operator gets
neither the partial result nor a durable record of which runs failed. For a
fleet-wide recovery scan, "one bad run blocks everyone" is the wrong
operator experience; the right behavior is to record the failure per-run
and continue, returning a structured `Vec<Result<RecoveryHydration,
RecoveryError>>` or a typed `RecoveryBatch { successes, failures }`. The
function is also reachable from CLI tooling (`recover` commands typically
scan all incomplete runs), so the abort behavior is operator-visible.

## Suggested Fix

Either:

1. Change the return type to `RecoveryResult<Vec<Result<RecoveryHydration, RecoveryError>>>`
   and replace each `?` with `push(Err(...))` followed by `continue`.
2. Or add a parallel API `recover_all_incomplete_runs_lenient` that returns
   a structured batch result and have callers choose between strict and
   lenient modes.

At minimum, document the abort-on-first-failure contract in the docstring
so operators know they need to handle the bad run individually before
re-running the batch.
