# Proof-to-Implementation Input — vb-om21

State 7 bridge input prepared by proof-planner. Mapping status remains planned; this does not approve implementation.

## Rust source refs
- `crates/vb_storage/src/keys.rs`: `run_event_key`, `run_prefix_key`, `sequenced_run_key`; key layout and big-endian sequence bytes.
- `crates/vb_storage/src/journal/replay.rs`: `events_for_run_from`, `validate_replay_sequence`; prefix-bounded scan and replay parity seam.
- `crates/vb_storage/src/recovery/recover.rs`: recovery entry points and empty-data behavior seam.
- `crates/vb_storage/src/recovery/types.rs`: required typed `TailMismatch` and `MissingJournal` semantics.
- `crates/vb_storage/src/trimming/logic.rs`: nearby max-seq prefix scan pattern.

## Required bridge claims
1. Prefix-bound proof claims map to the actual tail scan helper or recovery scan path; no toy key iterator.
2. Big-endian max and checked tail addition claims map to production key constructors/parsers and `checked_add`.
3. Metadata validation claims map to typed recovery outcomes before replay/truncation decisions.
4. Missing journal distinction maps query-empty zero tail separately from recovery-required `MissingJournal`.
5. Parser/fuzz claims map to length+prefix validation before sequence byte extraction.
6. Replay parity maps tail fallback to existing `WrongRun`/`SequenceGap` replay validation, not a replacement.
7. Bounded-resource claim maps to O(1) accumulator and no collection of all event payloads for pure tail query.

## Behavior evidence refs expected later
- `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` for acceptance behavior.
- `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_journal_tail_scan_fallback_tests`.
- `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_fjall_keyspace_manifest_tests` for key ordering baseline.
- `moon ci` as final canonical gate.

## Planned obligation source
Use `proof-obligations.planned.jsonl` as the machine-readable source for verifier artifacts, commands, assumptions, bounds, and expected evidence.
