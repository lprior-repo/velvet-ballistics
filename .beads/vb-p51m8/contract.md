# Bead vb-p51m8: Restore Lost Idempotency Test Scenarios

## Background
Eight named idempotency test scenarios were removed from `crates/workspace_tests/tests/vb_ko29_5_public_idempotency.rs` between commit `da633fe0` and current main. Two broad parity reviews mapped only three of the eight and approved generically; those approvals are provisional. This bead restores the meaningful cases against current contracts.

## Oracle Map: Old -> New

| # | Old Test Name | Scenario | New Test Name | Current Typed Error |
|---|---|---|---|---|
| 1 | `given_divergent_artifact_digest_when_strict_admission_runs_then_digest_mismatch_is_exact` | Divergent requested/stored artifact digest | Same | `AdmissionError::ArtifactDigestMismatch { requested, found: stored }` |
| 2 | `given_conflicting_certificate_proof_digest_when_strict_admission_runs_then_digest_mismatch_is_exact` | Conflicting verification proof digest | Same | `AdmissionError::ArtifactDigestMismatch { requested, found: proof_digest }` via `validate_accepted_artifact_envelope` |
| 3 | `given_stale_certificate_floor_when_strict_admission_runs_then_stale_error_and_journal_unchanged` | Stale certificate floor | Same | `AdmissionError::ArtifactCertificateStale { digest, accepted_at_seq, required_at_least }` + Display |
| 4 | `given_wrong_digest_and_stale_floor_when_strict_admission_runs_then_digest_mismatch_wins_and_journal_unchanged` | Wrong digest AND stale floor | Same | `AdmissionError::ArtifactDigestMismatch` (INV-002 fires before staleness check) |
| 5 | `given_completed_action_before_restart_when_replayed_then_no_redispatch_and_event_count_stable` | Completed action before restart/replay | Same | `ActionReplayTracker.is_resolved` returns true |
| 6 | `given_evicted_runtime_key_when_durable_journal_replayed_then_recovery_resolves_action` | Evict runtime idempotency key | Same | `ActionReplayTracker.is_resolved` returns true after replay |
| 7 | `given_failed_run_retried_twice_when_cli_retry_collides_then_duplicate_error_and_no_append` | Duplicate lifecycle retry | `given_duplicate_lifecycle_command_when_retried_then_lifecycle_duplicate_request_and_count_unchanged` | `CoreError::LifecycleDuplicateRequest` + journal count unchanged |
| 8 | `given_completed_run_when_cli_retry_uses_stale_key_then_stale_error_and_no_append` | Stale lifecycle retry | `given_completed_run_when_lifecycle_retry_uses_stale_key_then_lifecycle_stale_request` | `CoreError::LifecycleStaleRequest` + journal count unchanged |

## Implementation Details

### Fixture Pattern
Use the `compile_fixture` + `prepare_compiled_artifact` pattern from `vb_lp2v_proof_admission_bdd.rs`:
- Compile a minimal workflow via `vb_compile::compile_workflow`
- Create a valid artifact via `vb_storage::admission::prepare_compiled_artifact`
- Mutate artifact fields as needed for each scenario
- Serve via `FixedAcceptedStore` implementing `AcceptedArtifactStore`

### Scenario 1: Divergent Requested/Stored Digest
- Create valid artifact with digest D1
- Request admission with digest D2 (D1 != D2)
- Expect `ArtifactDigestMismatch { requested: D2, found: D1 }`
- Validation order: `validate_accepted_artifact_envelope` passes (proof matches stored), then INV-002 fails

### Scenario 2: Conflicting Verification Proof Digest
- Create valid artifact with digest D1
- Tamper: set `artifact.verification.digest = D2` (D1 != D2)
- Request admission with digest D1
- Expect `ArtifactDigestMismatch { requested: D1, found: D2 }` from `validate_accepted_artifact_envelope` (line 5 of chunk_004_validation.rs)
- This is the exact current rejection path

### Scenario 3: Stale Certificate Floor
- Create valid artifact, set `accepted_at_seq = EventSeq::new(4)`
- Request admission with `required_at_least = EventSeq::new(5)`
- Expect `ArtifactCertificateStale { digest: D, accepted_at_seq: 4, required_at_least: 5 }`
- Assert Display text contains expected fields
- Assert journal event count unchanged before/after

### Scenario 4: Wrong Digest AND Stale Floor
- Create valid artifact with digest D1, set `accepted_at_seq` stale
- Request admission with digest D2 (D1 != D2) and stale floor
- Expect `ArtifactDigestMismatch` (INV-002 at line 102 fires before staleness check at line 118)
- Assert journal event count unchanged

### Scenario 5: Completed Action Before Restart
- Append RunAccepted, ActionScheduled, ActionCompletedEvent to journal
- Replay via `replay_journal` with `ActionReplayTracker`
- Assert tracker resolves the completed action
- Assert no extra scheduling/redispatch
- Assert journal event vector unchanged

### Scenario 6: Evict Runtime Key, Replay Durable Journal
- Create journal with completed action
- Create `IdempotencyTracker::with_capacity(1)`, insert two tickets (first evicted)
- Replay durable journal with `ActionReplayTracker`
- Assert action remains resolved via durable tracker
- Use actual ticket lookup through runtime API (no public constructors)

### Scenarios 7-8: Strengthen Lifecycle Tests
- Add `before = journal.events_for_run(run).len()` before retry call
- Add `after = journal.events_for_run(run).len()` after retry call
- Assert `after == before`
- Preserve existing typed error assertions

## Current Validation Order (chunk_005_admit_core.rs)
1. `store.load_accepted_artifact(artifact_digest)` 
2. `validate_accepted_artifact_envelope(store, &artifact)` - checks proof digest == artifact digest, calls `verify_stored_artifact`
3. INV-002: `artifact.digest != artifact_digest` -> `ArtifactDigestMismatch`
4. INV-003: `artifact.verification.digest != artifact.digest` (redundant with step 2)
5. `artifact.accepted_at_seq < required_at_least` -> `ArtifactCertificateStale`
6. Capability checks

## Verification
- All 10 original tests must still pass
- 6 new tests added (scenarios 1-6)
- 2 existing tests strengthened (scenarios 7-8)
- Total: 18 tests
- Run: `cargo test -p velvet-ballistics-workspace-tests --test vb_ko29_5_public_idempotency`
