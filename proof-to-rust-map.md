# Proof-to-Rust Map: vb-aoah

**Bead**: vb-aoah
**Title**: storage: Add explicit migration skeleton and cleanup tests
**State**: 15 (landing — p15-repair)
**Generated**: 2026-05-27

---

## PROOF CLAIM → RUST SOURCE MAPPING

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|----------|-------|--------------------|------------------|---------------------|------------------------|----------|------------------|------------|
| PO-R01 | runtime_open_result no side effects | Yes | `crates/vb_storage/src/codec/validation.rs:10-17` | `restate_explicit_migration_skeleton_tests.rs::runtime_open_result` | `verification/kani/vb_aoah_runtime_open_no_side_effects.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_runtime_open_no_side_effects` | State 5 |
| PO-R02 | MigrationRegistry lookup totality | Yes | `crates/vb_storage/src/migrations.rs` (planned) | `restate_explicit_migration_skeleton_tests.rs::lookup_migration` | `verification/kani/vb_aoah_migration_registry_totality.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_migration_registry_totality` | State 5 |
| PO-R03 | verify_before_manifest_advance | Yes | `crates/vb_storage/src/migrations.rs` (planned) | `restate_explicit_migration_skeleton_tests.rs::validate_advance` | `verification/kani/vb_aoah_verify_before_manifest_advance.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_verify_before_manifest_advance` | State 5 |
| PO-R04 | cleanup success requires empty old keyspace | Yes | `crates/vb_storage/src/migrations.rs` (planned) | `restate_explicit_migration_skeleton_tests.rs::try_cleanup` | `verification/kani/vb_aoah_cleanup_success_requires_empty_old_keyspace.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_cleanup_success_requires_empty_old_keyspace` | State 5 |
| PO-R05 | reopen after migration no rerun | Yes | `crates/vb_storage/src/migrations.rs` (planned) | `restate_explicit_migration_skeleton_tests.rs::reopen_runs` | `verification/kani/vb_aoah_reopen_after_migration_no_rerun.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_reopen_after_migration_no_rerun` | State 5 |
| PO-R06 | empty old keyspace noop | Yes | `crates/vb_storage/src/migrations.rs` (planned) | `restate_explicit_migration_skeleton_tests.rs::migrate_empty_keyspace` | `verification/kani/vb_aoah_empty_old_keyspace_noop.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_empty_old_keyspace_noop` | State 5 |
| PO-R07 | migration accounting checked bounds | Yes | `crates/vb_storage/src/migrations.rs` (planned) | `restate_explicit_migration_skeleton_tests.rs::checked_add_bounded` | `verification/kani/vb_aoah_migration_accounting_checked_bounds.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_migration_accounting_checked_bounds` | State 5 |
| PO-R08 | proptest runtime open no side effects | Yes | `crates/vb_storage/src/migrations.rs` (planned) | `restate_explicit_migration_skeleton_tests.rs::vb_aoah_runtime_open_migration_required_no_side_effects` | N/A | proptest | `cargo test -p velvet-ballistics-workspace-tests vb_aoah_runtime_open_migration_required_no_side_effects` | State 9 |
| PO-R09 | proptest registry totality uniqueness | Yes | `crates/vb_storage/src/migrations.rs` (planned) | `restate_explicit_migration_skeleton_tests.rs::vb_aoah_migration_registry_totality_uniqueness` | N/A | proptest | `cargo test -p velvet-ballistics-workspace-tests vb_aoah_migration_registry_totality_uniqueness` | State 9 |
| PO-R10 | proptest verify before advance | Yes | `crates/vb_storage/src/migrations.rs` (planned) | `restate_explicit_migration_skeleton_tests.rs::vb_aoah_verify_before_manifest_advance` | N/A | proptest | `cargo test -p velvet-ballistics-workspace-tests vb_aoah_verify_before_manifest_advance` | State 9 |
| PO-R11 | proptest cleanup postcondition | Yes | `crates/vb_storage/src/migrations.rs` (planned) | `restate_explicit_migration_skeleton_tests.rs::vb_aoah_cleanup_empty_old_keyspace_postcondition` | N/A | proptest | `cargo test -p velvet-ballistics-workspace-tests vb_aoah_cleanup_empty_old_keyspace_postcondition` | State 9 |
| PO-R12 | proptest reopen idempotent | Yes | `crates/vb_storage/src/migrations.rs` (planned) | `restate_explicit_migration_skeleton_tests.rs::vb_aoah_reopen_after_migration_idempotent` | N/A | proptest | `cargo test -p velvet-ballistics-workspace-tests vb_aoah_reopen_after_migration_idempotent` | State 9 |
| PO-R13 | proptest empty keyspace explicit noop | Yes | `crates/vb_storage/src/migrations.rs` (planned) | `restate_explicit_migration_skeleton_tests.rs::vb_aoah_empty_old_keyspace_explicit_noop` | N/A | proptest | `cargo test -p velvet-ballistics-workspace-tests vb_aoah_empty_old_keyspace_explicit_noop` | State 9 |
| PO-R14 | proptest overflow returns error | Yes | `crates/vb_storage/src/migrations.rs` (planned) | `restate_explicit_migration_skeleton_tests.rs::vb_aoah_migration_accounting_overflow_returns_error` | N/A | proptest | `cargo test -p velvet-ballistics-workspace-tests vb_aoah_migration_accounting_overflow_returns_error` | State 9 |
| PO-R15 | fuzz hostile manifest | No (defense-in-depth) | `crates/vb_storage/src/migrations.rs` (planned) | `fuzz/fuzz_targets/vb_aoah_runtime_open_hostile_manifest.rs` | N/A | cargo-fuzz | `cargo fuzz run vb_aoah_runtime_open_hostile_manifest` | State 5 |
| PO-R16 | fuzz corrupt old keyspace | No (defense-in-depth) | `crates/vb_storage/src/migrations.rs` (planned) | `fuzz/fuzz_targets/vb_aoah_cleanup_corrupt_old_keyspace.rs` | N/A | cargo-fuzz | `cargo fuzz run vb_aoah_cleanup_corrupt_old_keyspace` | State 5 |
| PO-R17 | fuzz malformed input | No (defense-in-depth) | `crates/vb_storage/src/migrations.rs` (planned) | `fuzz/fuzz_targets/vb_aoah_empty_keyspace_malformed_input.rs` | N/A | cargo-fuzz | `cargo fuzz run vb_aoah_empty_keyspace_malformed_input` | State 5 |
| PO-R18 | fuzz boundary overflow | No (defense-in-depth) | `crates/vb_storage/src/migrations.rs` (planned) | `fuzz/fuzz_targets/vb_aoah_migration_accounting_boundary_overflow.rs` | N/A | cargo-fuzz | `cargo fuzz run vb_aoah_migration_accounting_boundary_overflow` | State 5 |

## SUMMARY

- **Total Obligations**: 18
- **Kani**: 7 (PO-R01 through PO-R07)
- **Proptest**: 7 (PO-R08 through PO-R14)
- **Cargo-fuzz**: 4 (PO-R15 through PO-R18)
- **Status**: All verified against adapters (test-first skeleton). Production wiring deferred.

---

**Generated by**: formal-verifier (p15-repair)
**Timestamp**: 2026-05-27
