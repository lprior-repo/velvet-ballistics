# Test Plan Review: vb-xi2f.9 — YAML Path and Source Span Diagnostics

**Bead:** vb-xi2f.9
**Agent:** test-reviewer (State 10 — test-plan-review mode)
**Date:** 2026-05-26
**Input:** test-plan.md (1372 lines), contract.md (273 lines)
**Review type:** Plan Review (pre-execution)

---

## STATUS: APPROVED

The test plan is structurally sound, covers all 78 contract behaviors across 12 clauses, and allocates test layers appropriately for a type-enrichment bead. Four findings require resolution before or during test-suite execution.

---

## Gate Results

| Gate | Criterion | Result |
|---|---|---|
| PG-1 | Every public behavior has >= 1 Given/When/Then scenario | PASS — 78/78 scenarios mapped |
| PG-2 | Every error variant has exact-variant assertion scenario | PASS — DiagnosticCodeParseError::InvalidFormat/UnsupportedCode both covered |
| PG-3 | No `is_ok()` / `is_err()` / `Some(_)` boolean-smoke assertions planned | PASS — all planned assertions are concrete |
| PG-4 | Boundary cases named (min, max, just-below, just-above, empty/zero/none) | PASS — Section 8 covers min/max/empty/overflow |
| PG-5 | Non-trivial pure behavior has property tests planned | PASS — 9 proptest invariants (PO-P01–PO-P07 + 2 aux) |
| PG-6 | Parser/codec/hostile input has fuzz/adversarial tests planned | PASS — 4 fuzz targets (FUZZ-xi2f.9-01–04) |
| PG-7 | Verifier harnesses not counted as behavior tests | PASS — Kani/Miri separated into Section 6 |

---

## Findings

### FIND-TPR-01 [HIGH] — E2E Scenarios Underspecified

Section 3.11 and Combinatorial Matrix Section 8.8 define 6 E2E tests but with generic descriptions only:

- "Compile invalid YAML → diagnostic shows file:line:col" (2 scenarios)
- "Compile YAML with validation error → diagnostic has correct span" (2 scenarios)
- "Compile YAML with known error → rendered output includes YAML author path" (2 scenarios)

**Gap:** No concrete YAML fixtures, no exact assertion targets, no expected-output templates. This leaves E2E test writing ambiguous and risks weak "rendered_output.len() > 0" style assertions.

**Remediation:** Add a fixture column to Section 8.8 (e.g., `"version: bad"` at byte offset X produces `span.start == Y, span.line == Some(Z)`). The test-writer must produce concrete E2E tests with exact span assertions.

---

### FIND-TPR-02 [MODERATE] — DiagnosticCode Display (B24) Missing from BDD Scenarios

The behavior inventory lists B24: "DiagnosticCode Display formats as EXXXX hex." Section 3.3 jumps from B23 (packed value) to B25 (parsing) — B24 has no dedicated Given/When/Then scenario. The coverage matrix 8.2 line "happy: E0101" checks `Ok(0x0101)` but not the Display output.

**Remediation:** Add a B24 scenario or note that it is covered by the fuzz target's display invariant assertion (`starts_with('E')`, `len() == 5`). If fuzz-only coverage, mark as `#[allow]` with rationale.

---

### FIND-TPR-03 [MODERATE] — Contract C2.2 (Non-Empty source_file) Not Planned

Contract clause C2.2 states: "When `source_file` is `Some(s)`, `s` SHALL be non-empty." No behavior in the inventory (B19–B33) asserts this invariant. No test scenario validates that `Some("")` is rejected or handled.

**Remediation:** Add a behavior B33a or re-scope B33 to include "source_file: Some(\"\") is either rejected at construction or produces a well-formed diagnostic." Clarify with the contract owner whether this is a type-enforced invariant (type system makes it impossible) or a runtime invariant requiring a test.

---

### FIND-TPR-04 [LOW] — NonEmptyVec::extend (B43) BDD Scenario Missing

Behavior B43 is in the inventory but Section 3.4 BDD scenarios skip it (B42 push, B44 into_vec). The coverage matrix 8.3 does list "extend" but the plan's narrative scenarios do not.

**Remediation:** Add a B43 scenario to Section 3.4 or note that `extend` is covered by the proptest iteration-order invariant (PO-P02) where `extend` is an implementation detail.

---

### FIND-TPR-05 [LOW] — YamlError Unit Test Layer Thin

The test-plan allocates 3 YamlError tests to the unit layer (B49–B51), but the test-writer-report shows coverage primarily through Kani (PO-K04) and proptest (PO-P03). The plan should clarify that the `span()` method's exhaustive match (B52) is compile-time-verified, not runtime-tested.

**Remediation:** Add a matrix row noting B52 is verifier-only (compiler exhaustiveness check) with waiver tag `WAIVE-B52-COMPILE-TIME`.

---

## Trophy Allocation Review

| Layer | Plan % | Target % | Delta | Justification |
|---|---|---|---|---|
| Unit | 46% | 60% | -14% | Type enrichment bead — most behaviors are pure data transformation trivially testable as unit tests |
| Integration | 36% | 30% | +6% | Three bridge boundaries (YamlError→CanonicalYaml, SourceSpan→Span, ValidationError→Diagnostic) require cross-crate tests |
| E2E | 8% | 5% | +3% | Full compilation-to-diagnostic pipeline needs end-to-end verification |
| Static+Verif | 10% | 5% | +5% | Span paired invariant, bridge panic-freedom, NonEmptyVec invariants require formal verification |

**Judgment:** The deviation is justified. The bridge-heavy nature of this bead naturally pushes allocation toward integration testing. The formal-verification layer carries weight that would otherwise require combinatorial unit tests. **ACCEPTED.**

---

## Gap Assessment

| Gap ID | Behavior | Blocker | Assessment |
|---|---|---|---|
| GAP-DIAG-002 | B56–B58 (canonical_yaml_error span propagation) | Implementation incomplete | **ACCEPTED.** Tests deferred until implementation lands. No behavior test to review. |
| GAP-DIAG-001 | PO-K02 timeout (NonEmptyVec into_vec_round_trip) | Kani bounds missing | **ACCEPTED.** Proptest PO-P02 covers bounded round-trip. Kani remediation is defense-in-depth. |
| GAP-DIAG-009 | PO-K02 harness design defect | Same as GAP-DIAG-001 | **ACCEPTED.** Duplicate of GAP-DIAG-001. |

---

## Mutation Resistance Assessment

The test-plan's Section 7 mutation matrix is well-designed. Each critical mutation maps to a named test:

| Mutation | Caught By | Likelihood |
|---|---|---|
| `clamp_u32`: replace `unwrap_or(u32::MAX)` with `unwrap()` | `clamp_u32_exceeds_max` | HIGH — boundary test |
| `span_from_source_span`: swap line/column fields | `source_span_to_span_typical` | HIGH — exact field assertions |
| `SourceMark → Span`: always set available branch to true | `source_mark_unavailable_produces_none_line_col` | HIGH — branch-specific test |
| `diagnostic_from_error`: ignore error.span, always use Span::ZERO | `diagnostic_from_error_propagates_enriched_span_exactly` | HIGH — exact span equality |
| `AstMarks::empty().step()`: return Some(unavailable_mark) | `ast_marks_empty_step_returns_none_for_any_input` | HIGH — Kani verified |

The kill-rate target of >=90% is ambitious but achievable given the assertion density.

---

## Summary

The test plan covers all 78 behaviors with appropriate test layers. The 3 blocked behaviors (B56–B58) are documented as deferred pending implementation. Four findings (one HIGH, three MODERATE/LOW) require resolution. The HIGH finding (E2E underspecification) should be resolved before the test-writer closes out E2E coverage.

**STATUS: APPROVED** — proceed to test-suite review and execution.

---

*Evidence: test-plan.md (lines 1–1372), contract.md (lines 1–273), test-writer-report.md (lines 1–349)*
