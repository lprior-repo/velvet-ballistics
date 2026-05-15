# Domain Model Review — vb-core-replay-divergence-recovery

## Domain Types and Invariants

### RecoveryError Enum
File: `crates/vb_storage/src/recovery/types.rs`

All 10 variants carry semantic meaning. Every variant maps to exactly one recovery failure mode:
- 4 digest mismatch variants (workflow source, compiled IR, action ABI, policy)
- 2 action safety variants (NonIdempotentActionBlocked, ReplayDivergence)
- 2 data absence variants (NoRecoveryData, TerminalStateMismatch)
- 2 corruption/overflow variants (CorruptSnapshot, FrameDimensionOverflow)

**Invariant**: No variant is a generic "Other" or stringly-typed error.

### DigestCheck Enum
File: `crates/vb_storage/src/recovery/types.rs`

Controls which digest layers are verified at recovery time. Levels: `Skip`, `WorkflowSource`, `CompiledIr`, `ActionAbi`, `Policy`, `All`.

**Invariant**: DigestCheck is a closed enum; recovery behavior is defined for each level.

### RecoveryFrameSeed
File: `crates/vb_storage/src/recovery/types.rs`

Persisted seed containing: run_id, seq, slot_values (DecodedSlots), slot_taint (DecodedTaint), written slots count, max parallel in-flight.

**Invariant**: slot_values.len() == slot_taint.len() for all recovered slots.

### UnsupportedRecoveryState
File: `crates/vb_storage/src/recovery/types.rs`

Tracks 4 boolean categories: slot_values, slot_taint, action_payloads, pending_actions. Any `true` value means full frame hydration is blocked.

**Invariant**: DurableFrameRecoveryBoundary::hydrate_run_frame succeeds iff all 4 are `false`.

### RunFrame
File: `crates/vb_core/src/frame/mod.rs`

Runtime frame state. Recovery hydration target. Slots are written with explicit Taint values.

**Invariant**: A recovered RunFrame must have identical slot values and taints to the pre-crash frame (modulo Object/List which are unsupported).

### ActionReplayTracker
File: `crates/vb_storage/src/recovery/replay/core.rs`

Tracks Completed actions during replay to block duplicate scheduling of non-idempotent actions.

**Invariant**: Once an action reaches Completed state in the tracker, a subsequent Scheduled event for the same action produces NonIdempotentActionBlocked.

### JournalEvent
File: `crates/vb_storage/src/journal/` (implicit)

All event types for a run: RunAccepted, StepStarted, SlotWrittenEvent, ActionScheduled, ActionCompleted, etc.

**Invariant**: Events are append-only and totally ordered by (run_id, seq) in the Fjall journal.

---

## Codec Constraint

**Confirmed**: No YAML codec appears in vb_storage/src/recovery/. All encoding/decoding uses `postcard` only.

Evidence: grep for yaml/yaml::/serde_yaml in vb_storage/src/recovery/ returns zero matches.

---

## Architectural Conformance

1. **Typed errors not stringly-typed**: All RecoveryError variants are concrete types with typed fields. ✓
2. **Fail-closed on unsupported state**: DurableFrameRecoveryBoundary rejects unsupported live frame state. ✓
3. **Digest mismatch produces typed errors**: 4 dedicated variants with step+detail. ✓
4. **Replay divergence produces typed error**: ReplayDivergence with StepIdx + String. ✓
5. **No YAML in recovery**: Confirmed by grep. ✓
6. **Postcard only in hydrate paths**: hydrate_run_frame and hydrate_run_frame_from_events use only postcard::decode. ✓
