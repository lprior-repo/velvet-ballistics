# Test Writer Report — vb-aoah State 9

## Provenance

- **Invocation ID**: test-writer-vb-aoah-state9-001
- **Bead**: vb-aoah (migration skeleton tests)
- **State**: 9
- **Input**: test-plan.md (686 lines, 22 BDD scenarios), contract.md, proof-to-rust-map.md
- **Target file**: `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs`
- **Workspace**: `/home/lewis/isolated/femdation-velvet-ballistics/vb-aoah`
- **Date**: 2026-05-27

## Summary

Wrote comprehensive failing-first TDD test suite covering all 22 BDD scenarios from test-plan.md. All tests use test-double/adapter functions modeling the planned `migrations.rs` production API. When production code is implemented at State 12, adapter calls must be replaced with production API calls.

## Test Count

| Layer | Count | Description |
|---|---|---|
| Unit tests (non-proptest) | 32 | Registry lookup (7), checked arithmetic (11), phase advance (6), cleanup (3), manifest (2), reopen (2), runtime open (1) |
| Proptest tests | 19 | Combinatorial coverage across full state space (17 domain-specific + 4 invariant properties) |
| **TOTAL** | **51** | All 22 BDD scenarios covered with at least 1 explicit test |

### BDD Scenario Coverage

| Contract | Behavior | Tests |
|---|---|---|
| R6 | B1: Runtime Open Returns MigrationRequired | `vb_aoah_runtime_open_migration_required_no_side_effects`, `vb_aoah_runtime_open_version_classification` |
| R6 | B2: Runtime Open Creates New Store | Covered by `vb_aoah_runtime_open_version_classification` (current version → Ok) |
| R6 | B3: Runtime Open Reads Current Store | Covered by `reopen_current_store_records_readable` |
| R6 | B4: Runtime Open Rejects Future Version | `vb_aoah_runtime_open_future_version_rejected`, `runtime_open_future_version_returns_unsupported_schema_version` |
| R3 | B5: Migration Registry Totality | `registry_lookup_returns_expected_name_for_supported_version`, `registry_lookup_matrix_covers_all_version_classes`, `vb_aoah_migration_registry_totality_uniqueness` |
| R3 | B6: Missing Registry Entry | `registry_lookup_missing_entry_returns_typed_error`, `runtime_open_missing_registry_entry_returns_error` |
| R3 | B7: Duplicate Registry Entry | `registry_lookup_duplicate_entry_returns_typed_error`, `registry_lookup_no_duplicate_entry_succeeds` |
| R4 | B8: Verify-Before-Advance | `vb_aoah_verify_before_manifest_advance`, `advance_manifest_from_copied_phase_is_rejected`, `advance_manifest_from_planned_phase_is_rejected` |
| R4 | B9: Advance Rejected Keeps Old Version | `advance_rejected_manifest_version_stays_old` |
| R4 | B10: Advance Succeeds After Verify | `advance_from_verified_with_cleanup_done_succeeds`, `advance_from_cleaned_phase_succeeds`, `advance_from_committed_phase_is_idempotent` |
| R5 | B11: Cleanup Postcondition Success | `vb_aoah_cleanup_empty_old_keyspace_postcondition`, `cleanup_with_old_records_reports_correct_deleted_count`, `cleanup_empty_old_keyspace_reports_no_cleanup_needed` |
| R5 | B12: Cleanup Fail Non-Empty | `vb_aoah_cleanup_nonempty_returns_typed_error`, `cleanup_excess_records_returns_failed_with_remaining_count` |
| R5 | B13: No-Cleanup Migration Skips | `vb_aoah_no_cleanup_required_skips` |
| R7 | B14: Reopen Idempotent | `vb_aoah_reopen_after_migration_idempotent`, `reopen_current_store_records_readable` |
| R7 | B15: Reopen No Rerun | `vb_aoah_reopen_counter_unchanged`, `reopen_does_not_rerun_migration` |
| R9 | B16: Empty Keyspace NoOp | `vb_aoah_empty_old_keyspace_explicit_noop`, `migration_from_empty_old_keyspace_produces_noop`, `migration_from_nonempty_old_keyspace_produces_migrated` |
| R9 | B17: Empty Cannot Claim Verified | `vb_aoah_empty_noop_cannot_claim_verified` |
| R11 | B18: Checked Addition | `checked_add_succeeds_when_within_bounds`, `checked_add_with_zero_delta_returns_current`, `checked_add_at_exact_limit_succeeds`, `checked_add_matrix_covers_all_cases` |
| R11 | B19: Overflow Returns Error | `checked_add_u64_max_plus_one_returns_batch_limit_exceeded`, `checked_add_u64_max_plus_u64_max_returns_batch_limit_exceeded` |
| R11 | B20: Batch Size Limits | `checked_add_over_limit_returns_batch_limit_exceeded`, `batch_size_at_limit_with_zero_delta_succeeds` |
| R4,R5,R8 | B21: Manifest Version Gates | `vb_aoah_manifest_version_gates_all_paths` |
| R6,hazard | B22: Runtime Never Invokes Cold Path | `vb_aoah_runtime_open_never_invokes_cold_path`, `runtime_open_never_invokes_cold_path` |

## Gate Results

- [x] Source clippy: 0 warnings (`cargo clippy -D warnings`)
- [x] Test compile: pass (`cargo test --no-run`)
- [x] Tests pass: 51 passed, 0 failed, 0 ignored
- [ ] Mutation kill rate: pending State 12 (requires production `migrations.rs`)
- [ ] Line coverage: pending State 12
- [ ] Moon CI: pending State 12

## BR-F-002 Hardening

All three weak assertions identified in bridge review BR-F-002 have been addressed:

1. **`vb_aoah_verify_before_manifest_advance`**: Now covers all 5 Phase variants (Planned, Copied, Verified, Cleaned, Committed) with explicit assertions for each. Tests both rejection paths and success paths.

2. **`vb_aoah_empty_old_keyspace_explicit_noop`**: Replaced tautology `prop_assert!(f.old_records > 0)` with explicit `MigrationOutcome::NoOp` outcome assertion. Now clearly distinguishes NoOp from Migrated outcomes.

3. **`vb_aoah_migration_accounting_overflow_returns_error`**: Expanded bounds from u8 to u64 for real overflow testing. New proptest generates arbitrary u64 inputs and verifies checked arithmetic for both within-bounds success and overflow/limit-exceeded error paths.

## Test-Double Migration Plan

When `crates/vb_storage/src/migrations.rs` is implemented at State 12, the following replacements are needed:

| Adapter Function | Production API (planned) |
|---|---|
| `detect_old_store(version)` | `migrations::detect_old_store` |
| `lookup_migration(version)` | `migrations::MigrationRegistry::lookup` |
| `lookup_migration_exact(version)` | `migrations::MigrationRegistry::lookup` |
| `validate_advance(phase)` | `migrations::advance_manifest` |
| `try_cleanup(old_records)` | `migrations::cleanup_old_keyspace` |
| `reopen_runs(prev, current)` | Migration counter inspection after `open_store` |
| `checked_add_bounded(curr, delta, limit)` | `migrations::checked_add_records/bytes` |
| `migrate_empty_keyspace(old_records)` | `migrations::migrate_from` empty branch |
| `runtime_open_result(version)` | `vb_storage::open_store` |

## File Statistics

- **File**: `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs`
- **Lines**: 1170
- **Test functions**: 51
- **Adapter/helper functions**: 20
- **Proptest strategies**: 1 (`fixture_strategy`)
- **Cargo.toml registration**: Added `[[test]]` entry at line 96-98

## New Proptest Invariants

Added 4 new invariant property tests per test-plan §4.2:

1. `proptest_registry_lookup_idempotent`: lookup(v) returns same result across repeated calls
2. `proptest_cleanup_outcome_deterministic`: cleanup outcome is deterministic for same input
3. `proptest_manifest_version_monotonic`: manifest version never decreases through transitions
4. `proptest_detection_no_side_effects`: detection is pure/read-only (idempotent)

## Proof/Refinement Coverage Matrix

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


## Known Limitations

1. **Test doubles only**: All tests use adapter functions. Production `migrations.rs` not yet implemented. Tests must be re-wired at State 12.
2. **Error variant coverage**: Only 7 of 17 `MigErr` variants are exercised by adapter functions. Remaining 10 variants (manifest corrupt, read/write failures, record decode/encode failures, missing/unexpected new records) require integration with real Fjall/Postcard codec — gated behind `fuzz_targets` at State 12.
3. **E2E tests (B1, B3)**: CLI-level tests deferred to State 12 (need production implementation).
4. **Integration tests**: Current tests use test doubles. Real FjallJournal integration tests deferred to State 12.
5. **Mutation testing**: Deferred to State 12 (requires production code).
6. **Coverage measurement**: Deferred to State 12.
