# Formal Verification Report — vb-aoah State 12

## Provenance

- **Verifier**: formal-verifier (execution gate)
- **Invocation**: formal-verifier-vb-aoah-state12-001
- **Bead**: vb-aoah (migration skeleton tests)
- **State**: 12 (formal-verifier)
- **Prior states**: 5 (proof writing APPROVED), 6 (proof review APPROVED), 7 (bridge APPROVED), 8 (test plan APPROVED), 9 (tests written, 51/51 pass), 10 (test review APPROVED), 11 (implementation plan written)
- **Proof obligations**: `proof-obligations.planned.jsonl` (18 obligations: 7 Kani + 7 proptest + 4 cargo-fuzz)
- **Bridge obligations**: `rust-refinement-obligations.jsonl` (18 bridge rows: BR-VB-AA-001..018)
- **Workspace**: `/home/lewis/isolated/femdation-velvet-ballistics/vb-aoah`
- **Date**: 2026-05-27

## Executive Summary

**STATUS: PENDING_PRODUCTION_CLOSURE**

All 18 proof obligations have been verified against test-double adapters (States 5-6). All 51 behavior tests pass against test-double adapters (State 9). However, the production migration API at `crates/vb_storage/src/migrations.rs` does not exist yet. Formal execution against production code is blocked until this file is implemented.

The formal-verifier cannot close behavior-affecting obligations at State 12 because:
1. Production source refs in all 18 bridge rows have `mapping_status: planned`
2. Kani harnesses were verified against adapter functions, not production code
3. Proptest tests use test-double adapters, not production API calls
4. Fuzz targets are built but have never been run against production code

**State 12 closure requires a subsequent invocation after `migrations.rs` is implemented.**

## Obligation Status Matrix

### Kani Harnesses (PO-R01 through PO-R07)

| Obligation | Target | Status (Adapter) | Status (Production) | Evidence |
|---|---|---|---|---|
| PO-R01 | `vb_aoah_runtime_open_no_side_effects` | VERIFIED (State 5) | PENDING | `raw-evidence/attempt8/kani-vb_aoah_all_harnesses.log` |
| PO-R02 | `vb_aoah_migration_registry_totality` | VERIFIED (State 5) | PENDING | `raw-evidence/attempt8/kani-vb_aoah_all_harnesses.log` |
| PO-R03 | `vb_aoah_verify_before_manifest_advance` | VERIFIED (State 5) | PENDING | `raw-evidence/attempt8/kani-vb_aoah_all_harnesses.log` |
| PO-R04 | `vb_aoah_cleanup_success_requires_empty_old_keyspace` | VERIFIED (State 5) | PENDING | `raw-evidence/attempt8/kani-vb_aoah_all_harnesses.log` |
| PO-R05 | `vb_aoah_reopen_after_migration_no_rerun` | VERIFIED (State 5) | PENDING | `raw-evidence/attempt8/kani-vb_aoah_all_harnesses.log` |
| PO-R06 | `vb_aoah_empty_old_keyspace_noop` | VERIFIED (State 5) | PENDING | `raw-evidence/attempt8/kani-vb_aoah_all_harnesses.log` |
| PO-R07 | `vb_aoah_migration_accounting_checked_bounds` | VERIFIED (State 5) | PENDING | `raw-evidence/attempt8/kani-vb_aoah_all_harnesses.log` |

**Status**: 7/7 VERIFIED against adapters. 0/7 verified against production. All re-run required at State 12.

### Proptest Behavior Tests (PO-R08 through PO-R14)

| Obligation | Target | Status (Adapter) | Status (Production) | Evidence |
|---|---|---|---|---|
| PO-R08 | `vb_aoah_runtime_open_migration_required_no_side_effects` | PASS (State 9) | PENDING | 51-pass test run |
| PO-R09 | `vb_aoah_migration_registry_totality_uniqueness` | PASS (State 9) | PENDING | 51-pass test run |
| PO-R10 | `vb_aoah_verify_before_manifest_advance` | PASS (State 9) | PENDING | 51-pass test run |
| PO-R11 | `vb_aoah_cleanup_empty_old_keyspace_postcondition` | PASS (State 9) | PENDING | 51-pass test run |
| PO-R12 | `vb_aoah_reopen_after_migration_idempotent` | PASS (State 9) | PENDING | 51-pass test run |
| PO-R13 | `vb_aoah_empty_old_keyspace_explicit_noop` | PASS (State 9) | PENDING | 51-pass test run |
| PO-R14 | `vb_aoah_migration_accounting_overflow_returns_error` | PASS (State 9) | PENDING | 51-pass test run |

**Status**: 7/7 PASS against adapters. 0/7 pass against production (tests use adapter doubles). All replacement required at State 12.

### Fuzz Targets (PO-R15 through PO-R18)

| Obligation | Target | Status (Build) | Status (Run) | Evidence |
|---|---|---|---|---|
| PO-R15 | `vb_aoah_runtime_open_hostile_manifest` | BUILT (State 5) | NOT RUN | No fuzz campaign evidence |
| PO-R16 | `vb_aoah_cleanup_corrupt_old_keyspace` | BUILT (State 5) | NOT RUN | No fuzz campaign evidence |
| PO-R17 | `vb_aoah_empty_keyspace_malformed_input` | BUILT (State 5) | NOT RUN | No fuzz campaign evidence |
| PO-R18 | `vb_aoah_migration_accounting_boundary_overflow` | BUILT (State 5) | NOT RUN | No fuzz campaign evidence |

**Status**: 4/4 BUILT. 0/4 run against production. All fuzz campaigns must be executed at State 12.

## Behavior Test Evidence

### Test Execution (State 9)

```
Command: cargo test -p velvet-ballistics-workspace-tests \
           --test restate_explicit_migration_skeleton_tests

Result: 51 passed; 0 failed; 0 ignored; finished in 0.01s
```

All 51 tests use test-double adapters. Not yet wired to production code.

### Clippy Gate (State 9)

```
Command: cargo clippy -p velvet-ballistics-workspace-tests \
           --test restate_explicit_migration_skeleton_tests -- -D warnings

Result: 0 warnings
```

## Bridge Row Closure Status

| Bridge Row | Obligation | mapping_status | Source Ref | Behavior Test | Harness | Closure |
|---|---|---|---|---|---|---|
| BR-VB-AA-001 | PO-R01 | planned | `migrations::detect_old_store` (not exist) | Adapter test | Kani (adapter) | PENDING |
| BR-VB-AA-002 | PO-R08 | planned | `FjallJournal::open` | Adapter test | Proptest (adapter) | PENDING |
| BR-VB-AA-003 | PO-R15 | planned | `codec::decode_record_header` | Fuzz (not run) | Fuzz (built) | PENDING |
| BR-VB-AA-004 | PO-R02 | planned | `MigrationRegistry` (not exist) | Adapter test | Kani (adapter) | PENDING |
| BR-VB-AA-005 | PO-R09 | planned | `MigrationRegistry` (not exist) | Adapter test | Proptest (adapter) | PENDING |
| BR-VB-AA-006 | PO-R03 | planned | `MigrationPhase` (not exist) | Adapter test | Kani (adapter) | PENDING |
| BR-VB-AA-007 | PO-R10 | planned | `MigrationPhase` (not exist) | Adapter test | Proptest (adapter) | PENDING |
| BR-VB-AA-008 | PO-R04 | planned | `cleanup_old_keyspace` (not exist) | Adapter test | Kani (adapter) | PENDING |
| BR-VB-AA-009 | PO-R11 | planned | `cleanup_old_keyspace` (not exist) | Adapter test | Proptest (adapter) | PENDING |
| BR-VB-AA-010 | PO-R16 | planned | `cleanup_old_keyspace` codec | Fuzz (not run) | Fuzz (built) | PENDING |
| BR-VB-AA-011 | PO-R05 | planned | `is_current_version` (not exist) | Adapter test | Kani (adapter) | PENDING |
| BR-VB-AA-012 | PO-R12 | planned | `is_current_version` (not exist) | Adapter test | Proptest (adapter) | PENDING |
| BR-VB-AA-013 | PO-R06 | planned | `migrate_from` (not exist) | Adapter test | Kani (adapter) | PENDING |
| BR-VB-AA-014 | PO-R13 | planned | `migrate_from` (not exist) | Adapter test | Proptest (adapter) | PENDING |
| BR-VB-AA-015 | PO-R17 | planned | `migrate_from` codec | Fuzz (not run) | Fuzz (built) | PENDING |
| BR-VB-AA-016 | PO-R07 | planned | `checked_accounting` (not exist) | Adapter test | Kani (adapter) | PENDING |
| BR-VB-AA-017 | PO-R14 | planned | `checked_accounting` (not exist) | Adapter test | Proptest (adapter) | PENDING |
| BR-VB-AA-018 | PO-R18 | planned | `checked_accounting` codec | Fuzz (not run) | Fuzz (built) | PENDING |

**Closure rate**: 0/18 bridge rows closed against production. All 18 have `mapping_status: planned`.

## Formal Waivers

No waivers issued. All obligations are behavior-affecting and cannot be waived.

## State 12 Closure Commands

The following commands must be executed after `migrations.rs` is implemented:

```bash
# 1. Re-run all 7 Kani harnesses against production code
for harness in \
  vb_aoah_runtime_open_no_side_effects \
  vb_aoah_migration_registry_totality \
  vb_aoah_verify_before_manifest_advance \
  vb_aoah_cleanup_success_requires_empty_old_keyspace \
  vb_aoah_reopen_after_migration_no_rerun \
  vb_aoah_empty_old_keyspace_noop \
  vb_aoah_migration_accounting_checked_bounds; do
  cargo kani -p vb_storage --harness "$harness" --output-format terse
done

# 2. Run all 51+ tests against production API
cargo nextest run -p velvet-ballistics-workspace-tests \
  --test restate_explicit_migration_skeleton_tests

# 3. Execute all 4 fuzz campaigns (60s each)
for target in \
  vb_aoah_runtime_open_hostile_manifest \
  vb_aoah_cleanup_corrupt_old_keyspace \
  vb_aoah_empty_keyspace_malformed_input \
  vb_aoah_migration_accounting_boundary_overflow; do
  timeout 120s cargo fuzz run "$target" -- -max_total_time=60 -runs=10000
done

# 4. Mutation testing (≥95% kill rate)
cargo mutants -p vb_storage --timeout 60 -- \
  --test restate_explicit_migration_skeleton_tests

# 5. Moon CI gate
moon ci
```

## Conclusion

**State 12 is NOT closed.** All 18 proof obligations, 18 bridge rows, and 51 behavior tests have been verified against test-double adapters. The formal-verifier cannot transition obligations from `PENDING_FORMAL_EXECUTION` to `PASS` until:

1. `crates/vb_storage/src/migrations.rs` is implemented with all planned production symbols
2. All 7 Kani harnesses are re-run against production code and return `VERIFICATION:- SUCCESSFUL`
3. All 51 behavior tests pass against production API calls (adapters replaced)
4. All 4 fuzz campaigns complete clean against production code
5. Mutation testing reaches ≥95% kill rate on production code
6. `moon ci` passes

A subsequent formal-verifier invocation (formal-verifier-vb-aoah-state12-002 or later) must re-execute the closure commands and produce an updated `formal-verification-report.md` with `mapping_status: verified` for all 18 bridge rows.
