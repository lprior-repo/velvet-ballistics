# Boundary Map — vb-815l8

- bead_id: vb-815l8
- scope: TEST-ONLY; one-line assertion + one-line import + comment cleanup at `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:7-13, 75-79`
- authored_at: 2026-07-01

## 1. Boundaries Touched By This Bead

| Boundary | File | Kind | Mutated by this bead? |
|----------|------|------|------------------------|
| Test-only file | `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs` | Test source | YES — one import added (lines 7-13), one assertion replaced (line 79), two comments replaced (lines 75-78). |
| Production recovery boundary | `crates/vb_runtime/src/recovery.rs` | Public API | NO — read-only survey for contract completeness. |
| Production storage recovery types | `crates/vb_storage/src/recovery/types.rs` | Public API | NO — read-only survey. |
| Production core frame construction | `crates/vb_core/src/frame/parts/impl_001_construct.rs` | Public API | NO — read-only survey. |
| Production runtime error type | `crates/vb_runtime/src/error/mod.rs`, `equality.rs`, `diagnostics.rs`, `display.rs` | Public API | NO — read-only survey. |
| Cargo test wiring | `crates/workspace_tests/Cargo.toml` | Build config | NO — `vb_runtime` is already a dev-dependency at line 43. |

## 2. Pure-Core / Imperative-Shell Split

### 2.1 Pure core (no I/O, no time, no allocation beyond stack)

- `RuntimeError` enum + `PartialEq` impl (`crates/vb_runtime/src/error/mod.rs`, `equality.rs`).
- `RecoveryResumeStatus` enum (`crates/vb_runtime/src/recovery.rs:41-57`).
- `RecoveryCannotResumeState` bit-mask + `is_resumable` (`crates/vb_storage/src/recovery/types.rs:1025-1039`).
- `RecoveryFrameSeed` struct + `cannot_resume_state` delegator (`crates/vb_storage/src/recovery/types.rs:730-810, 1202-1214`).

### 2.2 Imperative shell (storage, time, allocator)

- `RecoveryCannotResumeState::from_seed` (`crates/vb_storage/src/recovery/types.rs:949-957`) — applies the missing-component mask. The unconditional `mark_missing_components(MissingRunStateComponents::ALL)` call is the imperative step that drives the test outcome.
- `DurableFrameRecoveryBoundary::hydrate_run_frame` (`crates/vb_runtime/src/recovery.rs:99-106`) — sequential gate pipeline (reject → empty → apply).

### 2.3 Async shell

None. The recovery boundary is synchronous.

### 2.4 Storage / network / time / FFI / unsafe / parser

- Storage: `RecoveryFrameSeed` is read from durable journal events by `recover_runtime_frame_seed_from_events`, but the target test bypasses storage entirely and constructs the seed manually. Storage boundary is exercised indirectly by the contract but not by this test.
- Network: none.
- Time: none.
- FFI: none.
- Unsafe: `crates/vb_runtime/src/recovery.rs:1` has `#![forbid(unsafe_code)]`; test file line 1 has the same.
- Parser: `RecoveryFrameSeed` is parsed by storage's recovery decoder (postcard codec). The test exercises the post-parse boundary only.

## 3. Boundary Crossing Contract

### 3.1 Test → runtime boundary

```
[Test (integration_runtime_storage_fault_tolerance.rs)]
        │
        │   constructs RecoveryFrameSeed (manual)
        │   calls DurableFrameRecoveryBoundary::from_seed(seed)
        ▼
[Runtime boundary (vb_runtime::recovery)]
        │
        │   hydrate_run_frame() ──► reject_unsupported_live_frame_state()
        │                                │
        │                                ▼
        │                          Err(RuntimeError::InvalidRecoveryHydration)
        ▼
[Test asserts typed outcome]
```

The crossing is unidirectional: the test holds the seed, builds the boundary, invokes the boundary, and observes the typed outcome. No call-back, no shared mutable state, no async scheduling.

### 3.2 Boundary invariant under test

`hydrate_run_frame() == Err(InvalidRecoveryHydration)` for any `RecoveryFrameSeed`, because:

1. `RecoveryCannotResumeState::from_seed` unconditionally marks every `*_missing` flag true (`types.rs:949-957`).
2. `is_resumable()` returns false if any of the 13 flags is true (`types.rs:1025-1039`).
3. `reject_unsupported_live_frame_state` returns `Err(InvalidRecoveryHydration)` iff `is_resumable()` is false (`recovery.rs:109-115`).

A second, independent gate (`empty_recovered_frame` → `RunFrame::new` → step-count-zero reject at `core/frame/parts/impl_001_construct.rs:10-14`) would produce the same typed outcome for the zero-step seed.

## 4. Forbidden Boundary Mutations

This bead forbids:

- Adding a new `use` statement beyond `use vb_runtime::RuntimeError;`.
- Adding new helper functions, new test cases, or new test modules.
- Modifying any production-code file.
- Modifying `Cargo.toml` or build wiring.
- Modifying the seed construction (lines 50-72).
- Modifying the test name (`recovery_from_corrupt_snapshot_sequence_is_detected`), which is misleading but out of scope.

## 5. Layer-Level Boundary Map (read-only)

```
┌─────────────────────────────────────────────────────────────────┐
│ vb_workspace_tests (integration_runtime_storage_fault_tolerance)│  ← TEST (mutated)
│   imports: vb_core, vb_runtime::RuntimeError, vb_storage        │
└─────────────────────────────────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────────┐
│ vb_runtime::recovery (DurableFrameRecoveryBoundary)             │  ← PRODUCTION (read-only)
│   hydrate_run_frame → reject_unsupported_live_frame_state        │
│                       → empty_recovered_frame                    │
│                       → apply_recovered_steps                    │
│                       → apply_recovered_slots                    │
│                       → apply_recovered_pc                       │
└─────────────────────────────────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────────┐
│ vb_storage::recovery::types (RecoveryFrameSeed,                 │  ← PRODUCTION (read-only)
│                              RecoveryCannotResumeState)         │
│   from_seed: unconditional mark_missing_components(ALL)          │
└─────────────────────────────────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────────┐
│ vb_core::frame::parts::impl_001_construct (RunFrame::new)       │  ← PRODUCTION (read-only)
│   rejects step_count == 0 with CoreError::InvalidCompiledWorkflow│
└─────────────────────────────────────────────────────────────────┘
```

## 6. Open Boundary Questions

None. The bead is bounded to a single test file with a single-line assertion replacement. There is no boundary that requires renegotiation; the production contract being asserted is fixed and was locked in prior beads (see `delivery-scope.jsonl` clusters 3-12).