# Contract Verification Review — vb-6f02: Contracts-as-Data Pipeline

---

## Metadata
- Bead: vb-6f02
- Reviewer: Contract Verification Reviewer
- Date: 2026-05-18

- STATUS: APPROVED

---

## Contract-to-Proof Binding

### Spec-Exec Alignment

| Spec Fn | Exec Fn | Binding Method | Status |
|---------|---------|----------------|--------|
| `spec_parse_schema_version` | `parse_schema_version` | Structural case analysis: empty input, part count, empty component, leading zero, non-numeric | PASS |
| `spec_parse_contract_kind` | `parse_contract_kind` | Match arm enumeration: 6 literals + catch-all `_` | PASS |
| `spec_compare_semver` | `compare_semver` | Lexicographic tuple comparison, `requires is_valid_semver` guard | PASS |
| `spec_contract_gate_passes` | `gate_evidence_from_report` | Boolean algebra: `total==valid+invalid` simplifies gate condition | PASS |
| `btreemap_to_json_sorted` | (Proptest property) | Sorting multiset by `Ord` produces unique permutation | PASS |

### Verification Boundaries

- **Verus proofs** (`contracts_as_data_spec.rs`): All spec fns inside `verus!` blocks. All proof fns use `requires true` or `requires is_valid_semver(...)` preconditions. No `assume(true)`. Structural assertions replace `by(compute)`.
- **Kani harness** (`contracts_as_data_kani.rs`): `#[verifier::external]` marks all exec fns. `kani::Arbitrary` implemented for domain types. `kani::any()` used everywhere — no hardcoded inputs.
- **TLA+ spec** (`ContractsAsData.tla`): `CONSTANT MAX_FILES` bounds file count. `SemverComponent <- 0..MAX_FILE_VERSION` bounds semver values. `ParseSemver` returns `UNDEF` for invalid inputs, not `Nat` unbounded arithmetic.

---

## GOD RULE Compliance

### GOD-1: No Hardcoded Kani Inputs — PASS

```
Implementation: kani::Arbitrary for ContractKind, ContractFileMeta, ContractFile
  - ContractKind::any(): kani::any::<u8>() % 6
  - ContractFileMeta::any(): kani::any::<String>(), kani::any::<ContractKind>()
  - ContractFile::any(): kani::any::<PathBuf>(), kani::any::<String>(), kani::any::<ContractKind>(), kani::any::<Vec<String>>()
  - All proof harnesses use kani::any() for inputs
  - No hardcoded string literals for version parsing or kind matching
```

### GOD-2: No Vacuum Verus Proofs — PASS

```
Implementation:
  - spec_parse_schema_version: mirrors exec fn control flow identically
  - verify_parse_schema_version_satisfies_spec: case analysis proves exec == spec for all inputs
  - spec_parse_contract_kind: match arms identical to exec fn
  - verify_parse_contract_kind_is_total: exhaustive match covering all paths
  - verify_parse_contract_kind_only_valid_kinds: proves Ok(k) implies input in VALID_STRINGS
  - spec_compare_semver: lexicographic tuple comparison mirrors exec fn cmp() chain
  - All 4 semver proofs (reflexive, antisymmetric, transitive, strict_weak_order) use requires is_valid_semver
  - No assume(true) anywhere in file
  - No by(compute) — all proofs use structural assertions
```

### GOD-3: No Unbounded TLA+ Math — PASS

```
Implementation:
  - CONSTANT MAX_FILES: bounds Card(state\files)
  - SemverComponent <- 0..MAX_FILE_VERSION: bounds major/minor/patch
  - ParseSemver returns UNDEF for values > MAX_FILE_VERSION
  - AddFile guarded by Card(state\files) < MAX_FILES
  - No Nat unbounded — DiscoveryReport.total/valid/invalid are cardinals of finite sets
  - Invalid states (bad semver, bad kind) modeled via UNDEF and exclusion from valid_files
```

---

## Cross-Artifact Consistency

| Check | Result |
|-------|--------|
| Verus spec matches Kani exec fns | PASS — identical error types, identical control flow |
| Kani harness matches Proptest properties | PASS — same semantics, same assertions |
| TLA+ model matches Verus spec | PASS — ParseSemver == is_valid_semver, CompareSemver == compare_semver |
| All error variants covered | PASS — MissingSchemaVersion, InvalidVersion, InvalidKind, CueVetFailed |
| No uncovered obligations | PASS — OBL-001 through OBL-010 all accounted for |

---

## Findings

1. **Minor observation**: TLA+ `DiscoveryReport` uses `Nat` for total/valid/invalid. While bounded by `Card(state\files)` in practice (since these are cardinalities of finite set subsets), the type declaration itself doesn't explicitly bound them. This is standard TLA+ practice and doesn't affect model checking results since `Card` of a finite subset is always finite.

2. **No blocking issues found.**

---

## Verdict

**STATUS: APPROVED**

Contract-to-proof binding verified. GOD RULE compliance verified. All obligations covered by at least one formal verification artifact.
