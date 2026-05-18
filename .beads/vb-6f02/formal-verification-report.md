# State 11: Formal Verification Report

## Verifier: Kani (Bounded Model Checking)

| Field | Value |
|-------|-------|
| **STATUS** | NOT_AVAILABLE |
| **Version** | cargo-kani 0.67.0 |
| **Command** | `cargo kani -p velvet-ballastics-workspace-tests` |
| **Harnesses found** | 0 of 9 written |
| **Errors** | None (compiled but no harnesses discovered) |
| **Root cause** | `contracts_as_data_kani.rs` has `#![cfg(kani)]` crate-level gating. Integration test files in `tests/` directory are not compiled by `cargo kani`, so proof harnesses with `#[kani::proof]` are never seen. |
| **9 written harnesses** | kani_parse_schema_version_valid, kani_parse_schema_version_empty, kani_parse_schema_version_leading_zero, kani_parse_schema_version_non_numeric, kani_compare_semver_reflexive, kani_compare_semver_antisymmetric, kani_compare_semver_transitive, kani_compare_semver_version_constraint, kani_compare_semver_monotonicity |

**Action needed:** Move Kani harnesses to `src/` directory or use `cfg(kani)` at module level instead of crate level.

## Verifier: Verus (Verification)

| Field | Value |
|-------|-------|
| **STATUS** | FAIL |
| **Version** | Verus 0.2026.05.05.d03e906 |
| **Command** | `verus contracts/verus/contracts_as_data_spec.rs` |
| **Errors** | 16 errors, 1 warning |
| **Spec functions** | 5 (is_valid_semver, spec_parse_schema_version, spec_parse_contract_kind, spec_compare_semver, btreemap_to_json_sorted) |
| **Proof functions** | 8 (verify_parse_schema_version_satisfies_spec, verify_parse_contract_kind_is_total, verify_parse_contract_kind_only_valid_kinds, verify_semver_reflexive, verify_semver_antisymmetric, verify_semver_transitive, verify_semver_strict_weak_order, verify_btreemap_deterministic, verify_gate_condition) |

### Errors by category:

| Error | Count | Lines | Description |
|-------|-------|-------|-------------|
| E0601 | 1 | 672 | `main` function not found in crate |
| E0282 | 7 | 82, 100, 131, 189, 217, 596, 604 | Type annotations needed in closures |
| E0308 | 6 | 292, 293, 294, 295, 296, 297 | Mismatched types — `int` vs `i32` in spec_compare_semver return |
| E0277 | 1 | 596 | `V` doesn't implement `std::fmt::Display` |
| Warning | 1 | 635 | Unused doc comment on macro-generated item |

### Key issues:
1. **int vs i32**: `spec_compare_semver` returns `i32` but Verus's internal integer type is `int`. Need explicit casts or change return type.
2. **Closure type inference**: Verus needs explicit type annotations for closure parameters in `.iter().all()` and `.iter().any()` calls.
3. **No main function**: Verus needs a `main` function or needs to be invoked with `--crate-type lib`.

**Action needed:** Fix type annotations, add `int` → `i32` casts in spec_compare_semver, invoke with `--crate-type lib` or add main function.

## Verifier: TLC (Model Checking)

| Field | Value |
|-------|-------|
| **STATUS** | NOT_AVAILABLE |
| **Spec** | `contracts/tla/ContractsAsData.tla` (301 lines) |
| **Invariants** | 8 (INV-001 through INV-008) |
| **Properties** | 3 (OBL-009, OBL-010, OBL-011) |
| **Temporal** | 2 (LivenessValidated, LivenessGatePass) |
| **Commands** | `java -jar tla2tools.jar -config ContractsAsData.cfg ContractsAsData.tla` |
| **Configuration** | MAX_FILES = 5, MAX_FILE_VERSION = 10 |
| **Expected output** | All 8 invariants PASS, all 3 properties PASS, no deadlock states |
| **Root cause** | TLC Java tool not installed in PATH |

**Action needed:** Install tla2tools.jar and run TLC model checker.

## Verifier: Miri (Undefined Behavior)

| Field | Value |
|-------|-------|
| **STATUS** | FAIL |
| **Error** | Non-exhaustive patterns in `vb_validate/src/diag_render.rs:28` |
| **Missing variants** | MissingSchemaVersion, CueVetFailed, VersionMonotonicityBreach |
| **Also affected** | `vb_validate/src/diagnostic.rs:106` — same 3 variants missing |
| **Root cause** | This bead added 3 new ValidationError variants to vb_validate/src/lib.rs but did not update existing exhaustive match arms |
| **Classification** | BLOCK_REGRESSION |

## Summary

| Verifier | Status | Obligations Covered |
|----------|--------|-------------------|
| Kani | NOT_AVAILABLE | OBL-001 (4 harnesses), OBL-004 (2 harnesses), OBL-009 (1 harness), OBL-011 (1 harness) |
| Verus | FAIL | OBL-001, OBL-002, OBL-004, OBL-006 |
| TLC | NOT_AVAILABLE | OBL-009, OBL-010, OBL-011, INV-001..008 |
| Miri | FAIL | OBL-009 (runtime UB check) |
| proptest | PASS | OBL-001 (3 tests), OBL-002 (5 tests), OBL-004 (3 tests), OBL-005 (1 test), OBL-006 (2 tests), OBL-008 (1 test) |
| cargo test | PARTIAL | OBL-001..008 covered in binding + proptest (56 pass, 22 fail) |
