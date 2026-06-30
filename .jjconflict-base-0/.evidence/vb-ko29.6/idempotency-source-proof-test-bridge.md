# vb-ko29.6 Idempotency Source→Proof→Test Bridge

## Scope

This bridge maps idempotency permutations to concrete Rust source refs, verifier artifacts/evidence, and public behavior tests where they exist. It is machine-readable in `idempotency-source-proof-test-bridge.jsonl` using schema `idempotency_source_proof_test_bridge/v1`.

No `REVIEW_INPUT` artifact is counted as proof. TLA+ rows are treated as temporal model evidence and require concrete Rust source/test mapping for implementation closure.

## Status summary

- `PROVED`: 8 tuples
- `TESTED`: 12 tuples
- `BLOCKED`: 0 tuples
- `WAIVED`: 0 tuples

Every JSONL row carries the required dimensions: `run`, `action`, `step`, `key`, `digest`, `certificate`, `lifecycle`, `replaySeq`, `nextSeq`, `retry`, `sideEffect`, `crash`, `tracker`, `eviction`, `durability`, and `inputKind`.

## Primary evidence inputs inspected

- TLA+: `.evidence/vb-ko29.1/tla-idempotency-report.md` and `verification/tla/IdempotencySafety.tla`
- Verus: `.evidence/vb-ko29.2/verus-idempotency-binding-report.md`, `.evidence/vb-ko29.2/verus-idempotency-binding-map.jsonl`, `verification/verus/idempotency_*.rs`
- Kani: `.evidence/vb-ko29.3/kani-compile-unblock-report.md`, `.evidence/vb-ko29.4/kani-idempotency-generators-report.md`, `crates/vb_core/src/kani_idempotency_gates.rs`, `crates/vb_validate/src/kani_idempotency_contract.rs`
- Public tests: `.evidence/vb-ko29.5/public-idempotency-tests-report.md`, `.evidence/vb-ko29.5/test-review.md`, `.evidence/vb-ko29.5/cargo-test.raw.log`, `.evidence/vb-ko29.8/idempotency-suite.raw.log`, `.evidence/vb-ko29.8/black-hat-review.md`, `crates/workspace_tests/idempotency_suite/tests/vb_ko29_5_public_idempotency.rs`
- Loom/Miri: `.evidence/vb-ko29.7/loom-miri-idempotency-report.md`, `crates/vb_runtime/src/models/loom/idempotency_retry_eviction.rs`, `crates/vb_runtime/tests/vb_ko29_7_idempotency_miri.rs`

## Blocking / unmapped dimensions

None remain in this bridge.

## Direct stale-certificate admission closure

- `IDEMP-DIRECT-STALE-CERTIFICATE-ADMISSION` is now `TESTED` instead of blocked.
- Source mapping:
  - `crates/vb_runtime/src/admission.rs:302-313` defines `AdmissionError::ArtifactCertificateStale`.
  - `crates/vb_runtime/src/admission.rs:655-700` implements `admit_artifact_run_with_certificate_floor`, checks digest/proof identity first, then rejects `accepted_at_seq < required_at_least`.
  - `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:268-270` maps stale admission into `RuntimeError::AdmissionArtifactStale` at the shard boundary.
- Public behavior test: `crates/workspace_tests/idempotency_suite/tests/vb_ko29_5_public_idempotency.rs:223-273` asserts exact stale error fields, exact `Display` text, and unchanged journal count.
- Ordering behavior test: `crates/workspace_tests/idempotency_suite/tests/vb_ko29_5_public_idempotency.rs:275-315` asserts wrong digest plus stale floor returns `ArtifactDigestMismatch` and leaves the journal unchanged.
- Raw evidence: `.evidence/vb-ko29.8/idempotency-suite.raw.log` reports `13 passed; 0 failed`.
- Status is `TESTED` only; no proof evidence is claimed for this newly exposed public API path.

## Stale-certificate ordering closure

- Added `IDEMP-DIGEST-BEFORE-STALE-CERTIFICATE-ORDERING` as `TESTED`.
- Source mapping: `crates/vb_runtime/src/admission.rs:675-700` checks artifact digest mismatch and proof digest mismatch before the stale certificate floor.
- Public behavior test: `given_wrong_digest_and_stale_floor_when_strict_admission_runs_then_digest_mismatch_wins_and_journal_unchanged` at `crates/workspace_tests/idempotency_suite/tests/vb_ko29_5_public_idempotency.rs:275-315`.
- Raw evidence: `.evidence/vb-ko29.8/idempotency-suite.raw.log:9-24` reports `13 passed; 0 failed`, including the ordering test.
- Black-hat re-review: `.evidence/vb-ko29.8/black-hat-review.md` verdict `APPROVED`; blockers `None`.

## Miri closure

- `IDEMP-MIRI-TRACKER-NO-UB` is now `PROVED` instead of blocked.
- Passing command: `cargo +nightly-2026-04-04 miri test -p vb_runtime --test vb_ko29_7_idempotency_miri -- --nocapture`.
- Raw evidence: `.evidence/vb-ko29.7/miri-idempotency-alt-20260404.log` reports `1 passed; 0 failed`.
- Toolchain evidence: `.evidence/vb-ko29.7/miri-alt-20260404-version.log` (`rustc 1.96.0-nightly (2972b5e59 2026-04-03)`, `miri 0.1.0`).
- Superseded tooling note: default `+nightly` rust-src layout remains broken, but it is no longer the final Miri classification.

## NextSeq / EventSeq overflow closure

- `IDEMP-NEXTSEQ-OVERFLOW-FAILSAFE` is now `TESTED` instead of blocked.
- TLA+ evidence remains `.evidence/vb-ko29.1/logs/IdempotencySafetyOverflow.log` for bounded `SequenceOverflowFailSafe`.
- Rust source mapping: `crates/vb_storage/src/codec/mod.rs:46-51` uses `checked_add` and returns `JournalError::SequenceOverflow` at `u64::MAX`.
- Storage tests: `crates/vb_storage/src/codec/tests.rs:538-555` and `crates/vb_storage/src/tests.rs:1319-1327`.
- Raw command evidence: `.evidence/vb-ko29.6/vb_storage-next-seq-overflow.rtk-run.log`, exit `.evidence/vb-ko29.6/vb_storage-next-seq-overflow.rtk-run.exit` (`0`).
- Non-counted/superseded: Kani harness-filter attempt `kani_next_seq_monotonic_for_all_values` did not match any harness and exited `1` (`.evidence/vb-ko29.6/vb_storage-kani-next-seq-monotonic.log`, `.exit`); it is not counted as proof.

## Final public test status reflected

- `.evidence/vb-ko29.5/test-review.md` verdict: `APPROVED`.
- Raw execution evidence: `.evidence/vb-ko29.8/idempotency-suite.raw.log` reports `13 passed; 0 failed`.
- Final public mappings include these `TESTED` closure rows:
  - `IDEMP-DIRECT-STALE-CERTIFICATE-ADMISSION`
  - `IDEMP-DIGEST-BEFORE-STALE-CERTIFICATE-ORDERING`
  - `IDEMP-PROOF-DIGEST-CONFLICT`
  - `IDEMP-LIFECYCLE-CROSS-RUN-RETRY-ISOLATION`
- Updated stale/digest ordering behavior evidence paths now point at `.evidence/vb-ko29.8/idempotency-suite.raw.log` rather than compacted `rtk cargo` output.
- No public-test blockers remain.

## Reviewer handoff

Provide these files to `proof-reviewer`:

- `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.jsonl`
- `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.md`
- Source evidence reports listed above

Expected review focus: exactness of source refs and whether any `TESTED` rows need stronger formal closure. There are no remaining `BLOCKED` rows in this bridge.
