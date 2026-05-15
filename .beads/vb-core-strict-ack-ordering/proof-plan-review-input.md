# Proof Plan Review Input — vb-core-strict-ack-ordering

## State 4 Reviewer: proof-reviewer (prior to proof-writer)

---

## Contract Under Review

**Bead**: vb-core-strict-ack-ordering — "runtime/storage: Prove strict persistence before acknowledgement ordering"

**Central claim**: `ACK-ORDER-001/002` — every primitive returns `Ok(ack)` only after `persist_strict` confirms durable journal append.

---

## Obligations Reviewed

### TLA+ Layer (3 obligations)

| ID | Module | Key Invariants | Reviewer question |
|----|--------|---------------|------------------|
| TLA-BARRIER-001 | JournalBarrier | I1: ackSent → persisted ⊇ journaled; I2: Strict → ack only after persist; I3: Journaled → ack after journal; I4: persistError → ¬ackSent; I5: profile ∈ {Strict,Journaled,Volatile} | Is I2 strong enough to rule out async strict ack? (contract has UnsupportedAsyncStrictAck) |
| TLA-EVENTSEQ-001 | EventSeqOrdering | EO1: persisted ⊆ appended; EO2: appended bounded by u64; EO3: no gaps in normal replay | Does EO3 handle the gap-case for RECOVERY-003 correctly? |
| TLA-QUEUE-001 | QueuedStrictFlush | QF1: strictFlushComplete → all queued strict appended; QF2: persist_strict called exactly once; QF3: no new events during flush | Is QF3 a hard invariant or a fairness assumption? |

### Verus Layer (6 obligations)

| ID | Target | Spec/proof | Reviewer question |
|----|--------|------------|------------------|
| VERUS-DM-001 | verify_ack_after_persist | ack_point_is_after_append spec; prove_verify_ack_after_persist proof | Does spec cover Err path for each failing row? |
| VERUS-DM-002 | DURABILITY_MATRIX | required_primitives_set==matrix_primitives_set; no duplicates | Does proof handle the 11-row static table correctly or does it rely on const evaluation? |
| VERUS-DM-003 | EventSeq | constructor_injective; strictly_monotonic; serde_roundtrip_preserves | Is serde_roundtrip_preserves proven or assumed via trusted impl? |
| VERUS-DM-004 | AckPoint | ack_point_variant_count=2; no_before_journal_append_in_public_matrix | How does the proof rule out future refactors adding a matrix row with BeforeJournalAppend? |
| VERUS-JA-001 | append_strict | append_then_persist; ok_only_if_both_succeed | Is append_unpersisted treated as returns Ok ↔ event in journal? |
| VERUS-JA-002 | append_journaled | no_persist_strict_call | Does the proof prevent a future refactor from calling persist_strict in the journaled path? |

### Kani Layer (5 obligations — 2 missing from original proof-obligations.jsonl)

| ID | Target | Harness | Reviewer question |
|----|--------|---------|------------------|
| KANI-ACK-001 | DURABILITY_MATRIX | verify_no_before_journal_append_in_matrix | Does harness iterate all 11 rows at compile time or use const evaluation? |
| KANI-DISPATCH-001 | chunk_002.rs | verify_strict_profile_dispatches_to_append_strict | Is the profile enum matched exhaustively? |
| KANI-DISPATCH-002 | chunk_002.rs | verify_journaled_profile_dispatches_to_append_journaled | Same exhaustiveness question |
| KANI-CODEC-001 | records.rs | verify_record_kind_codec | Does kani treat serde as oracle or attempt to verify the impl? |
| **KANI-HYDRATE-001** | recover.rs | verify_hydrate_run_frame_digest_matches | **MISSING in original — added here** — covers RECOVERY-002 |
| **KANI-REPLAY-001** | replay.rs | verify_replay_divergence_detected | **MISSING in original — added here** — covers RECOVERY-003 |

### Loom Layer (4 obligations)

| ID | Target | Test | Reviewer question |
|----|--------|------|------------------|
| LOOM-QUEUE-001 | JournalWriterQueue | flush_batch_strict_ordering | Is the interleaving bound honest (not artificially large)? |
| LOOM-QUEUE-002 | JournalWriterQueue | concurrent_submit_flush_strict | Does loom model capture the mutex that protects the queue? |
| LOOM-QUEUE-003 | JournalWriterQueue | shutdown_drain_strict | Does shutdown_drain hold the flush lock across the barrier? |
| LOOM-QUEUE-004 | JournalWriterQueue | action_completion_cancel_during_flush | Does loom explore the cancel-safety of the ack path? |

### Miri / Proptest / Integration / Static

| ID | Layer | Reviewer question |
|----|------|------------------|
| MIRI-CODEC-001 | miri | Is record_kind_roundtrip test compiled with miri or is it a unit test skipped? |
| PROPTEST-EVENTSEQ-001 | proptest | Is 10k iterations enough for u64 space? Any shrinking strategy? |
| INTEGRATION-ACK-001 | integration | Does test inject persist failure via mock or real Fjall? |
| INTEGRATION-ACK-002 | integration | Does this test cover RECOVERY-003 (replay ordering) as well? |
| INTEGRATION-ACK-003 | integration | Does this cover all 11 primitives or just one? |
| INTEGRATION-ACK-004 | integration | Same coverage question as ACK-003 |
| STATIC-SCAN-001/002 | static | Does forbid(unsafe_code) apply transitively to all deps? |

---

## Open Questions for Reviewer

1. **OQ-1** (flush_batch barrier): TLA-QUEUE-001 + LOOM-QUEUE-001 are the primary evidence. Is the TLA model the authoritative spec for queue flush ordering, with Loom as the implementation reality-check?

2. **OQ-2** (BeforeJournalAppend reachability): VERUS-DM-004 + KANI-ACK-001 form a dual-lane proof. Should a future code change that adds a `BeforeJournalAppend` row be caught by Verus (compile-time proof) or Kani (enumeration)? Which lane is authoritative?

3. **OQ-3** (test_evidence stubs): INTEGRATION-ACK-001/003/004 + verify_matrix_replay_proofs() are the evidence. Is there a risk that only some primitives have real integration tests while others are stubs?

4. **Missing recovery coverage**: INTEGRATION-ACK-002 covers RECOVERY-001/002 but the trace matrix also shows RECOVERY-003. Does any single test cover replay divergence (RECOVERY-003)?

5. **TLA-QUEUE-001 QF3**: Is "no new events during flush" enforceable at the Rust level via a lock, or is it only a TLA+ invariant that requires external enforcement?

---

## Waiver Review

| Clause | Layer | Waived by | Reviewer challenge |
|--------|-------|-----------|-------------------|
| Fjall persist(SyncAll) internal UB | Kani | Oracle treatment | Is trusting Fjall as oracle acceptable given this is the core durability claim? |
| Codec fuzzing beyond RecordKind | Proptest | Bounded enum + Miri | Is Miri execution of RecordKind roundtrips sufficient代替fuzzing? |
| Async runtime scheduling | N/A | UnsupportedAsyncStrictAck | Does the error type actually prevent async strict ack at compile time? |

---

## Reviewer Decision Points

1. **Approve / Reject**: Are all 33 proof obligations correctly assigned to the cheapest lane that kills the real risk?
2. **Missing obligations**: Did we correctly identify KANI-HYDRATE-001 and KANI-REPLAY-001 as missing from the original proof-obligations.jsonl?
3. **OQ responses**: Are OQ-1/2/3 answered with sufficient evidence, or do they require additional obligations?
4. **Waiver adequacy**: Are the three waivers in verification-layers.md sufficient, or do they need formal acceptance from the black-hat reviewer?
5. **Traceability**: Does every contract clause in traceability-matrix.jsonl have at least one planned proof obligation?
