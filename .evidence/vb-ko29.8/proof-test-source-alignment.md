# vb-ko29.8 Proof/Test/Source Alignment

Scope: idempotency / rerun-safety evidence package for `vb-ko29`.

Machine-readable rows are in `.evidence/vb-ko29.8/proof-test-source-alignment.jsonl`. The authoritative detailed bridge remains `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.jsonl`; this file is the final packaging-level alignment index.

| Alignment id | Source surface | Proof / refinement evidence | Behavior evidence | Status |
|---|---|---|---|---|
| `ALIGN-STATIC-IDEMPOTENCY-GATES` | `vb_core::action`, `vb_validate::idempotency_contract`, exported `vb_compile::check_idempotency_gates` | `.evidence/vb-ko29.2/verus-idempotency-decision.log`; `.evidence/vb-ko29.4/vb_validate-kani_decision_001_all_combinations.log` | Internal verifier gate only | PROVED |
| `ALIGN-KEY-AND-CERTIFICATE-GATES` | `vb_core::action::verify_idempotency`; `validate_action_outcome`; storage/runtime certificate evidence | `.evidence/vb-ko29.4/*idempotency*.log`; `.evidence/vb-ko29.2/verus-idempotency-certificate-summary.log` | `.evidence/vb-ko29.5/cargo-test.raw.log` for digest conflict and stale certificate API paths | PROVED_TESTED |
| `ALIGN-JOURNAL-DUPLICATES` | storage journal duplicate detection and per-run replay | `.evidence/vb-ko29.1/logs/IdempotencySafetyDuplicateSuccess.log`; `IdempotencySafetyDuplicateFailure.log`; `IdempotencySafety.log` | `.evidence/vb-ko29.5/cargo-test.raw.log` duplicate success/failure and cross-run journal tests | TESTED |
| `ALIGN-RECOVERY-AND-EVICTION` | `ActionReplayTracker`, recovery replay, runtime idempotency tracker eviction | `.evidence/vb-ko29.1/logs/IdempotencySafetyCrashRecoverDuplicate.log`; `.evidence/vb-ko29.2/verus-idempotency-replay-tracker.log`; `.evidence/vb-ko29.7/loom-idempotency.log` | `.evidence/vb-ko29.5/cargo-test.raw.log` crash-restart and durable fallback tests | PROVED_TESTED |
| `ALIGN-LIFECYCLE-RETRY` | CLI lifecycle retry and lifecycle transition finality | `.evidence/vb-ko29.1/logs/IdempotencySafetyRetryCollision.log`; `IdempotencySafetyStaleTracker.log`; `IdempotencySafetyTerminalFinality.log` | `.evidence/vb-ko29.5/cargo-test.raw.log` retry collision, stale retry, cross-run retry tests | TESTED |
| `ALIGN-DIGEST-AND-ADMISSION` | runtime admission digest/proof/stale-certificate gates, including digest-before-stale ordering | `.evidence/vb-ko29.1/logs/IdempotencySafetyDivergentDigest.log`; `.evidence/vb-ko29.4/vb_core-idempotency_divergent_digest_symbolic_certificate_rejected-r2.log`; `.evidence/vb-ko29.2/verus-idempotency-certificate-summary.log` | `.evidence/vb-ko29.8/idempotency-suite.raw.log`; `.evidence/vb-ko29.8/vb-runtime-admission.raw.log`; black-hat approval `.evidence/vb-ko29.8/black-hat-review.md` | TESTED |
| `ALIGN-SEQUENCE-OVERFLOW` | `crates/vb_storage/src/codec/mod.rs:46-70` checked sequence arithmetic | `.evidence/vb-ko29.1/logs/IdempotencySafetyOverflow.log` | `.evidence/vb-ko29.6/vb_storage-next-seq-overflow.rtk-run.log`, exit `.evidence/vb-ko29.6/vb_storage-next-seq-overflow.rtk-run.exit` | TESTED |
| `ALIGN-MIRI-TRACKER-NO-UB` | `crates/vb_runtime/src/idempotency.rs:49-232` representative tracker operations | `.evidence/vb-ko29.7/miri-idempotency-alt-20260404.log`; toolchain evidence `.evidence/vb-ko29.7/miri-alt-20260404-version.log` | Not a behavior-test requirement | PROVED |

Final reviewer status: proof reviewer, test reviewer, and black-hat reviewer are approved; bridge status is `PROVED: 8`, `TESTED: 12`, `BLOCKED: 0`, `WAIVED: 0`.
