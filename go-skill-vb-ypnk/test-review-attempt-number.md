# Test Plan Review: MAJOR-5 — `$attempt.number` Restriction

**Mode**: 1 — Plan Inquisition
**Reviewer**: test-reviewer agent
**Date**: 2026-05-17

---

## VERDICT: REJECTED

---

## Axis 1 — Contract Parity

**MISSING**: No `contract.md` found in repository root or bead directory for MAJOR-5.
Mode 1 requires `contract.md` + `test-plan.md` to exist in same directory. Without
contract, cannot verify every pub fn has ≥1 BDD scenario. Found the test plan at
`/home/lewis/src/velvet-ballistics/test-plan-attempt-number.md` but no corresponding
`contract.md`.

**LETHAL**: See missing contract issue.

---

## Axis 2 — Assertion Sharpness

### LETHAL FINDINGS

**Scenario 1, 2, 3, 4 (B1 — Happy Path)**
- All say: "The compilation succeeds with no errors" or "The compilation succeeds"
- This is `is_ok()` as sole assertion — **LETHAL** per Axis 2
- Contract guarantee (test plan line 27-28): "The reference is retained in the AST as
  `AstExpression::Reference("$attempt.number")`" — **NOT ASSERTED**
- Contract guarantee (test plan line 28): "The reference is NOT resolved at compile time
  (runtime binding only)" — **NOT ASSERTED**

**Scenario 8 (for_each body)**
- Test name: `attempt_number_in_for_each_body_rejected`
- YAML (lines 274-289): No body step using `$attempt.number` exists
- `for_each` YAML shows `items:` and `do:` but NO body step referencing `$attempt.number`
- Note at line 291 admits: "(Note: The actual test would need a body step...)"
- **The YAML does not trigger the error under test** — **LETHAL**

**Scenario 12 (wait body)**
- **No YAML example provided** (lines 374-380)
- Only a note: "wait doesn't have a body per se, but the error should occur..."
- A scenario without an actual input YAML is not a test — **LETHAL**

### MAJOR FINDINGS

**Scenario 4**: "The compilation succeeds" — no AST content assertion (references retained,
not resolved)

---

## Axis 3 — Trophy Allocation

**LETHAL**: Unit test count < 5× public function count

Counting pub fns from the plan:
- `vb_compile::YamlCompiler::parse_ast()` → `Result<WorkflowAst, CompileErrors>`
- `validate_workflow_ast(ast: &WorkflowAst) -> Result<(), CompileErrors>` (proptest fn)
- `crate::expression::parse_expression()` (fuzz target)

Minimum 3 pub fns requiring coverage.

Unit tests listed in matrix (lines 470-480): 7 scenarios
7 / 3 = 2.33× — **below 5× threshold**

**LETHAL**: Pure function with no proptest invariant
- `validate_attempt_reference()` (lines 591-600 in implementation hints) is a critical pure
  function handling scope context
- Not referenced by any named proptest invariant
- The proptest invariant (lines 386-404) names `validate_workflow_ast`, not the internal
  scope-tracking function

---

## Axis 4 — Boundary Completeness

**MAJOR** (≥3 missing boundaries on one function):

For `parse_ast()` — no explicit boundaries for:
- `repeat.max_attempts`: minimum valid, maximum valid, 0, overflow
- Expression depth for nested `$attempt.number` references
- AST size limits with `$attempt.number`

For scope context tracking:
- Empty step list inside repeat
- Repeat with zero steps
- Maximum nesting depth (e.g., 10 levels) — plan shows 2 levels only

Missing boundaries per function: 3+ = MAJOR

---

## Axis 5 — Mutation Survivability

**MAJOR** per uncaught mutation:

| Mutation | Catching test? |
|----------|----------------|
| Change `>` to `>=` in `condition: $attempt.number > 1` | None — B1 scenarios don't assert AST content |
| Return `Ok(Default::default())` instead of real AST | None — B1 only asserts `is_ok()` |
| Skip `$attempt` check entirely | Would be caught by B2 scenarios ✓ |
| Accept bare `$attempt` | Named but no explicit scenario (Q2 says it should) |
| `is_valid_attempt_reference` always `true` | Would be caught by B2 ✓ |
| `is_valid_attempt_reference` always `false` | Would be caught by B1 ✓ |

B1 scenarios cannot detect "return Ok(Default::default())" mutation because they only
assert `is_ok()`, not AST content.

---

## Axis 6 — Evidence Plan Audit

**MINOR**: Scenario 11 YAML (lines 353-372) shows `reduce.body` with inline steps using
comment "# Invalid in reduce body" — need to verify `reduce.body` syntax supports inline
steps in the grammar.

**MINOR**: Proptest strategy (lines 392-395) says "Generate any valid WorkflowAst" — this
will NOT generate invalid ASTs with `$attempt.number` outside repeat. The anti-invariant
("Any AST where `$attempt.number` appears without a Repeat ancestor is invalid") requires
INVALID inputs to be tested. Strategy is circular — valid inputs can't prove invalid
cases are rejected.

---

## Additional MAJOR Findings

**Missing BDD scenarios**:
- `$attempt.number` in `together` body — listed in matrix (line 493) but no Scenario 13
- `$attempt.number` in `collect` body — mentioned in Q5 but no scenario
- Bare `$attempt` reference — mentioned in Q2 but no explicit scenario
- `$attempt.number.extra` accessor — mentioned in Q3 but only in matrix (line 480), not
  as a named BDD scenario

**Error contract mismatch**:
- Error definition (lines 509-517) has 3 fields: `reference`, `context`, `valid_context`
- BDD Then clauses only assert `reference` and `context`
- If `valid_context` is part of the error struct, it should be asserted

---

## Summary of LETHAL Findings

| # | Location | Finding |
|---|----------|---------|
| 1 | Axis 1 | No `contract.md` exists — cannot verify contract parity |
| 2 | Axis 2, Scenarios 1-4 | B1 assertions are `is_ok()` level — no AST content verification |
| 3 | Axis 2, Scenario 8 | YAML does not contain `$attempt.number` in `for_each` body — test is hollow |
| 4 | Axis 2, Scenario 12 | No YAML provided — scenario is placeholder text |
| 5 | Axis 3 | Unit test ratio 2.3× < 5× minimum |
| 6 | Axis 3 | `validate_attempt_reference` pure function has no proptest invariant |

---

## Mandatory Rewrite Checklist

Before resubmission:

1. **Add `contract.md`** for MAJOR-5 in same directory as `test-plan-attempt-number.md`
2. **Fix B1 Scenario Then clauses** — must assert:
   - `Ok(WorkflowAst)` with `$attempt.number` reference retained as `AstExpression::Reference`
   - Not just `is_ok()`
3. **Fix Scenario 8 YAML** — add a body step inside `for_each` that uses `$attempt.number`
4. **Add Scenario 13 for `wait`** — provide actual YAML or remove if not applicable
5. **Add missing scenarios**: bare `$attempt`, `$attempt.number.extra`, `together` body,
   `collect` body
6. **Fix proptest strategy** — must generate BOTH valid and invalid `$attempt.number`
   placements
7. **Increase unit test count** to achieve ≥5× pub fn coverage
8. **Verify `reduce.body` YAML syntax** is valid for the grammar
9. **Align error assertions** with all fields in `CompileError::InvalidVariableScope`

---

## Status: REJECTED

This plan cannot be approved. Multiple LETHAL findings indicate the test writer has not
verified the assertions match the contract guarantees. The B1 happy-path tests only
prove "compilation doesn't crash" — they do not prove the reference is correctly
retained in the AST as the contract requires.

Resubmit only after all LETHAL findings are resolved.
