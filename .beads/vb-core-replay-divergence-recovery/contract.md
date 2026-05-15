# Contract Specification — vb-core-replay-divergence-recovery

## Context

- Feature: Recovery subsystem for vb_storage/vb_runtime — typed replay with divergence detection and no-YAML hydration
- Domain terms:
  - `ReplayDivergence` — semantic mismatch during event replay (step index + detail string)
  - `NonIdempotentActionBlocked` — non-idempotent action would re-execute on recovery
  - `DigestCheck` — enum controlling which digests (workflow source, compiled IR, action ABI, policy) are verified
  - `RecoveryFrameSeed` — persisted seed for frame hydration (run_id, seq, taint map, slot values)
  - `UnsupportedRecoveryState` — tracks 4 unsupported categories: slot_values, slot_taint, action_payloads, pending_actions
  - `hydrate_run_frame` — snapshot+tail hydration path (Postcard only, NO YAML)
  - `hydrate_run_frame_from_events` — events-only hydration path
  - `replay_events` — core typed replay with action tracking and divergence detection
  - `verify_digests` — orchestrates workflow source and compiled IR digest checks
  - `ActionReplayTracker` — prevents duplicate scheduling of non-idempotent actions during replay
- Assumptions:
  - All recovery paths use only Postcard binary codec; no YAML parsing occurs in vb_storage/src/recovery/
  - Journal events are the source of truth; snapshots are corruptible but recoverable
  - Digest verification is configurable via `DigestCheck` levels
- Open questions: None

---

## Contract Clauses

### CC-001: No YAML in Recovery Paths
- Statement: Restart/replay never reparses YAML. Recovery uses only JournalEvent enums encoded via Postcard.
- Risk: parser_codec
- Rationale: grep confirms zero YAML imports in vb_storage/src/recovery/; hydrate.rs uses only postcard::decode

### CC-002: Snapshot+Tail Hydration Fidelity
- Statement: `hydrate_run_frame(snapshot, tail_events)` produces a RunFrame identical to the pre-crash frame, respecting run_id match, seq ordering, and zero-dim guard.
- Risk: persistence, temporal
- Rationale: `hydrate_run_frame` enforces run_id, seq ordering, zero-step-count guard; slot values and taints decoded via postcard

### CC-003: Typed Digest Mismatch Errors
- Statement: `verify_digests` produces typed `RecoveryError` variants (`WorkflowSourceDigestMismatch`, `CompiledIrDigestMismatch`, `ActionAbiMismatch`, `PolicyDigestMismatch`) with exact step and detail fields.
- Risk: persistence, temporal
- Rationale: RecoveryError enum has exhaustive variants with step_idx and detail fields

### CC-004: Typed Replay Divergence
- Statement: `replay_events` produces `ReplayDivergence { step: StepIdx, detail: String }` when semantic divergence is detected; `NonIdempotentActionBlocked` blocks duplicate non-idempotent action scheduling.
- Risk: temporal
- Rationale: ReplayEngine::replay_frame_through tracks action semantics; ActionReplayTracker blocks duplicate Scheduled events

### CC-005: Fail-Closed Corrupt/Incomplete Recovery
- Statement: Corrupt snapshot → `CorruptSnapshot` error; incomplete frame → `UnsupportedRecoveryState` gates hydration closed; `reject_unsupported_live_frame_state` fails the boundary.
- Risk: persistence
- Rationale: UnsupportedRecoveryState tracks 4 categories; DurableFrameRecoveryBoundary::hydrate_run_frame fails if any are true

### CC-006: Object/List Slots Explicitly Unsupported
- Statement: RecoveredSlots marks Object and List slot kinds as unsupported; they require typed replay from CompiledWorkflow and cannot be hydrated from events alone.
- Risk: persistence
- Rationale: Tests confirm recovered_object_slots_are_explicitly_unsupported, recovered_list_slots_are_explicitly_unsupported

### CC-007: Events-Only Hydration Correctness
- Statement: `hydrate_run_frame_from_events` correctly reconstructs frame state from JournalEvents without a snapshot, respecting seq ordering and taint preservation.
- Risk: persistence, temporal
- Rationale: FrameSeedAccumulator builds recovery seed from events; workflow-guided path uses ReplayEngine for semantic replay

### CC-008: Frame Seed Round-Trip Integrity
- Statement: `RecoveryFrameSeed` produced by summary recovery round-trips through serialization (Postcard) and hydrates identical frame state.
- Risk: persistence, parser_codec
- Rationale: DurableFrameRecoveryBoundary::hydrate_run_frame and factory round-trip tests exist

---

## Preconditions

- PRE-001: `recover_runtime_frame_seed` requires a valid journal with at least one RunAccepted event and a consistent run_id
- PRE-002: `replay_events` requires the events are in strictly increasing StepIdx order
- PRE-003: `hydrate_run_frame` requires snapshot.run_id == tail_events[0].run_id (or tail is empty)
- PRE-004: `verify_digests` requires the corresponding DigestCheck level is not `Skip`

---

## Postconditions

- POST-001: On success, `recover_runtime_summary` returns a `RuntimeSummary` matching the last persisted state
- POST-002: On digest mismatch, `verify_digests` returns `RecoveryError::WorkflowSourceDigestMismatch` or `CompiledIrDigestMismatch` with populated step and detail
- POST-003: On replay divergence, `replay_events` returns `RecoveryError::ReplayDivergence` with exact step and detail
- POST-004: On non-idempotent action re-execution attempt, `replay_events` returns `RecoveryError::NonIdempotentActionBlocked`
- POST-005: On corrupt snapshot, `load_snapshot` returns `RecoveryError::CorruptSnapshot`
- POST-006: On unsupported live frame state, `DurableFrameRecoveryBoundary::hydrate_run_frame` returns `RuntimeError::UnsupportedFullRecoveryHydration`

---

## Invariants

- INV-001: All JournalEvents in a run have strictly monotonically increasing StepIdx values
- INV-002: ActionReplayTracker blocks any Scheduled event for an action already marked Completed during replay
- INV-003: RecoveryFrameSeed.slot_taint and RecoveryFrameSeed.slot_values are byte-for-byte identical after round-trip via Postcard
- INV-004: UnsupportedRecoveryState is false for all 4 categories before DurableFrameRecoveryBoundary::hydrate_run_frame succeeds
- INV-005: hydrate_run_frame never calls any YAML parser; only postcard::decode on Snapshot and JournalEvent bytes

---

## Error Taxonomy

All recovery errors are typed variants of `RecoveryError`:

| Variant | Semantic | Fields |
|---|---|---|
| `WorkflowSourceDigestMismatch` | Workflow source digest does not match | `step: StepIdx`, `detail: String` |
| `CompiledIrDigestMismatch` | Compiled IR digest does not match | `step: StepIdx`, `detail: String` |
| `ActionAbiMismatch` | Action ABI digest does not match | `step: StepIdx`, `detail: String` |
| `PolicyDigestMismatch` | Policy digest does not match | `step: StepIdx`, `detail: String` |
| `NonIdempotentActionBlocked` | Non-idempotent action would re-execute | `step: StepIdx`, `detail: String` |
| `ReplayDivergence` | Semantic divergence during replay | `step: StepIdx`, `detail: String` |
| `NoRecoveryData` | No journal data for run | — |
| `CorruptSnapshot` | Snapshot bytes failed postcard decode | — |
| `TerminalStateMismatch` | Terminal event state mismatch | `step: StepIdx`, `detail: String` |
| `FrameDimensionOverflow` | Frame dimensions exceed limits | `detail: String` |

---

## Verus-Owned Clauses

- INV-001: JournalEvent seq ordering preserved by vb_storage journal writer — proven by miri on integration tests
- INV-003: RecoveryFrameSeed Postcard round-trip — proven by miri on unit tests
- INV-005: No YAML parser invocation in hydrate.rs — verified by grep + miri on integration tests

---

## TLA+-Owned Clauses

No TLA+ model is required for this bead. Rationale:
- Recovery behavior is single-writer sequential replay from a Fjall journal
- No concurrent workflow transitions; the replay is deterministic given the event stream
- State-over-time properties (seq ordering, no double-scheduling) are covered by miri on existing tests
- No distributed consensus, scheduler, or protocol with temporal liveness requirements

If future work introduces concurrent recovery workers, a TLA+ model will be required.

---

## Theorem-Owned Clauses

None. All critical invariants are covered by Verus/miri obligations or existing test evidence.

---

## Non-Goals

- Formal proof of Fjall journal durability (covered by integration tests and existing evidence)
- TLA+ model for sequential replay (deterministic single-writer; miri covers ordering)
- Recovery performance benchmarking (existing criterion evidence covers hot paths)
