# Implementation Report: vb-aoah

**Bead**: vb-aoah
**Title**: storage: Add explicit migration skeleton and cleanup tests
**State**: 15 (landing — p15-repair)
**Generated**: 2026-05-27

---

## Implementation Summary

**Test-first bead**. No production `migrations.rs` exists. All 51 behavior tests exercise test-double/adapter functions that model the planned production API. Production closure is deferred per STATE.md §State 12 Closure Requires.

## Planned Production Symbols

| Symbol | Type | Description | Test Adapter |
|--------|------|-------------|-------------|
| `detect_old_store` | fn | Detect schema version mismatch at runtime open | `runtime_open_result` |
| `MigrationRegistry::lookup` | fn | Look up migration function by old version | `lookup_migration` |
| `advance_manifest` | fn | Advance manifest phase after verification | `validate_advance` |
| `cleanup_old_keyspace` | fn | Clean up old keyspace records | `try_cleanup` |
| `migrate_records` | fn | Migrate records to new keyspace | `migrate_empty_keyspace` |
| `checked_add_records` | fn | Bounded record count arithmetic | `checked_add_bounded` |
| `checked_add_bytes` | fn | Bounded byte count arithmetic | `checked_add_bounded` |
| `MigrationPhase` | enum | Planned, Copied, Verified, Cleaned, Committed | `Phase` |
| `MigrationOutcome` | enum | Outcome variants after migration | `MigrationOutcome` |
| `CleanupResult` | enum | Cleanup outcome (Success/NoCleanupNeeded) | `CleanupResult` |
| `MigErr` | enum | 17 error variants covering all failure modes | `MigErr` |
| `SchemaVersion` | newtype | u16 wrapper for schema version | bare `u16` |
| `MigrationManifest` | struct | Tracks migration phase and version | `Fixture.phase` |
| `MigrationCounter` | struct | Tracks migration invocation count | `migration_runs` |
| `MigrationAccounting` | struct | Tracks record/byte counts with bounds | `fixture_record_count` |

## Error Variant Taxonomy (17 variants)

| Variant | Diagnostic Code | Exercised | Description |
|---------|----------------|-----------|-------------|
| `MigrationRequired` | 0x4021 | Yes | Schema version mismatch detected |
| `UnsupportedSchemaVersion` | 0x4022 | Yes | Version above CURRENT_SCHEMA_VERSION |
| `MigrationRegistryEntryNotFound` | 0x4023 | Yes | No migration registered for version |
| `DuplicateMigrationEntry` | 0x4024 | Yes | Duplicate registration detected |
| `ManifestAdvanceRejected` | 0x4025 | Yes | Phase transition rejected |
| `MigrationCleanupFailed` | 0x4026 | Yes | Cleanup did not complete |
| `BatchLimitExceeded` | 0x4027 | Yes | u64 overflow in accounting |
| `NoCleanupNeeded` | 0x4028 | Yes | Old keyspace already empty |
| `MigrationNotVerified` | 0x4029 | No | Awaiting production code |
| `MigrationCopyFailed` | 0x402A | No | Awaiting production code |
| `ManifestCorrupt` | 0x402B | No | Awaiting production code |
| `OldKeyspaceNotFound` | 0x402C | No | Awaiting production code |
| `NewKeyspaceWriteFailed` | 0x402D | No | Awaiting production code |
| `ManifestReadFailed` | 0x402E | No | Awaiting production code |
| `ManifestWriteFailed` | 0x402F | No | Awaiting production code |
| `MigrationTimeout` | 0x4030 | No | Awaiting production code |
| `MigrationCancelled` | 0x4031 | No | Awaiting production code |

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

## Wiring Mappings (Adapter → Production)

| # | Adapter Function | Production Target | Status |
|---|-----------------|-------------------|--------|
| 1 | `detect_old_store` | `migrations::detect_old_store` | PENDING |
| 2 | `lookup_migration` | `MigrationRegistry::lookup` | PENDING |
| 3 | `validate_advance` | `migrations::advance_manifest` | PENDING |
| 4 | `try_cleanup` | `migrations::cleanup_old_keyspace` | PENDING |
| 5 | `reopen_runs` | `MigrationCounter` inspection | PENDING |
| 6 | `migrate_empty_keyspace` | `migrations::migrate_records` | PENDING |
| 7 | `checked_add_bounded` | `migrations::checked_add_records/bytes` | PENDING |
| 8 | `cleanup_then_advance` | Composed production calls | PENDING |
| 9 | `runtime_open_result` | `migrations::detect_old_store` | PENDING |
| 10 | `lookup_migration_exact` | `MigrationRegistry::lookup` | PENDING |
| 11 | `cold_path_invoked` | State tracker inspection | PENDING |
| 12 | `manifest_version_after_phase` | Manifest field read | PENDING |

## Holzmann Checklist

| Rule | Status |
|------|--------|
| Illegal states unrepresentable | ✅ Enums for Phase, MigErr, MigrationOutcome, CleanupResult |
| Parse, don't validate | ✅ All adapter functions return `Result<_, MigErr>` |
| Types as documentation | ✅ No boolean domain parameters |
| Workflows | ✅ Explicit Phase typestate transitions |
| Newtypes | ⚠️ Version is bare u16 (acceptable for test model) |
| Panic vector | ✅ No `unwrap`, `expect`, `panic`, `unsafe` |

## GOD RULES Assessment

| Rule | Status |
|------|--------|
| GOD RULE 1 (No hardcoded Kani shapes) | ✅ Kani harnesses use `kani::Arbitrary` |
| GOD RULE 2 (Verus binds to implementation) | N/A — Verus excluded per reduced scope |
| GOD RULE 3 (TLA+ bounded math) | N/A — TLA+ excluded per reduced scope |
| GOD RULE 4 (Fix implementation, not proof) | N/A — No production code to fix yet |
| GOD RULE 5 (No blind verification) | ✅ All verifications scoped to adapters |

---

**Status**: PENDING_PRODUCTION_CLOSURE (test-first skeleton phase approved)
**Generated by**: formal-verifier (p15-repair)
**Timestamp**: 2026-05-27
