# vb-3l40a Admission Code Review

**CurrentSHA:** 1b0fe1f87c3a897f6873d02b524e4bfe84aa350c
**Baseline:** 61930db1055a84c1867769e88888e69682292459
**Parent verified:** Candidate commit has correct baseline parent.

## Scope
Independent review of workflow admission contract validation for vb-3l40a.
Focused on production runtime/shard/helper paths, API correctness, error
precedence, and engineering rules compliance. Test-only changes reviewed
separately in admission-test-suite-review.md.

## Production Code Changes

### crates/vb_runtime/src/shard/contract_validation.rs (NEW, 107 lines)
Three validation helper functions:
- `validate_contract_ids`: duplicate IDs, index==ID matching, non-empty names
- `validate_contract_capabilities`: granted vs required coverage
- `validate_workflow_action_binding`: every referenced Do action has contract

**Findings:**
- Uses `u16::try_from(index)` with proper error mapping for index conversion
- Uses `contracts.get(action_index)` for safe bounded access
- Bounded iteration over StepIdx values
- No unchecked indexing, casts, arithmetic, unwrap, expect, or panic
- Error precedence matches admission contract addendum

### crates/vb_runtime/src/shard/lifecycle/chunk_001.rs (62 lines changed)
- `handle_submit_with_inputs_grants_and_contracts`: calls validation BEFORE
  `handle_submit_with_inputs_contracts_and_header_mode` (no journal mutation)
- `handle_submit_with_grants_and_contracts`: same validation-before-enqueue pattern
- `validate_submit_admission_with_contracts`: refactored to delegate to helpers,
  passes `&workflow` (not digest), calls `build_admission` first

**Findings:**
- Both public with-contract ingress paths validate before enqueue
- No journal/header/state mutation on validation failure
- API change (digest -> workflow) is consistent across shard and runtime

### crates/vb_runtime/src/runtime.rs (4 lines changed)
- `submit_direct_with_grants_and_contracts`: passes `&workflow` instead of digest
- `submit_direct_with_inputs_grants_and_contracts`: same

**Findings:**
- Runtime API delegates validation to shard, which validates before enqueue
- Consistent with shard API change

### crates/vb_runtime/src/shard/mod.rs (1 line)
- Declares `pub mod contract_validation;`

## Contract Addendum Compliance

| Requirement | Status |
|-------------|--------|
| Validate action contract IDs against actual workflow Do actions | PASS |
| Dense registry model (index == ID) | PASS |
| Fabricated/sparse/reordered IDs rejected | PASS |
| Duplicate action IDs rejected | PASS |
| Missing contracts for referenced Do actions rejected | PASS |
| Extra contracts for unused actions allowed | PASS |
| Both Runtime APIs validate before enqueue | PASS |
| Both shard command handlers validate before journal/header mutation | PASS |
| No-contract behavior preserved | PASS |
| Error precedence: duplicate > index/name > capability > binding | PASS |

## Engineering Rules Compliance

| Rule | Status |
|------|--------|
| No unsafe | PASS |
| No unwrap/expect/panic (production) | PASS |
| No unchecked indexing/casts/arithmetic | PASS |
| Bounded iteration | PASS |
| Proper error propagation | PASS |

## Production Binding Verification
- Contract validation uses `CompiledWorkflow`, `ActionId`, `ActionContract`,
  `CompiledNodeKind::Do` — all production types from vb_core
- `validate_workflow_action_binding` iterates actual workflow nodes and
  inspects actual Do node action IDs
- Validation is bound to production behavior, not shadow types

## Test-Only Production Replacements (vb-8iilq)
The runtime_tests.rs changes replace some test submit paths with simpler
non-contract variants (`submit_compiled`, `submit_compiled_with_inputs`).
This is test fixture cleanup tracked as vb-8iilq, not a production behavior
change. It does not affect the admission validation logic under review.

## Disposition

STATUS: APPROVED

The production code changes are correct, bounded, and compliant with the
admission contract addendum and engineering rules. Both public ingress paths
validate before enqueue with no journal mutation on failure. The dense
registry model is correctly implemented with proper error precedence.

Test suite coverage and assertion quality reviewed separately in
admission-test-suite-review.md.
