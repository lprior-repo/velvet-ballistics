# Contract: `analyze_retry`

**Signature:** `(events: &[JournalEvent]) -> RetryAnalysis`

**Purpose:** Inspect an ordered per-run journal slice and report whether a retry is appropriate, which step failed, and the last successful step. Pure analysis; consumed by CLI report only.

## Eligibility

Only a terminal `RunFailedEvent` makes a run eligible for retry. Runs that are cancelled, finished, empty, or still active are not eligible and must preserve the existing non-failure reason literal.

## Failure Step Determination

The failure step is the latest unresolved observed `StepStarted` or `ActionFailedEvent`. A `StepSucceeded` event records the last successful step and clears any matching pending step. Do not invent a next step by adding one to a successful numeric ID: workflows branch and are not linear, and a u16 maximum boundary cannot wrap or saturate into an unobserved step.

If no unresolved step was observed, `failed_at_step` is `None`. A failed terminal run may still have `can_retry` set to `true` as an analysis report.

## Non-Failure Cases

When the run did not fail, return `failed_at_step: None`, the observed `last_successful_step`, `can_retry: false`, and the reason `"run did not fail (no retry needed)"`.

## Implementation Constraints

- Fold bounded by the event slice; no new loops
- No `unsafe`, `panic`, `unwrap`, index access, or type casts
- No execution or retry workflow side effects
- No public wire field changes

## Test Coverage

Retain the existing four tests. Add regression tests for: a late attempt after success, a cleared successful step, a u16 maximum step ID, and an `ActionFailedEvent`.
**Failure Observation Addendum**

`JournalEvent::StepFailed { run, seq, step, attempt }` is now included in failure observation. Recovery must retain the failed step's identity.

Observation accepts `StepStarted`, `StepFailed`, and `ActionFailedEvent`. Latest observed wins; matching `StepSucceeded` clears. Only terminal `RunFailed` is eligible. No numeric inference. Unknown `None` preserves eligibility.

Regression: `StepFailed17` with terminal `RunFailed` and no prior `StepStarted` yields `Some(17)`. All other accepted behavior preserved.