# Wave 3 Rust-Contract Audit — Chunk 11 (agent-11)

Scope: 9 bug IDs (storage / recovery / codec / digest).
Working directory: /home/lewis/src/velvet-ballistics (verified git root).
Audit date: 2026-06-24.

## Summary Table

| bug-id | pri | source-fix | test | typestate | invariant | error-taxonomy | targeted-cmd | result | verdict | evidence |
|---|---|---|---|---|---|---|---|---|---|---|
| vb-kzpnj | P2 | hydrate_tests.rs (133 lines, NOT registered in lib.rs); bug-specified tests at lines 281/287/374 absent | `cargo test -p vb_storage --lib hydrate` (filtered, 0 ran) | **BROKEN**: file is dead-code; tests named `validate_snapshot_metadata_accepts_matching_run` etc. are different from bug-spec `validate_tail_first_seq_contiguous_accepts_snapshot_plus_one/_empty_tail` | **BROKEN**: 6 `assert!(result.is_ok(), …)` calls at lines 23, 46, 69, 99, 105, 111 remain — matches!(result, Ok(())) pattern not applied | OK: `SnapshotRecoveryInputViolation` enum has typed variants | `cargo test -p vb_storage --lib --no-fail-fast hydrate` | 0 matched | **NOT-PATCHED** | hydrate_tests.rs not in lib.rs mod tree; bug-spec test names not found anywhere in crates/ |
| vb-l60gb | P1 | `map_artifact_envelope_error` in admission.rs:404-422; `AdmissionError` enum has 15+ variants | `cargo test -p vb_runtime --lib --no-fail-fast admission` | OK: typestate preserved; `ArtifactEnvelopeError → AdmissionError` mapping is one-to-one | OK: each distinct failure preserves its semantic identity | **OK**: distinct variants — `ArtifactInvalidGateCount`, `ArtifactInvalidProofFlag`, `ArtifactDigestMismatch`, `ArtifactEnvelopeDecodeFailed`, `ArtifactCertificateStale`, `ArtifactNotFound`, `CapabilityCountMismatch`, `CapabilityDenied`, `BudgetPolicyExceeded`, `ResourceCapacityExceeded`, `ResourceBudgetOverflow/Underflow`, `ResourceBudgetInvalidCapacity`, `ResourceStepCeilingExceeded`, `ResourcePerTickCeilingExceeded` | `cargo test -p vb_runtime --lib --no-fail-fast admission` | 79 passed | **PATCHED** | admission.rs lines 232-275; no `AdmissionArtifactInvalid` collapse; `map_admission_error` renamed to `map_artifact_envelope_error` |
| vb-lrxyq | P0 | `codec_miri_tests.rs` exists (432 lines, registered at lib.rs:27 under `#[cfg(miri)]`) | `cargo +nightly miri test -p vb_storage` (out of scope for regular test cmd) | OK: `RecordKind`, `JournalError` typestates preserved | OK: `panic_free_decode_header/_record/_verify_digest` enforce panic-freedom via `catch_unwind` | OK: returns `Result<(), JournalError>` for malformed input | `cargo test -p vb_storage --lib --no-fail-fast` | 1270 passed | **PATCHED** | NOTES flag confirms "FALSE PREMISE: file already exists at 432 lines"; cfg(miri) module compiles |
| vb-maupz | P3 | `submit_artifact_with_contracts` in admission.rs:327-417; post-`persist_strict()` no longer calls `verify_persisted_artifact_present()`; reads back via `journal.compiled_ir()` (lines 409-414) | `cargo test -p vb_storage --lib --no-fail-fast admission` | OK: `AcceptedArtifact` typestate preserved through full pipeline | OK: post-commit read-back is part of the same function, no separate failure window | OK: `JournalError::ArtifactMalformed` returned on missing/undreadable artifact | `cargo test -p vb_storage --lib --no-fail-fast admission` | 82 passed | **PATCHED** | admission.rs:406-414 — no separate verify call after persist_strict; function renamed from `submit_checked_artifact_with_evidence` |
| vb-n5ctl | P3 | `trim_events_for_run` in trimming/logic.rs:99-106 returns `TrimStatus::NoOp` without `batch.commit()` when `deleted_count == 0` | `cargo test -p vb_storage --lib --no-fail-fast trim` | OK: `TrimStatus::{NoOp, Trimmed}` typestate preserved | OK: no empty batch commit | OK: typed `TrimError` (NoDurableSnapshot, IncompleteTrim, RetentionPolicyBlocks, Journal) | `cargo test -p vb_storage --lib --no-fail-fast trim_zero_deletes_returns_noop_when_skip_noop_disabled` | 1 passed (38 trim-suite total) | **PATCHED** | logic.rs:99-106 explicitly short-circuits empty batches; tests in trimming/tests.rs assert `TrimStatus::NoOp` |
| vb-nsqpd | P2 | `queue/batch.rs` (37 lines, NOT 72 as close reason claims); `BatchBuilder::push` still uses unbounded `Vec<JournalEvent>::push` | `cargo test -p vb_storage --lib --no-fail-fast batch_builder` | OK: `JournalEvent` typestate preserved | **BROKEN**: unbounded growth violates "bounded Vec/HashMap" engineering rule | **MISMATCH**: `QueueFull` exists in writer.rs but NOT in batch.rs; `try_push` not implemented | `cargo test -p vb_storage --lib --no-fail-fast batch_builder_with_capacity` | 0 matched | **NOT-PATCHED** | batch.rs has only `new/push/len/is_empty/as_slice`; no `with_capacity`, no `try_push`, no `QueueFull`; close reason claims file is 18-72 lines but actual file is 1-37 |
| vb-odiyq | P2 | `probe()` in journal/chunk_003.rs:18-26 calls `self.journal.probe_health()` and `self.queue.probe_accepting_writes()`; `probe_health` in journal/core.rs:182-194 reads from all 9 keyspaces | `cargo test -p vb_runtime --lib --no-fail-fast storage_runtime_journal_probe_delegates` | OK: `DurabilityProfile::{Strict, Journaled}` typestate preserved | OK: read-only probes via `contains_key(empty_key)` per keyspace | OK: returns `Result<(), JournalError>` / `RuntimeError::from(...)` | `cargo test -p vb_runtime --lib --no-fail-fast storage_runtime_journal_probe_delegates_to_fjall_health` | 1 passed | **PATCHED** | chunk_003.rs:18-26; core.rs:182-194; journal::tests::storage_runtime_journal_probe_delegates_to_fjall_health green |
| vb-p1ogw | P3 | Duplicate of `vb-pctwr` (RE-020) — parent IN_PROGRESS; `storage_event` in chunk_002.rs:259-274 still clones event 3x via run/action/boundary matchers | `cargo test -p vb_runtime --lib --no-fail-fast journal` | OK: event typestates preserved via matchers | OK: clone is intentional trade-off, not an invariant violation | OK: `RuntimeError::UnsupportedOperation` returned for unsequenced append | `cargo test -p vb_runtime --lib --no-fail-fast journal` | 35 passed | **UNKNOWN** | bead closed as duplicate; parent bead vb-pctwr IN_PROGRESS; actual clone-reduction fix not yet landed |
| vb-p20gw | P3 | Duplicate of `vb-h62w4` (RA-030) — parent CLOSED; `answer_pending_ask_slot` function no longer exists in codebase (was at runtime_ask.rs:11-35 per bead spec) | `cargo test -p vb_runtime --lib --no-fail-fast ask` | OK: ask-related typestates intact | OK: routing tested via `shard_ask_answered_with_i64_value` and `vb1u88_ask_answer_unknown_run_not_found` | OK: not-found returns typed error | `cargo test -p vb_runtime --lib --no-fail-fast answer_pending` | 0 matched; 80 ask tests passed | **UNKNOWN** | function renamed/removed; cannot verify the original fix without parent-bead artifact |

## Counts

- bugs-checked: 9
- PATCHED: 5 (vb-l60gb, vb-lrxyq, vb-maupz, vb-n5ctl, vb-odiyq)
- NOT-PATCHED: 2 (vb-kzpnj, vb-nsqpd)
- UNKNOWN: 2 (vb-p1ogw, vb-p20gw — both duplicates; parent bead verification required)
- PARTIAL: 0
- pass: 5
- fail: 2
- partial: 0
- unknown: 2

## Typestate-Broken Cases

- **vb-kzpnj**: hydrate_tests.rs is dead code (not registered in lib.rs:117-130 mod tree). The bug-fix evidence claims `matches!(result, Ok(()))` at lines 281/287/374 but the file is only 133 lines and contains `assert!(result.is_ok(), …)` patterns at lines 23, 46, 69, 99, 105, 111 — none converted to the documented fix pattern.
- **vb-nsqpd**: BatchBuilder typestate contract is missing the bounded-capacity invariant. The struct has `events: Vec<JournalEvent>` with no capacity guard; `push()` is infallible and unbounded. Close reason claims `with_capacity + try_push + QueueFull` at queue/batch.rs:18-72, but actual file is 37 lines with no capacity API.

## Error-Taxonomy Mismatch Cases

- **vb-nsqpd**: `JournalError::QueueFull` is reachable only through `JournalWriterQueue::enqueue_*` (writer.rs:81, 125), NOT through `BatchBuilder`. The `BatchBuilder` API has no failure path; it silently grows until OOM. This violates the §17 typed-error contract for batch builders per the engineering rule "bounded Vec/HashMap".

## Top-3 NOT-PATCHED with one-line reason

1. **vb-nsqpd** (SA-006 BatchBuilder unbounded growth): `queue/batch.rs` has no `with_capacity`, no `try_push`, no `QueueFull` — actual file is 37 lines vs. close reason's claim of 72 lines with bounded API.
2. **vb-kzpnj** (E-003 is_ok() smoke tests in hydrate_tests.rs): `hydrate_tests.rs` is unregistered dead code in lib.rs and still uses `assert!(result.is_ok(), …)` at lines 23/46/69/99/105/111; bug-specified test names absent from crates/.
3. (n/a — only 2 NOT-PATCHED; UNKNOWN count of 2 for vb-p1ogw/vb-p20gw due to duplicate-bead parent dependencies)

## File Path Written

`/home/lewis/src/velvet-ballistics/to-fix/wave3/agent-11-rust-contract.md`