# Contract Specification — vb-qi37.1.5

## Context
- **Bead**: vb-qi37.1.5 — runtime/recovery: Prove replay digest mismatch detection
- **Feature**: Deterministic detection of digest mismatches during journal replay/recovery
- **Domain terms**:
  - `WorkflowDigest`: `[u8; 32]` wrapper, content-addressable identity of a compiled workflow
  - `DigestCheck`: verification level (`WorkflowSourceOnly`, `WorkflowAndIr`, `Full`)
  - `RecoveryError`: typed error taxonomy for all recovery failure modes
  - `EventSeq`: monotonically increasing sequence number for journal events
  - `JournalEvent`: durable event types written to the Fjall-backed journal
- **Assumptions**:
  - The journal is the single source of truth; recovery reads only from journal events
  - `WorkflowDigest` equality is byte-exact (`[u8; 32]` bitwise equality)
  - Corrupt bytes injected during tests are well-formed on the wire but wrong in value
- **Open questions**:
  - Does `DigestCheck::Full` include Action ABI and Policy digest checks in the current implementation?
  - What is the provenance of the policy digest recorded at admission?

## Preconditions

- **PRE-001**: `check_workflow_source_digest` requires a non-empty event list for the given `RunId`
- **PRE-002**: `verify_digests` requires `workflow_digest` and `ir_digest` to be the expected reference values; the journal `RunAccepted` event supplies the found values
- **PRE-003**: `recover_runtime_frame_seed_from_events` requires `events` to contain at least one `RunAccepted` event as the first element

## Postconditions

- **POST-001**: `check_workflow_source_digest` returns `Ok(())` iff the journal's `RunAccepted` event carries the expected `workflow` digest; returns `Err(WorkflowSourceDigestMismatch { expected, found })` otherwise
- **POST-002**: `check_compiled_ir_digest` returns `Ok(())` iff the two `WorkflowDigest` arguments are byte-equal; returns `Err(CompiledIrDigestMismatch { expected, found })` otherwise
- **POST-003**: `verify_digests` returns `Ok(())` only when all requested digest levels pass; returns the first mismatch error encountered in priority order (workflow source, then IR)
- **POST-004**: `reject_workflow_digest_mismatch` returns `Ok(())` when no `RunAccepted` event contradicts the expected digest or when the first `RunAccepted` matches; returns `Err(WorkflowSourceDigestMismatch)` when the `RunAccepted` digest differs from expected
- **POST-005**: Corruption injection tests fail with the exact `RecoveryError` variant:
  - Corrupt artifact digest → `WorkflowSourceDigestMismatch`
  - Corrupt journal sequence → `ReplayDivergence`
  - Corrupt slot value → `UnsupportedRecoveryState::slot_values_unsupported()`
  - Corrupt slot taint → `UnsupportedRecoveryState::event_slot_taint_unsupported()`

## Invariants

- **INV-001**: `WorkflowDigest` is a pure content identity — two digests are equal iff their 32 bytes are equal; no false positives or false negatives in digest comparison
- **INV-002**: `check_workflow_source_digest` is deterministic — same journal state always yields the same result
- **INV-003**: `RecoveryError` variants are exhaustive — every failure mode in the recovery system maps to exactly one variant
- **INV-004**: `UnsupportedRecoveryState` flags are monotonically additive — once set, a flag is never cleared by additional recovery events

## Error Taxonomy

| Error Variant | Trigger | Semantic Meaning |
|---|---|---|
| `WorkflowSourceDigestMismatch` | Journal `RunAccepted.workflow` ≠ expected | Workflow artifact was modified or swapped after admission |
| `CompiledIrDigestMismatch` | Computed IR digest ≠ recorded IR digest | Compiled artifact differs from what was admitted |
| `ActionAbiMismatch` | Action ABI digest mismatch during replay | Action interface changed post-admission |
| `PolicyDigestMismatch` | Policy digest mismatch during replay | Runtime policy changed post-admission |
| `ReplayDivergence` | Replay trajectory ≠ expected state machine path | Journal events were reordered, dropped, or corrupted |
| `UnsupportedRecoveryState` (slot_values_unsupported) | SlotWrittenEvent missing or corrupt body | Slot value cannot be reconstructed |
| `UnsupportedRecoveryState` (event_slot_taint_unsupported) | SlotWrittenEvent.taint absent in event-only path | Slot taint is not recorded in event-sourced mode |

## Contract Signatures

```rust
// vb_storage/src/recovery/recover.rs

/// Verifies workflow source digest matches stored record.
pub fn check_workflow_source_digest(
    journal: &FjallJournal,
    run: RunId,
    expected: WorkflowDigest,
) -> RecoveryResult<()>;

/// Verifies compiled IR digest matches.
pub fn check_compiled_ir_digest(
    expected: WorkflowDigest,
    found: WorkflowDigest,
) -> RecoveryResult<()>;

/// Verifies all digests at the requested check level.
pub fn verify_digests(
    journal: &FjallJournal,
    run: RunId,
    workflow_digest: WorkflowDigest,
    ir_digest: WorkflowDigest,
    found_ir_digest: WorkflowDigest,
    level: DigestCheck,
) -> RecoveryResult<()>;
```

## TLA+-Owned Clauses

**Not applicable**: The recovery digest verification is a deterministic pure-function property over an immutable journal event stream. There are no temporal/lifecycle/workflow state machines, no concurrency, no liveness requirements, and no fairness conditions. The digest comparison is a pure equality check over fixed byte arrays.

Rationale for non-applicability:
- No workflow protocol, scheduler, or queue behavior
- No concurrent actors or message passing
- No retry/lease/claim lifecycle
- No deadlock or liveness concerns
- No distributed state coordination

## Verus-Owned Clauses

- **VERUS-INV-001** (`INV-001`): `WorkflowDigest` byte-exact equality — `WorkflowDigest` is `[u8; 32]` and equality is bitwise
- **VERUS-POST-001** (`POST-001`): `check_workflow_source_digest` returns `Ok(())` iff journal's `RunAccepted.workflow == expected`; `Err(WorkflowSourceDigestMismatch)` otherwise
- **VERUS-POST-002** (`POST-002`): `check_compiled_ir_digest` returns `Ok(())` iff `expected == found`; `Err(CompiledIrDigestMismatch)` otherwise
- **VERUS-POST-003** (`POST-003`): `verify_digests` enforces digest level priority (workflow source before IR)
- **VERUS-POST-004** (`POST-004`): `reject_workflow_digest_mismatch` returns `Ok(())` on match or absent; `Err(WorkflowSourceDigestMismatch)` on mismatch

## Deferred Clauses (Formal Waivers Required)

### Action ABI Digest Verification (deferred to future phase)
- **Clause**: `ActionAbiMismatch` detection during recovery replay
- **Current state**: Not implemented in `verify_digests`; `DigestCheck::Full` silently passes (recover.rs:71-73)
- **Waiver required**: Owner, reason, expiry, limitation, compensating evidence
- **Compensating evidence**: Workflow source + IR digest checks are the primary defense-in-depth; Action ABI is a future hardening step

### Policy Digest Mismatch Detection (deferred)
- **Clause**: `PolicyDigestMismatch` detection during recovery replay
- **Current state**: `RecoveryError::PolicyDigestMismatch` variant defined but never instantiated in recovery code path
- **Waiver required**: Owner, reason, expiry, limitation, compensating evidence
- **Compensating evidence**: `RuntimePolicy` is enforced at admission; replay assumes same policy unless proven otherwise

## Non-goals

- Multi-node/distributed recovery
- Temporal model for recovery lifecycle (pure function, no state machine)
