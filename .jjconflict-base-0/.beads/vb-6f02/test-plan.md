# Test Plan: contracts-as-data suite (vb-6f02)

## Scope

Tests for the xtask contract-discovery module, CLI integration, CUE schema enforcement, and GateEvidence pipeline integration. This plan covers all requirements from contract.md (REQ-001 through REQ-009) and invariants (INV-001 through INV-006), excluding obligations already proven by formal verification artifacts.

## Artifacts Already Proven (DO NOT DUPLICATE)

The following obligations are covered by formal artifacts. Tests must NOT re-implement these proofs; they should only verify integration points.

| Obligation | Verifier | Artifact | What's Proven |
|------------|----------|----------|---------------|
| OBL-001 | Verus + Kani + Proptest | `contracts_as_data_spec.rs`, `contracts_as_data_kani.rs`, `contracts_as_data_props.rs` | `parse_schema_version` correctness: empty input → MissingSchemaVersion, malformed → InvalidVersion, valid → Ok. Zero assume(true). |
| OBL-002 | Verus + Kani + Proptest | Same three files | `parse_contract_kind` exhaustiveness: 6 enum variants + catch-all. Kani::Arbitrary + kani::any(). |
| OBL-003 | Kani | `contracts_as_data_kani.rs` | `parse_vet_exit_code`: all i32 values, exit_code 0 → Ok, non-zero → Err, no panic on negative/large values. |
| OBL-004 | Verus | `contracts_as_data_spec.rs` | `compare_semver`: reflexive, antisymmetric, transitive, strict weak order. Structural proofs. |
| OBL-005 | Verus + Proptest | `contracts_as_data_spec.rs`, `contracts_as_data_props.rs` | BTreeMap deterministic JSON: same multiset → same sorted JSON. Sorted keys assertion. |
| OBL-006 | Kani | `contracts_as_data_kani.rs` | `gate_evidence_from_report`: always Ok when valid+invalid==total, correct status/exit_code/why_failed. |

**Test strategy**: Tests cover integration layers (CLI invocation, cue vet execution, GateEvidence pipeline wiring) and edge cases not exercised by formal proofs (file system operations, directory walks, CUE schema validation end-to-end).

## Test Plan

---

### 1. Unit Tests: xtask contracts module

**Target**: `xtask/src/contracts.rs` (production code to be created)

#### 1.1 ContractKind parsing
| Test ID | Requirement | Description | Input | Expected |
|---------|-------------|-------------|-------|----------|
| TST-001 | REQ-003, INV-002 | Parse all 6 valid kinds | `"cli_envelope"`, `"ui_tokens"`, `"accepted_artifacts"`, `"evidence_bundle"`, `"diagnostics"`, `"gate_output"` | `Ok(ContractKind::X)` for each |
| TST-002 | REQ-003, INV-002 | Reject unknown kind | `"invalid_kind"`, `"CLI_ENVELOPE"`, `"cli-envelope"`, `""`, `"cli_envelope_extra"` | `Err(InvalidKind { kind })` for each |
| TST-003 | INV-002 | `all_values()` returns exactly 6 | Call `ContractKind::all_values()` | `.len() == 6`, sorted ordinally |
| TST-004 | INV-002 | `all_values()` matches enum variants | Compare `all_values()` to match arms | Each enum variant appears exactly once |

#### 1.2 schema_version parsing
| Test ID | Requirement | Description | Input | Expected |
|---------|-------------|-------------|-------|----------|
| TST-005 | REQ-003, INV-001 | Accept valid semver | `"1.0.0"`, `"0.1.0"`, `"0.0.1"`, `"999.999.999"`, `"1.2.3"` | `Ok(version)` for each |
| TST-006 | REQ-003, INV-001 | Reject empty string | `""` | `Err(MissingSchemaVersion)` |
| TST-007 | REQ-003, INV-001 | Reject wrong part count | `"1.0"`, `"1"`, `"1.0.0.0"`, `"abc"` | `Err(InvalidVersion { version })` |
| TST-008 | REQ-003, INV-001 | Reject leading zeros | `"01.0.0"`, `"1.01.0"`, `"1.0.01"` | `Err(InvalidVersion { version })` |
| TST-009 | REQ-003, INV-001 | Reject non-numeric parts | `"1.0.a"`, `"a.b.c"`, `"1.0.0a"` | `Err(InvalidVersion { version })` |
| TST-010 | REQ-003, INV-001 | Reject empty components | `"1..0"`, `"1.0."`, `".1.0"` | `Err(InvalidVersion { version })` |

#### 1.3 Semver comparison
| Test ID | Requirement | Description | Input | Expected |
|---------|-------------|-------------|-------|----------|
| TST-011 | REQ-005, INV-004 | Equal versions | `("1.0.0", "1.0.0")` | `Ok(Equal)` |
| TST-012 | REQ-005, INV-004 | Different major | `("1.0.0", "2.0.0")` | `Ok(Less)` |
| TST-013 | REQ-005, INV-004 | Different minor | `("1.0.0", "1.1.0")` | `Ok(Less)` |
| TST-014 | REQ-005, INV-004 | Different patch | `("1.0.0", "1.0.1")` | `Ok(Less)` |
| TST-015 | REQ-005, INV-004 | Reverse comparisons | `("2.0.0", "1.0.0")`, `("1.1.0", "1.0.0")`, `("1.0.1", "1.0.0")` | `Ok(Greater)` for each |
| TST-016 | REQ-005, INV-004 | Invalid inputs | `("1.0", "1.0.0")`, `("invalid", "1.0.0")` | `Err(String)` |

#### 1.4 File discovery
| Test ID | Requirement | Description | Setup | Expected |
|---------|-------------|-------------|-------|----------|
| TST-017 | REQ-002, INV-001 | Discover valid files in populated directory | 3 valid `.cue` files in temp dir | `DiscoveryReport` with `total=3`, `valid=3`, `invalid=0` |
| TST-018 | REQ-002, INV-002 | Discover files with invalid kind | 2 valid + 1 file with `kind: "invalid"` | `total=3`, `valid=2`, `invalid=1`, error in `errors_by_kind` |
| TST-019 | REQ-002, INV-001 | Discover files with invalid schema_version | 2 valid + 1 file with `schema_version: "01.0.0"` | `total=3`, `valid=2`, `invalid=1`, version violation recorded |
| TST-020 | REQ-002, INV-001 | Discover files missing schema_version | 2 valid + 1 file without `schema_version` field | `total=3`, `valid=2`, `invalid=1`, MissingSchemaVersion error |
| TST-021 | REQ-002, INV-001 | Discover files missing kind | 2 valid + 1 file without `kind` field | `total=3`, `valid=2`, `invalid=1`, InvalidKind error |
| TST-022 | REQ-002, INV-001 | Non-.cue files are ignored | Valid `.cue` + `.yaml` + `.txt` + `.json` | Only `.cue` files processed |
| TST-023 | REQ-002, INV-001 | Subdirectories recursed | Nested `contracts/subdir/file.cue` | Files in subdirectories discovered |

#### 1.5 Report summary
| Test ID | Requirement | Description | Expected |
|---------|-------------|-------------|----------|
| TST-024 | INV-006 | `total == valid + invalid` invariant | `report.summary.total == report.summary.valid + report.summary.invalid` |
| TST-025 | INV-006 | `errors_by_kind` sums to invalid count | `errors_by_kind.values().sum() == report.summary.invalid` |
| TST-026 | INV-005 | Files sorted by path | `report.files.windows(2).all(|w| w[0].path <= w[1].path)` |
| TST-027 | INV-005 | Error messages sorted | `report.files.iter().flat_map(|f| &f.vet_errors).windows(2).all(|w| w[0] <= w[1])` |

#### 1.6 GateEvidence construction
| Test ID | Requirement | Description | Expected |
|---------|-------------|-------------|----------|
| TST-028 | REQ-004, INV-006 | Gate passes when all valid | `report.invalid == 0` | `GateEvidence { status: Pass, exit_code: 0, why_failed: None }` |
| TST-029 | REQ-004, INV-006 | Gate fails when any invalid | `report.invalid > 0` | `GateEvidence { status: Fail, exit_code: 1, why_failed: Some(...) }` |
| TST-030 | REQ-004, INV-006 | GateEvidence fields set correctly | Any report | `kind == "contract-discovery"`, `gate_name == "contracts"` |
| TST-031 | REQ-004, INV-006 | All zeros report produces Pass | `total=0, valid=0, invalid=0` | `status: Pass, exit_code: 0, why_failed: None` |

---

### 2. Integration Tests: xtask CLI + Moon task

**Target**: CLI subcommand `cargo xtask contracts`, moon task definition

#### 2.1 CLI subcommand
| Test ID | Requirement | Description | Expected |
|---------|-------------|-------------|----------|
| TST-032 | REQ-002 | `cargo xtask contracts` runs without error | Execute in workspace root | Exit code 0 (all contracts valid) |
| TST-033 | REQ-009 | `--json` flag produces valid JSON | `cargo xtask contracts --json` | Valid JSON parseable as `DiscoveryReport` |
| TST-034 | REQ-002 | `--check` flag fails on invalid contracts | `cargo xtask contracts --check` with invalid contract | Exit code 1 |
| TST-035 | REQ-002 | `--dir` flag points to custom directory | `cargo xtask contracts --dir /tmp/empty_contracts` | Report with `total=0`, status `Pass` |
| TST-036 | REQ-002 | Default dir is `contracts/` | `cargo xtask contracts` | Scans `contracts/` directory |
| TST-037 | REQ-002 | Output to stdout (not file) | `cargo xtask contracts` | JSON printed to stdout |
| TST-038 | REQ-009 | JSON output compatible with moon consumers | Parse `--json` output | Contains `total`, `valid`, `invalid`, `errors_by_kind` fields |

#### 2.2 Moon task integration
| Test ID | Requirement | Description | Expected |
|---------|-------------|-------------|----------|
| TST-039 | OBL-009 (REQ-009) | Moon task `contracts` runs | `moon run :contracts` | Exit code 0 |
| TST-040 | OBL-009 (REQ-009) | Moon task fails on invalid | `moon run :contracts` with invalid contract | Exit code 1 |

---

### 3. Contract Validation Tests: CUE Schema Enforcement

**Target**: `contracts/*.cue` files, `cue vet` execution

#### 3.1 CUE schema validation
| Test ID | Requirement | Description | Expected |
|---------|-------------|-------------|----------|
| TST-041 | REQ-001, REQ-007, OBL-010 | All 6 contract CUE files pass `cue vet` | Run `cue vet contracts/*.cue` | All exit 0 |
| TST-042 | REQ-001, REQ-007, OBL-010 | manifest.cue passes `cue vet` | Run `cue vet contracts/manifest.cue` | Exit 0 |
| TST-043 | REQ-001, OBL-010 | Invalid CUE rejected by `cue vet` | Create temp file missing `schema_version` | `cue vet` exits non-zero |
| TST-044 | REQ-001, OBL-010 | Invalid kind rejected by `cue vet` | Create temp file with `kind: "bogus"` | `cue vet` exits non-zero |
| TST-045 | REQ-001, OBL-010 | CUE schema enforces kind values | Create temp with `kind: "cli_envelope"` (valid) | `cue vet` passes |
| TST-046 | REQ-001, OBL-010 | UI tokens schema enforces property types | Create temp with valid `#UITokens` | `cue vet` passes |
| TST-047 | REQ-001, OBL-010 | UI tokens schema rejects invalid property type | Create temp with `type: "invalid_type"` | `cue vet` fails |
| TST-048 | REQ-001, OBL-010 | Evidence bundle schema enforces status values | Create temp with valid `#EvidenceBundle` | `cue vet` passes |
| TST-049 | REQ-001, OBL-010 | Gate output schema enforces status values | Create temp with `status: "pass"` | `cue vet` passes |
| TST-050 | REQ-001, OBL-010 | Gate output optional why_failed | Create temp without `why_failed` | `cue vet` passes (optional field) |
| TST-051 | REQ-001, OBL-010 | Gate output why_failed requires hint+repair_command | Create temp with partial `why_failed` | `cue vet` fails |

#### 3.2 xtask cue vet execution
| Test ID | Requirement | Description | Expected |
|---------|-------------|-------------|----------|
| TST-052 | REQ-002, REQ-007 | xtask runs `cue vet` on each file | Valid contract | `vet_errors: []` |
| TST-053 | REQ-002, REQ-007 | xtask collects vet errors | Invalid contract | `vet_errors` contains error strings |
| TST-054 | REQ-002, REQ-007 | `cue` CLI unavailable handled gracefully | `cue` not in PATH | Error message, not panic |
| TST-055 | REQ-002, REQ-007 | `cue vet` exit code 0 = pass | Valid contract | `vet_errors: []` |
| TST-056 | REQ-002, REQ-007 | `cue vet` exit code != 0 = fail | Invalid contract | `vet_errors` populated, file marked invalid |

---

### 4. GateEvidence Integration Tests

**Target**: Integration with `xtask/src/evidence/tooling_and_gate_types.rs`

| Test ID | Requirement | Description | Expected |
|---------|-------------|-------------|----------|
| TST-057 | REQ-004 | GateEvidence serializes to JSON | Serialize GateEvidence | Valid JSON with all fields |
| TST-058 | REQ-004 | GateEvidence deserializes from JSON | Deserialize valid JSON | Round-trip preserves all fields |
| TST-059 | REQ-004 | GateStatus variants match enum | Serialize all 3 variants | `Pass`, `Fail`, `Skipped { reason }` |
| TST-060 | REQ-004 | Skipped gate has reason | `GateStatus::Skipped { reason: "..." }` | `why_failed` contains reason |
| TST-061 | REQ-004 | GateEvidence log path correct | Any report | `log == PathBuf::from(".evidence/contracts/last_run.log")` |
| TST-062 | REQ-004 | GateEvidence command correct | Any report | `command == "cargo xtask contracts --dir contracts"` |

---

### 5. Edge Case Tests

#### 5.1 Malformed inputs
| Test ID | Requirement | Description | Expected |
|---------|-------------|-------------|----------|
| TST-063 | INV-001 | File with no top-level fields | `{}` | MissingSchemaVersion error |
| TST-064 | INV-002 | File with extra unknown fields | `kind: "cli_envelope", schema_version: "1.0.0", extra_field: true` | Passes (schema allows extras) |
| TST-065 | INV-001 | schema_version with spaces | `"1. 0.0"` | InvalidVersion error |
| TST-066 | INV-001 | schema_version with negative numbers | `"-1.0.0"` | InvalidVersion error |
| TST-067 | INV-001 | schema_version with floating point | `"1.0.0.5"` | InvalidVersion error (4 parts) |
| TST-068 | INV-002 | kind with mixed case | `"Cli_Envelope"` | InvalidKind error |
| TST-069 | INV-002 | kind with special characters | `"cli-envelope"`, `"cli envelope"`, `"cli_envelope!"` | InvalidKind error |

#### 5.2 File system edge cases
| Test ID | Requirement | Description | Expected |
|---------|-------------|-------------|----------|
| TST-070 | OBL-008 | Empty contracts/ directory | Empty temp dir | `total=0, valid=0, invalid=0, status=Pass` |
| TST-071 | OBL-008 | contracts/ directory does not exist | Non-existent path | Error message, not panic |
| TST-072 | REQ-002 | contracts/ is a file not directory | Single file at `contracts/` path | Error message |
| TST-073 | REQ-002 | Permission denied on directory | Create dir without read permission | Error message, not panic |
| TST-074 | REQ-002 | Symlink in contracts/ directory | Symlink to valid `.cue` file | Followed and processed |
| TST-075 | REQ-002 | Binary file with .cue extension | Non-text file named `file.cue` | Error, not panic |
| TST-076 | REQ-002 | Very deeply nested directory | `contracts/a/b/c/d/e/file.cue` | Discovered correctly |
| TST-077 | REQ-002 | Hidden files (dotfiles) | `.hidden.cue` | Discovered (not excluded) |
| TST-078 | REQ-002 | Large number of files | 1000+ `.cue` files in contracts/ | Completes without OOM |
| TST-079 | REQ-002 | Unicode in file paths | `contracts/über.cue` | Handled correctly |

#### 5.3 Version monotonicity edge cases
| Test ID | Requirement | Description | Expected |
|---------|-------------|-------------|----------|
| TST-080 | REQ-005, INV-004 | Version upgrade (major) | Old: `"1.0.0"`, New: `"2.0.0"` | Pass (new > old) |
| TST-081 | REQ-005, INV-004 | Version downgrade (major) | Old: `"2.0.0"`, New: `"1.0.0"` | VersionViolation recorded |
| TST-082 | REQ-005, INV-004 | Version upgrade (minor) | Old: `"1.0.0"`, New: `"1.1.0"` | Pass |
| TST-083 | REQ-005, INV-004 | Version downgrade (minor) | Old: `"1.1.0"`, New: `"1.0.0"` | VersionViolation recorded |
| TST-084 | REQ-005, INV-004 | Version upgrade (patch) | Old: `"1.0.0"`, New: `"1.0.1"` | Pass |
| TST-085 | REQ-005, INV-004 | Version downgrade (patch) | Old: `"1.0.1"`, New: `"1.0.0"` | VersionViolation recorded |
| TST-086 | REQ-005, INV-004 | Same version | Old: `"1.0.0"`, New: `"1.0.0"` | Pass (not a decrease) |
| TST-087 | REQ-005, INV-004 | First version (no previous) | No manifest entry | Pass (no violation possible) |

---

### 6. BDD Scenarios: Operator Workflows

**Format**: Given/When/Then behavioral scenarios exercising the system from outside in.

#### Scenario 1: Happy path — valid contracts pass gate
```
Given a contracts/ directory with 3 valid .cue files (cli_envelope, ui_tokens, evidence_bundle)
When the operator runs "cargo xtask contracts --check"
Then the exit code is 0
And the JSON output contains {"total": 3, "valid": 3, "invalid": 0}
And GateEvidence.status == Pass
```

#### Scenario 2: Sad path — invalid kind fails gate
```
Given a contracts/ directory with 2 valid files and 1 file with kind: "bogus"
When the operator runs "cargo xtask contracts --check"
Then the exit code is 1
And the JSON output contains {"invalid": 1}
And GateEvidence.why_failed contains "1 contract(s) failed validation"
```

#### Scenario 3: Sad path — missing schema_version fails gate
```
Given a contracts/ directory with a file missing schema_version field
When the operator runs "cargo xtask contracts --check"
Then the exit code is 1
And the errors list contains MissingSchemaVersion
```

#### Scenario 4: Edge case — empty contracts directory
```
Given an empty contracts/ directory
When the operator runs "cargo xtask contracts"
Then the exit code is 0
And the JSON output contains {"total": 0, "valid": 0, "invalid": 0}
And GateEvidence.status == Pass
```

#### Scenario 5: Edge case — no contracts/ directory
```
Given no contracts/ directory exists
When the operator runs "cargo xtask contracts --check"
Then the exit code is 1 (or graceful error)
And the error message mentions the missing directory
```

#### Scenario 6: JSON output consumption
```
Given valid contracts in contracts/
When the operator runs "cargo xtask contracts --json"
Then the output is valid JSON
And the JSON contains "total", "valid", "invalid", "errors_by_kind" keys
And errors_by_kind keys are sorted lexicographically
```

#### Scenario 7: Version monotonicity enforcement
```
Given a contract file previously validated at schema_version "2.1.0"
And the operator updates the file to schema_version "1.9.0" (downgrade)
When the operator runs "cargo xtask contracts --check"
Then a VersionMonotonicityBreach error is reported
And GateEvidence.status == Fail
```

#### Scenario 8: CUE vet failure
```
Given a contracts/ directory with a file that has invalid CUE syntax
When the operator runs "cargo xtask contracts --check"
Then the exit code is 1
And the vet_errors list contains the CUE vet error message
```

#### Scenario 9: Idempotency — same input produces same output
```
Given a contracts/ directory with 5 files
When the operator runs "cargo xtask contracts --json" twice
Then both outputs are byte-identical JSON
```

#### Scenario 10: Custom directory via --dir
```
Given a directory /tmp/my-contracts with 2 valid .cue files
When the operator runs "cargo xtask contracts --dir /tmp/my-contracts"
Then the exit code is 0
And total == 2
```

---

### 7. Determinism Tests

**Target**: BTreeMap serialization, sorted output, stable ordering

| Test ID | Requirement | Description | Expected |
|---------|-------------|-------------|----------|
| TST-088 | INV-005, REQ-008 | Sorted output stable across runs | Run discovery 10 times | All outputs identical |
| TST-089 | INV-005 | BTreeMap JSON keys sorted | Serialize ReportSummary | Keys in lexicographic order |
| TST-090 | INV-005 | File paths sorted lexicographically | Files in arbitrary order in FS | Output sorted by PathBuf |
| TST-091 | REQ-009 | JSON output stable (no HashMap randomness) | Run with same input 10 times | Identical JSON output |
| TST-092 | INV-005 | errors_by_kind keys sorted | Multiple invalid files of different kinds | Keys sorted in JSON |

---

### 8. Forbidden-Scan Compliance

**Target**: No YAML/JSON/HTTP in runtime core (contracts/ is tooling, not core)

| Test ID | Requirement | Description | Expected |
|---------|-------------|-------------|----------|
| TST-093 | OBL-007 (REQ-006) | `contracts/` not under `crates/` | Check workspace structure | `contracts/` at workspace root, not in `crates/` |
| TST-094 | OBL-007 | xtask contracts module has no `use yaml` | Scan `xtask/src/contracts.rs` | No yaml imports |
| TST-095 | OBL-007 | xtask contracts module has no `use serde_json` for runtime | Scan `xtask/src/contracts.rs` | JSON serialization only in tests/CLI output |
| TST-096 | OBL-007 | CUE schemas not YAML | Check `contracts/*.cue` | All files use `.cue` extension, not `.yaml` |
| TST-097 | OBL-007 | Forbidden-scan passes on vb_core | `cargo xtask forbidden-scan --crate vb_core` | Exit 0 |

---

### 9. Proof Artifact Cross-Reference

**Purpose**: Verify production code uses the same types/functions that are already proven. No new proof — just integration verification.

| Test ID | Requirement | Description | Artifact Referenced | Expected |
|---------|-------------|-------------|---------------------|----------|
| TST-098 | OBL-001 | `parse_schema_version` in contracts.rs matches spec fn signature | `contracts_as_data_spec.rs:51` | Function exists, same params/return type |
| TST-099 | OBL-002 | `parse_contract_kind` in contracts.rs matches spec fn signature | `contracts_as_data_spec.rs:152` | Function exists, same params/return type |
| TST-100 | OBL-003 | `parse_vet_exit_code` in contracts.rs matches Kani harness | `contracts_as_data_kani.rs:152` | Function exists, same params/return type |
| TST-101 | OBL-004 | `compare_semver` in contracts.rs matches Verus spec | `contracts_as_data_spec.rs:307` | Function exists, same params/return type |
| TST-102 | OBL-005 | `ReportSummary` uses BTreeMap for errors_by_kind | `contracts_as_data_spec.rs:593` | Field type is `BTreeMap<ContractKind, u32>` |
| TST-103 | OBL-006 | `gate_evidence_from_report` matches Kani harness signature | `contracts_as_data_kani.rs:196` | Function exists, same params/return type |
| TST-104 | OBL-001 | Error variants match vb_validate ValidationError | `crates/vb_validate/src/lib.rs:97` | `MissingSchemaVersion`, `InvalidKind`, `CueVetFailed`, `VersionMonotonicityBreach` exist |
| TST-105 | OBL-002 | ContractKind derives Serialize/Deserialize | `contract.md:130` | All 6 derive macros present |

---

### 10. Mutation Testing Targets

**Target**: Verify test assertion strength via mutation operators

| Test ID | Mutation | Original | Expected Kill |
|---------|----------|----------|---------------|
| TST-106 | `==` → `!=` in total check | `total == valid + invalid` | TST-024 kills |
| TST-107 | `0` → `1` in gate status | `invalid == 0` → `invalid == 1` | TST-028, TST-029 kill |
| TST-108 | `>` → `>=` in comparison | `cmp(a, b) > 0` → `cmp(a, b) >= 0` | TST-011, TST-015 kill |
| TST-109 | `Err` → `Ok` in parsing | `Err(InvalidKind)` → `Ok(...)` | TST-002 kills |
| TST-110 | `Missing` → present in schema | Missing schema_version → present | TST-063 kills |
| TST-111 | `Pass` → `Fail` in gate | `GateStatus::Pass` → `GateStatus::Fail` | TST-028 kills |
| TST-112 | `sort` removed from output | Output unsorted | TST-088 kills |
| TST-113 | `BTreeMap` → `HashMap` | Deterministic → random ordering | TST-091 kills |

---

## Test Execution Commands

### Unit tests
```bash
cargo test -p xtask --lib contracts:: 2>&1
```

### Integration tests (CLI)
```bash
cargo test -p xtask --test contracts_integration 2>&1
```

### CUE validation tests
```bash
cue vet contracts/*.cue 2>&1
cargo test -p xtask --test contracts_cue_validation 2>&1
```

### BDD scenarios
```bash
cargo test -p xtask --test contracts_bdd 2>&1
```

### Determinism tests
```bash
cargo test -p xtask --test contracts_determinism 2>&1
```

### Forbidden-scan
```bash
cargo xtask forbidden-scan --crate vb_core 2>&1
```

### Proof artifact cross-reference
```bash
# Compile-time check: same types/functions must exist
cargo test -p workspace_tests --test contracts_as_data_kani 2>&1
cargo test -p workspace_tests --test contracts_as_data_props 2>&1
```

### Full suite
```bash
cargo test -p xtask 2>&1
cargo test -p workspace_tests 2>&1
```

---

## Test Count Summary

| Section | Tests | Type |
|---------|-------|------|
| 1. Unit tests: xtask contracts module | 38 | Unit |
| 2. Integration tests: CLI + Moon | 10 | Integration |
| 3. Contract validation: CUE schemas | 21 | Integration |
| 4. GateEvidence integration | 6 | Integration |
| 5. Edge cases | 27 | Unit/Integration |
| 6. BDD scenarios | 10 | Behavioral |
| 7. Determinism | 5 | Property |
| 8. Forbidden-scan | 5 | CI |
| 9. Proof cross-reference | 8 | Compilation |
| 10. Mutation testing | 8 | Mutation |
| **Total** | **138** | |

---

## Gaps and Waivers

| Obligation | Coverage | Notes |
|------------|----------|-------|
| OBL-001 | TST-005 through TST-009 (unit) + TST-098 (cross-ref) | Core parsing proven by Verus + Kani + proptest. Tests verify production integration. |
| OBL-002 | TST-001, TST-002, TST-003, TST-004 (unit) + TST-099 (cross-ref) | Exhaustiveness proven by Kani. Tests verify production integration. |
| OBL-003 | TST-052 through TST-056 (unit) + TST-100 (cross-ref) | Exit code handling proven by Kani. Tests verify cue vet execution integration. |
| OBL-004 | TST-011 through TST-016 (unit) + TST-101 (cross-ref) | Strict weak order proven by Verus. Tests verify production integration. |
| OBL-005 | TST-088 through TST-092 (determinism) + TST-102 (cross-ref) | BTreeMap determinism proven by Verus + proptest. Tests verify serialization integration. |
| OBL-006 | TST-028 through TST-031, TST-057 through TST-062 (integration) + TST-103 (cross-ref) | GateEvidence parity proven by Kani. Tests verify pipeline wiring. |
| OBL-007 | TST-093 through TST-097 (forbidden-scan) | WAIVED — existing forbidden-scan gate. Tests verify contracts/ is excluded. |
| OBL-008 | TST-070 (edge case) | Empty directory covered by proptest in workspace_tests. Integration test verifies xtask path. |
| OBL-009 | TST-039, TST-040 (moon task) | CI gate — moon task integration. Cannot be unit-tested; must run `moon run :contracts`. |
| OBL-010 | TST-041 through TST-051 (CUE validation) | CUE schema enforcement via `cue vet`. Integration test exercises real CUE tool. |

---

## Execution Order

```
Phase 1 (foundation — unit tests, compile):
  TST-001 through TST-031 (section 1: unit tests)
  TST-093 through TST-105 (sections 8, 9: forbidden-scan + cross-ref)

Phase 2 (integration — CLI, CUE, GateEvidence):
  TST-032 through TST-062 (sections 2, 3, 4: CLI, CUE, GateEvidence)

Phase 3 (edge cases — file system, monotonicity):
  TST-063 through TST-087 (section 5: edge cases)

Phase 4 (BDD + determinism):
  TST-088 through TST-092 (section 7: determinism)
  TST-093 through TST-100 (section 6: BDD)

Phase 5 (mutation):
  TST-106 through TST-113 (section 10: mutation testing)
```

---

## Notes for test-writer

1. **Do NOT re-implement parsing logic tests** — the Verus spec, Kani harness, and proptest suite in `workspace_tests` already prove `parse_schema_version`, `parse_contract_kind`, `compare_semver`, and `parse_vet_exit_code` correctness. Unit tests TST-001 through TST-016 verify that the production code in `xtask/src/contracts.rs` has the same API signatures and behavior.

2. **CUE files must exist before TST-041 through TST-051 run** — these integration tests assume `contracts/*.cue` files are present. The test-writer should create these files (see contract.md CUE schema templates) before running section 3 tests.

3. **`cue` CLI must be installed** — TST-041 through TST-056 require `cue vet`. The CI should run `just install-cue` before test execution. Tests that check for `cue` availability should skip gracefully if unavailable.

4. **BDD scenarios (section 6) use temp directories** — each scenario creates and cleans up its own temp directory. No shared mutable state between scenarios.

5. **Determinism tests (section 7) require real file system** — they use actual `.cue` files and the discovery function. Cannot be unit-tested in-memory.

6. **Mutation testing (section 10) is aspirational** — these are mutation operators to verify test assertion strength. Not all may be automatable with current tooling.
