# Implementation Plan — vb-aoah State 11

## Provenance

- **Planner**: holzman-rust (implementation planning gate)
- **Invocation**: holzman-rust-vb-aoah-state11-001
- **Bead**: vb-aoah (migration skeleton tests)
- **State**: 11
- **Test suite**: 51 tests, all passing (test-double adapters)
- **Input**: test-plan.md, contract.md, proof-to-rust-map.md, test-suite-review.md
- **Workspace**: `/home/lewis/isolated/femdation-velvet-ballistics/vb-aoah`
- **Date**: 2026-05-27

## Status: Test-First — No Production Code Change Required

This is a test-first bead. The production migration API at `crates/vb_storage/src/migrations.rs` does not exist yet. All 51 behavior tests use test-double adapter functions that model the planned production API. No production code is written in this state.

## Production Code Obligations (State 12)

The following production symbols must be implemented in `crates/vb_storage/src/migrations.rs` at State 12:

### Planned Symbols

| Symbol | Type | Contract Reference |
|---|---|---|
| `detect_old_store(version: u16) -> Result<(), JournalError>` | Public function | R6 |
| `MigrationRegistry` | Public struct | R3 |
| `MigrationRegistryEntry { name: &'static str, from_version: u16, to_version: u16 }` | Public struct | R3 |
| `MigrationRegistry::lookup(version: u16) -> Result<MigrationAction, JournalError>` | Public method | R3, B5-B7 |
| `MigrationPhase` (enum: Planned, Copied, Verified, Cleaned, Committed) | Public enum | R4 |
| `advance_manifest(phase: MigrationPhase) -> Result<MigrationPhase, JournalError>` | Public function | R4, B8-B10 |
| `cleanup_old_keyspace(journal: &FjallJournal, old_version: u16) -> Result<CleanupOutcome, JournalError>` | Public function | R5, B11-B13 |
| `CleanupOutcome` (enum: Success, OldKeyspaceNotEmpty, NoCleanupNeeded) | Public enum | R5 |
| `is_current_version(version: u16) -> bool` | Public function | R7, B14-B15 |
| `migrate_from(journal: &FjallJournal, old_version: u16) -> Result<MigrationOutcome, JournalError>` | Public function | R3-R5, B16-B17 |
| `MigrationOutcome` (enum: Migrated, NoOp) | Public enum | R9, B16-B17 |
| `checked_add_records(current: u64, delta: u64, limit: u64) -> Result<u64, JournalError>` | Public function | R11, B18-B20 |
| `checked_add_bytes(current: u64, delta: u64, limit: u64) -> Result<u64, JournalError>` | Public function | R11, B18-B20 |
| `migration_run_counter: AtomicU64` | Private state | R7, B14-B15 |
| `MIGRATE_FLAG_KEY: &str` | Public constant | R9 |

### Error Variants to Add to `JournalError`

| Variant | Diagnostic Code | Contract Reference |
|---|---|---|
| `MigrationRequired { from, to }` | `0x400D` (existing) | R6 — already exists |
| `UnsupportedSchemaVersion { version }` | `0x400C` (existing) | R6 — already exists |
| `UnsupportedMigrationSource { from, to }` | `0x4021` (new) | R3 |
| `MissingMigrationRegistryEntry { from, to }` | `0x4022` (new) | R3, B6 |
| `DuplicateMigrationRegistryEntry { from, to }` | `0x4023` (new) | R3, B7 |
| `MigrationManifestMissing` | `0x4024` (new) | — |
| `MigrationManifestCorrupt { reason_code }` | `0x4025` (new) | — |
| `MigrationManifestAdvanceRejected { from, to, phase }` | `0x4026` (new) | R4, B8-B9 |
| `MigrationReadFailed { keyspace }` | `0x4027` (new) | — |
| `MigrationWriteFailed { keyspace }` | `0x4028` (new) | — |
| `MigrationRecordDecodeFailed { record_kind }` | `0x4029` (new) | — |
| `MigrationRecordEncodeFailed { record_kind }` | `0x402A` (new) | — |
| `MigrationBatchLimitExceeded { limit }` | `0x402B` (new) | R11, B18-B20 |
| `MigrationVerificationFailed { reason_code, checked_count }` | `0x402C` (new) | R4 |
| `MigrationMissingNewRecord { record_kind }` | `0x402D` (new) | R4 |
| `MigrationUnexpectedNewRecord { record_kind }` | `0x402E` (new) | R4 |
| `MigrationCleanupFailed { remaining }` | `0x402F` (new) | R5, B12 |

**Note**: Diagnostic code range 0x4021-0x403F is proposed for migration error codes. Final allocation TBD by error/codes.rs owner.

### New Module Registration

In `crates/vb_storage/src/lib.rs`:
```rust
pub mod migrations;
```

### Constants to Add

In `crates/vb_storage/src/constants.rs`:
```rust
/// Migration audit-trail flag key for explicit no-op evidence.
pub const MIGRATE_FLAG_KEY: &str = "__migration_flag";

/// Maximum migration batch record count.
pub const MIGRATION_MAX_BATCH_RECORDS: u64 = 10_000;

/// Maximum migration batch byte size.
pub const MIGRATION_MAX_BATCH_BYTES: u64 = 10_485_760; // 10 MiB
```

## Test-to-Production Wiring Plan

When production `migrations.rs` is implemented, the following replacements must be made in the test file:

| Test Adapter (current) | Production Call (planned) | Test Functions Affected |
|---|---|---|
| `detect_old_store(version)` | `migrations::detect_old_store(version)` | `vb_aoah_runtime_open_migration_required_no_side_effects`, `proptest_detection_no_side_effects` |
| `lookup_migration(version)` | `MigrationRegistry::lookup(version)` | `vb_aoah_migration_registry_totality_uniqueness`, `proptest_registry_lookup_idempotent`, `registry_lookup_returns_expected_name_for_supported_version` |
| `lookup_migration_exact(version)` | `MigrationRegistry::lookup(version)` | `registry_lookup_returns_entry_for_known_version`, `registry_lookup_matrix_covers_all_version_classes`, `registry_lookup_*` |
| `lookup_migration_check_duplicate(v, flag)` | Registry with duplicate check | `registry_lookup_duplicate_entry_returns_typed_error` |
| `lookup_missing_entry(version)` | `MigrationRegistry::lookup(version)` with missing entry | `registry_lookup_missing_entry_returns_typed_error` |
| `validate_advance(phase)` | `advance_manifest(phase)` | `vb_aoah_verify_before_manifest_advance`, `advance_manifest_from_*`, `proptest_manifest_version_monotonic` |
| `try_cleanup(old_records)` | `cleanup_old_keyspace(journal, old_version)` | `vb_aoah_cleanup_empty_old_keyspace_postcondition`, `proptest_cleanup_outcome_deterministic`, `cleanup_*` |
| `cleanup_then_advance(old_records, flag)` | `cleanup_old_keyspace` + `advance_manifest` | `vb_aoah_cleanup_nonempty_returns_typed_error`, `vb_aoah_no_cleanup_required_skips` |
| `reopen_runs(prev, current)` | Migration counter inspection after `open_store` | `vb_aoah_reopen_after_migration_idempotent`, `vb_aoah_reopen_counter_unchanged` |
| `checked_add_bounded(curr, delta, limit)` | `checked_add_records(curr, delta, limit)` / `checked_add_bytes(curr, delta, limit)` | `vb_aoah_migration_accounting_overflow_returns_error`, `checked_add_*` |
| `migrate_empty_keyspace(old_records)` | `migrate_from(journal, old_version)` empty branch | `vb_aoah_empty_old_keyspace_explicit_noop`, `migration_from_*` |
| `manifest_version_after_phase(phase)` | Manifest inspection after phase transition | `vb_aoah_manifest_version_gates_all_paths`, `advance_rejected_manifest_version_stays_old` |
| `runtime_open_result(version)` | `FjallJournal::open(path, None)` with version detection | `vb_aoah_runtime_open_version_classification`, `runtime_open_*` |
| `is_future_version(version)` | `version > CURRENT_SCHEMA_VERSION` check in `open_store` | `vb_aoah_runtime_open_future_version_rejected` |
| `cold_path_invoked()` | Instrumentation check (never true) | `vb_aoah_runtime_open_never_invokes_cold_path` |

## Holzman Compliance Checklist

All production migration code must conform to:

- [ ] **No unsafe**: `#![forbid(unsafe_code)]` at file level
- [ ] **No unwrap/expect/panic/todo/unimplemented/dbg**: Zero tolerance
- [ ] **Checked arithmetic**: All arithmetic uses `checked_add`, `checked_sub`, `saturating_*`, or `wrapping_*` with explicit error returns
- [ ] **Checked indexing**: All indexing uses `.get()` with `Option` handling
- [ ] **No lossy `as` casts**: Use `From`/`TryFrom`/`Into` or explicit checked conversions
- [ ] **Typed errors**: All fallible operations return `Result<T, MigrationError>` or `Result<T, JournalError>`
- [ ] **Bounded resources**: Batch sizes, byte limits, loop iterations bounded with explicit limits
- [ ] **No ignored Results**: All `Result` values must be used or explicitly discarded with `let _ =`
- [ ] **No YAML/JSON/HTTP in runtime core**: Migration code is pure storage operations
- [ ] **Fjall + Postcard only**: Storage persistence through Fjall; record serialization through Postcard
- [ ] **Process-lock aware**: Migration must acquire or respect the process lock at `ProcessLock`

## Implementation Order (State 12)

1. Create `crates/vb_storage/src/migrations.rs` with all planned symbols
2. Add new `JournalError` variants for migration errors
3. Register diagnostic codes in `error/codes.rs` (codes 0x4021-0x402F)
4. Register `pub mod migrations;` in `lib.rs`
5. Add migration constants to `constants.rs`
6. Add version-detection logic to `FjallJournal::open` in `journal/core.rs`
7. Replace test-double adapters with production API calls in test file
8. Re-run all 51 tests, verifying they pass against production code
9. Re-run all 7 Kani harnesses against production code
10. Execute all 4 fuzz campaigns
11. Run mutation testing (≥95% kill rate target)
12. Run `moon ci` for canonical CI gate

## Source Coverage Matrix

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|----------|-------|--------------------|------------------|---------------------|------------------------|----------|------------------|------------|
| PO-R01 | runtime_open_result no side effects | Yes | `validation.rs:10-17` | `runtime_open_result` (L6) | `vb_aoah_runtime_open_no_side_effects.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_runtime_open_no_side_effects` | State 5 |
| PO-R02 | MigrationRegistry lookup totality | Yes | `migrations.rs` (planned) | `lookup_migration` (L1) | `vb_aoah_migration_registry_totality.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_migration_registry_totality` | State 5 |
| PO-R03 | verify_before_manifest_advance | Yes | `migrations.rs` (planned) | `validate_advance` (L6) | `vb_aoah_verify_before_manifest_advance.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_verify_before_manifest_advance` | State 5 |
| PO-R04 | cleanup requires empty old keyspace | Yes | `migrations.rs` (planned) | `try_cleanup` (L6) | `vb_aoah_cleanup_success_requires_empty_old_keyspace.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_cleanup_success_requires_empty_old_keyspace` | State 5 |
| PO-R05 | reopen after migration no rerun | Yes | `migrations.rs` (planned) | `reopen_runs` (L6) | `vb_aoah_reopen_after_migration_no_rerun.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_reopen_after_migration_no_rerun` | State 5 |
| PO-R06 | empty old keyspace noop | Yes | `migrations.rs` (planned) | `migrate_empty_keyspace` (L6) | `vb_aoah_empty_old_keyspace_noop.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_empty_old_keyspace_noop` | State 5 |
| PO-R07 | migration accounting checked bounds | Yes | `migrations.rs` (planned) | `checked_add_bounded` (L1) | `vb_aoah_migration_accounting_checked_bounds.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_migration_accounting_checked_bounds` | State 5 |
| PO-R08 | proptest runtime open no side effects | Yes | `migrations.rs` (planned) | B1-B4 (L2) | N/A | proptest | `cargo test vb_aoah_runtime_open_migration_required_no_side_effects` | State 9 |
| PO-R09 | proptest registry totality uniqueness | Yes | `migrations.rs` (planned) | B5-B7 (L1/L2) | N/A | proptest | `cargo test vb_aoah_migration_registry_totality_uniqueness` | State 9 |
| PO-R10 | proptest verify before advance | Yes | `migrations.rs` (planned) | B8-B10 (L2) | N/A | proptest | `cargo test vb_aoah_verify_before_manifest_advance` | State 9 |
| PO-R11 | proptest cleanup postcondition | Yes | `migrations.rs` (planned) | B11-B13 (L2) | N/A | proptest | `cargo test vb_aoah_cleanup_empty_old_keyspace_postcondition` | State 9 |
| PO-R12 | proptest reopen idempotent | Yes | `migrations.rs` (planned) | B14-B15 (L2) | N/A | proptest | `cargo test vb_aoah_reopen_after_migration_idempotent` | State 9 |
| PO-R13 | proptest empty keyspace explicit noop | Yes | `migrations.rs` (planned) | B16-B17 (L2) | N/A | proptest | `cargo test vb_aoah_empty_old_keyspace_explicit_noop` | State 9 |
| PO-R14 | proptest overflow returns error | Yes | `migrations.rs` (planned) | B18-B20 (L1/L2) | N/A | proptest | `cargo test vb_aoah_migration_accounting_overflow_returns_error` | State 9 |
| PO-R15 | fuzz hostile manifest | No (defense-in-depth) | `migrations.rs` (planned) | `fuzz_targets/vb_aoah_runtime_open_hostile_manifest.rs` | N/A | cargo-fuzz | `cargo fuzz run vb_aoah_runtime_open_hostile_manifest` | State 5 |
| PO-R16 | fuzz corrupt old keyspace | No (defense-in-depth) | `migrations.rs` (planned) | `fuzz_targets/vb_aoah_cleanup_corrupt_old_keyspace.rs` | N/A | cargo-fuzz | `cargo fuzz run vb_aoah_cleanup_corrupt_old_keyspace` | State 5 |
| PO-R17 | fuzz malformed input | No (defense-in-depth) | `migrations.rs` (planned) | `fuzz_targets/vb_aoah_empty_keyspace_malformed_input.rs` | N/A | cargo-fuzz | `cargo fuzz run vb_aoah_empty_keyspace_malformed_input` | State 5 |
| PO-R18 | fuzz boundary overflow | No (defense-in-depth) | `migrations.rs` (planned) | `fuzz_targets/vb_aoah_migration_accounting_boundary_overflow.rs` | N/A | cargo-fuzz | `cargo fuzz run vb_aoah_migration_accounting_boundary_overflow` | State 5 |


## Delivery Scope Impact

| Crate | Files Touched | Risk |
|---|---|---|
| `vb_storage` | `src/migrations.rs` (new), `src/lib.rs` (1 line), `src/error/mod.rs` (~30 lines), `src/error/codes.rs` (~20 lines), `src/constants.rs` (~5 lines), `src/journal/core.rs` (~15 lines) | LOW — test-first, well-specified |
| `workspace_tests` | `tests/restate_explicit_migration_skeleton_tests.rs` (replace adapters) | LOW — behavior under test already |

## Blockers

None. All prerequisites are met:
- Test plan APPROVED (State 8)
- Proof-to-rust bridge APPROVED (State 7)
- Test suite written and APPROVED (States 9-10)
- Bead contract clear and stable (State 3)
- Kani harnesses VERIFIED against adapters (State 5)
- Fuzz targets BUILT (State 5)
- No global readiness blockers in `global-readiness-report.md`
