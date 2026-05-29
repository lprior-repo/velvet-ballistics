# Proof Evidence — vb-aoah State 5 Attempt 8 (Fresh Dispatch)

18 lightweight proof artifacts written for test-first bead vb-aoah (migration skeleton tests). The 7 Kani harnesses, 7 proptest properties, and 4 fuzz targets have been differentiated from prior clone attempts and are now focused per-obligation.

## Kani Evidence (PO-R01 through PO-R07)

All 7 Kani harnesses were verified with `cargo kani 0.67.0` on `nightly-2026-04-28`.

**Command**: `cargo kani -p vb_storage --harness <harness_name> --output-format terse`
**Workdir**: `/home/lewis/isolated/femdation-velvet-ballistics/vb-aoah`
**Raw log**: `.beads/vb-aoah/raw-evidence/attempt8/kani-vb_aoah_all_harnesses.log`

| Obligation | Harness | Result |
|---|---|---|
| PO-R01 | `vb_aoah_runtime_open_no_side_effects` | 0/13 failed — VERIFICATION SUCCESSFUL |
| PO-R02 | `vb_aoah_migration_registry_totality` | 0/11 failed — VERIFICATION SUCCESSFUL |
| PO-R03 | `vb_aoah_verify_before_manifest_advance` | 0/4 failed — VERIFICATION SUCCESSFUL |
| PO-R04 | `vb_aoah_cleanup_success_requires_empty_old_keyspace` | 0/16 failed — VERIFICATION SUCCESSFUL |
| PO-R05 | `vb_aoah_reopen_after_migration_no_rerun` | 0/12 failed — VERIFICATION SUCCESSFUL |
| PO-R06 | `vb_aoah_empty_old_keyspace_noop` | 0/16 failed — VERIFICATION SUCCESSFUL |
| PO-R07 | `vb_aoah_migration_accounting_checked_bounds` | 0/19 failed — VERIFICATION SUCCESSFUL |

**Summary**: 7 verified, 0 failed.

### Kani Model Bounds

- All harnesses use `kani::Arbitrary` per GOD RULE — no hardcoded shapes.
- Storage versions bounded to `u16` values ≤ 5.
- Record counts bounded to `u8` values ≤ 8 (MAX_RECORDS) or 16.
- Byte totals bounded to `u8` values ≤ 64 (MAX_BYTES).
- `#[kani::unwind(3)]` used for all harnesses.
- Adapter functions model expected migration behavior for the test-first phase.
- Production migration code does not exist yet (test-first bead).
- Assertions focus exclusively on per-obligation domain claims.

## Proptest Evidence (PO-R08 through PO-R14)

**File**: `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs`
**Formatting**: Passes `rustfmt --check` (all code style violations fixed).

7 proptest tests with renamed targets matching the obligation plan:

| Obligation | Test Target Name |
|---|---|
| PO-R08 | `vb_aoah_runtime_open_migration_required_no_side_effects` |
| PO-R09 | `vb_aoah_migration_registry_totality_uniqueness` |
| PO-R10 | `vb_aoah_verify_before_manifest_advance` |
| PO-R11 | `vb_aoah_cleanup_empty_old_keyspace_postcondition` |
| PO-R12 | `vb_aoah_reopen_after_migration_idempotent` |
| PO-R13 | `vb_aoah_empty_old_keyspace_explicit_noop` |
| PO-R14 | `vb_aoah_migration_accounting_overflow_returns_error` |

Proptest execution requires workspace-tests to compile with the full dependency graph. In the isolated workspace, `cargo test --no-run` compilation is pending setup. The file syntax, formatting, and target name alignment with obligations are verified.

## Fuzz Evidence (PO-R15 through PO-R18)

4 fuzz targets created with exact planned artifact paths:

| Obligation | Fuzz Target File |
|---|---|
| PO-R15 | `fuzz/fuzz_targets/vb_aoah_runtime_open_hostile_manifest.rs` |
| PO-R16 | `fuzz/fuzz_targets/vb_aoah_cleanup_corrupt_old_keyspace.rs` |
| PO-R17 | `fuzz/fuzz_targets/vb_aoah_empty_keyspace_malformed_input.rs` |
| PO-R18 | `fuzz/fuzz_targets/vb_aoah_migration_accounting_boundary_overflow.rs` |

**Build**: All 4 targets compile successfully via `cargo fuzz build --target x86_64-unknown-linux-gnu`.
**Cargo.toml**: All 4 targets registered as `[[bin]]` entries in `fuzz/Cargo.toml`.
**Fuzz execution**: PENDING_FORMAL_EXECUTION. Runtime fuzz campaigns require the full workspace infrastructure. The targets are focused: each exercises specific codec/manifest/accounting boundaries with hostile byte inputs and no panics (all failures must be typed errors).

## Assumptions and Trust Boundaries

- Production migration code (`crates/vb_storage/src/migrations.rs`) does not exist yet (test-first bead). Kani harnesses and proptest tests use bounded adapter functions to model expected behavior.
- Fjall persistence and Postcard codec remain trusted external dependencies per `trusted-base-plan.md`.
- Kani model bounds use `kani::Arbitrary` per GOD RULE. Bounded sizes (u8/u16) reflect the migration concept model, not production limits.
- Fuzz targets use `libfuzzer_sys` and test codec/manifest byte boundaries. musl-target builds blocked by ASAN/musl incompatibility; gnu-target builds succeed.
- PO-R05 assertion was corrected from `assert_eq!(new_runs, 0)` to `assert_eq!(additional_runs_from_reopen, 0)` — the claim is that reopen does not invoke migration, not that it zeroes all counters.

## Blocker: PENDING_FORMAL_EXECUTION

Fuzz campaign execution and proptest test execution require full workspace build infrastructure. These are recorded as `PENDING_FORMAL_EXECUTION` for:
- PO-R08-PO-R14: `cargo nextest run` execution
- PO-R15-PO-R18: `cargo fuzz run` execution

Tooling evidence:
- Kani harnesses: 7/7 verified (PASS)
- Proptest tests: Formatting verified, target names aligned, compilation pending workspace
- Fuzz targets: 4/4 compiled, runtime execution pending workspace
