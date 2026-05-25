# vb-ko29.5 Public Idempotency Behavior Tests Report

## Executable Suite Placement

`crates/workspace_tests` itself remains excluded because its existing package
depends on deferred optional crates. The bead-specific executable suite lives at
`crates/workspace_tests/idempotency_suite`, preserving the repository rule that
cross-crate tests live under `crates/workspace_tests` while using a minimal
workspace member with only the dependencies needed by this idempotency suite.

## Commands

| Command | Exit | Evidence |
| --- | ---: | --- |
| `rtk cargo test -p velvet-ballistics-idempotency-workspace-tests --test vb_ko29_5_public_idempotency --no-run` | 0 | `.evidence/vb-ko29.5/cargo-test-no-run.log` |
| `rtk cargo test -p velvet-ballistics-idempotency-workspace-tests --test vb_ko29_5_public_idempotency -- --nocapture` | 0 | `.evidence/vb-ko29.5/cargo-test.log` |
| `rtk cargo test -p vb_runtime admission` | 0 | `.evidence/vb-ko29.5/cargo-test-vb-runtime-admission.log` |
| `rtk run "cargo test -p velvet-ballistics-idempotency-workspace-tests --test vb_ko29_5_public_idempotency --no-run"` | 0 | `.evidence/vb-ko29.5/cargo-test-no-run.raw.log` |
| `rtk run "cargo test -p velvet-ballistics-idempotency-workspace-tests --test vb_ko29_5_public_idempotency -- --nocapture"` | 0 | `.evidence/vb-ko29.5/cargo-test.raw.log` |
| `rtk run "cargo test -p vb_runtime admission"` | 0 | `.evidence/vb-ko29.5/cargo-test-vb-runtime-admission.raw.log` |
| `rustfmt --check crates/vb_runtime/src/admission.rs crates/vb_runtime/src/shard/lifecycle/chunk_001.rs crates/workspace_tests/idempotency_suite/tests/vb_ko29_5_public_idempotency.rs` | 0 | `.evidence/vb-ko29.8/touched-rustfmt-check.raw.log` |
| `rtk run "cargo test -p velvet-ballistics-idempotency-workspace-tests --test vb_ko29_5_public_idempotency -- --nocapture"` | 0 | `.evidence/vb-ko29.8/idempotency-suite.raw.log` |
| `rtk run "cargo test -p vb_runtime admission"` | 0 | `.evidence/vb-ko29.8/vb-runtime-admission.raw.log` |

Note: `rtk cargo` compacted the success output; the `.raw.log` files are the
non-empty passthrough cargo stdout/stderr captured through `rtk run`.

## Scenario Status Matrix

| Scenario | Status | Test / Evidence | Exact assertion |
| --- | --- | --- | --- |
| Duplicate success journal append | PASS | `given_duplicate_success_event_when_appended_then_duplicate_event_variant_and_count_unchanged` | `JournalError::DuplicateEvent { run, seq: EventSeq::new(0) }`, event count unchanged |
| Duplicate failure journal append | PASS | `given_duplicate_failure_event_when_appended_then_duplicate_event_variant_and_count_unchanged` | `JournalError::DuplicateEvent { run, seq: EventSeq::new(1) }`, event count unchanged |
| Divergent accepted-artifact digest | PASS | `given_divergent_artifact_digest_when_strict_admission_runs_then_digest_mismatch_is_exact` | `AdmissionError::ArtifactDigestMismatch { requested, found }` |
| Conflicting certificate/proof digest | PASS | `given_conflicting_certificate_proof_digest_when_strict_admission_runs_then_digest_mismatch_is_exact` | `AdmissionError::ArtifactDigestMismatch { requested, found: proof_digest }` |
| Stale certificate direct public admission path | PASS | `given_stale_certificate_floor_when_strict_admission_runs_then_stale_error_and_journal_unchanged` | `AdmissionError::ArtifactCertificateStale { digest, accepted_at_seq: EventSeq::new(4), required_at_least: EventSeq::new(5) }`, exact Display text, journal count unchanged |
| Wrong digest plus stale certificate floor ordering | PASS | `given_wrong_digest_and_stale_floor_when_strict_admission_runs_then_digest_mismatch_wins_and_journal_unchanged` | `AdmissionError::ArtifactDigestMismatch { requested, found: stored }` wins over stale, journal count unchanged |
| Crash restart no redispatch | PASS | `given_completed_action_before_restart_when_replayed_then_no_redispatch_and_event_count_stable` | `replay_journal` itself marks `ActionReplayTracker::is_resolved(action, step) == true`; scheduled count remains 1 |
| Tracker eviction durable fallback | PASS | `given_evicted_runtime_key_when_durable_journal_replayed_then_recovery_resolves_action` | Volatile tracker evicts, then durable `replay_journal` resolves completed action in a fresh `ActionReplayTracker` |
| CLI retry collision | PASS | `given_failed_run_retried_twice_when_cli_retry_collides_then_duplicate_error_and_no_append` | `CoreError::LifecycleDuplicateRequest { code: LIFECYCLE_DUPLICATE_REQUEST_CODE, command: Some("retry") }`, event count unchanged |
| CLI stale retry key | PASS | `given_completed_run_when_cli_retry_uses_stale_key_then_stale_error_and_no_append` | `CoreError::LifecycleStaleRequest { code: LIFECYCLE_STALE_REQUEST_CODE, command: Some("retry") }`, event count unchanged |
| Cross-run durable journal isolation | PASS | `given_same_sequence_in_different_runs_when_appended_then_cross_scope_journals_are_isolated` | Each run replays only its own same-sequence event |
| Cross-run lifecycle retry isolation | PASS | `given_distinct_run_scopes_when_lifecycle_retry_runs_then_each_scope_appends_its_own_retry` | Each failed run appends exactly one `RunRetried` independently |
| Retry-required policy duplicate dispatch denial | PASS | `given_retry_required_policy_when_same_key_dispatched_twice_then_second_dispatch_is_denied` | first `track_for_policy` true, second false, policy completion true |

## Result

The bead-specific workspace test suite compiles and runs as a dedicated workspace
member. Refreshed raw cargo evidence reports `13 passed; 0 failed`. The targeted
runtime admission raw evidence reports all admission-filtered tests passed. The
black-hat ordering blocker is covered by a public wrong-digest-plus-stale test.

## Remaining Blockers

None for the stale-certificate public admission API gap. The new public
`admit_artifact_run_with_certificate_floor` path denies stale accepted artifacts
when callers supply a freshness floor, while existing `admit_artifact_run` remains
backward-compatible for placeholder `accepted_at_seq == 0` artifacts.
