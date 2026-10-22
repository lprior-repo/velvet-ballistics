# Bead vb-p51m8: Restore Lost Idempotency Test Scenarios

## Background
Eight named idempotency test scenarios were removed from `crates/workspace_tests/tests/vb_ko29_5_public_idempotency.rs` between commit `da633fe0` and current main. Two broad parity reviews mapped only three of the eight and approved generically; those approvals are provisional. This bead restores the meaningful cases against current contracts.

## Repair Author
LOCAL5090, workspace `/home/lewis/src/isoloated/velvet-ballistics-vb-p51m8-local5090-finish`.

## Repairs Applied

### 1. Digest rebinding for stale certificate tests (fixes two failing tests)
Both `given_stale_certificate_floor_when_strict_admission_runs_then_stale_error_and_journal_unchanged` and `given_wrong_digest_and_stale_floor_when_strict_admission_runs_then_digest_mismatch_wins_and_journal_unchanged` mutated `accepted_at_seq` but only computed a new `accepted_artifact_digest`. Actual `verify_accepted_artifact` requires BOTH `artifact.digest` and `artifact.verification.digest` equal that computed canonical digest. After computing the canonical digest, we now bind both self-digest fields to it. This makes the floor and digest-precedence assertions actually reachable; `verify_accepted_artifact` passes and admission proceeds to the intended invariant. No production bug assumed.

### 2. Compile fixture Result propagation
Replaced the newly introduced `compile_fixture` `.expect("compile fixture")` with `Result<vb_core::CompiledWorkflow, String>` and propagated the error through `accepted_artifact_fixture`. This satisfies the no-unwrap/expect rule.

### 3. Full journal Vec equality (replaces length comparisons)
Four tests previously used `len()` equality to assert "journal unchanged":
- `given_completed_action_before_restart_when_replayed_then_no_redispatch_and_event_count_stable`
- `given_evicted_runtime_key_when_durable_journal_replayed_then_recovery_resolves_action`
- `given_duplicate_lifecycle_command_when_retried_then_lifecycle_duplicate_request_and_count_unchanged`
- `given_completed_run_when_lifecycle_retry_uses_stale_key_then_lifecycle_stale_request`

All four now capture the full event vector before and assert exact Vec equality after. The replay and eviction tests assert `replayed == after_events` (replayed equals the full journal vector); the lifecycle retry tests assert `after == before` (full vector unchanged).

### 4. Exact Display assertion for stale certificate error
Replaced the weak `display.contains("stale")` check with an exact format comparison using `format!` against the production Display implementation:
```
admission rejected: artifact certificate stale for digest {digest:?}: accepted_at_seq {accepted_at_seq:?} < required_at_least {required_at_least:?}
```

## Coverage Mapping: Old to Current

| Old Test Name | Scenario | Current Test Name | Status |
|---|---|---|---|
| `given_divergent_artifact_digest_when_strict_admission_runs_then_digest_mismatch_is_exact` | Divergent requested/stored artifact digest | Same | Restored (already present) |
| `given_conflicting_certificate_proof_digest_when_strict_admission_runs_then_digest_mismatch_is_exact` | Conflicting verification proof digest | Same | Restored (already present) |
| `given_stale_certificate_floor_when_strict_admission_runs_then_stale_error_and_journal_unchanged` | Stale certificate floor | Same | Restored + repaired (digest rebind + exact Display) |
| `given_wrong_digest_and_stale_floor_when_strict_admission_runs_then_digest_mismatch_wins_and_journal_unchanged` | Wrong digest AND stale floor | Same | Restored + repaired (digest rebind) |
| `given_completed_action_before_restart_when_replayed_then_no_redispatch_and_event_count_stable` | Completed action before restart/replay | Same | Restored + strengthened (full Vec equality) |
| `given_evicted_runtime_key_when_durable_journal_replayed_then_recovery_resolves_action` | Evict runtime idempotency key, replay durable journal | Same | Restored + strengthened (full Vec equality) |
| `given_failed_run_retried_twice_when_cli_retry_collides_then_duplicate_error_and_no_append` | Duplicate lifecycle retry | `given_duplicate_lifecycle_command_when_retried_then_lifecycle_duplicate_request_and_count_unchanged` (renamed) | Strengthened (full Vec equality) |
| `given_completed_run_when_cli_retry_uses_stale_key_then_stale_error_and_no_append` | Stale lifecycle retry | `given_completed_run_when_lifecycle_retry_uses_stale_key_then_lifecycle_stale_request` (renamed) | Strengthened (full Vec equality) |

## Verification Evidence

**Command:**
```bash
python3 /tmp/vb-run-gate.py 300 .beads/vb-p51m8/evidence/local5090-final-v2 env RUSTFLAGS=-Dwarnings rustup run nightly-2026-04-28 cargo test -p velvet-ballistics-workspace-tests --test vb_ko29_5_public_idempotency
```

**Results:**
- Exit code: 0
- Tests run: 16
- Tests passed: 16
- Tests failed: 0
- Duration: 0.4s (compilation cached)
- Evidence files: `.beads/vb-p51m8/evidence/local5090-final-v2.*`

All 16 named test cases run and pass, including the six restored scenarios and the two strengthened CLI journal guard tests.

## Remaining Limitations

1. The `given_evicted_runtime_key_when_durable_journal_replayed_then_recovery_resolves_action` test uses `make_ticket` to construct `ActionTicket` values for the volatile tracker. While `replay_journal` itself uses the actual production journal, the ticket construction uses field-by-field initialization rather than a production constructor. This is within the test scope and acceptable per the bead instructions.

2. No broad CI, Kani, or Verus verification was run per bead instructions. Only the targeted test suite via the immutable runner.

## Files Modified
- `crates/workspace_tests/tests/vb_ko29_5_public_idempotency.rs` (sole source file modified)
- `.beads/vb-p51m8/contract.md` (this file)
- `.beads/vb-p51m8/evidence/local5090-final-v2.*` (test evidence)
