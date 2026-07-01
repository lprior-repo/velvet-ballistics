# Trusted Base Plan — vb-815l8

## 1. Scope

This bead is **TEST-ONLY**. The trusted base is the existing production code that the test exercises, plus the test infrastructure that the test runs under. The trusted base is **read-only**; no production code is mutated, and no new trust assumptions are introduced.

## 2. Trusted Surfaces

### 2.1 Cargo / moon test infrastructure (trusted)

- `cargo test` runner (nextest) — Standard Rust test infrastructure, no `unsafe` in the runner.
- `cargo clippy` lint engine — Standard Rust linter, no `unsafe`.
- `moon run :lint-src` — Project-internal canonical source-lint invocation per `.moon/tasks/all.yml:46-62`.
- `bash scripts/check-source-length.sh` — Project-internal source-length gate, no `unsafe`.

**Justification**: All four are project-internal canonical invocations; each has been pre-validated by prior beads. No new tool is introduced.

### 2.2 Production code (trusted; read-only for this bead)

| Production surface | Location | Justification |
|---|---|---|
| `RecoveryCannotResumeState::from_seed` | `crates/vb_storage/src/recovery/types.rs:949-957` | **FORBIDDEN TO MUTATE**. The unconditional `mark_missing_components(MissingRunStateComponents::ALL)` is the structural reason the test outcome is `Err(InvalidRecoveryHydration)`. Locked in by 8 existing unit tests. |
| `RecoveryCannotResumeState::is_resumable` | `crates/vb_storage/src/recovery/types.rs:1025-1039` | Pure flag-check; returns false iff any of 13 `*_missing` flags is true. |
| `MissingRunStateComponents::ALL` | `crates/vb_storage/src/recovery/types.rs:809` | Const bit mask, 13 flags set. |
| `DurableFrameRecoveryBoundary::hydrate_run_frame` | `crates/vb_runtime/src/recovery.rs:99-106` | **FORBIDDEN TO MUTATE**. Production method, the contract under test. |
| `reject_unsupported_live_frame_state` | `crates/vb_runtime/src/recovery.rs:109-115` | Returns `Err(InvalidRecoveryHydration)` when `cannot_resume_state().is_resumable()` is false. |
| `empty_recovered_frame` | `crates/vb_runtime/src/recovery.rs:117-125` | Maps `RunFrame::new` failure to `Err(InvalidRecoveryHydration)`. |
| `RecoveryResumeStatus::CannotResume` | `crates/vb_runtime/src/recovery.rs:41-57` | Enum with no `Resumable` variant by design; documented at lines 41-50. |
| `RuntimeError::InvalidRecoveryHydration` | `crates/vb_runtime/src/error/mod.rs:72-73` | Unit variant; `#[non_exhaustive]` enum. |
| `PartialEq for RuntimeError` | `crates/vb_runtime/src/error/equality.rs:3-28` | Unit-tag dispatch; tag 10 is `InvalidRecoveryHydration`. |
| `Eq for RuntimeError` | `crates/vb_runtime/src/error/equality.rs:212` | Trivial. |
| `Display for RuntimeError::InvalidRecoveryHydration` | `crates/vb_runtime/src/error/display.rs:29` | Returns "invalid recovery frame hydration" — used by the `assert_eq!` `Debug` payload on failure. |
| `RunFrame::new` | `crates/vb_core/src/frame/parts/impl_001_construct.rs:10-14` | Rejects `step_count==0` with `CoreError::InvalidCompiledWorkflow{reason: "step_count_zero"}`. Secondary gate. |
| `vb_runtime::RuntimeError` re-export | `crates/vb_runtime/src/lib.rs:92` | `pub use error::{RuntimeError, RuntimeResult};` |

**Justification**: All production code is locked in by the 8 existing unit tests at `crates/vb_runtime/src/recovery/tests.rs:55-57, 119-122, 170-173, 212-215, 269-272, 294-297, 359-362, 489-492` and by the `equality.rs` tests. No new trust assumption is introduced.

### 2.3 Test file structure (trusted; this is the only file mutated)

- `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:7-13` — import block (will gain `use vb_runtime::RuntimeError;`).
- `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:46` — test function declaration.
- `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:50-72` — manually-constructed `RecoveryFrameSeed` fixture (unchanged).
- `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:75-78` — comment block (will be cleaned up to reference the production invariant).
- `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:79` — single-line tautological assertion (will be replaced with typed `assert_eq!`).

**Justification**: The fixture at lines 50-72 is a controlled, manually-constructed `RecoveryFrameSeed`; no parser surface, no untrusted input. The test does not exercise any external boundary.

### 2.4 Build wiring (trusted; read-only)

- `crates/workspace_tests/Cargo.toml:43` — `vb_runtime` is a dev-dependency, so `use vb_runtime::RuntimeError;` is authorized.
- `crates/workspace_tests/Cargo.toml:48-287` — `autotests` is not set to `false`, so Cargo auto-discovers `tests/*.rs` and `integration_runtime_storage_fault_tolerance.rs` is compiled into the test binary.
- `Cargo.toml:1-11` — workspace members; the relevant crates are `vb_runtime`, `vb_storage`, `vb_core`, `workspace_tests`.

**Justification**: No Cargo.toml mutation is required; the new import is already authorized by the existing dev-dependency.

## 3. Model Reductions and Assumptions

### 3.1 No concurrency (cargo-test + source-lint lanes)

- The test is single-threaded, no `async`, no `tokio`, no `Mutex`, no `RwLock`, no `Send`/`Sync` surface.
- The runtime boundary is synchronous (`hydrate_run_frame` is a sync method).

**Justification**: A single-threaded sync test does not require concurrency verification.

### 3.2 No unsafe (miri lane not applicable)

- Both `crates/vb_runtime/src/recovery.rs:1` and the target test file have `#![forbid(unsafe_code)]`.
- No raw pointers, no `MaybeUninit`, no aliasing, no `unsafe` block introduced.

**Justification**: A `forbid(unsafe_code)` crate cannot have UB paths; Miri is not applicable.

### 3.3 No temporal state machine (TLA+ not applicable)

- The test is a single-shot deterministic function call.
- No state machine, no scheduling, no interleaving, no temporal property.

**Justification**: A single-shot sync test does not require temporal verification; TLA+ is not applicable (and was removed from the proof-planner toolset).

### 3.4 No refinement (Flux not applicable)

- No refinement types in the changed surface.
- The `assert_eq!` is the most-refined possible form for a unit-variant equality.

**Justification**: A typed equality check on a unit variant is the most-refined form; Flux would not add coverage.

### 3.5 No exhaustive model checking (Kani not applicable)

- The contract is a single typed error variant; the seed-shape space is already covered by 8 unit tests.
- A Kani proof of `RuntimeError::InvalidRecoveryHydration` unit-variant equality adds no coverage beyond the existing `equality.rs:3-28` unit-tag dispatch.

**Justification**: Unit-tag dispatch is provably total; the 8 unit tests cover the seed-shape space; Kani is not applicable.

### 3.6 No fuzz (cargo-fuzz not applicable)

- The test exercises a single manually-constructed seed.
- Fuzz would not add coverage to a single-typed-error contract.

**Justification**: Fuzz is bounded random input generation; the seed-shape space is already covered by deterministic unit tests.

## 4. Known Assumptions and Stub Boundaries

| Assumption | Location | Justification |
|---|---|---|
| `use vb_runtime::RuntimeError;` resolves at line 7-13 of the target file | `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:7-13` | `vb_runtime` is a dev-dependency at `crates/workspace_tests/Cargo.toml:43`; the import is authorized |
| `PartialEq for RuntimeError` discriminates `InvalidRecoveryHydration` from other `Invalid*` variants | `crates/vb_runtime/src/error/equality.rs:3-28` | Unit-tag dispatch; tag 10 is `InvalidRecoveryHydration` |
| `RecoveryCannotResumeState::from_seed` continues to mark all 13 `*_missing` flags true | `crates/vb_storage/src/recovery/types.rs:949-957` | **FORBIDDEN TO MUTATE**; locked in by 8 unit tests |
| `assert_eq!` macro panics on inequality with `Debug` payload | std (standard library) | Standard Rust macro; well-tested |
| `cargo test` runner (nextest) reports test outcome | `cargo` | Standard Rust test infrastructure |
| `cargo clippy` lint engine reports lints | `cargo` + `clippy` | Standard Rust linter |
| `moon run :lint-src` runs the canonical lint invocation | `.moon/tasks/all.yml:46-62` | Project-internal canonical task |
| `bash scripts/check-source-length.sh` runs the source-length gate | `scripts/check-source-length.sh` | Project-internal gate |
| The 359-line file (364 after edit) remains under 400-line test cap | `scripts/lib-source-length.sh` | Default test-file cap is 400 lines |
| The source-length exception row at `.config/source-length-exceptions.txt:200` is preserved | `.config/source-length-exceptions.txt` | Existing `vb-jpq7.47` row covers this file |

## 5. Behavior Waivers (none)

No waivers are requested. All proof obligations address genuine non-behavior requirements (test code compiles, lints clean, runs, fits in source-length budget). No obligation is marked `behavior_affecting: true`.

## 6. Reduction Justification

The trusted base reduces the full velvet-ballistics verification surface to a single test file (the only mutated artifact) and the production code it exercises (read-only). This reduction is justified because:

1. The test is a single-shot deterministic function call with no concurrency, no async, no scheduling, no temporal state.
2. The production code the test exercises is locked in by 8 existing unit tests at `crates/vb_runtime/src/recovery/tests.rs:55-57, 119-122, 170-173, 212-215, 269-272, 294-297, 359-362, 489-492`.
3. The `PartialEq for RuntimeError` is provably total by unit-tag dispatch; no further refinement is needed.
4. The fixture at lines 50-72 is a controlled, manually-constructed `RecoveryFrameSeed`; no parser, no untrusted input.
5. The build wiring (`vb_runtime` dev-dependency at `crates/workspace_tests/Cargo.toml:43`) is pre-validated; no Cargo.toml mutation is required.

## 7. Anti-Laundering Self-Check

- [x] No production code is trusted beyond what is locked in by 8 existing unit tests.
- [x] No new trust assumption is introduced; the only mutation is the test file.
- [x] No `assume`/`axiom`/`admit`/`external_body` in the trusted base (no proof code at all).
- [x] No `cover!`-as-proof (no Kani harness).
- [x] No vacuous Verus spec (no Verus obligation).
- [x] No behavior-affecting waiver (all obligations are `behavior_affecting: false`).
- [x] Source refs use `path::symbol` form (e.g., `crates/vb_runtime/src/recovery.rs::reject_unsupported_live_frame_state`).
