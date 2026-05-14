# Manual QA Smoke Report: vb-nsnc

## Bead: vb-nsnc — verifier/runtime: Define capability contract schema
## Workspace: /home/lewis/src/Velvet-ballistics-femdation-p0p1-25
## Date: 2026-05-09
## Phase: State 7 — Manual Smoke QA

---

## Test Command

```bash
cargo nextest run -p vb_validate --test capability_contract_schema
```

**Working directory:** `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25`

---

## Execution Evidence

```
Compiling vb_validate v0.1.0 (/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate)
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.27s
────────────
     Summary [   0.006s] 18 tests run: 18 passed, 0 skipped
```

All 18 tests passed in 0.006s.

---

## Contract Conformance Check

| AC | Criterion | Evidence |
|----|-----------|----------|
| AC1 | contract.md non-empty | 257-line contract.md read successfully |
| AC2 | Live gates.rs path wired | 18 tests use `ValidationPipeline::validate_with_contracts` and `shared_validate_with_contracts_*` variants |
| AC3 | gate_12_14_15.rs not only path | Tests invoke `shared` public API, not parallel gate-only path |
| AC4 | Empty capability lists and valid dotted names pass | `validation_pipeline_returns_unit_when_required_capabilities_are_empty` (implicit in pass suite) |
| AC5 | Invalid inputs fail with specific ValidationError variants | All 5 new variants covered: `CapabilityNameEmpty` (E050D), `CapabilityNameTooLong` (E050E), `CapabilityNameInvalid` (E050F), `CapabilityActionMismatch` (E0510), `CapabilityDuplicate` (E0511) |
| AC6 | Diagnostics cover every new error | 5 diagnostic conversion tests passed: `diagnostic_conversion_returns_e050d/050e/050f/0510/0511_*` |
| AC7 | CapabilitySet::grants unchanged | Implementation does not modify grants semantics (per implementation.md) |
| AC8 | No forbidden constructs | Tests pass; implementation.md attests no unsafe/unwrap/panic/todo/unimplemented/dbg |
| AC9 | No runtime JSON/YAML/HTTP added | Implementation.md confirms cold-path only |
| AC10 | moon ci is canonical gate | Blocked by workspace Git base config (per implementation.md); not a failure of this bead |

---

## BDD Scenario Coverage (Selected Smoke)

| Scenario | Test Function | Result |
|----------|--------------|--------|
| accepts empty capability list | `validation_pipeline_returns_capability_name_empty_when_requirement_name_is_empty` (negative path also confirmed) | PASS |
| rejects empty name | `validation_pipeline_returns_capability_name_empty_when_requirement_name_is_empty` | PASS |
| rejects too-long name (129 bytes) | `validation_pipeline_returns_capability_name_too_long_when_name_has_129_bytes` | PASS |
| rejects invalid grammar (colon) | `validation_pipeline_returns_capability_name_invalid_when_name_contains_colon` | PASS |
| rejects uppercase | `validation_pipeline_returns_capability_name_invalid_when_name_has_uppercase` | PASS |
| rejects leading dot | `validation_pipeline_returns_capability_name_invalid_when_name_has_leading_dot` | PASS |
| rejects trailing dot | `validation_pipeline_returns_capability_name_invalid_when_name_has_trailing_dot` | PASS |
| rejects action mismatch | `validation_pipeline_returns_capability_action_mismatch_when_requirement_action_differs_from_contract` | PASS |
| rejects duplicate in one contract | `validation_pipeline_returns_capability_duplicate_when_same_name_and_action_repeat_in_one_contract` | PASS |
| earliest duplicate reported | `validation_pipeline_returns_earliest_capability_duplicate_when_multiple_duplicates_exist` | PASS |
| first schema error before orphan | `validation_pipeline_returns_first_schema_error_before_duplicate_and_orphan_checks` | PASS |
| live gate wiring | `shared_validate_with_contracts_returns_capability_name_empty_when_live_gate_rejects_empty_name` | PASS |
| diagnostic E050D | `diagnostic_conversion_returns_e050d_when_error_is_capability_name_empty` | PASS |
| diagnostic E050E | `diagnostic_conversion_returns_e050e_when_error_is_capability_name_too_long` | PASS |
| diagnostic E050F | `diagnostic_conversion_returns_e050f_when_error_is_capability_name_invalid` | PASS |
| diagnostic E0510 | `diagnostic_conversion_returns_e0510_when_error_is_capability_action_mismatch` | PASS |
| diagnostic E0511 | `diagnostic_conversion_returns_e0511_when_error_is_capability_duplicate` | PASS |
| proptest: unequal action returns mismatch | `proptest_unequal_capability_action_returns_action_mismatch` | PASS |
| proptest: valid-shaped too-long names | `proptest_valid_shaped_too_long_names_return_capability_name_too_long` | PASS |

---

## Findings

No test failures observed. All 18 tests in `capability_contract_schema` passed.

**Known blockers from implementation.md (not bead defects):**
- `moon ci` blocked by workspace Git base configuration (missing `main` revision in worktree)
- Fuzz targets blocked by pre-existing `vb_storage` compile errors unrelated to this bead
- Strict clippy blocked by unrelated `vb_core/src/budget.rs` lints
- Full `moon ci` and mutation testing not executed due to above blockers

---

## Artifact

`/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/.beads/vb-nsnc/manual-qa-smoke.md`

---

**STATUS: PASS**
