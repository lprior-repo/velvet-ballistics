# Round 4 Agent A9 — $attempt.number Scope Restriction (CRITICAL)

**Reviewer:** black-hat-reviewer · **Severity: 92/100 · SHIP-BLOCKER**

## Confirmed Gap

| # | Claim | Verified? |
|---|-------|-----------|
| 1 | `mod restrictions;` is NOT declared in `vb_compile/src/lib.rs` | **YES** (lib.rs:14-26 lists 13 modules; restrictions absent) |
| 2 | `crates/vb_compile/src/restrictions.rs` is a 10-line stub | **YES** |
| 3 | The 19 tests in `restrictions/tests/attempt_number_tests.rs` are dead code | **YES** |

## Production Validator Behavior

`vb_validate::references::validate_rooted_reference` (lines 145-162) does not contain `attempt` in its root match arms. For `$attempt.number`:
- root = `"attempt"` → falls to `_` arm → `ValidationError::UnknownReference { reference: "$attempt.number" }`
- `vb_compile::references::map_validation_error` (lines 316-352) maps to `CompileError::UnknownReferenceRoot { reference: "$attempt.number", root: "attempt" }`

**Production diagnostic: `UnknownReferenceRoot` — NOT `IllegalReference` and NOT `InvalidVariableScope` (which doesn't even exist in the enum).**

## Structural Finding: Cold AST StepKindAst::Repeat is a leaf with no body

`crates/vb_compile/src/ast/types.rs:173`:
```rust
Repeat { max_attempts: u16 },
```

`crates/vb_compile/src/ast/parse.rs:381-385`:
```rust
fn parse_repeat(body: &Yaml<'_>, index: usize) -> Result<StepKindAst, CompileError> {
    Ok(StepKindAst::Repeat {
        max_attempts: parse_u16_field(body, index, "max_attempts")?,
    })
}
```

The cold parser:
1. **Reads ONLY `max_attempts`**
2. **Never calls `parse_body_steps`**
3. **Silently drops the `steps:` body content of every `repeat:` in user YAML**

**There is literally no place in the cold AST where body steps can be stored.**

## Two AST types have diverged

| Type | Location | `Repeat` shape |
|------|----------|----------------|
| **Legacy** | `vb_yaml::ast::StepPrimitive::Repeat` (`types.rs:298-303`) | `{ max_attempts: u16, body: Vec<StepAst> }` ← **has body** |
| **Cold (new)** | `vb_compile::ast::StepKindAst::Repeat` (`types.rs:173`) | `{ max_attempts: u16 }` ← **NO body** |

## Worst-Consequence Workflow Examples

### A. The user writes a CORRECT workflow (B1 path) — SILENTLY DROPS THE BODY

```yaml
version: velvet-ballistics/v1
name: retry_with_attempt_logging
when: { manual: {} }
steps:
  - id: api_retry
    repeat:
      max_attempts: 3
      steps:
        - id: log_attempt
          save:
            current_attempt: $attempt.number
  - id: done
    finish: { result: 0 }
```

**Production behavior:**
1. `parse_repeat` reads only `max_attempts: 3` → builds `StepKindAst::Repeat { max_attempts: 3 }`
2. The `steps: [log_attempt]` body is **silently discarded at parse time** — no warning, no error
3. `validate_workflow_ast` matches `StepKindAst::Repeat { .. } => {}` — sees no body, validates nothing
4. **Workflow compiles without error and without the body**
5. Runtime executes an empty `repeat` 3 times — `log_attempt` step **never runs**
6. The user's `$attempt.number` reference never exists

The user gets a "successful" compile that does nothing useful. **This is a silent semantic loss of a step the user explicitly wrote.**

### B. The user writes an INCORRECT workflow (B2 path) — MISLEADING DIAGNOSTIC

`$attempt.number` in `vars:` produces `UnknownReferenceRoot { reference: "$attempt.number", root: "attempt" }`. The user sees: *"unknown reference root in $attempt.number: attempt"*. The error message gives NO indication that `$attempt.number` is a special variable that only exists inside `repeat:` bodies.

### C. The test contract claim is a hallucination

`attempt_number_tests.rs:11` declares:
> 4. Compilation fails with `InvalidVariableScope` when used outside repeat bodies

**`InvalidVariableScope` is not a variant of `CompileError`.** Searching `mod_compile_errors/kind.rs` confirms only these reference-related variants exist: `UnknownReferenceRoot`, `IllegalReference`, `UnknownReferenceName`, `UnsupportedAccessorReference`. The test file itself acknowledges the lie at line 348-355 by accepting the fallback `IllegalReference | UnknownReferenceRoot`.

The 19 tests cannot pass as written — they assert against an error variant that does not exist.

## Bead Tracking Status: UNTRACKED, LAUNDERED, AND DECEPTIVELY CLOSED

- `bd search "MAJOR-5"` returns: `No issues found matching 'MAJOR-5'`
- `bd search "attempt.number"` returns: `No issues found`
- `bd search "restrictions"` returns: `No issues found`

The gap is **NOT in the active bead queue.**

## Closest related beads — all CLOSED with laundered evidence

1. **`vb-xi2f.25`** "P0: lower nested repeat body steps" — CLOSED 2026-06-03. Proof review claims "594 passed, clippy clean, STATUS: APPROVED". The "fix" was a 2-line delegation in `canonical_body_step_width` to `canonical_step_width` for Repeat. **The proof never exercises a Repeat with actual body steps containing `$attempt.number`** — because the cold AST cannot carry the body, no test can construct such a step.

2. **`vb-xi2f.31`** "P1: digest covers repeat semantics" — CLOSED. Claims "635 vb_compile tests PASS". Digest at `part_05.rs:331-338` references the LEGACY AST that has body — but the production compiler pipeline now uses the cold AST which does NOT have body.

3. **`vb-xi2f.14`** "P0: nested loop and collection body lowering umbrella" — CLOSED. Aggregate claim: "866 tests pass, clippy clean". Closes all 5 children as completed.

**This is a textbook `STATUS: APPROVED` laundered review of a gap that is structurally unfixable within the current AST shape.**

## Severity: 92/100

| Dimension | Score |
|---|---|
| User harm | 25/25 — Silent loss of user-written body steps; no diagnostic to find it |
| Diagnostic quality | 23/25 — `UnknownReferenceRoot` is misleading |
| Test coverage | 18/20 — 19 tests written, all dead |
| Drift risk | 13/15 — Legacy AST vs cold AST divergence is documented nowhere |
| Bead tracking | 8/10 — Untracked; adjacent beads CLOSED with misleading evidence |
| Fixability | 5/5 — STRUCTURAL fix required |

## Verdict: SHIP-BLOCKER

This is a P0 structural gap. The fix requires:
1. Restore `StepKindAst::Repeat` to carry body steps
2. Update `parse_repeat` to call `parse_body_steps`
3. Re-plumb all 6+ match sites
4. Declare `mod restrictions;` in `lib.rs`
5. Add `InvalidVariableScope` variant
6. Open a tracking bead; reopen vb-xi2f.25 and vb-xi2f.31
7. Add a Kani harness with `kani::any()` on a varied `StepKindAst::Repeat { max_attempts, body }` shape
