# vb-1tq2d — independent test review

Reviewer: deepseek-review · Bead: `vb-1tq2d` · status **IN_PROGRESS** · notes **OPEN**.

## Verdict: REJECTED (bounded) — one vacuous-test regression + two rejected
error-discard mutations; the rest of the touched test surface is sound.

### 1. Vacuous test — `action_ticket_from_different_run_is_not_equal`

`crates/vb_core/src/action/tests.rs` (line 391). In `78352490`/`c816f6b3`/
`6e3c5375`/HEAD the test constructs two tickets that differ on `run` (1 vs 2) but
asserts nothing — the `assert_ne!(ticket_a, ticket_b)` was lost during the 3-way
merge conflict resolution. A test that only constructs values and returns is a
false-green: it proves nothing about ticket inequality.

- Restored by `f4a405469` (vb-k5936) and carried by merge `5766dff91`.
- `ActionTicket` derives `PartialEq/Eq/Hash/Debug`; the assertion is meaningful.
- **Approved:** restore `assert_ne!(ticket_a, ticket_b)`.

### 2. Rejected — `tick_all().ok()` error discard (`5766dff91`)

`crates/vb_runtime/tests/durable_retry_run_attempt_authority.rs`. The merge's
conflict resolution turned:

```rust
let tick_before      = runtime.tick_all();   // line 264
let tick_after_fail  = runtime.tick_all();   // line 314
```

into `runtime.tick_all().ok();` — silently swallowing the `Result`. This test's
whole purpose is to assert deterministic runtime outcomes; discarding tick errors
hides failures that would otherwise surface as `DEFECT` guards. **Rejected by
root**; not approved here. The repair (5090-owned) should assert the outcome, not
discard it. The pre-existing third `.ok()` (line 327, "complete with NEW ticket")
is inherited from both `c816f6b3` and `f4a40546` and is likewise a pre-existing
weakening worth the same repair, but it is not part of `5766dff91`'s introduced
delta.

### 3. Sound test surface (verified, unchanged by the REJECTED parts)

- `issue_action_ticket_captures_all_fields` — asserts `run_attempt == 2` and
  per-action `attempt == 3`; correct generation-vs-per-action split.
- Kani ticket inputs: `issue_action_ticket_with_zero_key_is_valid` /
  `_with_max_values` cover zero/max; `kani_action_ticket_has_valid_key` uses
  `kani::any()` plus a zero/max symbolic selector. (Module blocked from compiling
  only by the `attempt` defect above.)
- `durable_retry_run_attempt_authority.rs` real-runtime assertions —
  `stale_old_ticket_rejected_after_durable_retry`, `run_attempt_advances_but_per_action_attempt_does_not`,
  `future_run_attempt_ticket_rejected`, `zero_run_attempt_ticket_rejected` — all
  assert exact expected/found run-attempt values and unchanged counters; sound
  apart from the two `.ok()` discards.
- `shutdown_tests.rs` `DuplicateEvent { run, seq }` asserts the colliding `run` and
  `seq` are carried exactly (lines 49–61).
- BCD `checked_sub(1)` rejection is asserted cross-test (journal unchanged).
- Journal tag tests: `RetryCheckpoint = 37` accepted, `38` rejected.

### 4. Source-preservation test counterparts (independent)

All 13 saved SHAs' test/source counterparts are present in the current tree
(workspace_tests `vb_vt2f_direct_runtime_api_acceptance.rs` at
`crates/workspace_tests/tests/`; `vb_wave16_mqa.rs`; `runtime_version_barrier_tests.rs`;
`verified_artifact.rs` support; `flux_sequence/tests.rs`; executor conformance
tests). Full table in `coverage.md`. No meaningful test source dropped.

## Bead notes: OPEN

Restore `assert_ne!`; do NOT adopt `5766dff91`'s `.ok()` discards. No
production/VCS mutations made.
