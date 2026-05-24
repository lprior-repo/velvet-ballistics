# vb-ko29.5 Public Idempotency Test Review

STATUS: APPROVED

## Findings

1. **Resolved: direct stale-certificate public API gap is now covered.**  
   `AdmissionError` now exposes the exact public variant `ArtifactCertificateStale { digest, accepted_at_seq, required_at_least }` (`crates/vb_runtime/src/admission.rs:302-313`). `admit_artifact_run_with_certificate_floor` is public and rejects Strict/Journaled artifacts whose `accepted_at_seq` is below the caller-supplied floor (`crates/vb_runtime/src/admission.rs:655-729`). The old `admit_artifact_run` remains backward-compatible by delegating with `EventSeq::ZERO` (`crates/vb_runtime/src/admission.rs:638-653`).

2. **Resolved: stale-certificate behavior test is public, exact, and mutation-sensitive.**  
   `given_stale_certificate_floor_when_strict_admission_runs_then_stale_error_and_journal_unchanged` calls the new public API, constructs a certificate at `EventSeq::new(4)`, supplies floor `EventSeq::new(5)`, asserts the exact `AdmissionError::ArtifactCertificateStale` fields, asserts exact Display text, and checks no journal count change (`crates/workspace_tests/idempotency_suite/tests/vb_ko29_5_public_idempotency.rs:223-273`). Removing the floor comparison or returning a generic admission error would fail this test.

3. **Resolved: runtime mapping compiles with the new variant.**  
   Shard admission error mapping includes `AdmissionError::ArtifactCertificateStale { digest, .. } => RuntimeError::AdmissionArtifactStale { digest }` (`crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:265-270`). The targeted runtime admission test command passed with this variant in the public enum.

4. **No remaining prior blockers.**  
   Crash-restart replay hydration, durable recovery after tracker eviction, cross-scope isolation, retry collision, stale retry key, divergent digest, conflicting proof digest, duplicate success/failure, and retry-required duplicate dispatch denial remain covered by exact public assertions in the suite.

5. **Resolved: stale-ordering regression is covered by public behavior.**  
   `given_wrong_digest_and_stale_floor_when_strict_admission_runs_then_digest_mismatch_wins_and_journal_unchanged` constructs a wrong stored digest and a stale floor, calls the public `admit_artifact_run_with_certificate_floor`, and asserts exact `AdmissionError::ArtifactDigestMismatch { requested, found: stored }` plus unchanged journal count (`crates/workspace_tests/idempotency_suite/tests/vb_ko29_5_public_idempotency.rs:275-315`). This catches the prior black-hat blocker where stale-certificate checking could mask identity mismatch.

## Command Evidence References

- Scenario matrix and no remaining blocker statement: `.evidence/vb-ko29.5/public-idempotency-tests-report.md:28-58`
- Latest public idempotency suite raw run: `.evidence/vb-ko29.8/idempotency-suite.raw.log:9-24` reports `13 passed; 0 failed; 0 ignored`, including the wrong-digest-plus-stale ordering test.
- Latest targeted runtime admission raw run: `.evidence/vb-ko29.8/vb-runtime-admission.raw.log:7-69` reports `60 passed; 0 failed; 0 ignored` for admission-filtered unit tests, with integration target output continuing afterward.
- Black-hat re-review: `.evidence/vb-ko29.8/black-hat-review.md:7-21` approves the stale-ordering fix with no blockers.

## Mutation Thought Experiment

APPROVED. Named tests catch the relevant behavior-destroying mutations:

- Delete duplicate-event detection: duplicate success/failure tests fail on exact `JournalError::DuplicateEvent` and unchanged counts.
- Delete artifact/proof digest binding: divergent digest and conflicting proof-digest tests fail on exact `AdmissionError::ArtifactDigestMismatch` fields.
- Delete stale certificate floor comparison or return the wrong variant/fields/text: `given_stale_certificate_floor_when_strict_admission_runs_then_stale_error_and_journal_unchanged` fails.
- Reorder stale-certificate checking before digest/proof identity checks: `given_wrong_digest_and_stale_floor_when_strict_admission_runs_then_digest_mismatch_wins_and_journal_unchanged` fails because it expects digest mismatch to win over stale.
- Break backward-compatible old admission behavior: existing runtime admission tests under `cargo test -p vb_runtime admission` fail.
- Delete completed-action replay hydration: crash-restart and durable-recovery tests fail because `ActionReplayTracker::is_resolved(...)` remains false.
- Delete lifecycle retry idempotency/staleness checks: duplicate retry and completed-run stale retry tests fail on exact `CoreError` variants and unchanged event counts.
- Collapse journal/lifecycle run scopes: same-sequence journal isolation and two-run lifecycle retry isolation fail.
- Delete retry-required policy duplicate dispatch denial: the second `track_for_policy` assertion fails.
