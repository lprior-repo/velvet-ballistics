# Proof Strategy — vb-core-strict-ack-ordering

## State 4 Artifact

- **bead_id**: vb-core-strict-ack-ordering
- **state**: 4 (Proof Planning)
- **generated_by**: proof-planner (State 4)
- **date**: 2026-05-15

---

## Strategy Summary

This bead proves strict persistence-before-acknowledgement ordering for the velvet-ballistics runtime journal. The central claim is:

> **ACK-ORDER-001/002**: Every primitive in `DURABILITY_MATRIX` returns acknowledgement to the caller **only after** a `persist_strict` barrier confirms durable journal append — no exceptions, no bypasses.

Proof strategy is **3-tier refinement**:
1. **TLA+** (protocol/temporal) — proves barrier semantics, ack ordering, and queue flush are sound as state machines
2. **Verus** (Rust-local deductive) — proves `verify_ack_after_persist`, matrix completeness, `EventSeq` invariants, and `append_strict`/`append_journaled` postconditions from Rust types
3. **Kani** (bounded model-check) — enumerates all `AckPoint` values, dispatch paths, and codec roundtrips

Concurrency (queue flush ordering under concurrent submit) is handled by **Loom** and **TLA-QUEUE-001**.

---

## Verifier Lane Assignments

| Lane | Obligations | Rationale |
|------|-------------|-----------|
| **TLA+** | TLA-BARRIER-001, TLA-EVENTSEQ-001, TLA-QUEUE-001 | Temporal ordering, barrier state machine, queue flush invariants |
| **Verus** | VERUS-DM-001/002/003/004, VERUS-JA-001/002 | Rust-local invariants, pure postconditions, type refinement |
| **Kani** | KANI-ACK-001, KANI-DISPATCH-001/002, KANI-CODEC-001, KANI-HYDRATE-001, KANI-REPLAY-001 | Bounded enumeration of enum values, dispatch, codec, recovery |
| **Loom** | LOOM-QUEUE-001/002/003/004 | Concurrent interleavings of queue submit/flush/drain/cancel |
| **Miri** | MIRI-CODEC-001 | UB-free serde roundtrip for RecordKind |
| **Proptest** | PROPTEST-EVENTSEQ-001 | EventSeq monotonicity across 10k random u64 values |
| **Integration** | INTEGRATION-ACK-001/002/003/004 | End-to-end fail-closed ack ordering |
| **Static scan** | STATIC-SCAN-001/002 | Clippy + forbid(unsafe_code) |

---

## Open Question Responses (from Contract OQ-1/2/3)

### OQ-1: Does `flush_batch` guarantee same barrier as `append_strict` for Strict queued events?

**Answer strategy**: TLA-QUEUE-001 (QueuedStrictFlush) + LOOM-QUEUE-001
- TLA+ model (QF1-QF3) formally specifies `strictFlushComplete` requires all queued strict events appended + `persist_strict` called exactly once
- Loom concurrency tests confirm no unpersisted-ack race under concurrent submit
- **Residual risk**: If Fjall `persist(SyncAll)` itself has hidden non-determinism, only hardware-level testing can detect; Kani models this as oracle

### OQ-2: Is `BeforeJournalAppend` reachable through any public API?

**Answer strategy**: KANI-ACK-001 + VERUS-DM-004
- Kani harness enumerates all `DURABILITY_MATRIX` rows and confirms each `ack_point == AfterJournalAppend`
- Verus `proof_before_journal_append_unreachable` proves `AckPoint` has exactly two variants and no matrix row can construct `BeforeJournalAppend`
- **Waiver**: Fjall internal UB excluded; treated as oracle (waived in verification-layers.md)

### OQ-3: Are all 11 `test_evidence` paths in `DURABILITY_MATRIX` real tests or stubs?

**Answer strategy**: INTEGRATION-ACK-001/003/004
- Each integration test maps to a primitive handler in `DURABILITY_MATRIX`
- `verify_matrix_replay_proofs()` checks each row's `test_evidence` slice is non-empty — stubs will fail at compile time
- **Evidence**: `test-writer-report.md` shows which tests exist per primitive

---

## Artifact Inventory

| Obligation | Artifact | Command | Expected Evidence |
|------------|----------|---------|-------------------|
| TLA-BARRIER-001 | `specs/JournalBarrier.tla` + `specs/JournalBarrier.cfg` | `tlc -config specs/JournalBarrier.cfg specs/JournalBarrier.tla` | No invariant violations I1-I5; T1 satisfied; T2 under fairness |
| TLA-EVENTSEQ-001 | `specs/EventSeqOrdering.tla` + `specs/EventSeqOrdering.cfg` | `tlc -config specs/EventSeqOrdering.cfg specs/EventSeqOrdering.tla` | No invariant violations EO1-EO3 |
| TLA-QUEUE-001 | `specs/QueuedStrictFlush.tla` + `specs/QueuedStrictFlush.cfg` | `tlc -config specs/QueuedStrictFlush.cfg specs/QueuedStrictFlush.tla` | No invariant violations QF1-QF3 |
| VERUS-DM-001 | `crates/vb_runtime/src/durability_matrix.rs` | `verus crates/vb_runtime/src/durability_matrix.rs` | 0 Verus errors; `verify_ack_after_persist` proven |
| VERUS-DM-002 | `crates/vb_runtime/src/durability_matrix.rs` | `verus crates/vb_runtime/src/durability_matrix.rs` | 0 Verus errors; matrix completeness proven |
| VERUS-DM-003 | `crates/vb_storage/src/types.rs` | `verus crates/vb_storage/src/types.rs` | 0 Verus errors; EventSeq injectivity + roundtrip proven |
| VERUS-DM-004 | `crates/vb_runtime/src/durability_matrix.rs` | `verus crates/vb_runtime/src/durability_matrix.rs` | 0 Verus errors; exactly-2-variant + unreachable proven |
| VERUS-JA-001 | `crates/vb_storage/src/journal/append.rs` | `verus crates/vb_storage/src/journal/append.rs` | 0 Verus errors; `append_strict` postcondition proven |
| VERUS-JA-002 | `crates/vb_storage/src/journal/append.rs` | `verus crates/vb_storage/src/journal/append.rs` | 0 Verus errors; `append_journaled` no-barrier proven |
| KANI-ACK-001 | `crates/vb_runtime/src/durability_matrix.rs` | `cargo kani --harness verify_no_before_journal_append_in_matrix` | No counterexample; all rows AfterJournalAppend |
| KANI-DISPATCH-001 | `crates/vb_runtime/src/journal/chunk_002.rs` | `cargo kani --harness verify_strict_profile_dispatches_to_append_strict` | No counterexample; strict dispatch correct |
| KANI-DISPATCH-002 | `crates/vb_runtime/src/journal/chunk_002.rs` | `cargo kani --harness verify_journaled_profile_dispatches_to_append_journaled` | No counterexample; journaled dispatch correct |
| KANI-CODEC-001 | `crates/vb_storage/src/records.rs` | `cargo kani --harness verify_record_kind_codec` | No counterexample; all RecordKind roundtrips OK |
| KANI-HYDRATE-001 | `crates/vb_storage/src/recovery/recover.rs` | `cargo kani --harness verify_hydrate_run_frame_digest_matches` | No counterexample; digest match on recovery |
| KANI-REPLAY-001 | `crates/vb_storage/src/recovery/replay.rs` | `cargo kani --harness verify_replay_divergence_detected` | No counterexample; divergence caught |
| LOOM-QUEUE-001 | `crates/vb_storage/src/queue/mod.rs` | `cargo loom --test flush_batch_strict_ordering` | No concurrency violations |
| LOOM-QUEUE-002 | `crates/vb_storage/src/queue/mod.rs` | `cargo loom --test concurrent_submit_flush_strict` | No race condition for unpersisted ack |
| LOOM-QUEUE-003 | `crates/vb_storage/src/queue/mod.rs` | `cargo loom --test shutdown_drain_strict` | No ordering violation during drain |
| LOOM-QUEUE-004 | `crates/vb_storage/src/queue/mod.rs` | `cargo loom --test action_completion_cancel_during_flush` | No partial ack on cancel |
| MIRI-CODEC-001 | `crates/vb_storage/src/records.rs` | `cargo miri test --test record_kind_roundtrip` | No UB, no leaks, no panic |
| PROPTEST-EVENTSEQ-001 | `crates/vb_storage/src/types.rs` | `cargo proptest --test event_seq_ordering` | 0 failures; monotonicity holds across 10k iters |
| INTEGRATION-ACK-001 | `crates/vb_runtime/src/runtime.rs` | `cargo test submit_direct_returns_durability_error_before_ack --workspace` | Test passes; error propagated before ack |
| INTEGRATION-ACK-002 | `crates/vb_storage/src/recovery/recover.rs` | `cargo test restart_lookup_finds_persisted_header --workspace` | Test passes; digest matches |
| INTEGRATION-ACK-003 | `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` | `cargo test handle_action_completion_persists_before_ack --workspace` | Test passes; fail-closed |
| INTEGRATION-ACK-004 | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` | `cargo test handle_ask_completion_persists_before_ack --workspace` | Test passes; fail-closed |
| STATIC-SCAN-001 | `crates/vb_storage/src/journal/append.rs` | `cargo clippy --workspace --lib --bins -- -D warnings` | 0 warnings; no unsafe code |
| STATIC-SCAN-002 | `crates/vb_runtime/src/durability_matrix.rs` | `cargo clippy --workspace --lib --bins -- -D warnings` | 0 warnings; no unsafe code |

---

## Execution Order

1. **TLA+** (State 4-5): Write .tla/.cfg specs — no Rust code needed
2. **Verus** (State 5-6): proof-writer adds spec/fn/proof_fn to Rust files
3. **Kani/Loom/Miri** (State 11): formal-verifier adds harnesses + runs
4. **Integration + Proptest** (State 8): test-writer adds integration tests
5. **Static scan** (State 11): formal-verifier runs clippy gates

---

## Risk Summary

| Risk | Lane | Coverage |
|------|------|---------|
| Temporal: ack-before-persist | TLA-BARRIER-001 | I1-I5, T1 |
| Rust: matrix contains BeforeJournalAppend | VERUS-DM-001 + KANI-ACK-001 | Dual lane |
| Rust: append_strict missing barrier | VERUS-JA-001 | Oracle call composition |
| Rust: dispatch wrong path | KANI-DISPATCH-001/002 | Bounded enumeration |
| Concurrent: queue flush ordering | LOOM-QUEUE-001/002/003/004 + TLA-QUEUE-001 | Safety invariants |
| Concurrent: cancel during flush | LOOM-QUEUE-004 | Partial-ack prevention |
| UB: serde roundtrip RecordKind | MIRI-CODEC-001 | Miri execution |
| Recovery: digest mismatch | KANI-HYDRATE-001 + INTEGRATION-ACK-002 | Bounded + integration |
| Recovery: replay divergence | KANI-REPLAY-001 | Bounded model check |
