**FINDING: Gap in StepSucceeded matching semantics**

The test suite does not verify that `StepSucceeded` clears only a *matching* pending step. A mutation that clears `pending` on *any* `StepSucceeded` (not just matching) would pass all registered tests.

**Minimal regression test to add:**

```rust
#[test]
fn non_matching_success_does_not_clear_failure() {
    let a = analyze_retry(&[started(5), succeeded(3), failed_run()]);
    assert!(a.can_retry);
    assert_eq!(a.failed_at_step, Some(5));
    assert_eq!(a.last_successful_step, Some(3));
}
```

**Mutation coverage verified (all caught):**
- Dropping `StepFailed` branch → `retry_explicit_step_failed` fails
- First-instead-of-latest failure → `retry_last_started` fails (expects 8, would get 10)
- Inferring `+1` → `retry_step_zero` fails (expects 0, would get 1) and `failed_run_after_matching_success_has_unknown_step` fails (expects None, would get Some)

**STATUS: REJECTED** — add the non-matching success regression test above, then re-review.