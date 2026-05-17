# Codebase Map - vb-qi37.1.5

## Bead Title
runtime/recovery: Prove replay digest mismatch detection

## Session
State 2 explore (fresh session, verified against source)

## Scope
Digest mismatch detection for replay/recovery integrity.

## Risk Tags
- persistence (recovery integrity)
- critical (runtime safety)

## Required Verifier Modes
- integration tests with corruption injection
- formal proof of digest invariants

---

## Relevant Crates and Files

### vb_storage (primary)
**Path:** `crates/vb_storage/`

| File | Purpose |
|------|---------|
| `src/recovery/mod.rs` | Recovery module re-exports |
| `src/recovery/types.rs` | `RecoveryError` enum with digest mismatch variants |
| `src/recovery/recover.rs` | `check_workflow_source_digest`, `check_compiled_ir_digest`, `verify_digests` |
| `src/recovery/replay/mod.rs` | Replay module re-exports |
| `src/recovery/replay/core.rs` | `replay_events` with divergence detection and non-idempotent blocking |
| `src/recovery/replay/summary.rs` | `reject_workflow_digest_mismatch`, `recover_runtime_frame_seed_from_events_with_workflow` |
| `src/recovery/hydrate.rs` | `hydrate_run_frame`, `hydrate_run_frame_from_events` |
| `src/recovery/hydrate_support.rs` | Snapshot decoding and tail event application |
| `src/recovery/tests.rs` | Unit tests for digest mismatch detection (2464 lines) |
| `tests/recovery_integration.rs` | Integration tests (1175 lines) - MISSING corruption injection tests |

### vb_runtime
**Path:** `crates/vb_runtime/`

| File | Purpose |
|------|---------|
| `src/recovery.rs` | `DurableFrameRecoveryBoundary`, `SummaryRecoveryBoundary`, runtime recovery boundary |

### vb_core
**Path:** `crates/vb_core/`

| File | Purpose |
|------|---------|
| `src/ids/mod.rs` | `WorkflowDigest` definition (line 330: `[u8; 32]` wrapper) |
| `src/frame/mod.rs` | `RunFrame` type used in hydration |
| `src/replay.rs` | `ReplayEngine`, `ReplayError` for deterministic replay |

### reference
**Path:** `reference/src/replay_model.rs` (179 lines)

Canonical reference implementation for journal replay validation.

---

## Key Symbols and APIs

### Digest Mismatch Detection

| Symbol | File | Line | Purpose |
|--------|------|------|---------|
| `check_workflow_source_digest` | recover.rs | 21 | Verifies workflow source digest matches stored record |
| `check_compiled_ir_digest` | recover.rs | 42 | Verifies compiled IR digest |
| `verify_digests` | recover.rs | 54 | Verifies digests at requested `DigestCheck` level |
| `reject_workflow_digest_mismatch` | replay/summary.rs | 182 | Rejects on workflow digest mismatch in events |

### Recovery Error Variants

| Variant | File | Line | Purpose |
|---------|------|------|---------|
| `WorkflowSourceDigestMismatch` | types.rs | 24 | Workflow source digest mismatch |
| `CompiledIrDigestMismatch` | types.rs | 32 | Compiled IR digest mismatch |
| `ActionAbiMismatch` | types.rs | 40 | Action ABI digest mismatch (deferred) |
| `PolicyDigestMismatch` | types.rs | 46 | Policy digest mismatch at step |
| `ReplayDivergence` | types.rs | 62 | Replay diverged from expected trajectory |
| `NonIdempotentActionBlocked` | types.rs | 54 | Non-idempotent action cannot be re-executed |

### Recovery Types

| Type | File | Line | Purpose |
|------|------|------|---------|
| `DigestCheck` | types.rs | 364 | `WorkflowSourceOnly`, `WorkflowAndIr`, `Full` |
| `RecoveryFrameSeed` | types.rs | 283 | Minimal live-frame seed from durable events |
| `RecoveredSlotEntry` | types.rs | 202 | Slot value + taint recovered |
| `ActionReplayTracker` | types.rs | 322 | Tracks completed/failed actions to block re-execution |

---

## Existing Tests (Unit Level)

### vb_storage/src/recovery/tests.rs
- `compiled_ir_digest_mismatch_fails` (line 887)
- `check_workflow_source_digest_returns_mismatch_when_digests_differ` (line 1177)
- `check_compiled_ir_digest_returns_mismatch_when_digests_differ` (line 1224)
- `verify_digests_returns_mismatch_when_ir_differs` (line 1268)
- `workflow_digest_rejection_reports_exact_mismatch_and_accepts_match` (summary.rs line 935)
- `frame_seed_with_workflow_rejects_digest_mismatch_before_replay` (summary.rs line 365)
- `event_slot_values_cover_valid_corrupt_and_missing_frame_paths` (summary.rs line 981) - MISSING slot value corruption test
- `replay_divergence_*` tests in core.rs

---

## Gaps: Missing Corruption Injection Tests

The bead requires tests that **intentionally corrupt**:
1. **Artifact digest** - mutate `RunAccepted.workflow` field
2. **Journal sequence** - corrupt `EventSeq` ordering
3. **Slot value** - corrupt encoded slot bytes
4. **Taint** - corrupt `extra` (taint) field in `SlotWrittenEvent`

Each case must fail **deterministically** with precise `RecoveryError` diagnostic.

### MISSING Tests (in `tests/recovery_integration.rs`)
- `corrupt_artifact_digest_fails_with_workflow_source_digest_mismatch`
- `corrupt_journal_sequence_fails_with_replay_divergence`
- `corrupt_slot_value_fails_with_slot_values_unsupported`
- `corrupt_slot_taint_fails_with_event_slot_taint_unsupported`

---

## Dependencies

| Crate | Role |
|-------|------|
| `vb_core` | `WorkflowDigest`, `RunFrame`, `ReplayEngine` |
| `vb_storage` | `FjallJournal`, `JournalEvent`, `RecoveryError` |
| `vb_runtime` | Runtime recovery boundary |

---

## Open Questions

1. **Action ABI digest verification is deferred** (recover.rs line 71-72). Is this in scope for vb-qi37.1.5?
2. **Policy digest mismatch** (`PolicyDigestMismatch`) is defined but never instantiated. Is this in scope?
3. **Slot value drift** - the `UnsupportedRecoveryState::slot_values_unsupported()` is set when slot values are missing or corrupt, but is there a test that *injects* corrupt slot bytes?

---

## Recommended Downstream Owners

- **contract/rust-contract**: Digest invariants contract
- **proof-planner**: Formal proof of digest mismatch detection
- **test-planner**: Corruption injection test plan
- **formal-verifier**: Kani proof obligations for digest invariants
