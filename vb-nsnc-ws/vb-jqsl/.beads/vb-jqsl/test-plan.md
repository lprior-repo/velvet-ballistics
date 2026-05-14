# Test Plan: vb-jqsl — verify Hero Command and VerificationReport Certificates

## 1. Behavior Inventory

All behaviors are expressed as `[Subject] [action] [outcome] when [condition]`.

### Core Verification Pipeline

- **run_verification** parses YAML and returns `Ok(VerifyOk)` with digest_hex, checks, warnings when workflow is valid
- **run_verification** returns `Err(VerifyError::YamlParse)` when YAML syntax is invalid
- **run_verification** returns `Err(VerifyError::Compile)` when workflow fails schema/reference/control-flow/taint checks
- **run_verification** returns `Err(VerifyError::IrValidation)` when IR validation fails at Standard/Full profile
- **run_verification** returns `Err(VerifyError::BudgetPolicy)` when boundedness policy fails at Full profile
- **run_verification** runs only YAML+Compile gates for Quick profile
- **run_verification** runs YAML+Compile+IRValidation gates for Standard profile
- **run_verification** runs YAML+Compile+IRValidation+Budget gates for Full profile and fails closed on BudgetPolicy
- **run_verification** never panics; missing symbols return `CliExitCode::VerificationFailed` with diagnostic

### Exit Code Mapping

- **exit_code_for_error** returns `CliExitCode::ValidationFailed` for `YamlParse` and `Compile` errors
- **exit_code_for_error** returns `CliExitCode::VerificationFailed` for `IrValidation` and `BudgetPolicy` errors
- **exit_code_for_error** returns `CliExitCode::StorageError` for `StorageError` variants
- **exit_code_for_error** returns `CliExitCode::ReplayDivergence` for `ReplayDivergence` variants
- **exit_code_for_error** is deterministic: same error variant always yields same exit code regardless of format

### VerificationReport Assembly

- **assemble_verification_report** produces a `VerificationReport` with all fields populated from VerifyOk
- **assemble_verification_report** sets `profile` to the applied profile name string
- **assemble_verification_report** populates `artifact.source_digest_hex` from the workflow source bytes
- **assemble_verification_report** populates `artifact.ir_digest_hex` from the compiled IR digest
- **assemble_verification_report** populates `artifact.node_count` from compiled node count
- **assemble_verification_report** populates `artifact.passed_checks` from the checks vector
- **assemble_verification_report** populates `replay.gates_passed` from checks
- **assemble_verification_report** populates `replay.gate_sequence` in execution order
- **assemble_verification_report** sets `replay.replay_safe` based on profile and gate completeness
- **assemble_verification_report** populates `durability.profile` and `durability.durable` flag
- **assemble_verification_report** sets `durability.journal_written` to false (verify is read-only)
- **assemble_verification_report** sets `exit_code` to 0 for success
- **assemble_verification_report** returns empty `repair_hints` on success

### Repair Hint Generation

- **repair_hint_for_error** returns a non-empty vector for every `VerifyError` variant
- **repair_hint_for_error** populates `RepairHint.gate` with the concrete gate name
- **repair_hint_for_error** populates `RepairHint.hint` with a human-actionable message
- **repair_hint_for_error** populates `RepairHint.bead_reference` when a related bead exists
- **repair_hint_for_error** returns empty vector when error is `None` (success case)
- **repair_hint_for_error::YamlParse** hint cites the YAML parser and suggests syntax fix
- **repair_hint_for_error::Compile** hint cites the failing compile pass
- **repair_hint_for_error::IrValidation** hint cites "IrValidation" gate
- **repair_hint_for_error::BudgetPolicy** hint cites "BudgetPolicy" gate and suggests constraint relaxation
- **repair_hint_for_error::StorageError** hint cites storage admission requirement
- **repair_hint_for_error::ReplayDivergence** hint cites replay ABI requirement

### Output Format Parity

- **cmd_verify** produces identical exit codes for Text, Json, and Jsonl formats
- **cmd_verify** lists identical failing gates in both text and JSON output
- **cmd_verify** JSON output is valid UTF-8 and parseable by standard JSON parser
- **cmd_verify** JSON output contains all certificate fields without omission

### Panic Containment

- **cmd_verify** catches all panics from downstream crates and surfaces clean `CliExitCode::VerificationFailed`
- **cmd_verify** never exposes raw stack traces or `unwrap` failures to the operator
- **run_verification** with missing `vb_validate`/`vb_compile`/`vb_storage` symbols returns error not panic

---

## 2. Trophy Allocation

| Layer | Allocation | Rationale |
|---|---|---|
| **Unit** (`#[cfg(test)]` in `commands_verify.rs`) | ~35% — **22 unit tests planned (≥ 20)** | 4 `pub(crate)` functions × 5 assertions each (happy path, each error variant, invariant/property check); `exit_code_for_error` alone has 6 variants × 2 assertions = 12 tests |
| **Integration** (`tests/`) | ~45% | Full pipeline: `run_verification` with real `vb_yaml`, `vb_compile`, `vb_validate` crates; format parity between Text and Json |
| **E2E / Acceptance** | ~5% | CLI invocation: `velvet-ballastics verify <path>` end-to-end smoke |
| **Static** (`clippy`, `cargo-deny`) | ~10% | Zero `unsafe`/`unwrap`/`panic`, supply chain verification |
| **Proptest** | 5 property-based invariants | `run_verification` profile-gate exclusivity, `exit_code_for_error` totality, `repair_hint_for_error` non-emptiness, `assemble_verification_report` field validity (4 invariants × 3 test vectors each = 12 proptest cases) |
| **Fuzz** | ~5% | Malformed YAML inputs to `run_verification` and `parse_workflow_source` |

---

## 3. BDD Scenarios

### Happy Path — Minimal Valid Workflow

```
### Behavior: run_verification returns VerifyOk with all gates passed for minimal valid workflow
Given: a valid minimal workflow YAML at tests/fixtures/valid/minimal.yaml
When: run_verification is called with Quick profile
Then: result is Ok(VerifyOk) with non-empty digest_hex
And: result.checks contains "yaml_parse" and "compilation"

### Behavior: run_verification returns VerifyOk with all Standard gates passed
Given: a valid minimal workflow YAML
When: run_verification is called with Standard profile
Then: result is Ok(VerifyOk) with checks containing "yaml_parse", "compilation", "ir_validation"

### Behavior: run_verification returns VerifyOk with all Full gates passed
Given: a valid minimal workflow YAML that passes budget policy
When: run_verification is called with Full profile
Then: result is Ok(VerifyOk) with checks containing "yaml_parse", "compilation", "ir_validation", "budget_computation", "boundedness_policy"

### Behavior: assemble_verification_report produces complete certificate on success
Given: a VerifyOk with digest_hex="abc123" and checks=["yaml_parse","compilation"]
When: assemble_verification_report is called with Quick profile
Then: report.profile == "quick"
And: report.artifact.source_digest_hex is non-empty hex string
And: report.artifact.ir_digest_hex is non-empty hex string
And: report.artifact.node_count >= 1
And: report.replay.gates_passed contains "yaml_parse" and "compilation"
And: report.replay.gate_sequence == ["yaml_parse", "compilation"]
And: report.replay.replay_safe == true
And: report.durability.profile == VerifyProfile::Quick
And: report.durability.journal_written == false
And: report.exit_code == 0
And: report.repair_hints is empty
```

### YAML Parse Error Path

```
### Behavior: run_verification returns YamlParse error for malformed YAML
Given: a workflow YAML with syntax error (e.g., invalid indentation)
When: run_verification is called with Quick profile
Then: result is Err(VerifyError::YamlParse(msg)) where msg contains "YAML parse error"
And: msg does not contain a raw panic or stack frame

### Behavior: exit_code_for_error returns ValidationFailed for YamlParse
Given: VerifyError::YamlParse("bad yaml".to_string())
When: exit_code_for_error is called
Then: result == CliExitCode::ValidationFailed (exit code 1)

### Behavior: repair_hint_for_error returns non-empty hint for YamlParse
Given: VerifyError::YamlParse("syntax error".to_string()) and Quick profile
When: repair_hint_for_error is called
Then: result.len() >= 1
And: result[0].gate == "YamlParse"
And: result[0].hint is non-empty
```

### Compile Error Path

```
### Behavior: run_verification returns Compile error for invalid workflow structure
Given: a workflow YAML that fails schema validation
When: run_verification is called with any profile
Then: result is Err(VerifyError::Compile(errors)) where errors is non-empty

### Behavior: exit_code_for_error returns ValidationFailed for Compile
Given: VerifyError::Compile(vec!["error1".to_string()])
When: exit_code_for_error is called
Then: result == CliExitCode::ValidationFailed (exit code 1)

### Behavior: repair_hint_for_error returns hint citing compile pass for Compile error
Given: VerifyError::Compile(vec!["missing field".to_string()])
When: repair_hint_for_error is called
Then: result[0].gate == "Compile"
And: result[0].hint contains guidance to fix the compile error
```

### IR Validation Error Path

```
### Behavior: run_verification returns IrValidation error when IR validation fails
Given: a compiled workflow that fails IR validation
When: run_verification is called with Standard or Full profile
Then: result is Err(VerifyError::IrValidation(msg))

### Behavior: exit_code_for_error returns VerificationFailed for IrValidation
Given: VerifyError::IrValidation("IR invalid".to_string())
When: exit_code_for_error is called
Then: result == CliExitCode::VerificationFailed (exit code 2)

### Behavior: repair_hint_for_error cites IrValidation gate for IrValidation error
Given: VerifyError::IrValidation("validation failed".to_string())
When: repair_hint_for_error is called
Then: result[0].gate == "IrValidation"
And: result[0].hint cites the IrValidation gate specifically
```

### Budget Policy Error Path (Full Profile Only)

```
### Behavior: run_verification returns BudgetPolicy error at Full profile on boundedness violation
Given: a workflow that violates BoundednessPolicy at Full profile
When: run_verification is called with Full profile
Then: result is Err(VerifyError::BudgetPolicy(msg))
And: msg contains "budget policy violation"
And: exit_code_for_error(&result.unwrap_err()) == CliExitCode::VerificationFailed (exit code 2)

### Behavior: run_verification returns warnings at Standard profile for budget violations
Given: a workflow that would violate budget policy
When: run_verification is called with Standard profile
Then: result is Ok(VerifyOk) with warnings containing "budget policy warning"
And: result.checks contains "boundedness_policy_check"

### Behavior: exit_code_for_error returns VerificationFailed for BudgetPolicy
Given: VerifyError::BudgetPolicy("budget exceeded".to_string())
When: exit_code_for_error is called
Then: result == CliExitCode::VerificationFailed (exit code 2)

### Behavior: assemble_verification_report sets exit_code=2 on BudgetPolicy failure
Given: VerifyError::BudgetPolicy("budget exceeded".to_string()) and Full profile
When: assemble_verification_report is called with the error
Then: report.exit_code == 2
And: report.repair_hints is non-empty
And: report.repair_hints[0].gate == "BudgetPolicy"
```

### Storage Error Path

```
### Behavior: exit_code_for_error returns StorageError for StorageError variant
Given: VerifyError::StorageError("journal unavailable".to_string())
When: exit_code_for_error is called
Then: result == CliExitCode::StorageError (exit code 5)

### Behavior: repair_hint_for_error returns non-empty hint for StorageError
Given: VerifyError::StorageError("disk full".to_string())
When: repair_hint_for_error is called
Then: result[0].gate == "StorageError"
And: result[0].hint is non-empty
```

### Replay Divergence Path

```
### Behavior: exit_code_for_error returns ReplayDivergence for ReplayDivergence variant
Given: VerifyError::ReplayDivergence("ABI mismatch".to_string())
When: exit_code_for_error is called
Then: result == CliExitCode::ReplayDivergence (exit code 8)

### Behavior: repair_hint_for_error returns non-empty hint for ReplayDivergence
Given: VerifyError::ReplayDivergence("action ABI changed".to_string())
When: repair_hint_for_error is called
Then: result[0].gate == "ReplayDivergence"
And: result[0].hint is non-empty
```

### Format Parity

```
### Behavior: exit code is identical across Text and Json output formats
Given: a valid workflow at Quick profile
When: cmd_verify is called with Text format and with Json format
Then: both invocations return the same exit code (0)

### Behavior: failing gates appear in both text and JSON output
Given: an invalid workflow producing IrValidation error
When: cmd_verify is called with Text format and Json format
Then: text output mentions the failing gate name
And: JSON output contains the same gate name in the error field

### Behavior: JSON output is valid parseable JSON
Given: any verify invocation with --format json
When: the output is parsed with serde_json::from_str
Then: parsing succeeds without error
And: the resulting object contains all required certificate fields
```

### Invariant Scenarios

```
### Behavior: INV-001 — exit code is stable across format variants
Given: any VerifyError variant
When: exit_code_for_error is called any number of times
Then: result is always the same CliExitCode value
And: format (Text/Json/Jsonl) does not affect the exit code

### Behavior: INV-002 — human and machine output report identical failing gates
Given: any verify invocation that produces an error
When: the text output is inspected and the JSON output is inspected
Then: the set of failing gate names is identical in both

### Behavior: INV-003 — no panic surfaces to operator
Given: a workflow that triggers a panic in a downstream crate
When: cmd_verify is called
Then: the operator sees a clean error message
And: no raw panic message or stack trace is printed to stderr

### Behavior: INV-004 — JSON output contains all certificate fields without truncation
Given: a valid workflow verified with Full profile
When: assemble_verification_report is called and serialized to JSON
Then: the JSON object has all fields: profile, artifact, replay, durability, repair_hints, exit_code
And: no field value is truncated or omitted
```

### ReplayEvidence and DurabilityEvidence Scenarios

```
### Behavior: assemble_verification_report sets replay_safe=false when gates are incomplete
Given: a VerifyOk with checks=["yaml_parse"] (compilation gate did not run)
When: assemble_verification_report is called with Quick profile
Then: report.replay.replay_safe == false
And: report.replay.gates_passed == ["yaml_parse"]

### Behavior: assemble_verification_report sets journal_written=false for all verify invocations
Given: any VerifyOk result from any profile (Quick/Standard/Full)
When: assemble_verification_report is called
Then: report.durability.journal_written == false
And: report.durability.durable reflects the durability mode that was checked

### Behavior: INV-003 — no panic surfaces to operator (concrete scenario)
Given: a valid workflow YAML that triggers a panic in vb_compile or vb_validate internals
When: run_verification is called with Standard profile
Then: run_verification returns Err(VerifyError::Compile(msg)) or Err(VerifyError::IrValidation(msg))
And: the error message contains no raw panic text, no "thread 'tokio-runtime-worker'", and no stack frame addresses
And: the panic is not propagated as an unhandled exception to the caller

---

## 4. Proptest Invariants

### run_verification invariants

**Property**: For any valid workflow text and Quick/Standard/Full profile, `run_verification` either returns `Ok` with non-empty `digest_hex` and non-empty `checks`, or returns a classified `VerifyError`.

- **Valid workflow**: any YAML that parses without error through `vb_yaml::parse_workflow_source`
- **Strategy**: `proptest::string_regex` for well-formed YAML, filtered to valid workflow structure

**Property**: `run_verification` at Quick profile never returns `IrValidation` or `BudgetPolicy` errors.

- **Strategy**: `proptest` generates any workflow; call with `VerifyProfile::Quick`; assert result is never `Err(VerifyError::IrValidation(_))` or `Err(VerifyError::BudgetPolicy(_))`

**Property**: `run_verification` at Full profile with a workflow that passes all gates returns `Ok` with `checks` containing at least 5 entries: `yaml_parse`, `compilation`, `ir_validation`, `budget_computation`, `boundedness_policy`.

### exit_code_for_error invariants

**Property**: `exit_code_for_error` is a total function over all `VerifyError` variants; it never panics or returns an unexpected code.

- **Strategy**: Iterate all variants of `VerifyError` enum (YamlParse, Compile, IrValidation, BudgetPolicy, StorageError, ReplayDivergence); each must produce a defined `CliExitCode`.

**Property**: For `YamlParse` and `Compile`, exit code is always 1 (ValidationFailed). For `IrValidation` and `BudgetPolicy`, exit code is always 2 (VerificationFailed). For `StorageError`, exit code is always 5. For `ReplayDivergence`, exit code is always 8.

### repair_hint_for_error invariants

**Property**: `repair_hint_for_error` always returns a non-empty `Vec<RepairHint>` for every error variant.

- **Strategy**: Generate each `VerifyError` variant; assert `result.len() >= 1` and `result[0].gate` is the correct gate name.

**Property**: Repair hint `gate` field is never empty and matches the error variant name exactly.

**Property**: Repair hint `hint` field is never empty and does not contain raw stack frames or panic messages.

### assemble_verification_report invariants

**4 proptest invariants** — `proptest` generates 100 `VerifyOk`+`VerifyProfile`+`source_bytes` combos covering all three profiles and gate-count variations:

1. **Property**: All fields of `VerificationReport` are non-optional (no `Option` types that could be `None`).

2. **Property**: `artifact.source_digest_hex` and `artifact.ir_digest_hex` are valid lower-case hex strings of length 64.

3. **Property**: `replay.gate_sequence.len() == replay.gates_passed.len()` — one sequence entry per gate, in execution order.

4. **Property**: `exit_code` is always a valid `u8` in the range 0–8 matching a defined `CliExitCode` discriminant.

*Note*: The Kani harness `assemble_verification_report_kani` (Section 6) provides formal bounded proof of the same properties; proptest provides empirical coverage. Both layers are present; no waiver is needed.*

---

## 5. Fuzz Targets

### Target: `fuzz_run_verification_quick`

- **Input**: arbitrary `&[u8]` bytes representing a workflow YAML
- **Risk**: malformed YAML causes panic in downstream parser
- **Oracle**: `run_verification` must return `Result<VerifyOk, VerifyError>`, not panic; error messages must be non-empty human-readable strings
- **Corpus seeds**: 1000 seeds from `tests/fixtures/valid/`, `tests/fixtures/invalid/`, plus empty file, 1MiB file, deeply nested YAML (5000 levels), duplicate keys, binary bytes injected into YAML strings

### Target: `fuzz_run_verification_standard`

- **Input**: same as above
- **Oracle**: must return classified error or Ok; IR validation failures must produce `IrValidation` variant (not panic)

### Target: `fuzz_run_verification_full`

- **Input**: same as above
- **Oracle**: must return classified error or Ok; budget policy violations at Full profile must produce `BudgetPolicy` (not panic)

### Target: `fuzz_exit_code_for_error`

- **Input**: random `VerifyError` variant serialized as string, or raw bytes interpreted as error variant
- **Oracle**: must return a `CliExitCode` with discriminant 1, 2, 5, or 8 — never 0, 3, 4, 6, or 7

### Target: `fuzz_repair_hint_for_error`

- **Input**: random `VerifyError` variant + profile enum
- **Oracle**: must return non-empty `Vec<RepairHint>` with non-empty `gate` and `hint` fields

### Target: `fuzz_parse_workflow_source`

- **Input**: arbitrary `&[u8]` bytes fed directly to `vb_yaml::parse_workflow_source`
- **Risk**: hostile input (binary blobs, giant payloads, unicode injection) causes panics or memory explosion in the YAML parser
- **Oracle**: `parse_workflow_source` must return `Result<Workflow, YamlParseError>`, never panic; `Err` must carry a non-empty message
- **Corpus seeds**: empty bytes, 1-byte UTF-8, 10MB YAML document, deeply nested YAML (10 000 levels), duplicate keys, mixed tabs/spaces, shebang prefix (`#!/usr/bin/env yaml`), binary bytes (`\x00\xff\xfe`), JSON document presented as YAML, JSON5 comments in YAML, flow-style vs block-style variants
- **Waiver reference**: covered by ERR-002 (`cargo-fuzz run -p vb_yaml fuzz_workflow_parse -- -max_len=65536`) in `proof-obligations.jsonl`; this target provides additional boundary coverage at the `vb_yaml` crate surface

---

## 6. Kani Harnesses

### Harness: `assemble_verification_report_kani`

**Property to prove**: For any `VerifyOk` and `VerifyProfile`, all fields of `VerificationReport` are initialized:

```
assemble_verification_report(&ok, profile, &bytes)
  → report.profile == profile.as_str()
  → !report.artifact.source_digest_hex.is_empty()
  → !report.artifact.ir_digest_hex.is_empty()
  → report.artifact.node_count > 0
  → !report.replay.gates_passed.is_empty()
  → report.replay.gate_sequence.len() == report.replay.gates_passed.len()
  → matches!(report.exit_code, 0)
  → report.repair_hints.is_empty()
```

**Bound**: `node_count` bounded to `u16::MAX`; `gates_passed` bounded to 10 entries (max gates in any profile).

### Harness: `full_profile_fail_closed_kani`

**Property to prove**: When `run_verification` is called with `VerifyProfile::Full` and budget policy fails, the result is `Err(VerifyError::BudgetPolicy(_))` with `exit_code == 2`.

```
run_verification(text, bytes, VerifyProfile::Full)
  ∧ budget_violation_detected
  → is_err(result)
  ∧ matches!(err, VerifyError::BudgetPolicy(_))
  ∧ exit_code_for_error(&err) == CliExitCode::VerificationFailed
```

**Bound**: Profile is always Full; text is bounded to 1MiB.

### Harness: `budget_policy_error_kani`

**Property to prove**: `repair_hint_for_error(BudgetPolicy(msg), Full)` returns a non-empty hint citing "BudgetPolicy" gate.

```
repair_hint_for_error(&VerifyError::BudgetPolicy(msg), VerifyProfile::Full)
  → !result.is_empty()
  ∧ result[0].gate == "BudgetPolicy"
  ∧ !result[0].hint.is_empty()
```

### Harness: `verification_report_json_completeness_kani`

**Property to prove**: When a `VerificationReport` is serialized to JSON via `serde_json::to_string`, the resulting string is valid UTF-8 and parses back to a JSON object containing all required keys: `profile`, `artifact`, `replay`, `durability`, `repair_hints`, `exit_code`.

---

## 7. Mutation Testing Checkpoints

| Mutant | What Changes | Kill Condition |
|---|---|---|
| `exit_code_for_error` returns `Success` for `YamlParse` | `match` arm changed | `cargo mutants` detects YamlParse input no longer returns exit code 1 |
| `exit_code_for_error` returns `Success` for `Compile` | `match` arm changed | `cargo mutants` detects Compile input no longer returns exit code 1 |
| `exit_code_for_error` returns `Success` for `IrValidation` | `match` arm changed | `cargo mutants` detects IrValidation input no longer returns exit code 2 |
| `exit_code_for_error` returns `Success` for `BudgetPolicy` | `match` arm changed | `cargo mutants` detects BudgetPolicy input no longer returns exit code 2 |
| `exit_code_for_error` returns `Success` for `StorageError` | `match` arm changed | `cargo mutants` detects StorageError input no longer returns exit code 5 |
| `exit_code_for_error` returns `Success` for `ReplayDivergence` | `match` arm changed | `cargo mutants` detects ReplayDivergence input no longer returns exit code 8 |
| `repair_hint_for_error` returns `Vec::new()` for `YamlParse` | function body replaced | `cargo mutants` detects YamlParse error no longer produces hint |
| `repair_hint_for_error` returns `Vec::new()` for `Compile` | function body replaced | `cargo mutants` detects Compile error no longer produces hint |
| `repair_hint_for_error` returns `Vec::new()` for `IrValidation` | function body replaced | `cargo mutants` detects IrValidation error no longer produces hint |
| `repair_hint_for_error` returns `Vec::new()` for `BudgetPolicy` | function body replaced | `cargo mutants` detects BudgetPolicy error no longer produces hint |
| `repair_hint_for_error` returns `Vec::new()` for `StorageError` | function body replaced | `cargo mutants` detects StorageError no longer produces hint |
| `repair_hint_for_error` returns `Vec::new()` for `ReplayDivergence` | function body replaced | `cargo mutants` detects ReplayDivergence no longer produces hint |
| `assemble_verification_report` drops `exit_code` field | struct field removed | `cargo mutants` detects exit_code not being set |
| `assemble_verification_report` sets `journal_written = true` | constant changed | `cargo mutants` detects journal_written incorrectly true (verify is read-only) |
| `Quick` profile runs IR validation gates | profile check removed | unit test on Quick profile shows IR validation gates present |

**Mutation kill rate target**: ≥ 90% on `commands_verify.rs` via `cargo mutants --scope smoke`.

---

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|---|---|---|---|
| Happy: Quick profile | Valid minimal workflow YAML | Ok(VerifyOk) with yaml_parse+compilation checks | unit |
| Happy: Standard profile | Valid minimal workflow YAML | Ok(VerifyOk) with +ir_validation | unit |
| Happy: Full profile | Valid workflow passing budget | Ok(VerifyOk) with all 5 checks | unit |
| Err: YamlParse | Invalid YAML syntax | Err(YamlParse) + exit code 1 | unit + fuzz |
| Err: Compile | Invalid workflow structure | Err(Compile) + exit code 1 + errors array | unit + fuzz |
| Err: IrValidation at Standard | IR fails validation | Err(IrValidation) + exit code 2 | unit |
| Err: BudgetPolicy at Full | Boundedness violation | Err(BudgetPolicy) + exit code 2 + repair hint | unit + kani |
| Err: BudgetPolicy at Standard | Boundedness violation | Ok with warnings (not error) | unit |
| Err: StorageError | Storage unavailable | Err(StorageError) + exit code 5 | manual-qa |
| Err: ReplayDivergence | ABI mismatch | Err(ReplayDivergence) + exit code 8 | manual-qa |
| Format: Text output | Any valid workflow | Human-readable text with checks listed | integration |
| Format: JSON output | Any valid workflow | JSON with all certificate fields | integration |
| Format: JSONL output | Any valid workflow | One JSON object per line | integration |
| Invariant: Exit code parity | Any error + any format | Same exit code across Text/Json/Jsonl | unit + proptest |
| Invariant: Gate parity | Any error | Same failing gates in text and JSON | integration |
| Invariant: No panic | Workflow triggering downstream panic | Clean error message, exit code 2 | miri |
| Invariant: JSON completeness | Valid workflow at Full profile | JSON has profile, artifact, replay, durability, repair_hints, exit_code | kani |

---

## 9. Proof Obligations Mapping

All 20 proof obligations from `proof-obligations.jsonl` are addressed:

| ID | Layer | Test(s) |
|---|---|---|
| PRE-001 | waiver | Auth is trivially absent — no test needed |
| PRE-002 | proptest | `run_verification_produces_report_for_valid_workflow` — property test on all profile+workflow combinations |
| PRE-003 | miri | `cargo miri test -p velvet_ballastics run_verification` — symbol resolution UB check |
| POST-001 | kani | `kani --harness assemble_verification_report_kani` — bounded proof of report completeness |
| POST-001b | proptest | `verify_format_json_completeness` — JSON parses and has all fields |
| POST-002 | kani | `kani --harness full_profile_fail_closed_kani` — fail-closed bounded proof |
| POST-003 | proptest | `repair_hint_exhaustiveness` — every error variant produces non-empty hint |
| INV-001 | cargo-mutants | `cargo mutants --scope smoke exit_code_for_error` — exit code format-independent |
| INV-002 | proptest | `verify_format_parity` — same gates in text and JSON |
| INV-003 | miri | `cargo miri test -p velvet_ballastics run_verification_no_panic` — no panic propagation |
| INV-004 | kani | `kani --harness verification_report_json_completeness_kani` — all JSON fields present |
| ERR-001 | proptest | `verify_yaml_parse_error` — YamlParse → exit 1, non-empty message, both formats |
| ERR-002 | cargo-fuzz | `cargo fuzz run -p vb_yaml fuzz_workflow_parse` — malformed YAML produces classified error |
| ERR-003 | proptest | `verify_ir_validation_error` — IrValidation → exit 2, non-empty error |
| ERR-004 | kani | `kani --harness budget_policy_error_kani` — BudgetPolicy → exit 2, non-empty repair hint |
| ERR-005 | manual-qa | `moon run :qa-verify-storage-error` — StorageError → exit 5, both formats |
| ERR-006 | manual-qa | `moon run :qa-verify-replay-error` — ReplayDivergence → exit 8, both formats |
| ERR-007 | cargo-mutants | `cargo mutants --scope smoke exit_code_for_error` — exit code 2 only from IrValidation/BudgetPolicy/ReplayDivergence |
| GATE-001 | gauntlet-standard | `moon run :verify-standard` — full gauntlet-standard pass before release |
| GATE-002 | gauntlet-all | `moon run :verify-all` — full gauntlet-all as release gate |

---

## 10. Test File Locations

| Test Type | File |
|---|---|
| Unit: run_verification | `crates/velvet_ballastics/src/commands_verify.rs` (#[cfg(test)] module) |
| Unit: exit_code_for_error | `crates/velvet_ballastics/src/commands_verify.rs` (#[cfg(test)] module) |
| Unit: repair_hint_for_error | `crates/velvet_ballastics/src/commands_verify.rs` (#[cfg(test)] module) |
| Unit: assemble_verification_report | `crates/velvet_ballastics/src/commands_verify.rs` (#[cfg(test)] module) |
| Integration: format parity | `crates/velvet_ballastics/tests/cli_integration.rs` |
| Integration: JSON completeness | `crates/velvet_ballastics/tests/cli_integration.rs` |
| Integration: error path end-to-end | `crates/velvet_ballastics/tests/cli_integration.rs` |
| Proptest: run_verification properties | `crates/velvet_ballastics/src/commands_verify.rs` (proptest module) |
| Proptest: repair_hint exhaustiveness | `crates/velvet_ballastics/src/commands_verify.rs` (proptest module) |
| Proptest: exit_code invariants | `crates/velvet_ballastics/src/commands_verify.rs` (proptest module) |
| Fuzz: workflow parse | `fuzz/fuzz_run_verification.rs` (cargo-fuzz target) |
| Kani: report completeness | `kani/commands_verify.rs` (kani harness) |
| Kani: fail-closed | `kani/commands_verify.rs` (kani harness) |
| Kani: budget policy error | `kani/commands_verify.rs` (kani harness) |
| Mutation: exit_code_for_error | `crates/velvet_ballastics/` (cargo mutants) |

---

## 11. Exit Criteria

- [ ] Every `VerifyError` variant has a unit test demonstrating it produces the correct exit code and repair hint
- [ ] Every `VerifyError` variant has a proptest invariant confirming non-empty hints
- [ ] `assemble_verification_report` has 100% line coverage (LLVM-cov)
- [ ] `exit_code_for_error` achieves ≥90% mutation kill rate via cargo-mutants
- [ ] All BDD scenarios execute successfully on `cargo test`
- [ ] `cargo miri test` passes with zero UB on pure verification functions
- [ ] Kani harnesses compile and prove report completeness and fail-closed properties
- [ ] Fuzz targets compile and run (smoke scope) without panics
- [ ] Manual QA sign-off obtained for ERR-005 (StorageError) and ERR-006 (ReplayDivergence)
- [ ] `moon run :verify-standard` passes as GATE-001
- [ ] `moon run :verify-all` passes as GATE-002
