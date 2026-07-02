# Codebase Map - vb-8mdp.6: ActionTicket Idempotency Hydration Tests

## Bead Scope
Add deterministic idempotency hydration tests for ActionTicket persistence/hydration path.

## 1. Core Types

### ActionTicket (vb_core)
- **File**: `crates/vb_core/src/action.rs:138`
- **Type signature**:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub struct ActionTicket {
      pub run: RunId,           // Owning run
      pub step: StepIdx,        // Step that issued the action
      pub seq: SeqNo,           // Monotonic sequence within the run
      pub action: ActionId,     // Action being invoked
      pub attempt: u16,         // Current attempt number (1-indexed)
      pub idempotency_key: u128, // Idempotency key for deduplication and replay
      pub capacity: u16,        // Maximum attempts allowed (capacity bound from retry policy)
  }
  ```

### compute_action_idempotency_key (vb_core)
- **File**: `crates/vb_core/src/action.rs:157`
- **Signature**: `pub fn compute_action_idempotency_key(run: RunId, seq: SeqNo, action: ActionId) -> u128`
- Deterministic key derivation using wrapping arithmetic with hardcoded constants.

### action_ticket_has_valid_key (vb_core)
- **File**: `crates/vb_core/src/action.rs:171`
- **Signature**: `pub fn action_ticket_has_valid_key(ticket: ActionTicket) -> bool`
- Returns true when `ticket.idempotency_key == compute_action_idempotency_key(ticket.run, ticket.seq, ticket.action)`

### RetrySafety::KeyRequired (vb_core)
- **File**: `crates/vb_core/src/action.rs:46-53`
- **Variant**: `KeyRequired = 1` — "safe to retry IF an idempotency key is present"
- Part of `enum RetrySafety { Safe = 0, KeyRequired = 1, Unsafe = 2 }`

### Idempotency enum (vb_core)
- **File**: `crates/vb_core/src/action.rs` (search for `pub enum Idempotency`)
- Variants: `DeterministicPure`, `IdempotentExternal`, `AtLeastOnceExternal`

## 2. Journal/Storage Persistence

### JournalEvent variants with ActionTicket
- **File**: `crates/vb_storage/src/events.rs:95-127`
- **`ActionScheduledTicket`** (line 95):
  ```rust
  ActionScheduledTicket {
      run: RunId,
      seq: EventSeq,
      ticket: ActionTicket,   // Full action ticket issued by runtime
      input: SlotIdx,
      output: SlotIdx,
  }
  ```
- **`ActionCompletedEnvelope`** (line 108):
  ```rust
  ActionCompletedEnvelope {
      run: RunId,
      seq: EventSeq,
      ticket: ActionTicket,   // Full action ticket completed by runtime
      output: SlotIdx,
      outcome: DurableActionOutcome,
      value: Vec<u8>,
      encoded_len: u32,
      taint: Taint,
      value_digest: [u8; 32], // BLAKE3 digest of `value`
  }
  ```

### Hydration Entry Points (vb_storage)
- **File**: `crates/vb_storage/src/recovery/hydrate.rs`
- **`hydrate_run_frame`** (line 202): Reconstruct RunFrame from snapshot + tail events
- **`hydrate_run_frame_from_events`** (line 344): Reconstruct RunFrame from events only

### Hydrate Support Functions (vb_storage)
- **File**: `crates/vb_storage/src/recovery/hydrate_support.rs`
- **`verify_action_ticket_event`** (line 57): Validates ticket.run == run, attempt bounds (0 < attempt <= capacity), idempotency key validity via `action_ticket_has_valid_key`
- **`verified_action_envelope_digest`** (line 79): Verifies action envelope digest
- **`apply_tail_events`** (line 262): Applies tail journal events to mutable RunFrame
- **`compute_parallel_in_flight`** (line 480): Computes parallel in-flight counters from action events

### ActionReplayTracker (vb_storage)
- **File**: `crates/vb_storage/src/recovery/types.rs:415`
- **Type**:
  ```rust
  pub struct ActionReplayTracker {
      scheduled_tickets: HashMap<(ActionId, StepIdx), ActionScheduleEvidence>,
      completed: HashSet<(ActionId, StepIdx)>,
      failed: HashSet<(ActionId, StepIdx)>,
      completed_envelopes: HashMap<(ActionId, StepIdx), ActionCompletionEvidence>,
  }
  ```
- **`ActionScheduleEvidence`** (line 423): `{ ticket: ActionTicket, input: SlotIdx, output: SlotIdx }`
- **`ActionCompletionEvidence`** (line 430): `{ ticket: ActionTicket, output: SlotIdx, encoded_len: u32, taint: Taint, value_digest: [u8; 32] }`
- Key methods:
  - **`mark_scheduled_ticket_effect`** (line 455): Records ticket schedule, detects duplicates, returns `ActionReplayEffect::Apply|Duplicate`
  - **`require_scheduled_ticket`** (line 486): Enforces completion envelope matches schedule
  - **`mark_completed_envelope_effect`** (line 511): Records completion, detects divergent duplicates
  - **`is_resolved`** (line 589): `completed.contains(&(action, step)) || failed.contains(&(action, step))`

### IdempotencyTracker (vb_runtime)
- **File**: `crates/vb_runtime/src/idempotency.rs:34`
- **Type**:
  ```rust
  pub struct IdempotencyTracker {
      completed: Map<u128, ActionTicket>,      // Keyed by idempotency_key
      order: Vec<u128>,                        // Insertion order for FIFO eviction
      capacity: usize,
      cursor: usize,
      at_least_once_completed: Set<u128>,
  }
  ```
- Key methods:
  - **`mark_completed`** (line 108): Records completion by idempotency_key, returns `Err(ActionError::CompletionAlreadyRecorded)` on duplicate
  - **`is_completed`** (line 151): `completed.contains_key(&ticket.idempotency_key)`
  - **`is_duplicate_completion`** (line 158)
  - **`track_for_policy`** (line 173): Policy-aware dispatch tracking

## 3. Test Patterns

### Existing Hydration Tests (vb_storage)
- **File**: `crates/vb_storage/src/recovery/tests.rs:2063` (`hydrate_run_frame_tests` module)
- Key test helpers:
  - **`action_ticket`** (line 2094): Creates ticket with `compute_action_idempotency_key(run, seq, action)`
  - **`action_scheduled_ticket_event`** (line 2111): Builds `JournalEvent::ActionScheduledTicket`
  - **`action_completed_envelope_event`** (line 2127): Builds `JournalEvent::ActionCompletedEnvelope`
- Existing tests at line 2772+ cover:
  - `hydrate_run_frame_from_events_applies_duplicate_identical_action_completed_envelope_once`
  - `hydrate_run_frame_from_events_rejects_divergent_action_completed_envelope_duplicate`
  - `hydrate_run_frame_from_events_deduplicates_identical_action_scheduled_ticket`
  - `hydrate_run_frame_from_events_rejects_divergent_action_scheduled_ticket`
  - `hydrate_run_frame_from_events_rejects_completion_output_that_differs_from_schedule`

### vb_h6ix_tests (vb_storage)
- **File**: `crates/vb_storage/src/recovery/vb_h6ix_tests.rs`
- Tests attempt-based filtering in `ActionReplayTracker`

### Public Idempotency Tests (workspace_tests)
- **File**: `crates/workspace_tests/idempotency_suite/tests/vb_ko29_5_public_idempotency.rs`
- **`ticket`** helper (line 37): `ActionTicket { run, step, seq, action, attempt, idempotency_key, capacity }`
- Uses `FjallJournal` for journal persistence tests

### Proptest patterns (vb_runtime)
- **File**: `crates/vb_runtime/src/verification/proptest/mod.rs`
- **`arb_ticket`** (line 11): Arbitrary ActionTicket generation

## 4. Relevant Files Summary

| File | Relevance |
|------|-----------|
| `crates/vb_core/src/action.rs:138` | ActionTicket struct definition |
| `crates/vb_core/src/action.rs:157` | `compute_action_idempotency_key` |
| `crates/vb_core/src/action.rs:171` | `action_ticket_has_valid_key` |
| `crates/vb_core/src/action.rs:46` | RetrySafety::KeyRequired |
| `crates/vb_storage/src/events.rs:95-127` | JournalEvent ActionScheduledTicket/ActionCompletedEnvelope |
| `crates/vb_storage/src/recovery/hydrate.rs:202,344` | `hydrate_run_frame`, `hydrate_run_frame_from_events` |
| `crates/vb_storage/src/recovery/hydrate_support.rs:57,79` | `verify_action_ticket_event`, `verified_action_envelope_digest` |
| `crates/vb_storage/src/recovery/types.rs:415` | ActionReplayTracker |
| `crates/vb_runtime/src/idempotency.rs:34` | IdempotencyTracker |
| `crates/vb_storage/src/recovery/tests.rs:2063` | `hydrate_run_frame_tests` module |
| `crates/vb_storage/src/recovery/vb_h6ix_tests.rs` | Attempt-based filtering tests |
| `crates/workspace_tests/idempotency_suite/tests/vb_ko29_5_public_idempotency.rs` | Public idempotency API tests |

## 5. Risk Tags

- **persistence**: Durable journal records carry ActionTicket across crash recovery
- **idempotency**: Duplicate detection via idempotency_key in hot vs cold paths
- **hydration**: Reconstructing frame state from partial snapshot + tail events
- **concurrency**: ActionReplayTracker HashMap access in multi-threaded replay
- **recovery**: Cross-crash state reconstruction correctness

## 6. Verifier Modes

- **proptest**: Property-based tests in `vb_storage/src/recovery/tests.rs` and `vb_runtime/src/verification/proptest/mod.rs`
- **kani**: Bounded model checking in `crates/vb_storage/src/kani_recovery_hydrate.rs` and `crates/vb_runtime/src/verification/kani/kani_shard_lifecycle_harnesses.rs`
- **integration**: `crates/workspace_tests/idempotency_suite/tests/vb_ko29_5_public_idempotency.rs`

## 7. Open Questions / MISSING Coverage

1. **MISSING**: Deterministic tests for `hydrate_run_frame` with `KeyRequired` ActionTicket in `ActionScheduledTicket` → `ActionCompletedEnvelope` event chain
2. **MISSING**: Tests proving `ActionReplayTracker.mark_scheduled_ticket_effect` correctly rejects divergent tickets with same (action, step) but different ticket identity
3. **MISSING**: Tests for `verify_action_ticket_event` with invalid idempotency_key specifically
4. **MISSING**: Tests proving `IdempotencyTracker.mark_completed` correctly evicts oldest entry at capacity during hydration replay
5. **UNKNOWN**: Whether `vb_storage/src/recovery/tests.rs` has dedicated `KeyRequired` + idempotency_key tests (grep found no matches)
