# Proof Repair Guide — vb-core-strict-ack-ordering

**Bead ID**: vb-core-strict-ack-ordering
**Workspace**: /tmp/vb-ws/vb-core-strict-ack-ordering
**State**: 6 → back to proof-writer for repair

---

## Overview

**STATUS: REJECTED — 6 fatal defects, 5 moderate defects**

This guide records repair instructions for each defective proof artifact. After all repairs are complete, re-run `proof-reviewer` in State 6.

---

## FATAL DEFECTS — Must Fix Before Re-Submission

---

### REP-001: Uncomment and Fix All Verus Specs

**Affected obligations**: VERUS-DM-001, VERUS-DM-002, VERUS-DM-003, VERUS-DM-004, VERUS-JA-001, VERUS-JA-002

**Artifacts**:
- `verus_artifacts/durability_matrix.verus`
- `verus_artifacts/types_eventseq.verus`
- `verus_artifacts/append_strict_journaled.verus`

**Repair steps**:

1. **Uncomment all `verus!` blocks** in each file. Remove `//` from every line of the `verus! { ... }` blocks.

2. **Fix `durability_matrix.verus` line 65**: Remove the `assert_seqs_equal()` call. Replace with direct structural assertion:
   ```rust
   assert(matrix_primitives_set() == required_primitives_set()) by {
       // Direct set equality: each REQUIRED_PRIMITIVES entry has a matching
       // DURABILITY_MATRIX row with the same primitive name
       assert(forall |i: int| 0 <= i < 11 ==> {
           let prim = REQUIRED_PRIMITIVES[i as int];
           exists |j: int| 0 <= j < 11 && DURABILITY_MATRIX[j as int].primitive == prim
       });
   }
   ```

3. **Add Verus proof annotations to source files**: The proof-writer report says "Verus specs are written as `verus! { ... }` blocks to be added to the existing Rust source files." Copy the uncommented spec/proof functions into:
   - `crates/vb_runtime/src/durability_matrix.rs` (VERUS-DM-001/002/004)
   - `crates/vb_storage/src/types.rs` (VERUS-DM-003)
   - `crates/vb_storage/src/journal/append.rs` (VERUS-JA-001/002)

4. **Verify**: Run `verus crates/vb_runtime/src/durability_matrix.rs` and confirm 0 Verus errors.

---

### REP-002: Fix TLA-BARRIER-001 — Replace `IF TRUE` with Dual-Path Model

**Affected obligation**: TLA-BARRIER-001

**Artifact**: `specs/JournalBarrier.tla`

**Problem**: `AppendStrict` action (line 56) uses `IF TRUE`, making the persist barrier always succeed. The failure path (append succeeds, persist fails) is not exercised.

**Repair steps**:

1. Add a state variable `persistSucceeds: BOOLEAN` to model whether the barrier succeeds.

2. Modify `AppendStrict`:
   ```tla
   AppendStrict(e) ==
     /\ profile = "Strict"
     /\ ackSent = FALSE
     /\ \E succeed \in BOOLEAN :
         IF succeed
         THEN /\ appendedEvents' = appendedEvents \cup {e}
              /\ persistedEvents' = appendedEvents'  \* barrier reached
              /\ persistError' = FALSE
              /\ ackSent' = FALSE
         ELSE /\ appendedEvents' = appendedEvents \cup {e}
              /\ UNCHANGED <<persistedEvents, persistError>>
   /\ UNCHANGED profile
   ```

   This allows TLC to explore both success and failure paths.

3. Ensure `PersistError` action remains for independent failure injection:
   ```tla
   PersistError ==
     /\ profile = "Strict"
     /\ persistError' = TRUE
     /\ ackSent' = FALSE
     /\ UNCHANGED <<appendedEvents, persistedEvents>>
     /\ UNCHANGED profile
   ```

4. Verify `tlc -config specs/JournalBarrier.cfg specs/JournalBarrier.tla` reports no invariant violations on I1-I5 and T1 holds.

---

### REP-003: Fix TLA-QUEUE-001 — Add Append Pre-condition to CompleteFlush

**Affected obligation**: TLA-QUEUE-001

**Artifact**: `specs/QueuedStrictFlush.tla`

**Problem**: `CompleteFlush` can fire without all queued events being appended. QF1 checks the resulting state where `queue' = {}`, making the quantifier vacuously true.

**Repair steps**:

1. Restructure so `CompleteFlush` only fires when all events are appended. Remove `CompleteFlush` as a separate action. Instead, let `CallPersistStrict` advance to `strictFlushComplete`:

2. Replace `CompleteFlush` with a condition in the action chain:
   ```tla
   (* AppendAllQueuedStrict: append all strict events before calling barrier *)
   AppendAllQueuedStrict ==
     /\ flushInProgress
     /\ strictFlushComplete = FALSE
     /\ \A e \in Nat :
          [profile |-> "Strict", event |-> e] \in queue
          => e \in appendedEvents
     /\ strictFlushComplete' = TRUE
     /\ UNCHANGED <<queue, flushInProgress, appendedEvents, persistBarrierCalled>>
   ```

   Or merge the check into `CallPersistStrict`:
   ```tla
   CallPersistStrict ==
     /\ flushInProgress
     /\ strictFlushComplete = FALSE
     /\ persistBarrierCalled = 0
     (* QF1: all strict events must be appended before barrier *)
     /\ \A e \in Nat :
          [profile |-> "Strict", event |-> e] \in queue
          => e \in appendedEvents
     /\ persistBarrierCalled' = 1
     /\ UNCHANGED <<queue, flushInProgress, strictFlushComplete, appendedEvents>>
   ```

3. Update `Next` to include `AppendAllQueuedStrict` or use the merged `CallPersistStrict`.

4. Update QF1 to check the state when `flushInProgress = FALSE` AND `strictFlushComplete = TRUE`:
   ```tla
   QF1 == strictFlushComplete =>
       \A e \in Nat :
         [profile |-> "Strict", event |-> e] \in queue
         => e \in appendedEvents
   ```
   (Note: after CompleteFlush, queue' = {}, so this must be checked at the transition.)

5. Better approach: Add a separate `strictFlushCompleteWithAllAppended` state variable, or use a two-step flush where `strictFlushComplete' = TRUE` only after appending all events.

---

### REP-004: Rewrite KANI-DISPATCH-001/002 — Verify Actual Dispatch Behavior

**Affected obligations**: KANI-DISPATCH-001, KANI-DISPATCH-002

**Artifacts**:
- `kani_harnesses/verify_strict_profile_dispatches_to_append_strict.rs`
- `kani_harnesses/verify_journaled_profile_dispatches_to_append_journaled.rs`

**Problem**: Both harnesses use `kani::any()` then trivially re-check the same match. No actual dispatch verification.

**Repair steps for KANI-DISPATCH-001**:

```rust
#[kani::proof]
fn verify_strict_profile_dispatches_to_append_strict() {
    use vb_runtime::{DurabilityProfile, StorageRuntimeJournal};
    use vb_storage::FjallJournal;

    // Construct a journal with Strict profile
    let profile = DurabilityProfile::Strict;
    let journal: FjallJournal = kani::any();  // Use mock/stub for Kani

    // Create a StorageRuntimeJournal with Strict profile
    let mut runtime_journal: StorageRuntimeJournal = StorageRuntimeJournal::new(
        journal,
        profile,
    );

    // Create a test event
    let event: vb_storage::JournalEvent = kani::any();

    // Call append_storage_event with Strict profile
    // The harness should verify this calls append_strict (not append_journaled)
    let result = runtime_journal.append_storage_event(&event);

    // Verify: if profile is Strict, result reflects append_strict behavior
    // (not append_journaled which would not call persist_strict)
    kani::assert(
        result.is_ok(),  // or more specific assertion about the behavior
        "append_storage_event with Strict profile must succeed via append_strict",
    );
}
```

**Key**: The harness must actually call `append_storage_event` on a `StorageRuntimeJournal` and verify the dispatch goes to the correct method.

---

### REP-005: Replace KANI-HYDRATE-001 Placeholder with Real Harness

**Affected obligation**: KANI-HYDRATE-001

**Artifact**: `kani_harnesses/verify_hydrate_run_frame_digest_matches.rs`

**Repair steps**:

```rust
#[kani::proof]
fn verify_hydrate_run_frame_digest_matches() {
    use vb_storage::recovery::{recover_runtime_frame_seed_from_events, RecoveryFrameSeed};
    use vb_core::RunId;

    // Construct a minimal event sequence for a run with known header
    let run_id: RunId = RunId::new();
    let events: Vec<JournalEvent> = kani::any();

    // Call the recovery function
    let result = recover_runtime_frame_seed_from_events(run_id, &events);

    // RECOVERY-002: if recovery succeeds, digest must match persisted header
    match result {
        Ok(seed) => {
            // The header digest in the run events must match the recovered seed digest
            kani::assert(
                seed.matches_expected_header(),
                "Recovered frame seed digest must match persisted header digest",
            );
        }
        Err(_) => {
            // Recovery failure is acceptable for invalid sequences
        }
    }
}
```

---

### REP-006: Replace KANI-REPLAY-001 Placeholder with Real Harness

**Affected obligation**: KANI-REPLAY-001

**Artifact**: `kani_harnesses/verify_replay_divergence_detected.rs`

**Repair steps**:

```rust
#[kani::proof]
fn verify_replay_divergence_detected() {
    use vb_storage::recovery::{replay_events, ReplayDivergence, ReplayTracker};
    use vb_core::RunId;

    // Construct an out-of-order event sequence (divergent)
    let events: Vec<JournalEvent> = kani::any();
    let mut tracker: ReplayTracker = kani::any();

    // Call replay_events with divergent ordering
    let result = replay_events(events, &mut tracker);

    // RECOVERY-003: out-of-order events must produce Err(ReplayDivergence)
    match result {
        Err(ReplayDivergence { .. }) => {
            // Expected — divergence was detected
        }
        Ok(_) => {
            // Divergence not detected — contract violated
            kani::assert(false, "replay_events must detect ordering divergence");
        }
    }
}
```

---

## MODERATE DEFECTS — Fix Before Approval

---

### REP-007: Write Behavioral Integration Tests

**Affected obligations**: INTEGRATION-ACK-001, INTEGRATION-ACK-002, INTEGRATION-ACK-003, INTEGRATION-ACK-004

**Artifacts**:
- `integration_tests/submit_direct_durability_test.rs`
- `integration_tests/recovery_digest_match_test.rs`
- `integration_tests/action_completion_ack_test.rs`
- `integration_tests/ask_completion_ack_test.rs`

**Problem**: Tests only construct error variants, never call runtime or verify behavior.

**Repair for INTEGRATION-ACK-001**:
```rust
#[test]
fn submit_direct_returns_durability_error_before_ack_when_header_cannot_persist() {
    // 1. Create mock FjallJournal that returns error on persist
    // 2. Create Runtime with this journal
    // 3. Call submit_direct with a valid action
    // 4. Assert: Err(RuntimeError::AdmissionHeaderPersistenceFailed(_)) returned
    // 5. Assert: no acknowledgement present in response
}
```

Each integration test must follow the GIVEN/WHEN/THEN pattern with a mock FjallJournal that injects the specific failure.

---

### REP-008: Re-annotate Loom Tests with `#[loom::test]`

**Affected obligations**: LOOM-QUEUE-001, LOOM-QUEUE-002, LOOM-QUEUE-003, LOOM-QUEUE-004

**Artifact**: `loom_models/queue_concurrency.rs`

**Repair steps**:

1. Change all `#[test]` to `#[loom::test]`:
   ```rust
   #[loom::test]
   fn flush_batch_strict_ordering() { ... }
   ```

2. Change `thread::spawn` to `loom::thread::spawn`:
   ```rust
   let enqueue_handle = loom::thread::spawn(move || { ... });
   ```

3. Verify with `cargo loom --test flush_batch_strict_ordering` — loom must explore thread interleavings, not just run once.

---

### REP-009: Fix Proptest Monotonicity Test

**Affected obligation**: PROPTEST-EVENTSEQ-001

**Artifact**: `proptest_cases/event_seq_ordering.rs`, line 44

**Problem**: `prop_assert!(v1 < v2 ==> { ... })` can be vacuous when `!(v1 < v2)`.

**Repair**:
```rust
proptest! {
    #[test]
    fn event_seq_monotonic(v1: u64, v2: u64) {
        prop_assume!(v1 < v2);  // Filter to valid pairs
        let seq1 = EventSeq::new(v1);
        let seq2 = EventSeq::new(v2);
        prop_assert!(
            seq1.get() < seq2.get(),
            "EventSeq monotonicity violated: new({}) >= new({})",
            v1, v2
        );
    }
}
```

---

### REP-010: Fix Hardcoded RecordKind List in KANI-CODEC-001

**Affected obligation**: KANI-CODEC-001

**Artifact**: `kani_harnesses/verify_record_kind_codec.rs`

**Problem**: Hardcoded 21-variant list may diverge from actual `RecordKind` enum.

**Repair**: Use `strum::VariantNames` or derive to auto-discover variants:
```rust
// Instead of hardcoding:
let variants: [RecordKind; 21] = [ ... ];

// Use macro to enumerate:
use strum::VariantArray;
let variants = RecordKind::VARIANTS;
```

Or use `const GENERIC` bounds if available.

---

## Repair Completion Checklist

After all repairs, verify:

- [ ] All `verus!` blocks uncommented; `verus` runs with 0 errors
- [ ] `jq -c . proof-obligations.planned.jsonl` validates
- [ ] TLA+ models run with `tlc` and produce no invariant violations
- [ ] KANI-DISPATCH-001/002 harnesses actually call `append_storage_event`
- [ ] KANI-HYDRATE-001 harness creates journal and calls recovery function
- [ ] KANI-REPLAY-001 harness constructs out-of-order events and checks for `ReplayDivergence`
- [ ] All 4 integration tests have mock FjallJournal and behavioral assertions
- [ ] All 4 loom tests use `#[loom::test]` and `loom::thread::spawn`
- [ ] Proptest `event_seq_monotonic` uses `prop_assume!`
- [ ] RecordKind codec harness uses auto-enumeration, not hardcoded list

---

*Proof repair guide for vb-core-strict-ack-ordering. Generated at State 6 rejection.*
