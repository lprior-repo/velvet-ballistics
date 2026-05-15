# Test Plan — vb-core-strict-ack-ordering

## Bead Context
- **bead_id**: vb-core-strict-ack-ordering
- **phase**: State 8 (test-writer)
- **contract**: ACK-ORDER-001/002, DISPATCH-001/002, RECOVERY-001/002/003, FAIL-001/002

## Test Inventory (27 Proof Obligations → 27 Test Cases)

### 1. Integration Tests (4 tests) — `crates/vb_runtime/tests/`

| Obligation | Test File | Target Function | Failure Mode |
|------------|-----------|-----------------|--------------|
| INTEGRATION-ACK-001 | `submit_direct_durability_test.rs` | `Runtime::submit_direct` + mock persist failure | Fails if ack returned before persist barrier |
| INTEGRATION-ACK-002 | `recovery_digest_match_test.rs` | `recover_runtime_frame_seed_from_events` | Fails if digest mismatch or recovery produces wrong state |
| INTEGRATION-ACK-003 | `action_completion_ack_test.rs` | `handle_action_completion` | Fails if ack sent before ActionCompleted persisted |
| INTEGRATION-ACK-004 | `ask_completion_ack_test.rs` | `handle_ask_completion` | Fails if ack sent before AskAnswered persisted |

### 2. Proptest (1 test) — `crates/vb_storage/src/types.rs`

| Obligation | Test Location | Target | Property |
|------------|---------------|--------|----------|
| PROPTEST-EVENTSEQ-001 | `#[cfg(test)]` + proptest | `EventSeq` | new(v).get()==v, monotonicity, serde roundtrip |

### 3. Kani Harnesses (6 proofs) — `crates/vb_storage/` or `kani_harnesses/`

| Obligation | Harness | Target | Verification |
|------------|---------|--------|--------------|
| KANI-ACK-001 | `verify_no_before_journal_append_in_matrix` | `DURABILITY_MATRIX` | All rows have `AfterJournalAppend` |
| KANI-DISPATCH-001 | `verify_strict_profile_dispatches_to_append_strict` | `append_storage_event` | Strict → `append_strict` dispatch |
| KANI-DISPATCH-002 | `verify_journaled_profile_dispatches_to_append_journaled` | `append_storage_event` | Journaled → `append_journaled` dispatch |
| KANI-CODEC-001 | `verify_record_kind_codec` | `RecordKind` serde | All variants roundtrip via serde + PostCard |
| KANI-HYDRATE-001 | `verify_hydrate_run_frame_digest_matches` | `recover_runtime_frame_seed_from_events` | Recovered digest matches persisted header |
| KANI-REPLAY-001 | `verify_replay_divergence_detected` | `replay_events` | Out-of-order steps → `ReplayDivergence` |

### 4. Loom Tests (4 tests) — `crates/vb_runtime/src/` or `tests/`

| Obligation | Test | Concurrency Property |
|------------|------|---------------------|
| LOOM-QUEUE-001 | `flush_batch_strict_ordering` | All queued strict events receive barrier before flush returns |
| LOOM-QUEUE-002 | `concurrent_submit_flush_strict` | No unpersisted ack during concurrent submit+flush |
| LOOM-QUEUE-003 | `shutdown_drain_strict` | `shutdown_drain` drains all strict events with barrier |
| LOOM-QUEUE-004 | `action_completion_cancel_during_flush` | Cancel during flush produces no partial ack |

### 5. Miri Test (1 test) — `crates/vb_storage/tests/`

| Obligation | Test | Target |
|------------|------|--------|
| MIRI-CODEC-001 | `record_kind_roundtrip` | `RecordKind` serde roundtrip under Miri (no UB) |

### 6. TLA+ Specs (3 specs) — `specs/`

These are verified via TLC model checker, not unit tests:
- TLA-BARRIER-001: JournalBarrier.tla — ackSent → persistedEvents = journaledEvents
- TLA-EVENTSEQ-001: EventSeqOrdering.tla — persistedSeqs ⊆ appendedSeqs
- TLA-QUEUE-001: QueuedStrictFlush.tla — strictFlushComplete → all events appended

## Failing-First Strategy

1. **Integration tests** — Write tests that call real functions. Tests fail when:
   - `submit_direct` does not propagate persist failures as typed errors
   - Recovery digest does not match persisted header
   - Action/ask completion does not call `append_strict` barrier before returning

2. **Proptest** — Tests already pass (EventSeq is a u64 wrapper). These prove the property holds.

3. **Kani harnesses** — Currently hardcoded to `kani::assert(true)`. Rewrite to:
   - Actually enumerate matrix rows and assert on each
   - Use real dispatch logic (not vacuous `kani::any()`)

4. **Loom tests** — Currently use real queue operations. Ensure `#[loom::test]` and assertions hold.

5. **Miri test** — Hardcoded variant list. Use `strum::VariantArray` for auto-enumeration.

## Test Count Per Obligation

| Category | Count | File Count |
|----------|-------|------------|
| Integration | 4 | 4 files |
| Proptest | 1 | 1 file |
| Kani | 6 | 6 files |
| Loom | 4 | 1 file (queue_concurrency.rs) |
| Miri | 1 | 1 file |
| TLA+ | 3 | 3 .tla files (no test files) |
| **TOTAL** | **19** | **14 files** |

## Gap: Missing test-plan.md from S7

The S7 test-planner artifact (test-plan.md) was not persisted. This plan replaces it.
