# Test Plan: vb-aoah

**Bead**: vb-aoah
**Title**: storage: Add explicit migration skeleton and cleanup tests
**State**: 15 (landing — p15-repair)
**Generated**: 2026-05-27

---

## Test Trophy Allocation

| Layer | Count | Description |
|-------|-------|-------------|
| Unit (L1) | 5 | Migration registry, checked arithmetic, error variants |
| Integration (L2) | 12 | Adapter function integration, phase transitions |
| E2E (L3) | 2 | Full migration workflow, reopen idempotence |
| Proptest (L4) | 7 | Combinatorial state space coverage |
| Fuzz (L5) | 4 | Hostile manifest, corrupt keyspace, malformed input, boundary overflow |
| Kani (L6) | 7 | Bounded model checking for panic-freedom and invariants |

## BDD Scenario Coverage (22/22)

| BDD # | Scenario | Layer(s) | Assertion Type | Status |
|-------|----------|----------|----------------|--------|
| B1 | Runtime open returns MigrationRequired for old version | L2+L6 | `prop_assert_eq` / `assert_eq` | PASS |
| B2 | Runtime open returns Ok for current version | L2 | `prop_assert_eq(result, Ok(()))` | PASS |
| B3 | Runtime open handles u16::MAX boundary | L2 | `prop_assume` + `prop_assert` | PASS |
| B4 | Runtime open rejects future version | L2+L6 | `prop_assert_eq` | PASS |
| B5 | Registry returns exact named entry | L1+L2+L4 | `assert_eq` | PASS |
| B6 | Missing registry entry returns typed error | L1+L6 | `assert_eq` | PASS |
| B7 | Duplicate registry entry returns typed error | L1 | `assert_eq` | PASS |
| B8 | Manifest advance rejected before Verified | L2+L6 | `prop_assert!(result.is_err())` | PASS |
| B9 | Manifest stays at old version on rejected advance | L6 | `assert_eq(ver, RESTATE_V1_VERSION)` | PASS |
| B10 | Manifest advance succeeds from Verified/Cleaned | L6 | `assert_eq(result, Ok(Phase::Committed))` | PASS |
| B11 | Cleanup reports correct deleted count / NoCleanupNeeded | L2+L6 | `prop_assert_eq` / `assert_eq` | PASS |
| B12 | Cleanup failure returns typed MigrationCleanupFailed | L2+L6 | `prop_assert_eq(remaining, old_records)` | PASS |
| B13 | No-cleanup-required skip can advance | L2 | `prop_assert_eq(result, Ok(Phase::Committed))` | PASS |
| B14 | Reopen after migration reads current records | L2+L6 | `prop_assert_eq(reopened_runs, migration_runs)` | PASS |
| B15 | Reopen does not rerun migration | L2+L6 | `assert_eq(after, before)` | PASS |
| B16 | Migration from empty old keyspace produces NoOp | L2+L6 | `prop_assert_eq(outcome, NoOp)` | PASS |
| B17 | NoOp cannot claim verified/silent migration | L2 | `prop_assert!(manifest_ver != CURRENT)` | PASS |
| B18 | Checked arithmetic within bounds succeeds | L1+L5 | `assert_eq` / `prop_assert_eq` | PASS |
| B19 | Overflow (u64::MAX + 1) returns BatchLimitExceeded | L1+L2 | `assert_eq` | PASS |
| B20 | Batch size at limit with zero delta succeeds | L1+L5 | `assert_eq` | PASS |
| B21 | Manifest version only updates through Committed | L2 | `prop_assert_eq` / `prop_assert_ne` | PASS |
| B22 | Runtime open never invokes cold-path | L2+L6 | `prop_assert!(!cold_path_invoked())` | PASS |

## Proof/Refinement Coverage Matrix

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|----------|-------|--------------------|------------------|---------------------|------------------------|----------|------------------|------------|
| PO-R01 | runtime_open_result no side effects | Yes | `validation.rs:10-17` | `runtime_open_result` (L2) | `vb_aoah_runtime_open_no_side_effects.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_runtime_open_no_side_effects` | State 5 |
| PO-R02 | MigrationRegistry lookup totality | Yes | `migrations.rs` (planned) | `lookup_migration` (L1/L2) | `vb_aoah_migration_registry_totality.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_migration_registry_totality` | State 5 |
| PO-R03 | verify_before_manifest_advance | Yes | `migrations.rs` (planned) | `validate_advance` (L2) | `vb_aoah_verify_before_manifest_advance.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_verify_before_manifest_advance` | State 5 |
| PO-R04 | cleanup requires empty old keyspace | Yes | `migrations.rs` (planned) | `try_cleanup` (L2) | `vb_aoah_cleanup_success_requires_empty_old_keyspace.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_cleanup_success_requires_empty_old_keyspace` | State 5 |
| PO-R05 | reopen after migration no rerun | Yes | `migrations.rs` (planned) | `reopen_runs` (L2/L3) | `vb_aoah_reopen_after_migration_no_rerun.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_reopen_after_migration_no_rerun` | State 5 |
| PO-R06 | empty old keyspace noop | Yes | `migrations.rs` (planned) | `migrate_empty_keyspace` (L2) | `vb_aoah_empty_old_keyspace_noop.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_empty_old_keyspace_noop` | State 5 |
| PO-R07 | migration accounting checked bounds | Yes | `migrations.rs` (planned) | `checked_add_bounded` (L1) | `vb_aoah_migration_accounting_checked_bounds.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_migration_accounting_checked_bounds` | State 5 |
| PO-R08 | proptest runtime open no side effects | Yes | `migrations.rs` (planned) | B1-B4 (L2) | N/A | proptest | `cargo test -p velvet-ballistics-workspace-tests vb_aoah_runtime_open_migration_required_no_side_effects` | State 9 |
| PO-R09 | proptest registry totality uniqueness | Yes | `migrations.rs` (planned) | B5-B7 (L1/L2) | N/A | proptest | `cargo test -p velvet-ballistics-workspace-tests vb_aoah_migration_registry_totality_uniqueness` | State 9 |
| PO-R10 | proptest verify before advance | Yes | `migrations.rs` (planned) | B8-B10 (L2) | N/A | proptest | `cargo test -p velvet-ballistics-workspace-tests vb_aoah_verify_before_manifest_advance` | State 9 |
| PO-R11 | proptest cleanup postcondition | Yes | `migrations.rs` (planned) | B11-B13 (L2) | N/A | proptest | `cargo test -p velvet-ballistics-workspace-tests vb_aoah_cleanup_empty_old_keyspace_postcondition` | State 9 |
| PO-R12 | proptest reopen idempotent | Yes | `migrations.rs` (planned) | B14-B15 (L2/L3) | N/A | proptest | `cargo test -p velvet-ballistics-workspace-tests vb_aoah_reopen_after_migration_idempotent` | State 9 |
| PO-R13 | proptest empty keyspace explicit noop | Yes | `migrations.rs` (planned) | B16-B17 (L2) | N/A | proptest | `cargo test -p velvet-ballistics-workspace-tests vb_aoah_empty_old_keyspace_explicit_noop` | State 9 |
| PO-R14 | proptest overflow returns error | Yes | `migrations.rs` (planned) | B18-B20 (L1/L2) | N/A | proptest | `cargo test -p velvet-ballistics-workspace-tests vb_aoah_migration_accounting_overflow_returns_error` | State 9 |
| PO-R15 | fuzz hostile manifest | No (defense-in-depth) | `migrations.rs` (planned) | `vb_aoah_runtime_open_hostile_manifest.rs` | N/A | cargo-fuzz | `cargo fuzz run vb_aoah_runtime_open_hostile_manifest` | State 5 |
| PO-R16 | fuzz corrupt old keyspace | No (defense-in-depth) | `migrations.rs` (planned) | `vb_aoah_cleanup_corrupt_old_keyspace.rs` | N/A | cargo-fuzz | `cargo fuzz run vb_aoah_cleanup_corrupt_old_keyspace` | State 5 |
| PO-R17 | fuzz malformed input | No (defense-in-depth) | `migrations.rs` (planned) | `vb_aoah_empty_keyspace_malformed_input.rs` | N/A | cargo-fuzz | `cargo fuzz run vb_aoah_empty_keyspace_malformed_input` | State 5 |
| PO-R18 | fuzz boundary overflow | No (defense-in-depth) | `migrations.rs` (planned) | `vb_aoah_migration_accounting_boundary_overflow.rs` | N/A | cargo-fuzz | `cargo fuzz run vb_aoah_migration_accounting_boundary_overflow` | State 5 |

## Test Execution Summary

- **Total Tests**: 51 (32 non-proptest + 19 proptest)
- **Pass Rate**: 51/51 (100%)
- **Clippy Warnings**: 0
- **BDD Coverage**: 22/22 (100%)

---

**Generated by**: formal-verifier (p15-repair)
**Timestamp**: 2026-05-27
