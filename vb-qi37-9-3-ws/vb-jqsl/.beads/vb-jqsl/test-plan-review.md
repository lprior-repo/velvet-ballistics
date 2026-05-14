# Test Plan Review: vb-jqsl — Mode 1 Plan Inquisition

## STATUS: APPROVED

---

## LETHAL FINDINGS — ALL RESOLVED

### LETHAL-1 — `cmd_verify` in BDD scenarios but missing from contract signatures

**Resolution**: `cmd_verify` added to contract.md "Contract Signatures" section with full signature:

```rust
pub fn cmd_verify(
    workflow_path: &Path,
    profile: VerifyProfile,
    format: OutputFormat,
) -> CliExitCode;
```

Format Parity and INV-* BDD scenarios continue to reference `cmd_verify` at the integration/CLI layer, consistent with the contract now declaring it as the CLI boundary entry point.

### LETHAL-2 — Unit test count not verified against 5× density threshold

**Resolution**: Section 2 Trophy Allocation now explicitly states:

> **22 unit tests planned (≥ 20)** — 4 `pub(crate)` functions × 5 assertions each (happy path, each error variant, invariant/property check); `exit_code_for_error` alone has 6 variants × 2 assertions = 12 tests

Confirmed: 4 × 5 = 20 minimum required; 22 planned. Threshold satisfied.

---

## MAJOR FINDINGS — ALL RESOLVED

### MAJOR-1 — BudgetPolicy Full profile scenario missing `exit_code == 2` inline assertion

**Resolution**: BudgetPolicy Full scenario (test-plan.md line 187) now reads:

```
Then: result is Err(VerifyError::BudgetPolicy(msg))
And: msg contains "budget policy violation"
And: exit_code_for_error(&result.unwrap_err()) == CliExitCode::VerificationFailed (exit code 2)
```

Exit code is now asserted inline within the same scenario. Separate `exit_code_for_error` → 2 scenario (INV-001 coverage) remains as additional invariant documentation.

### MAJOR-2 — `assemble_verification_report` no explicit proptest invariants

**Resolution**: Section 4 now declares **4 named proptest invariants** with concrete count:

> **4 proptest invariants** — `proptest` generates 100 `VerifyOk`+`VerifyProfile`+`source_bytes` combos covering all three profiles and gate-count variations

Properties: (1) all fields non-optional, (2) hex strings of length 64, (3) gate_sequence.len == gates_passed.len, (4) exit_code in valid range. Note added referencing Kani harness as complementary formal proof layer; no waiver needed.

### MAJOR-3 — No explicit `parse_workflow_source` fuzz target

**Resolution**: New fuzz target `fuzz_parse_workflow_source` added to Section 5 with:

- Input: arbitrary `&[u8]` fed directly to `vb_yaml::parse_workflow_source`
- Oracle: must return `Result<Workflow, YamlParseError>`, never panic; `Err` must carry non-empty message
- Corpus: empty bytes, 1-byte UTF-8, 10MB YAML, 10 000-level nesting, duplicate keys, mixed tabs/spaces, shebang, binary bytes, JSON as YAML, JSON5/YAML mix, flow/block style variants
- Waiver reference: ERR-002 (`cargo-fuzz -p vb_yaml fuzz_workflow_parse`) cited as PRE-002 compensating evidence

---

## MINOR FINDINGS — ALL RESOLVED

### MINOR-1 — `replay_safe` false case not tested

**Resolution**: New BDD scenario added to "ReplayEvidence and DurabilityEvidence Scenarios":

```
### Behavior: assemble_verification_report sets replay_safe=false when gates are incomplete
Given: a VerifyOk with checks=["yaml_parse"] (compilation gate did not run)
When: assemble_verification_report is called with Quick profile
Then: report.replay.replay_safe == false
And: report.replay.gates_passed == ["yaml_parse"]
```

### MINOR-2 — `journal_written` never gets "should be false" confirmation

**Resolution**: New BDD scenario added:

```
### Behavior: assemble_verification_report sets journal_written=false for all verify invocations
Given: any VerifyOk result from any profile (Quick/Standard/Full)
When: assemble_verification_report is called
Then: report.durability.journal_written == false
And: report.durability.durable reflects the durability mode that was checked
```

### MINOR-3 — Strict durability evidence open question unresolved

**Resolution**: Contract.md open question #2 now marked **Resolved**:

> ~~Whether `strict` profile durability evidence requires journal existence proof or is inferred from `Strict` durability mode flag~~ — **Resolved**: `strict` profile durability evidence is the `Strict` flag itself; `journal_written == false` confirms verify is read-only and that no journal record was created. The durability mode flag is sufficient evidence; no journal existence proof is required for verify's static-analysis contract.

Proof obligations unchanged — no new waiver needed; POST-002 Kani harness already covers fail-closed on durability evidence.

---

## PROOF OBLIGATION COVERAGE AUDIT — RE-VERIFIED

All 20 proof obligations remain mapped. No new gaps introduced. New entry for `fuzz_parse_workflow_source` adds ERR-002b coverage at the `vb_yaml` surface boundary.

---

## MANDATE CHECKLIST

| # | Finding | Status |
|---|---|---|
| LETHAL-1 | `cmd_verify` added to contract signatures | ✅ FIXED |
| LETHAL-2 | Unit count ≥ 20 explicitly stated | ✅ FIXED |
| MAJOR-1 | BudgetPolicy exit_code==2 inline | ✅ FIXED |
| MAJOR-2 | `assemble_verification_report` proptest invariants | ✅ FIXED |
| MAJOR-3 | `fuzz_parse_workflow_source` target added | ✅ FIXED |
| MINOR-1 | `replay_safe == false` scenario added | ✅ FIXED |
| MINOR-2 | `journal_written == false` confirmation added | ✅ FIXED |
| MINOR-3 | Strict durability open question resolved | ✅ FIXED |

---

## SUMMARY

| Finding | Severity | Lines | Status |
|---|---|---|---|
| `cmd_verify` in BDD but not in contract signatures | LETHAL-1 | contract.md:65-74 | ✅ FIXED |
| Unit count not explicitly stated ≥ 20 | LETHAL-2 | test-plan.md:76 | ✅ FIXED |
| Vague `Then:` assertions (variant-only checks) | MAJOR-1 | test-plan.md:190 | ✅ FIXED |
| `assemble_verification_report` no explicit proptest invariants | MAJOR-2 | test-plan.md:348-360 | ✅ FIXED |
| No explicit `parse_workflow_source` fuzz target | MAJOR-3 | test-plan.md:393-408 | ✅ FIXED |
| `replay_safe` false case not tested | MINOR-1 | test-plan.md:292-297 | ✅ FIXED |
| `journal_written` no negative confirmation scenario | MINOR-2 | test-plan.md:299-303 | ✅ FIXED |
| Strict durability evidence open question unresolved | MINOR-3 | contract.md:23 | ✅ FIXED |

---

**APPROVED** — All 8 findings resolved. Plan is ready for implementation phase.

(End of file - total 120 lines)
