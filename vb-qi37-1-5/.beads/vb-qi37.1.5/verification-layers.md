# Verification Layers — vb-qi37.1.5

## Boundary
- **Verus-owned kernel**: Digest equality (`WorkflowDigest` byte comparison), `check_workflow_source_digest`, `check_compiled_ir_digest`, `verify_digests`, `reject_workflow_digest_mismatch` pure functions
- **Kani scope**: Bounded state-transition safety for recovery error variant exhaustive matching
- **Unit/property tests**: Corruption injection tests for all four missing test cases
- **Runtime shell**: `FjallJournal` I/O, journal event deserialization, `RunFrame` hydration
- **External systems**: None

## Layer Assignment

| Clause ID | Layer | Justification |
|---|---|---|
| INV-001 | `verus` + `proptest` | Byte-exact digest equality; proptest for brute-force equality exploration |
| INV-002 | `verus` | Deterministic `check_workflow_source_digest` — pure function, no side effects |
| INV-003 | `verus` | `RecoveryError` exhaustive variant mapping via Verus proof |
| INV-004 | `verus` | `UnsupportedRecoveryState` monotonic flag union — pure function |
| PRE-001 | `verus` | Non-empty event list precondition for `check_workflow_source_digest` |
| PRE-002 | `verus` | `verify_digests` reference-value precondition |
| PRE-003 | `verus` | `RunAccepted` as first event precondition |
| POST-001 | `verus` + `kani` | Workflow source digest mismatch detection — `Ok(())` vs `Err(WorkflowSourceDigestMismatch)` |
| POST-002 | `verus` + `kani` | IR digest mismatch detection — `Ok(())` vs `Err(CompiledIrDigestMismatch)` |
| POST-003 | `verus` | `verify_digests` orchestration with level priority |
| POST-004 | `verus` + `kani` | `reject_workflow_digest_mismatch` — Ok vs Err behavior |
| POST-005 | `integration-tests` | Corruption injection tests — exact error variant per corruption type |
| ERR-MAP-001 | `verus` + `kani` | Every recovery failure mode maps to exactly one `RecoveryError` variant |

## Verus Scope

### Target: `crates/vb_storage/src/recovery/recover.rs`

**Functions**:
- `check_workflow_source_digest`
- `check_compiled_ir_digest`
- `verify_digests`

**Spec functions** (no ghost code required):
- `spec_check_workflow_source_digest_matches`: `WorkflowDigest × [JournalEvent] → bool`
- `spec_check_ir_digest_equal`: `WorkflowDigest × WorkflowDigest → bool`

**Invariants to prove**:
- `check_workflow_source_digest(journal, run, expected) == Ok(())` iff the first `RunAccepted` event in `journal.events_for_run(run)` has `workflow == expected`
- `check_compiled_ir_digest(expected, found) == Ok(())` iff `expected == found` (byte equality)
- `verify_digests` returns first error in priority order (workflow source → IR)

**Trusted boundary**: `WorkflowDigest` constructor and equality are trusted; `FjallJournal` I/O is excluded from Verus proof

**Shell exclusions**: All I/O (`FjallJournal` reads), deserialization, async scheduling, wall-clock time

### Target: `crates/vb_storage/src/recovery/types.rs`

**Types**:
- `WorkflowDigest` (line 330 in vb_core)
- `RecoveryError` enum
- `UnsupportedRecoveryState`

**Invariants**:
- `RecoveryError` variants are exhaustive
- `UnsupportedRecoveryState::union` is monotonic: `flag_set ⟹ flag_set ∪ new_flags`

## Kani Scope

**Target**: `crates/vb_storage/src/recovery/recover.rs`

Bounded model check for:
- All `DigestCheck` levels produce correct error variant
- `verify_digests` level priority is respected
- `check_workflow_source_digest` returns `NoRecoveryData` when run has no events

**Bounds**: `RunId`, `WorkflowDigest`, `DigestCheck` — small finite state space

## Integration Test Scope (State 8)

Corruption injection in `crates/vb_storage/tests/recovery_integration.rs`:

| Test | Corruption Type | Expected Error |
|---|---|---|
| `corrupt_artifact_digest_fails_with_workflow_source_digest_mismatch` | Mutate `RunAccepted.workflow` field | `WorkflowSourceDigestMismatch` |
| `corrupt_journal_sequence_fails_with_replay_divergence` | Corrupt `EventSeq` ordering | `ReplayDivergence` |
| `corrupt_slot_value_fails_with_slot_values_unsupported` | Corrupt slot bytes in `SlotWrittenEvent` | `UnsupportedRecoveryState::slot_values_unsupported()` |
| `corrupt_slot_taint_fails_with_event_slot_taint_unsupported` | Corrupt `extra` (taint) field | `UnsupportedRecoveryState::event_slot_taint_unsupported()` |

## Waivers

- **TLA+**: No temporal/workflow/protocol behavior in scope — TLA+ would be a formal façade with no discriminatory power
- **Lean/Aeneas/Hax**: Digest equality is `[u8; 32]` bit comparison — Verus is sufficient
- **Loom/Shuttle**: No concurrent recovery paths; replay is single-threaded and sequential
- **Miri**: No unsafe code in vb_storage recovery; `#[forbid(unsafe_code)]` enforced
