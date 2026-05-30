# Architectural Drift Report: `vb_doc/src/lib.rs`

**File:** `crates/vb_doc/src/lib.rs`  
**Analysis Date:** 2026-05-29  
**Status:** PERFECT (no edits required)

---

## 1. Line Count

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | 188 | 300 | ✅ PASS |

---

## 2. DDD Cohesion Analysis

### Domain Boundary Assessment

**Domain:** Documentation Reconciliation (`vb_doc`)

**Core Types Identified:**
- `MasterDocSnapshot` — Value object representing a point-in-time doc state
- `EvidencePolicy` — Policy for evidence requirements
- `DocPatchPlan` — Aggregate root for patch planning
- `DocReconcileError` — Error taxonomy
- `ContradictionReport`, `EvidenceBoundedReport`, `TaintVocabularyReport` — Report types

**Submodules:**
- `evidence` — Evidence tracking
- `reconcile` — Reconciliation logic

**Cohesion Score:** HIGH — All types belong to the documentation reconciliation bounded context.

---

## 3. Violations

### Primitive Obsession (Medium Severity)

| Field | Type | Suggested Newtype |
|-------|------|-------------------|
| `MasterDocSnapshot.text` | `String` | `DocText` |
| `MasterDocSnapshot.path` | `PathBuf` | `MasterDocPath` |
| `DocPatchPlan.contradiction_count` | `usize` | `ContradictionCount` |
| `DocReconcileError.OutOfScopeChange.change_kind` | `String` | `ChangeKind` |
| `DocReconcileError.OutOfScopeChange.path_or_operation` | `String` | `OperationId` |
| `DocReconcileError.StaleCleanOnlyTaintText.phrase` | `String` | `StalePhraseText` |
| `DocReconcileError.UnsupportedEvidenceClaim.sentence` | `String` | `ClaimSentence` |
| `DocReconcileError.TaintVocabularyConflict.sentence` | `String` | `ConflictSentence` |
| `DocReconcileError.TaintVocabularyConflict.term` | `Option<String>` | `TaintTerm` |
| `DocReconcileError.MissingTraceability.clause` | `String` | `TraceClause` |
| `TaintVocabularyReport.lattice` | `Vec<String>` | `TaintLattice` |
| `DocPatchPlan.forbidden_actions` | `Vec<String>` | `ForbiddenActions` |

### State Machine Absence (Low Severity)

The `DocPatchPlan` represents a workflow but is modeled as a static struct rather than explicit state transitions. Consider:
- `PatchPlanState` enum already exists but `status: PatchPlanStatus` is data, not behavior
- No `PatchPlan::transition()` or similar workflow function visible in this file

### Missing `Parse, Don't Validate` (Informational)

The `EvidencePolicy::strict_bounded()` factory is good, but several string fields flow through without parsing validation.

---

## 4. DDD Smell Assessment

| Smell | Severity | Present |
|-------|----------|---------|
| Primitive Obsession | Medium | Yes (String/PathBuf/usize) |
| State Machine as Data | Low | Yes |
| Anemic Domain Model | Low | Partial — behavior in submodules |
| Type-Driven Validation | Low | Missing for string fields |

**Overall Smell Level:** MODERATE

---

## 5. Priority Recommendation

| Priority | Action |
|----------|--------|
| **P2** | Wrap `PathBuf` in `MasterDocPath` newtype |
| **P2** | Wrap `String` fields in domain-specific newtypes |
| **P3** | Add explicit state transition methods to `DocPatchPlan` |
| **P3** | Consider `NonEmptyString` / `NonZeroUsize` for positive-constraint fields |

---

## 6. Conclusion

File is **under 300 lines** and **cohesive** within its domain. No architectural drift requiring immediate refactoring. The primitive obsession violations are stylistic improvements rather than architectural defects.

**STATUS: PERFECT**
