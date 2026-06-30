reviewer_skill: test-reviewer
reviewer_invocation_id: test-reviewer-001
writer_invocation_id: test-writer-001
STATUS: APPROVED

# Test Plan Review: vb-zioy

**bead:** vb-zioy
**date:** 2026-05-25

---

## Summary

The test plan for vb-zioy is comprehensive, well-structured, and aligns with the contract. It identifies 12 behaviors, maps them to BDD Given/When/Then scenarios, allocates an appropriate testing trophy, and specifies exact assertions on error variant fields.

**STATUS: APPROVED** (plan quality)

---

## Gates

### 1. Every public behavior in contract.md has at least one Given/When/Then scenario

**PASS.** All 12 behaviors from the contract have explicit BDD scenarios in Section 3 of the test plan:
- Behaviors 1-3: `emit_single_body_set` direct unit tests (empty, multi-step, non-Set)
- Behaviors 4-8: Caller integration tests (for_each, collect, aggregate, repeat, parallel)
- Behaviors 9-12: Compile-time and existing-test-update scenarios

### 2. Every error variant has a scenario asserting the exact variant and fields

**PASS.** Both error variants (`StepFieldShape` and `UnsupportedStepPrimitive`) have exact field assertions planned:
- `StepFieldShape { step: diagnostic_step, field: "steps", expected: "exactly one set step" }`
- `UnsupportedStepPrimitive { step: diagnostic_step, primitive: "wait" }`

### 3. Assertions are concrete

**PASS.** No `is_ok()`, `is_err()`, `Some(_)`, or boolean smoke assertions are planned. All scenarios specify exact tuple matching on variant fields.

### 4. Boundary cases are named

**PASS.** Boundary cases explicitly identified:
- Empty body (`body: &[]`)
- Multi-step body (2+ steps)
- Single non-Set body (1 step, wrong primitive)
- Single valid Set body (success path)
- Step index boundaries (`StepIdx::MAX` overflow in related helpers)

### 5. Non-trivial pure behavior has property tests planned

**PASS.** Four proptest invariants are planned in Section 4, covering empty body, multi-step body, non-Set body, and empty body parity.

### 6. Parser/codec/hostile input has fuzz or adversarial input tests planned

**N/A (waived).** The test plan correctly waives fuzz targets per the delivery scope and verifier-lane-decisions. This bead makes no parsing or codec boundary changes.

### 7. Verifier harnesses do not count as behavior tests

**PASS.** Kani is explicitly waived. Proptest is planned as property-based behavior tests, not proof harnesses.

---

## Findings

### Finding TPR-001: Minor — Parallel branch diagnostic step ambiguity

The test plan assumes `branch_index` as `diagnostic_step` for `emit_together_branches` (per proof-to-implementation-input.md), but the contract notes this as a deferred design decision under Hazard H3. The plan should have included an explicit assertion comment clarifying whether the expected step is the parent `parallel` step index or the branch ordinal. This ambiguity does not block plan approval but should be resolved during suite implementation.

**Severity:** Info

### Finding TPR-002: Minor — Proptest module linking open question unresolved

The test plan lists an open question (Section 10, Q3) about whether proptest files need `mod` declarations in `lib.rs`. The plan does not make a decision — it only notes the issue. For a complete plan, this should have been resolved with a specific action (e.g., "Link under `#[cfg(test)]` in `lib.rs`" or "Move tests into existing `tests.rs`").

**Severity:** Info

---

## Traceability

| Contract Requirement | Plan Section | Status |
|---|---|---|
| REQ-001: `emit_single_body_set` reports source AST step in `StepFieldShape` | Section 3, scenarios 1-2 | Covered |
| REQ-002: `emit_single_body_set` reports source AST step in `UnsupportedStepPrimitive` | Section 3, scenario 3 | Covered |
| REQ-003: Signature accepts `diagnostic_step` separate from compiled node id | Section 9, compile-time gate | Covered |
| REQ-004: `lower_canonical_collect` passes original source index | Section 3, scenarios 5, 11-12 | Covered |
| REQ-005: All scoped primitive lowering functions report correct source step | Section 3, scenarios 4-8 | Covered |

---

## Conclusion

The test plan is thorough, traceable, and correctly identifies all behaviors, boundary conditions, and mutation checkpoints. Minor ambiguities around parallel branch semantics and proptest linking are noted but do not block approval.

**STATUS: APPROVED**
