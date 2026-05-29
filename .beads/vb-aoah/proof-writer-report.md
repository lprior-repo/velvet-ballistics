# Proof Writer Report — vb-aoah State 5 Attempt 8 (Fresh Dispatch)

Status: State 5 artifact writing and Kani verification completed. 18 lightweight proof artifacts written, 7 Kani harnesses verified, 4 fuzz targets built, 7 proptest test names aligned with obligations.

## Prior Attempt Summary

Attempts 1-7 for vb-aoah State 5 suffered from a fundamental defect: all 7 Kani harness files were identical clones containing every assertion from all obligations in each file. The harnesses were not differentiated per-obligation. Fuzz targets existed with different names than the planned obligations. Proptest test names did not match the planned obligation targets. Attempts 6 and 7 were purely archival/bookkeeping repairs that did not fix the underlying artifact quality.

This attempt 8 is a fresh dispatch that rewrites/replaces all 18 artifacts with focused, obligation-specific content.

## Obligations Touched

All 18 planned obligations from `proof-obligations.planned.jsonl`:

### Kani (7 obligations — PO-R01 through PO-R07)
| ID | File | Status |
|---|---|---|
| PO-R01 | `crates/vb_storage/src/vb_aoah_runtime_open_no_side_effects_kani.rs` | Written + Verified |
| PO-R02 | `crates/vb_storage/src/vb_aoah_migration_registry_totality_kani.rs` | Written + Verified |
| PO-R03 | `crates/vb_storage/src/vb_aoah_verify_before_manifest_advance_kani.rs` | Written + Verified |
| PO-R04 | `crates/vb_storage/src/vb_aoah_cleanup_success_requires_empty_old_keyspace_kani.rs` | Written + Verified |
| PO-R05 | `crates/vb_storage/src/vb_aoah_reopen_after_migration_no_rerun_kani.rs` | Written + Verified (repaired assertion) |
| PO-R06 | `crates/vb_storage/src/vb_aoah_empty_old_keyspace_noop_kani.rs` | Written + Verified |
| PO-R07 | `crates/vb_storage/src/vb_aoah_migration_accounting_checked_bounds_kani.rs` | Written + Verified |

All 7 Kani harnesses pass: **7 verified, 0 failures** (cargo-kani 0.67.0, nightly-2026-04-28). Raw evidence: `.beads/vb-aoah/raw-evidence/attempt8/kani-vb_aoah_all_harnesses.log`.

### Proptest (7 obligations — PO-R08 through PO-R14)
| ID | Target |
|---|---|
| PO-R08 | `vb_aoah_runtime_open_migration_required_no_side_effects` (renamed from `vb_aoah_runtime_open_no_side_effects`) |
| PO-R09 | `vb_aoah_migration_registry_totality_uniqueness` (renamed from `vb_aoah_migration_registry_totality`) |
| PO-R10 | `vb_aoah_verify_before_manifest_advance` (unchanged) |
| PO-R11 | `vb_aoah_cleanup_empty_old_keyspace_postcondition` (renamed from `vb_aoah_cleanup_success_requires_empty_old_keyspace`) |
| PO-R12 | `vb_aoah_reopen_after_migration_idempotent` (renamed from `vb_aoah_reopen_after_migration_no_rerun`) |
| PO-R13 | `vb_aoah_empty_old_keyspace_explicit_noop` (renamed from `vb_aoah_empty_old_keyspace_noop`) |
| PO-R14 | `vb_aoah_migration_accounting_overflow_returns_error` (renamed from `vb_aoah_migration_accounting_checked_bounds`) |

File: `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs`. Formatting passes `rustfmt --check`. Test compilation is PENDING workspace-level infrastructure.

### Fuzz (4 obligations — PO-R15 through PO-R18)
| ID | File | Status |
|---|---|---|
| PO-R15 | `fuzz/fuzz_targets/vb_aoah_runtime_open_hostile_manifest.rs` | Created + Built |
| PO-R16 | `fuzz/fuzz_targets/vb_aoah_cleanup_corrupt_old_keyspace.rs` | Created + Built |
| PO-R17 | `fuzz/fuzz_targets/vb_aoah_empty_keyspace_malformed_input.rs` | Created + Built |
| PO-R18 | `fuzz/fuzz_targets/vb_aoah_migration_accounting_boundary_overflow.rs` | Created + Built |

All 4 targets registered in `fuzz/Cargo.toml` as `[[bin]]` entries. All 4 compile successfully with `cargo fuzz build --target x86_64-unknown-linux-gnu`.

## Artifacts Changed

### New/Replaced (18 proof artifacts)
- 7 Kani harnesses in `crates/vb_storage/src/` — fully rewritten with per-obligation focus
- 1 proptest test file in `crates/workspace_tests/tests/` — test names aligned with obligation targets
- 4 fuzz targets in `fuzz/fuzz_targets/` — created with exact planned names

### Modified (2 configuration files)
- `fuzz/Cargo.toml` — registered 4 new `[[bin]]` entries

### Generated (1 bead artifact)
- `.beads/vb-aoah/proof-evidence.md` — rewritten
- `.beads/vb-aoah/proof-writer-report.md` — rewritten (this file)

## Repairs Applied

1. **PO-R05 assertion repair**: Initial assertion `assert_eq!(new_runs, 0)` was incorrect. The claim is that reopen does NOT invoke migration (additional_runs_from_reopen == 0), not that it zeroes all counters. Fixed adapter to return `(total_runs, additional_runs_from_reopen)` tuple with assertion on `additional_runs_from_reopen == 0`.

2. **PO-R01 unused field removed**: Removed `keyspace_exists` field from AoahInput struct (not used in assertion logic).

3. **PO-R02 unused const removed**: Removed `RESTATE_V1_VERSION` (unused, the adapter uses direct `version < 2` check).

4. **PO-R03 unused field/const removed**: Removed `OLD_VERSION` const and `phase_copied` field.

5. **Proptest formatting**: Applied `rustfmt` to satisfy the zero-tolerance source lint rule.

## Key Design Decisions

- **Test-first adapters**: Since production migration code does not exist, harnesses use adapter functions (`adapter_*`) that model expected behavior. These are honest test doubles that will be replaced by production function calls after State 7 implementation.
- **kani::Arbitrary per GOD RULE**: Every harness uses `#[derive(kani::Arbitrary)]` on an input struct, not hardcoded storage shapes.
- **Per-obligation focus**: Each harness tests exactly one domain claim with assertions scoped accordingly. No cross-contamination.
- **Fuzz target naming**: Targets use the exact artifact paths from `proof-obligations.planned.jsonl` (PO-R15 through PO-R18), enabling exact-command matching.
- **Proptest target naming**: Test function names match the `target` field in obligations, enabling `cargo nextest run -- <target>` exact filtering.

## Blockers

- `PENDING_FORMAL_EXECUTION`: Fuzz campaigns (PO-R15-PO-R18) and proptest execution (PO-R08-PO-R14) require full workspace infrastructure not available in the isolated workspace.
- `PRODUCTION_BINDING_PENDING`: All harnesses use adapter functions. Production migration API binding requires State 7 implementation.

## Verification Command Evidence

```
cargo kani -p vb_storage \
  --harness vb_aoah_runtime_open_no_side_effects \
  --harness vb_aoah_migration_registry_totality \
  --harness vb_aoah_verify_before_manifest_advance \
  --harness vb_aoah_cleanup_success_requires_empty_old_keyspace \
  --harness vb_aoah_reopen_after_migration_no_rerun \
  --harness vb_aoah_empty_old_keyspace_noop \
  --harness vb_aoah_migration_accounting_checked_bounds \
  --output-format terse
Result: Complete - 7 successfully verified harnesses, 0 failures, 7 total.
```
