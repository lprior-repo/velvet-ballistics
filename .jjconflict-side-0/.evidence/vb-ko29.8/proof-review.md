# vb-ko29.8 Final Proof Review: vb-ko29 Idempotency / Rerun Safety

Reviewer: proof-reviewer  
Date: 2026-05-24  
Scope: epic `vb-ko29` final proof package, including children `vb-ko29.1`, `vb-ko29.2`, `vb-ko29.3`, `vb-ko29.4`, `vb-ko29.5`, `vb-ko29.6`, `vb-ko29.7`.  
Review mode: proof artifacts and evidence only. No production code, tests, proof code, harnesses, specs, models, dependencies, beads, or CI were modified.

## Verdict

Verdict: `APPROVED`

The prior blockers are closed by source/proof/test bridge rows backed by raw evidence. The bridge now reports `PROVED: 8`, `TESTED: 12`, `BLOCKED: 0`, `WAIVED: 0`; reviewed evidence supports that summary. No residual blocker prevents proof-package approval.

## Evidence inspected

- TLA+: `verification/tla/IdempotencySafety.tla`, `.evidence/vb-ko29.1/tla-idempotency-report.md`, `.evidence/vb-ko29.1/logs/*.log`.
- Verus: `verification/verus/idempotency_decision.rs`, `verification/verus/idempotency_replay_tracker.rs`, `verification/verus/idempotency_certificate_summary.rs`, `.evidence/vb-ko29.2/verus-idempotency-binding-report.md`, `.evidence/vb-ko29.2/verus-idempotency-binding-map.jsonl`, raw Verus logs and trust scan.
- Kani: `crates/vb_core/src/kani_idempotency_gates.rs`, `.evidence/vb-ko29.3/kani-compile-unblock-report.md`, `.evidence/vb-ko29.4/kani-idempotency-generators-report.md`, raw Kani logs.
- Public tests / adversarial review: `.evidence/vb-ko29.5/public-idempotency-tests-report.md`, `.evidence/vb-ko29.5/test-review.md`, `.evidence/vb-ko29.5/cargo-test.raw.log`, `.evidence/vb-ko29.5/cargo-test-vb-runtime-admission.raw.log`, `.evidence/vb-ko29.8/idempotency-suite.raw.log`, `.evidence/vb-ko29.8/vb-runtime-admission.raw.log`, `.evidence/vb-ko29.8/black-hat-review.md`.
- Bridge: `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.md`, `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.jsonl`, `.evidence/vb-ko29.6/vb_storage-next-seq-overflow.rtk-run.log`, `.evidence/vb-ko29.6/vb_storage-next-seq-overflow.rtk-run.exit`.
- Loom/Miri: `crates/vb_runtime/src/models/loom/idempotency_retry_eviction.rs`, `crates/vb_runtime/tests/vb_ko29_7_idempotency_miri.rs`, `.evidence/vb-ko29.7/loom-miri-idempotency-report.md`, `.evidence/vb-ko29.7/loom-idempotency.log`, `.evidence/vb-ko29.7/miri-idempotency-alt-20260404.log`, `.evidence/vb-ko29.7/miri-alt-20260404-version.log`.

## Findings

1. **APPROVED — `IDEMP-NEXTSEQ-OVERFLOW-FAILSAFE` is now mapped to executable Rust behavior.**  
   Artifact refs: `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.md:49-56`, `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.jsonl:15`; source refs `crates/vb_storage/src/codec/mod.rs:46-51`, `crates/vb_storage/src/codec/mod.rs:53-70`; test refs `crates/vb_storage/src/codec/tests.rs:538-555`, `crates/vb_storage/src/tests.rs:1319-1327`.  
   Evidence: `.evidence/vb-ko29.1/logs/IdempotencySafetyOverflow.log:23-24`; `.evidence/vb-ko29.6/vb_storage-next-seq-overflow.rtk-run.log:4-10`; `.evidence/vb-ko29.6/vb_storage-next-seq-overflow.rtk-run.exit:1`.  
   Review: TLA+ still proves bounded `SequenceOverflowFailSafe`; Rust now uses `checked_add` and returns typed `JournalError::SequenceOverflow` at `u64::MAX`, with targeted tests passing. The superseded Kani harness-filter miss is explicitly not counted as proof.

2. **APPROVED — `IDEMP-MIRI-TRACKER-NO-UB` has raw Miri pass evidence.**  
   Artifact refs: `.evidence/vb-ko29.7/loom-miri-idempotency-report.md:10-19`, `.evidence/vb-ko29.7/loom-miri-idempotency-report.md:28-38`, `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.jsonl:16`; source/test refs `crates/vb_runtime/tests/vb_ko29_7_idempotency_miri.rs:25-45`.  
   Evidence: `.evidence/vb-ko29.7/miri-idempotency-alt-20260404.log:163-169`; `.evidence/vb-ko29.7/miri-alt-20260404-version.log:1-8`.  
   Review: The exact toolchain `nightly-2026-04-04` ran the scoped Miri test successfully (`1 passed; 0 failed`). The default `+nightly` rust-src failure is retained only as superseded tooling evidence and is not represented as a final pass.

3. **APPROVED — direct stale-certificate admission is now public, tested, and ordered behind digest binding.**  
   Artifact refs: `.evidence/vb-ko29.5/test-review.md:7-20`, `.evidence/vb-ko29.5/public-idempotency-tests-report.md:28-58`, `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.md:30-48`, `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.jsonl:17`, `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.jsonl:20`, `.evidence/vb-ko29.8/black-hat-review.md:7-23`; source refs `crates/vb_runtime/src/admission.rs:302-313`, `crates/vb_runtime/src/admission.rs:667-700`; test refs `crates/workspace_tests/idempotency_suite/tests/vb_ko29_5_public_idempotency.rs:223-315`.  
   Evidence: `.evidence/vb-ko29.8/idempotency-suite.raw.log:9-24`; `.evidence/vb-ko29.8/vb-runtime-admission.raw.log:7-69`; `.evidence/vb-ko29.8/touched-rustfmt-check.raw.log`.  
   Review: The public API exposes `AdmissionError::ArtifactCertificateStale` and `admit_artifact_run_with_certificate_floor`. The public stale test still asserts exact variant fields, exact display text, and unchanged journal count. The added adversarial public test proves wrong artifact digest plus stale `accepted_at_seq` returns `AdmissionError::ArtifactDigestMismatch`, not stale. Black-hat re-review is `APPROVED` with blockers `None`. Refreshed raw evidence reports `13 passed; 0 failed` for the bead-specific suite.

4. **APPROVED — bridge coverage has no remaining blocked or waived rows.**  
   Artifact refs: `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.md:9-16`, `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.md:26-28`.  
   Review: The machine-readable bridge has 20 rows: 8 `PROVED`, 12 `TESTED`, 0 `BLOCKED`, 0 `WAIVED`. Required dimensions are present per bridge report. TLA+ rows are not counted alone for implementation closure where Rust mapping is required.

5. **APPROVED — bridge JSONL notes cleanup is complete.**  
   Artifact refs: `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.jsonl:5`, `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.jsonl:14`, `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.jsonl:18`.  
   Review: The prior stale wording about direct stale-certificate admission being blocked has been removed. Related notes now point to `IDEMP-DIRECT-STALE-CERTIFICATE-ADMISSION` as the separate closure row, consistent with bridge status `TESTED` and bridge summary `BLOCKED: 0`.

## False-claim checks

- **Miri pass claim:** supported. `.evidence/vb-ko29.7/miri-idempotency-alt-20260404.log:166-169` reports the Miri test passed under `nightly-2026-04-04`; version evidence is present in `.evidence/vb-ko29.7/miri-alt-20260404-version.log:1-8`.
- **Default Miri failure handling:** honest. The default `+nightly` failure is classified as `SUPERSEDED_TOOLING_NOTE`, not as a pass.
- **Direct stale-certificate admission claim:** supported. Public API/source refs exist and `.evidence/vb-ko29.8/idempotency-suite.raw.log:18-24` shows the stale-certificate test passed.
- **Digest-before-stale ordering claim:** supported. `crates/vb_runtime/src/admission.rs:678-699` checks artifact/proof digest identity before the freshness floor; `.evidence/vb-ko29.8/idempotency-suite.raw.log:15-24` shows the combined wrong-digest-plus-stale public test passed.
- **nextSeq/EventSeq overflow claim:** supported. TLA overflow evidence remains passing, and Rust `codec::next_seq` uses `checked_add` with tests proving `JournalError::SequenceOverflow` at `u64::MAX`.
- **Bridge tuple counts / blocked count:** supported by bridge report (`PROVED: 8`, `TESTED: 12`, `BLOCKED: 0`, `WAIVED: 0`) and no row status reviewed as final `BLOCKED`.
- **Verus trust expansion:** prior trust scan remains acceptable; Verus artifacts are mirror/binding evidence, not direct production-body proofs, and are treated only within that declared boundary.
- **Kani non-vacuity:** scoped Kani claims remain bounded and cover-backed; no unbounded generalization is needed for final blocker closure.
- **Loom scope:** Loom remains bounded (`max_branches = 1000`, `preemption_bound = Some(3)`) and documents the volatile capacity-one stale-key gap; durable fallback is covered separately by public tests.

## Residual blockers

None.

STATUS: APPROVED
