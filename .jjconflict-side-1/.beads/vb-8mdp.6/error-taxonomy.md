# Error Taxonomy — Idempotency Hydration for ActionTicket

## Top-Level Error Namespace

All errors are typed; no `String` or unstructured error messages.

### vb_core::ActionError (vb_core/src/action.rs:229)

| Variant | Semantic | Cause | Retry-Safe? |
|---------|----------|-------|-------------|
| `UnknownAction { action: ActionId }` | Action ID not registered | Dispatch to unregistered action | N/A |
| `InvalidTicket` | Ticket not found in flight | Stale or forged ticket reference | No |
| `PayloadTooLarge { max_bytes, actual_bytes }` | Input/output exceeds contract | Caller violation | No |
| `OutputSlotOutOfBounds { slot, max_slots }` | Output slot index invalid | Caller violation | No |
| `NonIdempotentReplayBlocked` | Non-idempotent action replay attempted | Recovery attempted replay of unsafe action | N/A |
| `CompletionAlreadyRecorded` | Duplicate completion attempted | Bug in action completion logic | N/A |
| `QueueFull` | Action dispatch queue at capacity | Resource exhaustion | N/A |
| `EncodingFailed` | Output encoding failed | Internal error | N/A |
| `DispatchFailed` | Action dispatch internal failure | Internal error | N/A |

### vb_core::IdempotencyViolation (vb_core/src/action.rs:67)

| Variant | Semantic | Trigger | Recovery |
|---------|----------|---------|----------|
| `MissingKey(SideEffect)` | Action has side effects but no idempotency key | KeyRequired action dispatched with empty key_slots | Provide valid idempotency key |
| `SecretInKey(u32)` | Key ingredient contains secret-tainted value | Slot with `Taint::Secret | Taint::DerivedFromSecret` used in key derivation | Remove secret from key slots |
| `RandomInKey(u32)` | Key ingredient contains random value | Slot with `Taint::Random` used in key derivation | Use deterministic slot values |
| `TimeInKey(u32)` | Key ingredient contains time-dependent value | Slot with `Taint::TimeDependent` used in key derivation | Use time-independent slot values |

### vb_storage::RecoveryError (vb_storage/src/recovery/types.rs:37)

| Variant | Semantic | Trigger | Is Idempotency-Related? |
|---------|----------|---------|--------------------------|
| `Journal(JournalError)` | Journal operation failed | I/O or corruption | Sometimes |
| `WorkflowSourceDigestMismatch { expected, found }` | Workflow source digest mismatch | Stored digest ≠ recomputed digest | No |
| `CompiledIrDigestMismatch { expected, found }` | IR digest mismatch | Stored IR digest ≠ recomputed digest | No |
| `ActionAbiMismatch { action_id }` | Action ABI digest mismatch | Action ABI changed since admission | No |
| `PolicyDigestMismatch { step }` | Policy digest mismatch | Policy changed mid-run | No |
| `NonIdempotentActionBlocked { action, step }` | Non-idempotent action cannot replay | Recovery encountered non-idempotent action | **YES — PRIMARY** |
| `ReplayDivergence { step, detail }` | Replay diverged from expected trajectory | Ticket/envelope mismatch on duplicate detection | **YES — PRIMARY** |
| `SlotTaintReadFailed { slot }` | Slot taint read failed | Taint metadata corrupt | No |
| `CorruptSlotTaint { slot }` | Slot taint metadata corrupt | Persistence corruption | No |
| `NoRecoveryData { run }` | No snapshot or events found | Run never hydrated or data lost | No |
| `CorruptSnapshot { run, seq }` | Snapshot corrupt | Persistence corruption | No |
| `TerminalStateMismatch { expected, found }` | Terminal state mismatch | Event sequence inconsistency | No |
| `FrameDimensionOverflow { run }` | Frame dimensions exceed u16 | Event count overflow | No |

## Idempotency-Specific Error Detail

### RecoveryError::NonIdempotentActionBlocked

**When**: During hydration, `apply_tail_events` encounters an `ActionScheduledTicket` for an action that is already marked `completed` or `failed` in the tracker.

**Semantic**: The action was already resolved in the original execution. During recovery replay, we cannot re-execute it because it is not idempotent and re-execution would produce duplicate side effects.

**Key**: `(action, step)` — same as `ActionReplayTracker` identity key.

**Example**: A `SideEffect::Writes` action with `RetrySafety::Unsafe` completed before crash. On recovery, we see the `ActionScheduledTicket` event, but the tracker shows the action is already resolved. We must fail closed.

### RecoveryError::ReplayDivergence

**Sub-variants by detail string**:

| Detail String | Cause |
|---------------|-------|
| `"divergent action schedule ticket"` | Same `(action, step)` scheduled twice with different `ActionTicket` evidence |
| `"action completion envelope does not match schedule ticket"` | Completion envelope's ticket doesn't match the scheduled ticket |
| `"action completion envelope does not match schedule ticket"` (from mark_completed_envelope_effect) | Completion envelope differs from existing envelope for same `(action, step)` |
| `"action completion envelope missing schedule ticket"` | `require_scheduled_ticket` called but no schedule evidence exists |
| `"divergent action completion envelope"` | Envelope for same `(action, step)` has different evidence than stored envelope |

**Critical distinction from NonIdempotentActionBlocked**:
- `NonIdempotentActionBlocked`: Action was ALREADY resolved (completed/failed) before the duplicate event.
- `ReplayDivergence`: A NEW event contradicts existing evidence for the same `(action, step)` — this indicates a bug or data corruption, not a valid duplicate.

## Error Recovery Strategy

| Error | Strategy | Can Retry Hydration? |
|-------|----------|---------------------|
| `NonIdempotentActionBlocked` | Fail closed; do not replay non-idempotent action | Yes (will block again) |
| `ReplayDivergence` | Fail closed; data inconsistency detected | No (will fail again) |
| `NoRecoveryData` | Fail with typed error | No |
| `CorruptSnapshot` | Fail with typed error | No |
| `WorkflowSourceDigestMismatch` | Fail with typed error | No |

## Error Construction Preconditions

All `RecoveryError` variants are constructed via typed `Result` returns from total functions. No `unwrap`, `expect`, or `panic` in the recovery path.

### vb_storage::SnapshotRecoveryInputViolation (hydrate.rs:94)

Internal precondition validation before full hydration:

```rust
pub(crate) enum SnapshotRecoveryInputViolation {
    SnapshotRunMismatch { snapshot_run: RunId, snapshot_seq: EventSeq },
    TailRunMismatch { expected: RunId, actual: RunId },
    TailSeqNotAfterSnapshot { snapshot_seq: EventSeq, actual_seq: EventSeq },
    NoRecoveryData { run: RunId },
}
```

These are converted to `RecoveryError` via `validate_snapshot_metadata`, `validate_tail_run_metadata`, `validate_tail_seq_after_snapshot`, and `validate_recovery_data_present`.