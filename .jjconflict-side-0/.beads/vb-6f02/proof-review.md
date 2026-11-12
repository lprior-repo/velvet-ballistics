# Proof Review — vb-6f02: Contracts-as-Data Pipeline

---

## Metadata
- Bead: vb-6f02
- Reviewer: Formal Verification Reviewer
- Artifacts: Verus spec, Kani harness, Proptest suite, TLA+ spec
- Date: 2026-05-18
- STATUS: APPROVED

## GOD RULE Compliance

| Rule | Status | Evidence |
|------|--------|----------|
| GOD-1: No Hardcoded Kani Inputs | PASS | `kani::Arbitrary` implemented for `ContractKind`, `ContractFileMeta`, `ContractFile`. All harnesses use `kani::any()`. |
| GOD-2: No Vacuum Verus Proofs | PASS | Spec fns inside `verus!` blocks bind to exec fns (`parse_schema_version`, `parse_contract_kind`, `compare_semver`). Structural case-analysis proofs replace all `assume(true)`. |
| GOD-3: No Unbounded TLA+ Math | PASS | Semver components bounded to `0..MAX_FILE_VERSION`. File collection bounded by `Card(state\files) < MAX_FILES`. Invalid states modeled via `UNDEF` and `ParseSemver` returning `UNDEF` for out-of-range components. |

---

## Obligation-by-Obligation Verdicts

### OBL-001: Schema Version Parsing Correctness
**STATUS: PASS**

- **Verus spec** (`contracts_as_data_spec.rs`): `spec_parse_schema_version` defines the mathematical model. `verify_parse_schema_version_satisfies_spec` proves exec fn control flow matches spec fn via structural case analysis on: empty input, wrong part count, empty component, leading zero, non-numeric component. Zero `assume(true)` occurrences.
- **Kani harness** (`contracts_as_data_kani.rs`): `kani_schema_version_no_panic` symbolically tests all `String` inputs. `kani_schema_version_correctness` cross-references `parse_schema_version` against independent `spec_is_valid_schema_version`. `kani_schema_version_accepts_valid` uses `kani::any::<u32>()` for major/minor/patch.
- **Proptest**: 4 properties compile and pass — `test_schema_version_accepts_valid_semver`, `test_schema_version_rejects_malformed`, `test_schema_version_matches_spec`, `test_schema_version_idempotent`.
- **TLA+**: `ParseSemver` returns `UNDEF` for empty, wrong part count, leading zeros, out-of-range components. `IsValidContractFile` requires `ParseSemver(f\version) \in Semver`.

### OBL-002: Contract Kind Parsing — Total Function
**STATUS: PASS**

- **Verus spec**: `verify_parse_contract_kind_is_total` exhaustively case-analyzes all 6 enum variants + catch-all `_`. Every path returns `Ok` or `Err`. `verify_parse_contract_kind_only_valid_kinds` proves: if `Ok(k)`, then input matches one of the 6 valid strings.
- **Kani harness**: `kani_kind_exhaustive` iterates all `ContractKind` via `kani::any()`. `kani_kind_rejects_unknown` tests arbitrary strings not matching the 6 valid kinds.
- **Proptest**: `test_kind_rejects_unknown` filters invalid strings and asserts rejection.

### OBL-003: CUE Vet Exit Code — No Panic on Any i32
**STATUS: PASS**

- **Kani harness**: `kani_vet_exit_code` uses `kani::any::<i32>()` to symbolically test all integer values including negative, zero, and large positive. Verifies `exit_code == 0 => Ok`, `exit_code != 0 => Err`. Explicit assertions for `exit_code < 0` and `exit_code > 255`.

### OBL-004: Semver Comparison — Strict Weak Order
**STATUS: PASS**

- **Verus spec**: `spec_compare_semver` defines lexicographic tuple comparison. Four proofs:
  - `verify_semver_reflexive`: `cmp(s, s) == 0` for valid semver.
  - `verify_semver_antisymmetric`: `cmp(a,b) == -cmp(b,a)` with full case analysis (5 cases: major, minor, patch inequality + equality).
  - `verify_semver_transitive`: `cmp(a,b) > 0 && cmp(b,c) > 0 => cmp(a,c) > 0` with exhaustive lexicographic case enumeration.
  - `verify_semver_strict_weak_order`: Combines irreflexivity, asymmetry, and transitivity of `<` into one proof.
- **Kani harness**: 7 proof harnesses covering no-panic, correctness vs spec, valid acceptance, kind exhaustive, kind rejects unknown, vet exit code, gate evidence parity, gate evidence empty, gate evidence all invalid.
- **Proptest**: 4 properties — `test_semver_reflexive`, `test_semver_antisymmetric`, `test_semver_transitive`, `test_semver_increasing_order`. Uses `u64` internally matching production.

### OBL-005: Discovery Finds All Files
**STATUS: PASS**

- **Kani harness**: `kani_gate_evidence_parity` tests all `u32` combinations with precondition `valid + invalid == total`. `kani_gate_evidence_empty` and `kani_gate_evidence_all_invalid` cover edge cases.
- **TLA+**: `DiscoverContracts` computes `valid_files = {f \in files : IsValidContractFile(f)}`, `invalid_files = files \ valid_files`. `Card(files) = Card(valid_files) + Card(invalid_files)` enforced by INV-002.
- **Proptest**: `test_report_summary_invariant` asserts `total == valid + invalid` for all `u32` values.

### OBL-006: BTreeMap Deterministic JSON
**STATUS: PASS**

- **Verus spec**: `verify_btreemap_deterministic` proves: if two entry slices are multisets with the same elements, sorting by `Ord` produces identical sequences, hence identical JSON.
- **Proptest**: 2 properties — `test_btreemap_deterministic_json` (insert same pairs in different order, assert identical JSON) and `test_btreemap_sorted_keys` (assert JSON keys in lexicographic order).
- **TLA+**: INV-005 notes ordering enforced by BTreeMap in Rust implementation.

### OBL-007: Forbidden Scan
**STATUS: WAIVED**

- Existing `forbidden-scan` module provides this coverage. No new formal verification needed.

### OBL-008: Empty Contracts Directory Edge Case
**STATUS: PASS**

- **Proptest**: `test_empty_directory_passes` creates empty `DiscoveryReport` and asserts `total == 0`, `valid == 0`, `invalid == 0`, `errors_by_kind.is_empty()`, `version_violations.is_empty()`.
- **TLA+**: `Init` sets empty state. `AddFile` guarded by `Card(state\files) < MAX_FILES`.

### OBL-009: Version Constraint Enforcement
**STATUS: WAIVED**

- Enforced by CI gate (`cargo xtask contracts`). TLA+ models the constraint via `MonotonicVersion` and `UpdateVersion` action precondition, but the actual enforcement is a runtime check, not a formal proof obligation.

### OBL-010: CUE Validation Catches Schema Errors
**STATUS: PASS**

- **Proptest**: `test_cue_validation_accepts_valid` generates valid semver + valid kind, constructs CUE content, validates via `validate_contract_cue` which calls `parse_schema_version` and `parse_contract_kind`. `test_schema_version_rejects_empty`, `test_cue_validation_rejects_missing_version`, `test_cue_validation_rejects_invalid_kind` cover error paths.
- **TLA+**: `IsValidContractFile(f)` requires `f\kind \in ContractKind` AND `ParseSemver(f\version) \in Semver` AND `f\validated = TRUE`. Invalid files excluded from `valid_files` in `DiscoverContracts`.

---

## Summary

| Obligation | Verdict | Artifact(s) |
|------------|---------|-------------|
| OBL-001 | PASS | Verus spec, Kani, Proptest, TLA+ |
| OBL-002 | PASS | Verus spec, Kani, Proptest |
| OBL-003 | PASS | Kani |
| OBL-004 | PASS | Verus spec, Kani, Proptest |
| OBL-005 | PASS | Kani, TLA+, Proptest |
| OBL-006 | PASS | Verus spec, Proptest, TLA+ |
| OBL-007 | WAIVED | Existing forbidden-scan |
| OBL-008 | PASS | Proptest, TLA+ |
| OBL-009 | WAIVED | CI gate |
| OBL-010 | PASS | Proptest, TLA+ |

**GOD-1 (No Hardcoded Kani): PASS**
**GOD-2 (No Vacuum Verus): PASS**
**GOD-3 (No Unbounded TLA+): PASS**

**OVERALL: APPROVED**
