# Type Contracts — vb-815l8

- bead_id: vb-815l8
- scope: TEST-ONLY, Rust-local, one-line assertion replacement + one-line import + comment cleanup
- authored_at: 2026-07-01

## 1. Types Used In The Replacement Assertion (read-only; not constructed by this bead)

### 1.1 `RuntimeError`

```
crates/vb_runtime/src/error/mod.rs:7
pub enum RuntimeError { /* ... */
    /// Durable recovery frame seed was internally inconsistent.
    InvalidRecoveryHydration,  // line 73
}
#[non_exhaustive]  // line 6
```

- Kind: `#[non_exhaustive]` enum (line 6).
- Variant kind: unit variant (line 73).
- `PartialEq`: implemented at `crates/vb_runtime/src/error/equality.rs:3-7,28`, dispatching on `runtime_error_unit_tag` (tag 10 maps to `InvalidRecoveryHydration`).
- `Eq`: implemented at `crates/vb_runtime/src/error/equality.rs:212`.
- Re-export: `crates/vb_runtime/src/lib.rs:92` — `pub use error::{RuntimeError, RuntimeResult};`.

### 1.2 `RuntimeResult<T>`

- Type alias for `Result<T, RuntimeError>`.
- Used by `DurableFrameRecoveryBoundary::hydrate_run_frame` (`crates/vb_runtime/src/recovery.rs:38,99`).
- Re-export: `crates/vb_runtime/src/lib.rs:92`.

### 1.3 `RecoveryFrameSeed`

- Struct at `crates/vb_storage/src/recovery/types.rs:730-810`.
- 13 fields: `summary`, `first_step`, `step_count`, `slot_count`, `pc`, `steps`, `slots`, `pending_actions`, `unsupported` (+ bit flags referenced by `from_seed`).
- Constructed literally at `integration_runtime_storage_fault_tolerance.rs:50-72`; the construction is out of scope for this bead (only the assertion mutates).

### 1.4 `DurableFrameRecoveryBoundary`

- Struct at `crates/vb_runtime/src/recovery.rs:60-63`.
- Constructor `from_seed(RecoveryFrameSeed) -> Self` at line 68.
- Trait method `hydrate_run_frame(&self) -> RuntimeResult<RunFrame>` at line 99.

## 2. Smart Constructors / Boundary Parsers (test surface, not constructed here)

The test already constructs `RecoveryFrameSeed` manually; no parser or smart constructor is added by this bead. The construction is verified by type-check, not validated by the boundary (the boundary rejects the seed rather than constructing it).

## 3. Test-Surface Type-Level Contract

### 3.1 The new assertion

```rust
assert_eq!(
    boundary.hydrate_run_frame(),
    Err(RuntimeError::InvalidRecoveryHydration),
    "durable frame hydration must reject any frame seed: a frame seed alone never \
     carries the full RunState, so cannot_resume_state().is_resumable() is never empty",
);
```

- LHS: `RuntimeResult<RunFrame>` (alias for `Result<RunFrame, RuntimeError>`).
- RHS: `Result<RunFrame, RuntimeError>` constructed from the unit variant `RuntimeError::InvalidRecoveryHydration`.
- Equality: `PartialEq for Result<RunFrame, RuntimeError>` requires `PartialEq for RunFrame` and `PartialEq for RuntimeError`. Both are present. The discrimination is exact: `assert_eq!` will fail for any other `Err(_)` variant or any `Ok(_)` value.
- Message: embedded contract rationale; non-empty `&'static str` is the `core::fmt::Debug` human-readable trace when the assertion fires.

### 3.2 The added import

```rust
use vb_runtime::RuntimeError;
```

- Path: `vb_runtime::RuntimeError` resolves to the re-export at `crates/vb_runtime/src/lib.rs:92`.
- Authorized: `vb_runtime` is a dev-dependency of `velvet-ballistics-workspace-tests` at `crates/workspace_tests/Cargo.toml:43`.
- Style: matches `crates/workspace_tests/tests/integration_storage_runtime_recovery.rs:13` (existing precedent in the same crate).

### 3.3 Comment cleanup contract

The current comments at `integration_runtime_storage_fault_tolerance.rs:75-78` contain two false claims:

1. "Hydration should succeed because the seed itself is valid (corrupt snapshot is a storage-layer concern; the boundary only validates the seed shape)."
2. "A seed with step_count=0 and no workflow may still be a valid empty-run seed."

Both must be replaced with a comment that references the production-code invariant:

- `RecoveryResumeStatus::CannotResume` invariant (`crates/vb_runtime/src/recovery.rs:41-50`).
- `reject_unsupported_live_frame_state` returning `Err(InvalidRecoveryHydration)` when `cannot_resume_state().is_resumable()` is false (`crates/vb_runtime/src/recovery.rs:109-115`).

The new comment must:

- State that the boundary is NOT permissive on empty seed.
- State that `seed.cannot_resume_state()` unconditionally marks every `*_missing` flag true because `RecoveryCannotResumeState::from_seed` always calls `mark_missing_components(MissingRunStateComponents::ALL)` (`crates/vb_storage/src/recovery/types.rs:949-957`).

## 4. Typestate / State-Machine Contract

This bead does not introduce or modify a typestate. The target boundary has no typestate (it is a single-shot boundary that always rejects). The `RecoveryResumeStatus` enum is the only state machine and is not modified.

## 5. Forbidden-Construction Rules (Hammock-style anti-patterns)

The replacement assertion does not introduce any of:

- Primitive obsession — the only primitive involved is the literal `9002` for `RunId::new(9002)`, which is the existing test fixture and not modified.
- Boolean behavior flag — the prior `assert!(result.is_ok() || result.is_err())` is exactly this anti-pattern; the replacement removes it.
- Stringly typed ID — none; the assertion uses typed `RuntimeError::InvalidRecoveryHydration`.
- `Option` lifecycle state — none; `RuntimeError::InvalidRecoveryHydration` is a unit variant of an enum, not an `Option`.

## 6. Equality / Discrimination Proof (typed `assert_eq!`)

The replacement uses `assert_eq!` rather than `matches!` because:

1. `RuntimeError::PartialEq` is exact and discriminates by unit tag (`crates/vb_runtime/src/error/equality.rs:3-28`).
2. `assert_eq!` produces a `Debug` payload on failure that names the actual error variant; `matches!` only confirms inclusion.
3. The reference pattern at `crates/vb_runtime/src/recovery/tests.rs:55-57, 119-122, 170-173` uses `assert_eq!` for the same contract surface.

A `matches!(result, Err(RuntimeError::InvalidRecoveryHydration))` fallback (Pattern B in `codebase-map.md` §1.3) is acceptable if a future `#[non_exhaustive]` exhaustiveness concern arises, but Pattern A (`assert_eq!`) is preferred for parity with the existing reference tests in the same crate as the boundary.

## 7. Open Type-Contract Questions

None. The replacement is single-line, fully typed, and fully mirrors an existing canonical pattern. There is no remaining primitive-obsession, boolean-flag, or `Option`-lifecycle risk in the change.