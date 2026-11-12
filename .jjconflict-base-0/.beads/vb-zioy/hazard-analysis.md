# Hazard Analysis

## H1: Wrong Diagnostic Step Index (PRIMARY)

**Description**: `emit_single_body_set` reports `id.as_usize()` (the synthetic body step) instead of the original source step index.

**Impact**: Users see a step number that does not exist in their YAML. They cannot locate the offending step.

**Trigger**: Any scoped primitive (collect, for_each, aggregate, repeat, parallel branch) with `body.len() != 1` or a non-`Set` body primitive.

**Mitigation in Contract**: Require `emit_single_body_set` to accept a `diagnostic_step: usize` parameter. All error construction inside the function must use this parameter.

**Residual Risk**: Callers might still pass the wrong `diagnostic_step`. Need compile-time enforcement or review.

## H2: Inconsistent Diagnostic Index Across Scoped Primitives

**Description**: Each scoped primitive caller (collect, for_each, aggregate, repeat, parallel) must pass the correct source index. If one caller is fixed and another is not, the user experience becomes inconsistent.

**Impact**: Some primitives report correct indices, others report wrong indices. Unpredictable behavior.

**Mitigation**: Fix all callers simultaneously. The shared `emit_single_body_set` contract change forces all callers to provide a diagnostic index.

## H3: Parallel Branch Ambiguity

**Description**: `lower_canonical_parallel` calls `emit_single_body_set` for each branch with `entry` as the synthetic id and `branch_index` as the available source index. The diagnostic index should probably be the branch's step index within the parallel body, not the parallel primitive's own index.

**Impact**: Even after the fix, the "correct" diagnostic index for parallel branch body errors may be ambiguous.

**Mitigation**: Document that parallel branch errors report the branch's step index (if available) or the parallel primitive's index with a qualified field name.

## H4: Index Namespace Confusion

**Description**: `usize` is used for both source step indices and derived values (e.g., `id.as_usize()` where `id` is `StepIdx`). This creates ambiguity about which namespace a `usize` belongs to.

**Impact**: Future developers may inadvertently use `id.as_usize()` for diagnostics.

**Mitigation**: The contract proposes adding `diagnostic_step: usize` as an explicit parameter, making the intent clear at the type signature level. A stronger mitigation would be a newtype `SourceStepIdx(usize)` but that is outside this bead's scope.

## H5: Regression in Error Message Format

**Description**: Changing `emit_single_body_set` signature requires updating all call sites. A typo or oversight could change the `field` or `expected` strings in error messages.

**Impact**: User-facing error text changes unexpectedly.

**Mitigation**: Snapshot tests for error messages (if any) must be reviewed. The contract requires no string changes except the `step` value.

## H6: Proptest / Kani Harness Drift

**Description**: Existing property tests and Kani harnesses call `emit_single_body_set` with the current signature. Adding a parameter breaks them.

**Impact**: Compilation failures in test code.

**Mitigation**: Update all test call sites as part of the implementation. The `delivery-scope.jsonl` lists scoped test files.

## Temporal Hazards

None. This is a single-phase, deterministic compilation step with no async, concurrency, or state machine transitions.

## Bounded State Hazards

- `body.len()` is bounded by YAML sequence limits (`SequenceLimit`)
- `StepIdx` is bounded by `u16::MAX`
- No unbounded recursion in lowering

## Performance Hazards

None. The fix is a parameter pass and integer usage change; no algorithmic change.
