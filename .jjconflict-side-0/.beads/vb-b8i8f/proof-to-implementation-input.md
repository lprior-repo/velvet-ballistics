# Proof-to-Implementation Input: vb-b8i8f

## Bridge Purpose

This document prepares the State 7 `proof-to-implementation` bridge with the mapping from planned proof obligations to Rust source references, behavior test requirements, and refinement harness targets. The bridge agent will use this to create `rust-refinement-obligation/v1` rows and map proof claims to concrete implementation obligations.

## Proof Obligation → Source Mapping

| Obligation ID | Verifier | Production Target | Source File | Symbol |
|---------------|----------|-------------------|-------------|--------|
| PO-VERUS-001 | verus | handle_cancel, handle_kill | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` | `Shard::handle_cancel` (L101), `Shard::handle_kill` (L120) |
| PO-VERUS-002 | verus | terminal winner invariant | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` | L108-115 (cancel), L125-134 (kill) |
| PO-VERUS-003 | verus | stale authority rejection | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` | `Shard::handle_timer` (L64), `Shard::handle_ask_answer` (L2) |
| PO-VERUS-004 | verus | kind family validation | `crates/vb_storage/src/codec/validation.rs` | `is_known_record_kind` (L23), `validate_kind_family` (L42) |
| PO-VERUS-005 | verus | replay sequence contiguity | `crates/vb_storage/src/journal/replay.rs` | `validate_replay_sequence` |
| PO-KANI-001 | kani | cancel/kill live-only | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` | `Shard::handle_cancel` (L101), `Shard::handle_kill` (L120) |
| PO-KANI-002 | kani | single terminal winner | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` | `Shard::handle_cancel` L108, `Shard::handle_kill` L131 |
| PO-KANI-003 | kani | stale authority | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` | `Shard::handle_timer` (L64), `Shard::handle_ask_answer` (L2) |
| PO-KANI-004 | kani | kind 28 admission | `crates/vb_storage/src/codec/validation.rs` | `is_known_record_kind` (L23), `validate_kind_family` (L42), `validate_known_kind` (L35) |
| PO-KANI-005 | kani | replay with killed | `crates/vb_storage/src/journal/replay.rs` | `events_for_run`, `validate_replay_sequence` |
| PO-FLUX-001 | flux-rs | cancel/kill return type | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` | `Shard::handle_cancel`, `Shard::handle_kill` return `RuntimeResult<()>` |
| PO-FLUX-002 | flux-rs | terminal membership | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` | `self.terminal_runs` check in handle_cancel/handle_kill |
| PO-FLUX-003 | flux-rs | timer/ask post-terminal | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` | `Shard::handle_timer` (L64), `Shard::handle_ask_answer` (L2) |
| PO-FLUX-004 | flux-rs | kind range refinement | `crates/vb_storage/src/codec/validation.rs` | `validate_kind_family` (L42) range `10..=28` |
| PO-FLUX-005 | flux-rs | playback contiguity | `crates/vb_storage/src/journal/replay.rs` | `events_for_run` return Vec<JournalEvent> |
| PO-PROP-001 | proptest | cancel/kill live-only | `crates/vb_runtime/src/runtime.rs` | `Runtime::cancel_run` (L174), `Runtime::kill_run` (to be added) |
| PO-PROP-002 | proptest | terminal idempotency | `crates/vb_runtime/src/runtime.rs` | `Runtime::cancel_run`, `Runtime::kill_run` |
| PO-PROP-003 | proptest | stale authority | `crates/vb_runtime/src/runtime.rs` | `Runtime::timer_entry_fired` (L382), `Runtime::complete_action` (L318) |
| PO-PROP-004 | proptest | kind28 round-trip | `crates/vb_storage/src/codec/mod.rs` | `encode_record`, `decode_record` for `JournalEvent::RunKilled` |
| PO-PROP-005 | proptest | replay sequence | `crates/vb_storage/src/journal/replay.rs` | `events_for_run` |
| PO-FUZZ-001 | cargo-fuzz | kind validation fuzz | `crates/vb_storage/src/codec/validation.rs` | `validate_kind_family` |
| PO-FUZZ-002 | cargo-fuzz | journal decode fuzz | `crates/vb_storage/src/codec/mod.rs` | `decode_record::<JournalEvent>` |

## Public API Addition Required

The following production source change is required before proof obligations targeting `Runtime::kill_run` can be satisfied:

```rust
// crates/vb_runtime/src/runtime.rs — NEW method
pub fn kill_run(&self, run: RunId) -> RuntimeResult<()> {
    let shard = self.shard_for(run)?;
    shard.enqueue(ShardCommand::Kill { run, reason: None })
}
```

This mirrors the existing `cancel_run` pattern (L174-177). It routes through `shard_for` for shard assignment and enqueues a typed `ShardCommand::Kill`.

## Storage Codec Changes Required

Two changes in `crates/vb_storage/src/codec/validation.rs`:

1. `is_known_record_kind` (L23-25): add `28` to the matches! pattern:
   ```rust
   // BEFORE: matches!(kind, 1 | 2 | 3 | 10..=27 | 30 | 40 | 50)
   // AFTER:  matches!(kind, 1 | 2 | 3 | 10..=28 | 30 | 40 | 50)
   ```

2. `validate_kind_family` (L46): extend journal range from `10..=27` to `10..=28`:
   ```rust
   // BEFORE: MAGIC_JOURNAL_EVENT => matches!(kind, 10..=27),
   // AFTER:  MAGIC_JOURNAL_EVENT => matches!(kind, 10..=28),
   ```

## Shard Lifecycle Changes Required

In `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs`:

1. `handle_cancel` (L101-118): must return `Err(RuntimeError::RunNotFound)` (or typed terminal error) when run is missing or already terminal, instead of `Ok(())`.
2. `handle_kill` (L120-135): same change; must return `Err` for missing/terminal runs.
3. Both handlers must NOT append journal events, emit trace, increment counters, or discard sequence on error paths.

## Existing Test File to Extend

`crates/workspace_tests/tests/cancel_kill_lattice_tests.rs` (existing registered target):
- Add kill lattice tests once `Runtime::kill_run` is available.
- Extend cancel tests to assert typed errors for missing/terminal runs.
- Existing tests that assert `Ok(())` for missing/terminal cancel must be updated to expect `Err`.

## Existing proptest Range to Update

`crates/workspace_tests/tests/postcard_envelope_wire_tests.rs`:
- Proptest range `10u16..=27u16` must become `10u16..=28u16` for journal kind generation.
- Existing assertions on record kind ranges must include kind 28.

## Required Behavior Tests (State 8-10)

The bridge must ensure these behavior test scenarios are planned:

1. **Happy path kill**: submit run → tick → kill_run → tick → assert terminal state with killed kind.
2. **Kill missing run**: kill_run on never-submitted run → assert Err(RunNotFound).
3. **Kill after cancel**: cancel_run → tick → kill_run → assert Err.
4. **Kill after finish**: submit → tick (to completion) → kill_run → assert Err.
5. **Stale timer after kill**: submit → tick (suspend) → capture timer → kill_run → tick → timer_entry_fired → assert Err.
6. **Stale action after cancel**: submit action workflow → tick → cancel → tick → complete_action → assert Err.
7. **Journal evidence**: cancel/kill → inspect journal events → assert exactly one terminal event.
8. **Encode/decode round-trip**: serialize JournalEvent::RunKilled → deserialize → assert equality.
9. **Replay with killed**: append events including RunKilled → replay → assert contiguous sequence.

## Bridge Contractor Notes

- State 7 `proof-to-implementation` must create `rust-refinement-obligation/v1` rows for each of the 22 proof obligations.
- Each refinement obligation must name concrete `source_refs`, `behavior_test_refs`, and `refinement_harness_refs`.
- `mapping_status: planned` is allowed at State 7; must be `materialized` by State 11 (implementation) and `verified` by State 12 (closure).
- The public `Runtime::kill_run` method must exist before obligation PO-PROP-001 can be executed.
- Storage codec changes must precede PO-KANI-004 and PO-PROP-004 execution.
