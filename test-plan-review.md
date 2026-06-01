# Test Plan Review — vb-xi2f.24 State 10

**Review mode**: Plan review (contract.md + test-plan.md)
**Reviewer**: test-reviewer
**Date**: 2026-06-01
**Status**: APPROVED WITH FINDINGS

## Summary

The test plan for vb-xi2f.24 (Reduce Multi-Step Body Lowering) is comprehensive and well-structured. It identifies 48 behaviors across 12 contract clauses, allocates trophies across unit/integration/e2e/static/proptest layers with clear rationale, and provides detailed BDD scenarios with exact assertions and boundary cases. The plan correctly separates behavior tests from proof harnesses (Kani/Flux/proptest/fuzz are verification-lane, not test-lane).

Three findings below require attention before bead closure; none are lethal to the plan's overall correctness.

---

## Gate Results

| Gate | Result | Details |
|------|--------|---------|
| 1. Every public behavior has BDD scenario | APPROVED WITH FINDINGS | 48 behaviors mapped. B07 has placeholder calculation. C8 (nested reduce) scenarios defined in plan but not executable until Phase 3. |
| 2. Every error variant has scenario with exact variant+fields | APPROVED | B08-B10 (UnsupportedStepPrimitive), B44-B47 (StepFieldShape, diagnostic codes), B46 (StepIndexOutOfRange), B54-B56 (empty body) all specify exact variant destructuring with field values. |
| 3. Assertions are concrete | APPROVED | All BDD scenarios specify exact values (`Ok(3)`, `Err(UnsupportedStepPrimitive { primitive: "finish" })`, `StepIdx::new(13)`). No `is_ok()`/`is_err()`/`Some(_)` in any planned scenario. |
| 4. Boundary cases named | APPROVED | Empty body (B54-B56), u16::MAX (B46 plan, combinatorial table), zero overhead, usize::MAX overflow, ForEach width advancement, deeply nested body — all named with exact input/output. |
| 5. Non-trivial pure behavior has property tests planned | APPROVED | 13 proptest invariants planned (PO-001 through PO-013) covering width-node parity, offset monotonicity, chain integrity, single-step regression, ForEach width, nested next, overflow, empty body, no-panic, diagnostic codes, digest determinism, try_from_parts, collision safety. |
| 6. Fuzz/adversarial input tests planned | APPROVED | 2 fuzz targets planned (FZ-001 reduce lowering panic, FZ-002 diagnostic codes) with corpus seeds and status BLOCKED_TOOLING. Kani+proptest compensating coverage documented. |
| 7. Verifier harnesses not counted as behavior tests | APPROVED | Kani harnesses listed in Section 6 explicitly marked as verification-lane, not test-lane. Proptest marked "verification artifacts, not behavior tests" in trophy allocation. |
| 8. Proof-to-implementation rows covered by executable behavior tests | APPROVED WITH FINDINGS | 32 proof obligations mapped. 13 proptest invariants each have a corresponding BDD scenario. Pending: C8 nested reduce scenarios are plan-only (not yet executable) — see finding F-002. |

---

## Findings

### F-001 (MEDIUM): Contract C1 scope ambiguity

**Severity**: MEDIUM
**Rule**: Contract parity — plan must cover all contract clauses.
**Affected**: Contract C1 / test plan behaviors B01-B11

**Details**:
Contract C1 states: `canonical_body_step_width() shall accept Reduce, Collect, Together, Repeat, and Choose variants, returning the total width`. However, Section 5 (Out-of-Scope) states: "Multi-step body support for ForEach, Together, Collect, Repeat (separate beads)."

The test plan includes:
- B01-B03, B08-B10: Set, Do, ForEach, Finish, Wait, Ask — covered
- B11: Reduce width — covered
- Collect, Together, Repeat, Choose: listed in combinatorial table as error cases (UnsupportedStepPrimitive)

**Impact**: If Collect/Together/Repeat/Choose acceptance IS in-scope for this bead, the test plan is missing happy-path width scenarios for these primitives. If they ARE out-of-scope, the contract C1 should be narrowed to match the out-of-scope section.

**Recommendation**: Clarify with the product owner whether C1's "shall accept" is aspirational (future beads) or binding (this bead). If binding, add BDD scenarios for Collect/Together/Repeat/Choose width calculation to the test plan Section 3 (C1). If aspirational, update contract C1 to scope to `{Set, Do, ForEach, Reduce}` only.

### F-002 (MEDIUM): C7/C8 scenarios not executable until Phase 3

**Severity**: MEDIUM
**Rule**: Plan must map to executable test phases.
**Affected**: B35-B38 (single-step equivalence), B39-B43 (nested reduce semantics)

**Details**:
C7 (Single-Step Body Compatibility) and C8 (Nested Reduce Semantics) are defined in the test plan with full BDD scenarios. However, these scenarios require `emit_reduce_body_steps` to exist — which is the implementation scope of this bead and does not exist yet.

The test plan implementation sequencing (Section 10) correctly places these in Phase 2 (FAIL initially, pass after implementation) for B12-B16, B17-B22, B23-B28, B35-B38, B39-B43, B54-B56, and Phase 3 (proptest) for verification harnesses.

**Impact**: These are the core behaviors of the bead. The scenarios are correctly planned but cannot be executed in the current state. This is normal for a TDD workflow.

**Recommendation**: No action needed on the plan — the sequencing is correct. The test-suite review will track whether Phase 3 unblocking is complete.

### F-003 (LOW): B07 placeholder calculation

**Severity**: LOW
**Rule**: Concrete assertions over placeholder values.
**Affected**: B07 (nested Reduce in body width)

**Details**:
BDD scenario for B07 contains: `Then: returns Ok(8) // outer overhead=3 + nested: body_width([Reduce{body:[Set]}], 3) = 3+body_width([Set],0)+body_width([],3)+? → need exact compute`

The `?` placeholder and tentative value `8` indicate the expected width for nested reduce was not fully calculated during test planning.

**Impact**: The test-writer must compute the correct value before writing the test. If the contract is wrong about the expected value, the test would encode the wrong assertion.

**Recommendation**: Compute the exact expected value: `canonical_body_step_width(Reduce{body:[Set]})` = overhead 3 (ReduceStart+ReduceNext+ReduceFinish) + body_width([Set], 0) = 3 + 1 = 4. So outer Reduce with body=[nested Reduce{body:[Set]}] has width = body_width([Reduce{body:[Set]}], 3) = 3 (overhead) + 4 (nested reduce width) = 7. NOT 8. Correct this in the BDD scenario.

---

## Plan Quality Assessment

| Attribute | Score | Notes |
|-----------|-------|-------|
| Behavior coverage | 48/48 behaviors | All contract clauses mapped |
| Trophy allocation | Well-reasoned | 18 unit / 20 integration / 3 e2e / 3 static / 4 proptest |
| BDD scenario precision | Excellent | Exact values, exact error variants, exact field assertions |
| Boundary naming | Comprehensive | Empty body, u16::MAX, zero overhead, usize::MAX overflow, ForEach width advancement |
| Mutation checkpoint detail | Excellent | 17 mutation targets with named tests that catch them |
| Combinatorial matrix | Comprehensive | 3 tables covering unit width, error diagnostics, integration multi-step, e2e |
| Implementation sequencing | Correct | Phase 1 (pass now), Phase 2 (TDD red → green), Phase 3 (proptest verification) |
| Contract parity | Gap (C1) | See F-001 |

---

## Verdict

The test plan is structurally sound, covers all contract clauses (with the C1 scope ambiguity noted in F-001), and provides detailed, concrete BDD scenarios for all 48 behaviors. The trophy allocation, mutation checkpoints, and combinatorial matrix are industry-grade. The plan correctly sequences tests in phases aligned with implementation readiness.

**STATUS: APPROVED WITH FINDINGS** — 3 findings (1 MEDIUM for C1 scope, 1 MEDIUM for C7/C8 phasing, 1 LOW for B07 calculation). None block test writing or execution. F-001 should be resolved via contract clarification or plan amendment before bead closure.
