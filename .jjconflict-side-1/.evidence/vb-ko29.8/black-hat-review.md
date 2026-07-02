# Black Hat Re-Review: vb-ko29 Stale Certificate Ordering Fix

Verdict: APPROVED

## Prior blocker disposition

1. **RESOLVED — digest/proof mismatch now wins before stale-certificate floor.**
   - Source: `crates/vb_runtime/src/admission.rs:667-700`.
   - Review: `admit_artifact_run_with_certificate_floor` now loads and envelope-validates the artifact, then checks `artifact.digest != artifact_digest` at `:675-683` and `artifact.verification.digest != artifact.digest` at `:685-692` before checking `accepted_at_seq < required_at_least` at `:694-700`. The stale floor no longer masks identity mismatch.

2. **RESOLVED — adversarial public ordering test exists and has pass evidence.**
   - Test: `crates/workspace_tests/idempotency_suite/tests/vb_ko29_5_public_idempotency.rs:275-315`.
   - Evidence: `.evidence/vb-ko29.8/idempotency-suite.raw.log:9-24` reports `13 passed; 0 failed`, including `given_wrong_digest_and_stale_floor_when_strict_admission_runs_then_digest_mismatch_wins_and_journal_unchanged ... ok` at `:15`.

3. **RESOLVED — approval evidence no longer hides the prior ordering blocker.**
   - Evidence: `.evidence/vb-ko29.8/proof-review.md:35-38`, `.evidence/vb-ko29.8/proof-review.md:52-53`, `.evidence/vb-ko29.8/final-evidence-decision.md:11-13`, `.evidence/vb-ko29.5/public-idempotency-tests-report.md:36-51`.
   - Review: The refreshed evidence explicitly calls out digest-before-stale ordering and points to the raw pass log. The prior blanket approval over an unresolved ordering gap has been corrected.

## Blockers

None.

STATUS: APPROVED
