# Wave 3 — Agent 12: Ad-Hoc Storage-Codec Deep Dive

**Scope:** 10 bugs in chunk-12, focus on `crates/vb_storage/src/codec/`
**Checks per bug:**
1. `envelope-trailing-bytes` — Section 18: decoder must reject trailing bytes beyond declared `payload_len`.
2. `record-kind-parity` — Record-kind IDs must match payload variant (no SlotWritten/StepSucceeded collision).
3. `magic-validation` — Magic number validated before any allocation.
4. `trim-fail-closed` — Trim operations fail closed on malformed keys.

**Verdict codes:** `PATCHED` | `NOT-PATCHED` | `PARTIAL` | `UNKNOWN`

## Codec baseline evidence (one-time probes)

| Probe | Result | Reference |
|---|---|---|
| `decode_ignores_trailing_bytes_beyond_payload` | PASS (decoder ACCEPTS trailing bytes) | `crates/vb_storage/src/codec/tests.rs:1498-1523` |
| `decode_rejects_header_only_input_with_nonzero_payload_len` | PASS | `crates/vb_storage/src/codec/tests.rs:1526-1554` |
| `step_succeeded_event_maps_to_slot_written_kind` | PASS (StepSucceeded → RecordKind::SlotWritten, id 12) | `crates/vb_storage/src/codec/tests.rs:1616-1629`; `crates/vb_storage/src/events.rs:371` |
| `record_kind_ids_are_distinct` | PASS (no `StepSucceeded` variant exists) | `crates/vb_storage/src/codec/tests.rs:1328-1361`; `crates/vb_storage/src/records.rs:139-197` |
| `decode_record_returns_bad_magic_when_magic_differs` | PASS | `crates/vb_storage/src/tests.rs` |
| `trim_events_for_run_fails_closed_on_malformed_event_key` | PASS (returns `IncompleteTrim`) | `crates/vb_storage/src/trimming/logic.rs:75-77`; `crates/vb_storage/src/trimming/tests.rs:845-892` |
| `encode_record` magic-before-alloc | PASS (`validate_record_kind_family` runs before `postcard::to_allocvec`) | `crates/vb_storage/src/codec/mod.rs:66-67` |
| `decode_record_header` magic-before-alloc | PASS (magic check at `header.rs:35-39` precedes family/size/CRC checks) | `crates/vb_storage/src/codec/header.rs:26-58` |
| `vb_storage --lib` full sweep | 1270 passed; 0 failed | `cargo test -p vb_storage --lib --no-fail-fast` |

## Per-bug matrix

| bug-id | pri | envelope-trailing-bytes | record-kind-parity | magic-validation | trim-fail-closed | targeted-cmd | result | verdict | evidence |
|---|---|---|---|---|---|---|---|---|---|
| vb-p7zza | P2 | N/A (cli_envelope, not vb_storage) | N/A | N/A | N/A | `cargo test -p velvet-ballistics --bin velvet-ballistics cli_envelope` | 9/9 pass | NOT-PATCHED | `crates/vb_cli/src/cli_envelope.rs:44,91,132,169` still carry `#[allow(dead_code)]`; `EnvelopeError` enum at line 170-174 still exists; `build_envelope`/`from_str` still public, not `#[cfg(test)]`. Bead `bd show vb-p7zza` shows status `CLOSED` with close reason `Closed` (no implementation evidence). Source unchanged. |
| vb-pbp6z | P2 | N/A (hydrate_run_frame_from_events is recovery, not envelope decode) | N/A | N/A | N/A | `cargo test -p vb_storage --lib hydrate_run_frame` | 37/37 pass | PATCHED | Close reason cites `rtk cargo test -p vb_runtime`/`moon ci` evidence; `hydrate.rs:321` and `hydrate_support.rs:264` show single-pass implementations; all `hydrate_run_frame_from_events_*` tests pass. |
| vb-pctwr | P3 | N/A (runtime engine, not codec) | N/A | N/A | N/A | `cargo test -p vb_runtime --lib journal` (no RE-020 specific test found) | tests pass | PARTIAL | Bead is `IN_PROGRESS`; `chunk_002.rs:259-274` `storage_event` still calls `event.clone()` on the passed `RuntimeJournalEvent` 3× (lines 260, 263, 266). No RE-020 regression test in source. |
| vb-qagk2 | P1 | OK (decode tests pass; this is replay) | OK | OK (replay path) | N/A | `cargo test -p velvet-ballistics --test lifecycle_integration replay_with_malformed_event_returns_replay_corruption replay_with_missing_event_returns_replay_corruption` | 2/2 pass | PARTIAL | `crates/vb_storage/src/journal/replay.rs:72-85` `events_for_run_bounded` does NOT contain the claimed `run_seq_gap.contains_key(...)` gap-marker check; `rtk grep "run_seq_gap" crates` returns 0 source matches. Tests pass only because `inject_seq_gap` (`injection.rs:37-54`) writes a record whose `()` payload fails `decode_journal_event`, not because of the documented fix. |
| vb-qapik | P4 | N/A (write_recovered_snapshot, recovery) | N/A | N/A | N/A | `cargo test -p vb_storage --lib compute_retained_terminal_runs` | 2/2 pass | UNKNOWN | `crates/vb_storage/src/recovery/snapshot_write.rs` does not exist (`rtk ls` shows `recover.rs`, `hydrate.rs`, `hydrate_support.rs` only); the named function `write_recovered_snapshot` is not in source. Recovery write path lives in `hydrate_support.rs`. Cannot verify the identical-payloads claim without the target file. |
| vb-rvgjy | P0 | N/A (recovery hydration) | N/A | N/A | N/A | `cargo test -p vb_storage --lib trim_all_eligible_runs` and `tail_seq_equal_to_snapshot_seq_fails` (in `slot_written_ordering_integration_tests`) | 4+ pass | PATCHED | Close reason enumerates 3 independent fixes (legacy_slot_taint, no_output skip, snapshot+tail contiguity relaxation); `trimming/logic.rs:325-382` shows `compute_retained_terminal_runs` + pure `retained_terminal_runs_top_n` split. |
| vb-s18xp | P2 | N/A (runtime admission) | N/A | N/A | N/A | `cargo test -p velvet-ballistics-workspace-tests --test vb_qi37_4_2_strict_runtime_admission` | 22/22 pass | PATCHED | Close reason cites `moon ci`; admission tests pass. |
| vb-s9iyv | P3 | N/A (storage admission rename) | N/A | N/A | N/A | `cargo test -p vb_storage --lib append_queued_unfsynced` | 2+ pass | PATCHED | `crates/vb_storage/src/journal/internal.rs:70` `append_queued_unfsynced` replaces `append_queued_indexed_unpersisted`; `journal/tests.rs:830-865` covers rename. Wave-15 follow-up `vb-y8tyj` finishes the inner-helper rename. |
| vb-sit0c | P3 | N/A (trim) | N/A | N/A | OK | `cargo test -p vb_storage --lib trim_events_for_run` | 1+ pass | PATCHED | Despite close reason `RESOLVED_REJECTED` (deferred per Fjall borrowed-slice API), the fix is in source: `crates/vb_storage/src/trimming/logic.rs:94` `batch.remove(&self.events, key.clone())` with the SC-008 fix comment at lines 87-93 documenting the `key.to_vec()` → `key.clone()` change. Allocation eliminated. |
| vb-tn131 | P2 | N/A (vb_core value) | N/A | N/A | N/A | `cargo test -p vb_core --lib action_name` (no exact match) | tests pass | PATCHED | Close reason cites red regression test + black-hat + test-reviewer approval. Bug is in `vb_core::action::classification.rs:39` (CV-101, untrimmed ActionName), not codec. |

## Summary

- **bugs-checked:** 10
- **PASS / PATCHED:** 5 (vb-pbp6z, vb-rvgjy, vb-s18xp, vb-s9iyv, vb-sit0c)
- **PARTIAL:** 2 (vb-pctwr, vb-qagk2)
- **NOT-PATCHED:** 1 (vb-p7zza)
- **UNKNOWN:** 1 (vb-qapik)
- **others (N/A to codec checks, bead status only):** 1 (vb-tn131)

## Codec-specific violations observed (independent of bead status)

1. **envelope-trailing-bytes VIOLATION** (Section 18): `crates/vb_storage/src/codec/payload.rs:56-82` `decode_record_payload` never asserts `payload_end == bytes.len()`. The test `decode_ignores_trailing_bytes_beyond_payload` (`tests.rs:1498-1523`) enshrines the bug. Any record with valid declared payload plus appended garbage decodes successfully. Tracked as `vb-mrwe.1` (not in this chunk) in `to-fix/03-storage-recovery-defects.md:3-15`.

2. **record-kind-parity VIOLATION** (Section 18): `crates/vb_storage/src/events.rs:371` collapses `StepSucceeded` and `SlotWrittenEvent` onto `RecordKind::SlotWritten` (id 12). `RecordKind` enum (`records.rs:139-197`) has no `StepSucceeded` variant. The test `step_succeeded_event_maps_to_slot_written_kind` (`tests.rs:1616-1629`) codifies the collision. Tracked as `P0 storage record kind parity for StepSucceeded` in `to-fix/03-storage-recovery-defects.md:68-81` (not in this chunk).

3. **magic-validation ORDER:** OK. `encode_record` (`mod.rs:66`) and `encode_record_header` (`header.rs:21`) call `validate_kind_family` before any allocation. `decode_record_header` (`header.rs:35-39`) checks magic against `expected_magic` immediately after the 60-byte bound check, before any payload extraction or digest computation.

4. **trim-fail-closed:** OK. `crates/vb_storage/src/trimming/logic.rs:23,75,222` all check `key.len() < 17` and return `TrimError::IncompleteTrim`. `count_trimmable_events` and `latest_durable_snapshot_seq` apply the same gate. Tests `trim_events_for_run_fails_closed_on_malformed_event_key` and `trim_eligibility_diagnostic_fails_closed_on_malformed_event_key` pass.

## Top-3 NOT-PATCHED / PARTIAL with reason

1. **vb-p7zza (NOT-PATCHED):** Bead is `CLOSED` but `crates/vb_cli/src/cli_envelope.rs` retains all four `#[allow(dead_code)]` annotations (lines 44, 91, 132, 169), the dead `EnvelopeError` enum (lines 170-184), and the dead `from_str`/`build_envelope` outside `#[cfg(test)]`. No code change applied despite closure. Status: source contradicts bead.

2. **vb-qagk2 (PARTIAL):** Close reason claims a `run_seq_gap.contains_key(...)` fix in `events_for_run_bounded` (`crates/vb_storage/src/journal/replay.rs:72-85`). That code path contains no such check; `rtk grep "run_seq_gap" crates` returns 0 source matches. Tests pass only because `inject_seq_gap` (`injection.rs:37-54`) writes an undecodable `()` payload that triggers `PostcardDecodeFailed` rather than a true gap-marker rejection. The production fix described in the bead is absent.

3. **vb-pctwr (PARTIAL):** Bead is `IN_PROGRESS`. `crates/vb_runtime/src/journal/chunk_002.rs:259-274` `storage_event` still calls `event.clone()` three times (lines 260, 263, 266) before matching. No `RE-020` regression test exists in the test suite (`rtk grep "RE-020\|storage_event_clones" crates` = 0 matches). Cannot be considered PATCHED.

## File path

`/home/lewis/src/velvet-ballistics/to-fix/wave3/agent-12-adhoc-storage-codec.md`
