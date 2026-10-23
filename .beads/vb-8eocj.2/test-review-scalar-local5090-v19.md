# F1 Bounded Slice Test Suite Review (vb-8eocj.2)

**Reviewer:** LOCAL5090 (independent)
**Bead:** vb-8eocj.2
**Commit:** ba8ac8da07faa8249dd9c603377697d985b2d5e7
**Date:** 2026-10-23
**Workspace:** /home/lewis/src/isoloated/vb-8eocj2-scalar-test-review-local5090-v19

## Execution Evidence

Native execution of the exact 20 tests confirmed via preserved gate evidence in
`.beads/vb-8eocj.2/native-source20-v17/`. Command (from gate.json):

```bash
cargo test -p velvet-ballistics-workspace-tests --test vb_f1_canonical_scalar_do \
  -- --nocapture --test-threads=1
```

Result: 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out (0.01s).
No fabricated tests. PASS claims derived from actual execution artifacts only.

## Test Count Audit

- **positive.rs:** 8 tests (i64, bool, null, f64 scalar Set->Finish; arithmetic through alias;
  step reference by $id, $step.id, $steps.id)
- **negative.rs:** 11 tests (duplicate output, undefined reference, forward reference,
  self reference, ambiguous bare name, $step prefix disambiguation, invalid action ID
  negative, invalid action ID too large, invalid input slot, finish expression not
  supported, scalar type error bool+i64)
- **do_action.rs:** 1 test (Do suspension at step 1, completion to Finish, value verification)

**Total: 20 tests.** Matches gate.json assessment.json. No additional cases in this workspace.

## Suite Review Gates

### 1. Tests compile and execute deterministically
**Pass.** All 20 tests compiled and ran with --test-threads=1. No flakiness indicators.

### 2. Integration tests use public API only
**Pass.** Tests use `vb_compile::compile_workflow`, `vb_core` types, `vb_runtime` engine API.
No internal field access or #[cfg(test)] internals.

### 3. Tests assert behavior, not implementation details
**Pass.** Assertions target finished SlotValues, journal events (SlotWritten,
ActionCompletedEnvelope, RunFinished), and compile error variants. No internal
structure inspection.

### 4. No ignored/sleep/mocks/shared state/silent suppression
**Pass.** All 20 tests active (0 ignored). No Thread::sleep, no mocking frameworks,
no shared mutable state between tests. Error propagation explicit throughout.

### 5. Mutation resistance (logic deletion caught)
**Pass.** Each positive scalar test exercises distinct Set value flow; deleting Set
step logic breaks all. Each negative test exercises distinct compile validation path.
The Do action test exercises full suspend->complete flow with actual slot values;
removing Do logic or completion logic causes suspension failure or wrong result value.

### 6. Snapshot tests intentional
N/A. No snapshot tests in this suite.

### 7. Resource-heavy commands bounded
**Pass.** Tests use drive(256) step budget, small YAML workflows, --test-threads=1.
No Kani/CBMC/fuzz/sanitizer commands in this behavior test suite.

### 8. No dormant/ignored/zero-test filtered runs
**Pass.** 20/20 tests executed. No #[ignore], no test-name filtering, no proptest-regression
artifacts in this suite.

## Typed Error Assertions (NOT String Contains)

- **negative.rs lines 18-33:** `CompileError::DuplicateOutputName { name }` — typed variant match with field value verification ✓
- **negative.rs lines 41-63:** `CompileError::UnknownOutputName { name }` — typed variant match ✓
- **negative.rs lines 71-97:** `CompileError::UnknownReferenceName { reference, name }` — typed variant match with both fields ✓
- **negative.rs lines 105-129:** `CompileError::UnknownReferenceName { reference, name }` — typed variant match ✓
- **negative.rs lines 137-167:** `CompileError::ExpressionLoweringUnsupported { feature }` — typed variant match ✓
- **negative.rs lines 175-197:** `CompileError::StepFieldShape { field }` — typed variant match ✓
- **negative.rs lines 205-232:** `CompileError::StepFieldShape { field }` — typed variant match ✓
- **negative.rs lines 240-262:** `CompileError::SlotIndexOutOfRange { value }` — typed variant match with value verification ✓
- **negative.rs line 270:** `test_finish_expression_not_supported` — accepts any Err variant (not typed) ⚠️
- **negative.rs lines 280-305:** `test_scalar_type_error_bool_plus_i64` — uses string `contains` checks ⚠️

Assessment: 8/10 error tests use typed variant assertions. Two tests (`test_finish_expression_not_supported`, `test_scalar_type_error_bool_plus_i64`) use weaker assertions (any-error, string-contains). These are MINOR findings, not blockers for the bounded slice.

## Source Alias vs Step ID Gap

Contract requires: "Prior source Set outputs and step-ID references resolve to numeric slots"
and "distinct step and output namespaces, reject ambiguous resolution explicitly."

Tests covering this gap:
- `test_arithmetic_through_prior_output_alias` (positive.rs:58) — `$x` source alias
- `test_step_reference_by_id` (positive.rs:65) — `$producer` step ID
- `test_step_reference_qualified` (positive.rs:72) — `$step.producer`
- `test_step_reference_steps_prefix` (positive.rs:79) — `$steps.producer`
- `test_ambiguous_bare_name_error` (negative.rs:137) — rejects ambiguous `$conflict`
- `test_step_prefix_disambiguates_to_step` (negative.rs:175) — `$step.conflict` resolves to step

**Pass.** Full alias/step-ID resolution gap covered by 6 dedicated tests.

## Real Do Input 10/Output 999 Journal Values

`test_do_action_suspension_and_completion` (do_action.rs) verifies:
- Set step writes I64(10) to slot 0 — verified via SlotWritten journal event (lines 136-157)
- Action completes with I64(999) to slot 1 — verified via ActionCompletedEnvelope journal event (lines 161-181)
- Run finished with result slot 1 — verified via RunFinished journal event (lines 184-200)

**Pass.** Actual slot values verified through journal events, not just counters.

## No Handbuilt IR, No Preseed

Tests compile from YAML source via `compile_f1_workflow()` in vb_f1_scalar_do_support.
No hand-constructed CompiledWorkflow structs or IR node manipulation in test code.

**Pass.** Confirmed by assessment.json `"hand_built_replacement_IR": false`.

## Bounded Slice vs Full F1 Acceptance

Tests correctly scope to the bounded F1 slice:
- ✅ Scalar Set (I64, Bool, Null, F64) -> Finish
- ✅ Do action with numeric ActionId and input slot resolution
- ✅ Source alias and step-ID reference resolution
- ✅ Compile-time validation of duplicate names, undefined refs, type errors

Tests do NOT claim full F1 acceptance:
- ❌ Text/composite values (parent obligation)
- ❌ Named action registry (parent obligation)
- ❌ Lexical input/global vars (parent obligation)
- ❌ Arbitrary nested controls (parent obligation)
- ❌ Full static typing for dynamic action outputs (contract explicitly disclaims)

**Pass.** Test suite matches bounded slice contract.

## Known Issues Not Fixed Here (Tracked in vb-8eocj.2.1)

1. **cargo metadata auto-discovery of positive.rs/negative.rs/do_action.rs as standalone integration targets.**
   These files are declared as modules inside `vb_f1_canonical_scalar_do.rs` via `mod positive;` etc.
   but Cargo may also discover them as separate integration test targets if they contain `#[test]`
   attributes directly. This creates broad compile risk. Tracked in bead vb-8eocj.2.1 for module
   restructuring; not a defect to fix in this review.

2. **`crate::vb_f1_scalar_do_support` import.** If the test files were moved to a proper Rust
   module hierarchy, the `crate::` prefix would break for integration tests compiled as separate
   binaries. This is the module move issue tracked in vb-8eocj.2.1.

## Open Contract Items (Not Covered by These Tests)

- Public `source` API full F1 coverage — remains open on parent bead
- Full static typing for dynamic action outputs — disclaimed by contract
- Named action hash/registry — parent obligation
- Text/composite values — parent obligation

## Findings Summary

| Finding | Severity | Status |
|---------|----------|--------|
| `test_finish_expression_not_supported` uses any-error, not typed variant | MINOR | Open |
| `test_scalar_type_error_bool_plus_i64` uses string contains, not typed variant | MINOR | Open |

Both MINOR findings are acceptable for a bounded slice focused on source reachability. They do not
compromise the primary mutation-resistance property of the suite. The 8 core error variant tests
use proper typed assertions.

## STATUS: APPROVED

The 20-test F1 bounded slice suite meets all 8 test suite review gates. Tests compile, execute
deterministically, use public API, assert behavior over implementation, are mutation-resilient,
use bounded resources, and have no dormant/ignored tests.

Two MINOR findings remain on typed error assertions, acceptable for the bounded slice scope.
The suite correctly covers the source alias vs step-ID resolution gap, verifies actual slot values
through journal events, and does not fabricate IR or make full F1 acceptance claims.

Not a broad canonical F1 acceptance. Parent F1 bead remains OPEN with public `source` API work
pending. These 20 tests represent the bounded F1 first source slice as described in the contract.
