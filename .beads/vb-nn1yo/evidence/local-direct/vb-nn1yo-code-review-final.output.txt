**Findings:**

1. **Eligibility logic correct:** Only terminal `RunFailedEvent` triggers retry eligibility. `RunCancelled`, `RunFinished`, empty slices, and active runs all return `can_retry: false` with the exact reason literal `"run did not fail (no retry needed)"`.

2. **Failure step determination correct:** The fold tracks `pending` (latest unresolved `StepStarted`/`StepFailed`/`ActionFailedEvent`) and clears it on matching `StepSucceeded`. No numeric inference (`saturating_add(1)` removed). `failed_at_step` is `None` when no unresolved step observed, while `can_retry` remains `true`.

3. **Implementation constraints satisfied:** Single fold over event slice, no loops, no `unsafe`/`panic`/`unwrap`/index access/casts, pure analysis with no side effects, no public wire field changes.

4. **Test coverage complete:** All four required regression tests present (`retry_last_started`, `failed_run_after_matching_success_has_unknown_step`, `retry_step_max`, `retry_failed_action`), plus the `StepFailed` regression (`retry_explicit_step_failed` yielding `Some(17)`), non-failure cases, and property test. Existing test module retained via `#[path]`.

5. **Failure Observation Addendum met:** `StepFailed` included in failure observation, preserving step identity. Latest observed wins; matching `StepSucceeded` clears.

No open findings.

**STATUS: APPROVED**