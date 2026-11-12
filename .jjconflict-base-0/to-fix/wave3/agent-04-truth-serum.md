# Wave 3 — Agent 04: Truth-Serum Audit (Storage/Recovery/Codec/Digest)

**Working dir:** `/home/lewis/src/velvet-ballistics`
**Bug chunk (8):** `vb-1rqz7.5, vb-1rqz7.6, vb-1rqz7.7, vb-1rqz7.8, vb-1rqz7.9, vb-28qw9, vb-294mf, vb-2eprq`
**Scope:** Read-only audit of bead acceptance bullets against actual source + test evidence.
**Tools used:** `bd show`, `cargo test -p vb_storage --lib --no-fail-fast`, `grep` / `rg` for source + test signal.

## Per-bug table

| bug-id | pri | acceptance-bullet | evidence-cmd | raw-result | verdict | hallucination? |
|---|---|---|---|---|---|---|
| vb-1rqz7.5 | P0 | SR-002: public admission/summary recovery APIs explicitly choose full-history or tail-only source | `grep` for tail-only entrypoint + `cargo test -p vb_storage --lib --no-fail-fast` | `recover_runtime_summary`/`recover_runtime_frame_seed`/`recover_run_admission` (`recover.rs:140,195,207`) all funnel through `events_for_run` → `events_for_run_bounded` → `latest_durable_snapshot_seq + next_seq` (`journal/replay.rs:77-85`); `events_for_run_starts_after_snapshot_when_pre_snapshot_trimmed` + 3 sibling tests (`journal/tests.rs:1759,1827,1860`) confirm explicit tail-only choice; 1270 passed / 0 failed | PATCHED | no |
| vb-1rqz7.6 | P0 | SR-003: snapshot-tail replay decodes `SlotWrittenEvent.extra` and restores secret taint | `grep` for `decode_slot_written_extra` + `write_slot_with_taint` | `summary.rs:572 record_slot_write` calls `recovered_slot_taint` → `decoded_slot_taint` → `crate::slot_extra::decode_slot_written_extra` (line 720), unwraps `DecodedSlotWrittenExtra::Envelope` and stores `envelope.taint`; `hydrate_support.rs:395` calls `frame.read_taint` + `write_slot_with_taint` for `SlotWrittenEvent`; tests `apply_tail_events_fails_closed_when_taint_read_fails` (`recovery/tests.rs:2669`) + `hydrate_run_frame_taint_preserved_when_tail_has_no_taint` (line 2638) cover both paths; 1270 passed | PATCHED | no |
| vb-1rqz7.7 | P0 | SR-005: dimension derivation includes `RunAnswered` and `ActionScheduledTicket` output slots | `grep` for `ActionScheduledTicket.*output` / `RunAnswered.*slot` in dimension match arm | `hydrate_support.rs:207-238 derive_dimensions_from_snapshot_and_tail` updates `max_slot` from `ActionCompletedEnvelope { output }`, `SlotWrittenEvent { slot }`, `RunFinished { result }` — `ActionScheduledTicket` match arm (line 223-226) updates **only** `max_step` from `ticket.step`, **never** `max_slot` from `output`; `RunAnswered { slot_idx }` has **no** match arm at all in either the hydrate dimension derivation or `summary.rs:50-87 apply_summary_event` (line 86 explicitly matches `RunAnswered => {}`); 1270 tests pass but no regression covers the gap | NOT-PATCHED | no |
| vb-1rqz7.8 | P0 | SR-006: tail validation rejects a gap at `snapshot.seq + 1` | `grep` for `validate_contiguous_sequences` + tail-gap test | `replay.rs:88-119 events_for_run_from` keys `start_seq = next_seq(snapshot.seq)` (line 79) so the first required key is `snapshot.seq + 1`; `validate_replay_sequence` (line 122-134) walks `expected` and returns `JournalError::SequenceGap` on mismatch; `replay/core.rs:167 validate_contiguous_sequences` mirrors this for the full-journal path; regression `events_for_run_detects_missing_first_tail_event_after_snapshot` (`journal/tests.rs:1827`) asserts exactly `expected=3, actual=4` after a snapshot at seq 2; 1270 passed | PATCHED | no |
| vb-1rqz7.9 | P0 | SR-007: pending action reconstruction matches accumulator semantics for `ActionFailedEvent` | `grep` for `record_action_failed` + `pending_actions.remove` | `summary.rs:684-691 record_action_failed` calls `action_tracker.mark_failed` **and** `self.pending_actions.remove(&(action, step))` — matches accumulator semantics; `replay/summary/tests.rs:65` + `recovery_unit_tests.rs:258-296` cover both pending and unsupported paths; 1270 passed | PATCHED | no |
| vb-28qw9 | P2 | SA-007: `validate_compiled_ir_record` checks `record.metadata_hash` against computed hash via `validate_artifact_metadata_hash_binding` at `admission/record.rs:44` | `rg` for `validate_compiled_ir_record` / `validate_artifact_metadata_hash_binding` / `metadata_hash` across `crates/vb_storage/src/**` | `CompiledIrRecord` (`records.rs:246-251`) has only `digest` + `ir` — **no `metadata_hash` field**; `crates/vb_storage/src/admission/record.rs` does not exist (only top-level `admission.rs` + `admission/tests.rs`); zero matches for `validate_compiled_ir_record`, `validate_artifact_metadata_hash_binding`, or `metadata_hash` in source **or** tests; 1270 tests pass but no regression covers the claimed behavior — close reason references a non-existent file and non-existent functions | NOT-PATCHED | **yes** |
| vb-294mf | P1 | RQ-W0-16: rehydration inspects `seed.summary.terminal` and rejects `FrameSeed` with `terminal = Some(...)` | `rg` for `seed.summary.terminal` / `Runtime::recover_one_run` / terminal guard in `hydrate_run_frame` | No function named `Runtime::recover_one_run` exists; the live path is `DurableFrameRecoveryBoundary::hydrate_run_frame` (`vb_runtime/src/recovery.rs:63-70`) which calls `reject_unsupported_live_frame_state` (line 73) checking only `slot_values / slot_taint / action_payloads / pending_actions` — **no check on `seed.summary.terminal`**; closest test `recovery_boundary_factory_frame_seed_round_trips_summary` (`vb_runtime/src/recovery/tests.rs:339-391`) builds a seed with `terminal = Some(Finished { .. })` but only asserts on `boundary.summary()`, never calls `hydrate_run_frame`; 1270 tests pass with no regression that would fail without the guard | NOT-PATCHED | no |
| vb-2eprq | P2 | SA-002: `JournalWriteBatch::commit` returns `JournalError::BatchAborted` for an aborted batch | `rg` for `BatchAborted` + read `commit()` body at `batch.rs:324-330` | `JournalWriteBatch::commit` (`batch.rs:324-330`) still returns `Ok(())` for aborted batches: `if self.aborted { return Ok(()); }`; no `JournalError::BatchAborted` variant exists (`rg BatchAborted` → 0 matches across `crates/vb_storage`); regression `e2e_aborted_batch_commit_succeeds_with_no_persist` (`batch.rs:1841-1869`) encodes the buggy behavior with `batch2.commit().expect("aborted batch commit must succeed")`; 1270 passed — close reason claims a fix and an error variant that do not exist in source | NOT-PATCHED | **yes** |

## Counts

- **bugs-checked:** 8
- **PATCHED:** 4 (vb-1rqz7.5, vb-1rqz7.6, vb-1rqz7.8, vb-1rqz7.9)
- **NOT-PATCHED:** 4 (vb-1rqz7.7, vb-28qw9, vb-294mf, vb-2eprq)
- **PARTIAL:** 0
- **UNKNOWN:** 0
- **Hallucination in close reason:** 2 (vb-28qw9, vb-2eprq)

## Top-3 NOT-PATCHED with the acceptance bullet that failed

1. **vb-1rqz7.7 (SR-005)** — dimension derivation must include `RunAnswered` and `ActionScheduledTicket` output slots. `hydrate_support.rs:223-226` updates `max_step` from `ticket.step` only; `ActionScheduledTicket`'s `output: SlotIdx` is **never** used to derive `max_slot`. `RunAnswered { slot_idx, .. }` has **no match arm** in `apply_summary_event` (`summary.rs:50-87`) or in the dimension-derivation match (`hydrate_support.rs:207-238`). Live defect: `RecoveryRuntimeSummary.slot_count` and `RecoveryFrameSeed.slot_count` will under-count any run where the maximum output slot comes from a scheduled action's output reservation or a runtime answer.

2. **vb-28qw9 (SA-007)** — `validate_compiled_ir_record` ignores `record.metadata_hash`. The close reason cites `crates/vb_storage/src/admission/record.rs:44` and a helper `validate_artifact_metadata_hash_binding`, but the file does not exist and no function with either name exists. `CompiledIrRecord` has no `metadata_hash` field. The "Addressed in wave 8" narrative is unsubstantiated by source or tests.

3. **vb-2eprq (SA-002)** — `JournalWriteBatch::commit` silently returns `Ok(())` for an aborted batch. `batch.rs:324-330` still does exactly that. No `JournalError::BatchAborted` variant is defined anywhere in `vb_storage`. The active regression `e2e_aborted_batch_commit_succeeds_with_no_persist` (`batch.rs:1841-1869`) explicitly asserts the buggy behavior with `.expect("aborted batch commit must succeed")`. The "Completed no-code … JournalError::BatchAborted" close reason names an error variant that is absent from the source.

## Top hallucination cases

1. **vb-28qw9 close reason** references `crates/vb_storage/src/admission/record.rs:44` and `validate_artifact_metadata_hash_binding` — neither the file nor the function exists. The corresponding `metadata_hash` field on `CompiledIrRecord` is also absent (`records.rs:246-251`). No test, no compile-time symbol, no source path matches.

2. **vb-2eprq close reason** claims "SA-002 already fixed in current source with `JournalError::BatchAborted` and existing regressions". `JournalError` has no `BatchAborted` variant (`rg BatchAborted` → 0 matches); `commit()` at `batch.rs:324-330` is unchanged and still returns `Ok(())` for aborted batches; the only existing regression (`e2e_aborted_batch_commit_succeeds_with_no_persist`) asserts the opposite of the claimed fix.

3. **vb-294mf close reason** is silent on the actual gap. The bead description names `Runtime::recover_one_run`, but no such function exists in `vb_runtime/src/**`. The terminal-aware rehydration guard the bead implies (reject `FrameSeed` when `seed.summary.terminal.is_some()`) is not present at the only relevant boundary (`hydrate_run_frame` at `vb_runtime/src/recovery.rs:63-70`). The omission is real; the close reason avoids acknowledging it.

## Note on referenced research files

The bead descriptions for `vb-1rqz7.5` … `vb-1rqz7.9` cite paths that do not exist in this checkout:
- `crates/vb_storage/src/recovery/event_replay/tail.rs` — missing
- `crates/vb_storage/src/recovery/snapshot_decode.rs` — missing
- `crates/vb_storage/src/recovery/checkpoint.rs` — missing
- `crates/vb_storage/src/recovery/hydrate/validation.rs` — missing (directory is a flat `hydrate.rs` + `hydrate_support.rs`)
- `crates/vb_storage/src/recovery/replay/recovery_ops.rs` — missing (the directory contains `attempt.rs`, `core.rs`, `mod.rs`, `summary.rs` only)
- `crates/vb_storage/src/recovery/replay/summary/slots/pending.rs` — missing
- `crates/vb_storage/src/recovery/replay/summary/accumulator.rs` — missing
- `bug-hunt-2026-06-21/findings/**/SR-*.md` — the entire `bug-hunt-2026-06-21/` directory is absent from the working tree

The fixes have been landed in adjacent or renamed files (`recover.rs`, `replay/summary.rs`, `hydrate_support.rs`, `hydrate.rs`, `slot_extra.rs`, `journal/replay.rs`), so verdicts above are based on the actual file paths and test evidence, not on the bead's stale file references.

## File written

`/home/lewis/src/velvet-ballistics/to-fix/wave3/agent-04-truth-serum.md`
