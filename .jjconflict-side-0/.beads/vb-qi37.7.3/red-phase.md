# RED PHASE STATUS: vb-qi37.7.3 — ir: Validate symbol, action, and resource references

**Bead ID:** vb-qi37.7.3
**Date:** 2026-05-09
**Workspace:** /home/lewis/src/Velvet-ballistics-femdation-p0p1-25

---

## STATUS: IMPLEMENTATION AND TESTS NOT YET WRITTEN

The specific public helper functions and exact test names specified in the approved test-plan.md **do not exist** in the codebase. The vb-qi37.7.3 tests as planned have NOT been written.

---

## 1. Contract Function Existence Check

| Contract Function | Status | Location |
|---|---|---|
| `validate_symbol_references(parts: &WorkflowParts) -> Result<(), WorkflowError>` | **MISSING** | Not found in vb_core or vb_validate |
| `validate_resource_references(parts: &WorkflowParts) -> Result<(), WorkflowError>` | **MISSING** | Not found in vb_core or vb_validate |
| `validate_action_references(parts: &WorkflowParts, action_contracts: &[ActionContract]) -> Result<(), ValidationError>` | **MISSING** | Not found in vb_core or vb_validate |
| `validate(parts: &WorkflowParts) -> Result<(), ValidationError>` | EXISTS | vb_validate::shared::validate |
| `validate_with_contracts(parts: &WorkflowParts, action_contracts: &[ActionContract]) -> Result<(), ValidationError>` | EXISTS | vb_validate::shared::validate_with_contracts |
| `CompiledWorkflow::try_from_parts(parts: WorkflowParts) -> Result<Self, WorkflowError>` | EXISTS | vb_core::workflow::CompiledWorkflow |

---

## 2. Test Existence Check

### Tests from test-plan.md Section 3 (BDD Scenarios)

None of the following exact test names exist:

**Symbol reference tests (Behaviors 1-7):**
- `validate_symbol_references_returns_unit_when_all_symbol_carriers_are_in_bounds` — **MISSING**
- `validate_symbol_references_returns_symbol_out_of_bounds_when_accessor_field_equals_symbols_count` — **MISSING**
- `validate_symbol_references_returns_symbol_out_of_bounds_when_symbol_constant_equals_symbols_count` — **MISSING**
- `validate_symbol_references_returns_symbol_out_of_bounds_when_build_object_field_equals_symbols_count` — **MISSING**
- `validate_symbol_references_rejects_accessor_field_when_symbols_count_is_zero` — **MISSING**
- `core_admission_returns_symbol_out_of_bounds_when_accessor_field_equals_symbols_count` — **MISSING**

**Resource reference tests (Behaviors 8-12):**
- `validate_resource_references_returns_unit_when_declared_resources_cover_actual_usage` — **MISSING**
- `validate_resource_references_returns_too_large_when_max_steps_exceeds_hard_limit` — **MISSING**
- `validate_resource_references_returns_exceeded_when_node_count_exceeds_max_steps` — **MISSING**
- (14 resource tests total specified in plan — all MISSING)

**Action reference tests (Behaviors 13-15):**
- `validate_action_references_returns_unit_when_do_actions_match_contract_ids` — **MISSING**
- `validate_action_references_returns_first_missing_contract_in_node_index_order` — **MISSING**
- `validate_action_references_returns_first_orphan_contract_in_supplied_order` — **MISSING**
- (6 action tests total specified in plan — all MISSING)

### Existing Related Tests (NOT vb-qi37.7.3 specific)

| Test | Location | Status |
|---|---|---|
| `phase46_rejects_accessor_field_symbol_out_of_bounds` | vb_core/src/workflow/tests.rs:2242 | PASSES |
| `phase46_accepts_accessor_field_symbol_at_boundary` | vb_core/src/workflow/tests.rs:2251 | PASSES |
| `phase46_rejects_accessor_field_symbol_zero_when_no_symbols` | vb_core/src/workflow/tests.rs:2289 | PASSES |
| `gate_08_accessor_parity.rs` (11 tests) | vb_validate/tests/ | PASSES (vb-qi37.7.4) |

---

## 3. Evidence Command Output

```bash
# Public helper function check — none found
$ rg "pub fn validate_symbol_references|pub fn validate_resource_references|pub fn validate_action_references" crates/vb_core crates/vb_validate
(no output — functions do not exist)

# Specific test name check — none found
$ rg "validate_symbol_references_returns_|validate_resource_references_returns_|validate_action_references_returns_" crates/
(no output — tests do not exist)

# Existing test status
$ cargo test -p vb_validate --test gate_08_accessor_parity
cargo test: 11 passed (1 suite, 0.00s)  # vb-qi37.7.4 accessor — PASSES

# Core admission symbol validation (via try_from_parts — NOT the planned public helpers)
$ cargo test -p vb_core phase46
cargo test: 32 passed, 1565 filtered out (9 suites, 0.00s)

# Overall test suite
$ cargo test -p vb_core -p vb_validate 2>&1 | grep "^test result"
test result: ok. 1323 passed; 0 failed
test result: ok. 5 passed; 0 failed
test result: FAILED. 95 passed; 2 failed  # These 2 failures are aggregate_budget Red, NOT vb-qi37.7.3
```

---

## 4. What EXISTS vs What is MISSING

### EXISTS (partial implementation):
- `CompiledWorkflow::try_from_parts` performs symbol validation internally
- `vb_validate::shared::validate` runs gate 8 (accessor paths) which validates accessor field symbols
- `vb_validate::shared::validate_with_contracts` includes gate 12 (action contracts)
- Some resource validation is performed by `try_from_parts`

### MISSING (requires implementation):
- Public `validate_symbol_references` function covering all three carriers (accessor fields, constants, build-object fields)
- Public `validate_resource_references` function with separate `ResourceContractTooLarge` and `ResourceContractExceeded` errors
- Public `validate_action_references` function with exact bijection checking
- All 36 unit tests and 28 integration tests specified in test-plan.md
- Verifier error variants `ValidationError::SymbolReferenceOutOfRange` with E050D code
- E050E, E050F diagnostic codes for resource errors

---

## 5. Required Actions to Complete RED PHASE

1. **Write the public helper functions** in vb_validate or vb_core:
   - `validate_symbol_references` — must check accessor fields, constants, build-object fields
   - `validate_resource_references` — must check all 6 resource members against hard limits and actual usage
   - `validate_action_references` — must check Do/action contract bijection

2. **Write the 36 unit tests** with exact names from test-plan.md

3. **Write the 28 integration tests** with exact names from test-plan.md

4. **Add diagnostic codes** E050D, E050E, E050F to ValidationError

5. **Verify tests FAIL before implementation** — once written, run `cargo nextest run` to confirm RED state

---

## 6. Files That Would Be Created/Modified

When vb-qi37.7.3 RED phase is entered:

- `crates/vb_validate/src/shared.rs` — add `validate_symbol_references`, `validate_resource_references`, `validate_action_references` public functions
- `crates/vb_validate/src/gates.rs` or new module — add internal gate implementations
- `crates/vb_validate/src/error.rs` — add `SymbolReferenceOutOfRange { symbol, source, source_index }` variant with E050D
- `crates/vb_validate/tests/vb_qi37_7_3_symbol_resource_action.rs` — integration tests
- `crates/vb_core/src/workflow/tests.rs` — add direct helper tests
- `tests/fixtures/vb_qi37_7_3/*.vbir` — CLI test fixtures

---

**RED PHASE NOT YET ENTERED**: The vb-qi37.7.3 tests as specified have not been written. The approved test-plan.md describes 36 unit tests + 28 integration tests that do not currently exist in the codebase.
