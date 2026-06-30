# Proof Plan Review: vb-tub4

## Reviewer Information
- **reviewer_skill**: proof-plan-reviewer
- **review_state**: 4
- **reviewed_artifacts**: proof-strategy.md, verifier-lane-decisions.jsonl, proof-obligations.planned.jsonl, proof-coverage-matrix.md, trusted-base-plan.md, waiver-candidates.jsonl, traceability-matrix.jsonl

## STATUS: REJECTED

## Summary
Lane decisions are sound (8/8 accepted). Waiver handling is correct (no behavior-affecting waivers). Trusted base scoping is appropriate. However, **proof-obligations.planned.jsonl has critical schema violations** that prevent proof-writer from executing obligations.

---

## Findings

### CRITICAL — Blocker 1: `schema_version` Missing from All Obligations
- **Artifact**: `proof-obligations.planned.jsonl`
- **Finding Code**: `E_SCHEMA_MISSING_FIELD`
- **Severity**: blocker
- **Message**: Every `proof-obligation/v1` entry must have `schema_version`. None of the 27 obligations have this field.
- **Required Fix**: Add `"schema_version": "proof-obligation/v1"` to every JSON object in `proof-obligations.planned.jsonl`.

### CRITICAL — Blocker 2: `workdir` Missing from All Obligations
- **Artifact**: `proof-obligations.planned.jsonl`
- **Finding Code**: `E_SCHEMA_MISSING_FIELD`
- **Severity**: blocker
- **Message**: Every `proof-obligation/v1` entry must have `workdir` for command execution. None of the 27 obligations specify a workdir.
- **Required Fix**: Add `"workdir": "/home/lewis/src/velvet-ballistics"` to every obligation entry.

### CRITICAL — Blocker 3: `model_bounds` Field Absent (Uses `bound` Instead)
- **Artifact**: `proof-obligations.planned.jsonl`
- **Finding Code**: `E_SCHEMA_ALIAS_FIELD`
- **Severity**: blocker
- **Message**: The schema requires `model_bounds` but the file uses `bound`. This is a schema alias violation.
- **Required Fix**: Rename `bound` field to `model_bounds` in all 27 obligations.

### MINOR — Issue 4: `owner_state` Should Be `owner`
- **Artifact**: `proof-obligations.planned.jsonl`
- **Finding Code**: `E_SCHEMA_ALIAS_FIELD`
- **Severity**: advisory
- **Message**: Schema uses `owner` not `owner_state`. While `owner_state` conveys similar meaning, schema compliance requires `owner`.
- **Required Fix**: Rename `owner_state` to `owner` in all obligations.

### MINOR — Issue 5: Missing `artifact` Field
- **Artifact**: `proof-obligations.planned.jsonl`
- **Finding Code**: `E_SCHEMA_MISSING_FIELD`
- **Severity**: advisory
- **Message**: `artifact` field (the verification harness path) is not specified. Should reference the harness file.
- **Required Fix**: Add `"artifact": "crates/vb_core/src/<filename>"` to each obligation.

### MINOR — Issue 6: Missing `required` and `behavior_affecting` Fields
- **Artifact**: `proof-obligations.planned.jsonl`
- **Finding Code**: `E_SCHEMA_MISSING_FIELD`
- **Severity**: advisory
- **Message**: Required boolean fields `required` and `behavior_affecting` are absent.
- **Required Fix**: Add both fields to each obligation. For this bead: `"required": true`, `"behavior_affecting": false` (all harness-internal fixes).

### MINOR — Issue 7: proof-strategy.md Table Is Incomplete
- **Artifact**: `proof-strategy.md`
- **Finding Code**: `E_SCOPE_MISCLASSIFIED_BEHAVIOR`
- **Severity**: advisory
- **Message**: The proof-strategy table lists 14 harness violations, but `proof-obligations.planned.jsonl` contains 27 obligations. The table should enumerate all violations for traceability.
- **Required Fix**: Expand the proof-strategy table to cover all 27 hardcoded harness violations, or cross-reference the obligations file explicitly.

---

## Waiver Check
- **Result**: PASS — No behavior-affecting waivers. `waiver-candidates.jsonl` correctly identifies that all 29 violations are harness-internal hardcoded inputs fixable via `kani::any()`.

## Trusted Base Check
- **Result**: PASS — `trusted-base-plan.md` correctly scopes all changes to `#[cfg(kani)]` harness modules. No production Rust code is touched.

## Non-Vacuity Check
- **Result**: PASS — Obligations include concrete bounds (e.g., `assume(a <= u64::MAX/2 || b <= u64::MAX/2)`) and assumptions that prevent vacuity.

## Lane Decision Coverage
- **8 lane decisions reviewed**: vb-tub4-seed-001 through vb-tub4-seed-008
- **8/8 accepted**: All lanes correctly use Kani as the sole verifier. Non-applicability of tla+/verus/flux/loom/miri/proptest is justified (GOD RULE violations are harness-internal hardcoded inputs, not protocol/concurrency/refinement type issues).

## Traceability Check
- **8 proof seeds mapped**: All seeds trace to source files/lines in `traceability-matrix.jsonl`.
- **Gap**: The matrix references seeds but doesn't map to individual obligation IDs (vb-tub4-obl-001 through vb-tub4-obl-027). This is a documentation gap but not a blocking issue.

---

## Verdict
**REJECTED** — 3 critical blockers prevent proof-writer execution: missing `schema_version`, missing `workdir`, and wrong field name `bound` instead of `model_bounds`. Proof-writer cannot validate or execute obligations without these fields.

**Required State**: Return to State 4 with fixes applied to `proof-obligations.planned.jsonl`. Lane decisions and waiver handling are approved; no replanning needed for those aspects.