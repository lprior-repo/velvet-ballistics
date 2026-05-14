# Verification Layers: cli/runtime — Converged Binary Mode Activation Boundaries

## Boundary

- **Verified kernel**: `crates/velvet_ballastics/src/main.rs` command dispatch and handler functions
- **Runtime shell**: Binary invocation; CLI argument parsing; exit code propagation
- **External systems excluded from formal proof**: Filesystem (workflow files), Fjall database files, Unix sockets for IPC

## Layer Assignment

### Pure Mode Activation Tests (POST-002, INV-001, INV-002)

- **PRE-002** → `static-scan` + `cargo-fuzz` (args parsing produces correct Command variant)
  - Verify: `parse_args` produces no unexpected subsystem initialization
- **POST-002** → `proptest` + `manual-qa`
  - Prove: `cmd_validate` can be called with no `--db` argument present and succeeds
  - Prove: `cmd_verify` does not call `vb_storage::FjallJournal::open`
  - Prove: `cmd_compile` does not call `vb_storage::FjallJournal::open`
- **INV-001** → `miri` (memory aliasing and UB in pure handler paths)
  - Verify: Pure command handlers do not alias or share state with storage subsystem
- **INV-002** → `static-scan` (linker/crate dependency scan)
  - Verify: `vb_ui_makepad` is not linked into pure command binary path
  - Verify: `vb_ui_makepad` is not linked into storage command binary path

### Storage Mode Activation Tests (POST-003, INV-001)

- **POST-003** → `proptest` + `manual-qa`
  - Prove: `cmd_run` with `--durability none` does NOT open Fjall
  - Prove: `cmd_run` with `--durability journaled|strict` DOES open Fjall
  - Prove: `cmd_submit` opens Fjall regardless of durability mode
  - Prove: storage-dependent commands fail fast with `ModeError::StorageInitFailed` when `--db` path is invalid
- **INV-001** → `cargo-fuzz` (malformed file paths as storage paths)
  - Prove: invalid `--db` paths produce structured diagnostic, not UB

### Runtime Mode Activation Tests (POST-003, INV-004)

- **POST-003** → `manual-qa`
  - Prove: `cmd_ipc_serve` creates Runtime instance and opens FjallJournal
- **INV-004** → `miri` (Runtime instance lifecycle)
  - Verify: Runtime::new_with_journal is called only from Runtime mode commands

### Error Path Tests (Error Taxonomy)

- **ERR-StorageInitFailed** → `proptest` + `cargo-fuzz` + `manual-qa`
  - Prove: `FjallJournal::open` failure produces `ModeError::StorageInitFailed` with correct path
  - Prove: exit code is `CliExitCode::StorageError`
  - Prove: structured JSON error output includes path and cause
- **ERR-RuntimeInitFailed** → `manual-qa`
  - Prove: Runtime creation failure produces `ModeError::RuntimeInitFailed`
- **ERR-PureCommandStorageAccessAttempted** → `proptest` + `manual-qa`
  - DEFECT TEST: if a pure command handler somehow calls storage init, it is caught and reported

### Exit Code Stability Tests (INV-003, POST-005)

- **INV-003** → `proptest` + `manual-qa`
  - Prove: `validate` on a valid workflow succeeds with exit 0 regardless of whether any storage path exists
  - Prove: `validate` on an invalid workflow fails with exit != 0 regardless of whether any storage path exists
  - Prove: `verify` on a valid workflow succeeds with exit 0 without any `--db` argument
  - Prove: `bench-run` succeeds without `--db` argument
  - Prove: `agent-context` succeeds without `--db` argument
- **POST-005** → `proptest`
  - Prove: pure command exit codes are independent of subsystem availability

### Static Analysis Layers

- **static-scan** → unsafe, panic, unwrap/expect, unchecked indexing
  - Verify: no `unwrap()`, `expect()`, `panic!` in pure command handler paths
  - Verify: no unchecked indexing in command dispatch
- **cargo-geiger** → unsafe dependency scan
  - Verify: no `unsafe` in velvet_ballastics crate
- **cargo-deny** → supply chain
  - Verify: vb_storage, vb_runtime, vb_ipc, vb_ui dependencies are properly audited
- **cargo-machete** → unused dependency
  - Verify: pure command dependencies do not accidentally pull in vb_storage

## Lean Scope

### Theorem Module
`proof/ModeActivation.hs` (hypothetical Lean 4 formalization)

### Rust Target
`crates/velvet_ballastics/src/main.rs` — `command_mode()`, pure handler functions

### Abstraction Relation
```
command_mode(cmd) = Pure  ⟺  ∀h ∈ handlers(cmd). h ∉ StorageInit ∧ h ∉ RuntimeInit ∧ h ∉ UiInit
command_mode(cmd) = Storage  ⟺  ∃h ∈ handlers(cmd). h ∈ StorageInit ∧ h ∉ RuntimeInit ∧ h ∉ UiInit
command_mode(cmd) = Runtime  ⟺  ∃h ∈ handlers(cmd). h ∈ RuntimeInit ∨ (StorageInit ∧ RuntimeInit)
```

### Theorems
- THM-001: `command_mode(cmd) = Pure → handlers(cmd) ∩ {FjallJournal::open, Runtime::new, Makepad::init} = ∅`
- THM-002: `command_mode(cmd) = Storage → handlers(cmd) ∩ {FjallJournal::open} ≠ ∅`
- THM-003: `command_mode(cmd) = Runtime → handlers(cmd) ∩ {Runtime::new} ≠ ∅`

### Non-goals
- I/O shell correctness (tested by manual QA)
- Async runtime behavior (not applicable — no async in CLI)
- Makepad UI rendering correctness (separate UI bead)
- Fjall durability guarantees (tested by vb_storage bead)

## Waivers

- **UI mode activation**: Not implemented yet — waived until UI bead is created. The contract includes UI mode in the classification matrix but implementation of UI commands is deferred.
- **bench-run storage dependency**: Uncertain whether bench-run currently accesses storage. Needs investigation. If it does access storage, either fix it to be pure, or update the contract. Waived pending investigation.
- **agent-context storage dependency**: Uncertain whether agent-context currently accesses storage. Needs investigation. If it does access storage, either fix it to be pure, or update the contract. Waived pending investigation.
- **status command storage dependency**: Uncertain whether status accesses runtime state or storage. Needs investigation. Waived pending investigation.
- **Lean formalization**: Lean proof is not yet written. Waived until a future formal methods bead. Evidence is provided by `proptest` invariant exploration and `manual-qa` instead.
- **cargo-fuzz for command dispatch**: Fuzzing of CLI argument parsing is not yet implemented. Waived until fuzz infrastructure is extended to cover argument parsing.

## Verification Checkpoint Mapping

| Checkpoint | Layer | Evidence |
|------------|-------|----------|
| Gate 0: Research | manual review | Research notes document subsystem init paths |
| Gate 1: Tests | proptest + manual-qa | Mode activation tests written and failing |
| Gate 2: Implementation | moon ci | All tests pass |
| Gate 3: Integration | manual-qa | E2E verification of pure commands without `--db` argument |
