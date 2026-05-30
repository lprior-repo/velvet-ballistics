# Architectural Drift Report: `vb_doc/src/lib.rs`

**File**: `crates/vb_doc/src/lib.rs`  
**Total Lines**: 188  
**Status**: PERFECT (no edits required)

---

## 1. Line Count Check

| Metric | Value | Threshold | Result |
|--------|-------|-----------|--------|
| Total lines | 188 | 300 max | ✅ PASS |

---

## 2. DDD Cohesion Analysis

### Module Purpose
This is the public API surface for `vb_doc` — a document reconciliation and evidence policy domain module. It exports snapshot, policy, and patch-plan types.

### Cohesion Score: **GOOD**
All exported types form a coherent domain concept around:
- **Master document snapshots** (`MasterDocSnapshot`)
- **Evidence policy enforcement** (`EvidencePolicy`, `RequiredEvidence`)
- **Patch plan construction** (`DocPatchPlan`, `PatchTarget`, `PatchEdit`, `PatchPlanStatus`)
- **Reconciliation errors** (`DocReconcileError` variants)
- **Domain reports** (`ContradictionReport`, `EvidenceBoundedReport`, `TaintVocabularyReport`)

Submodules (`evidence`, `reconcile`) provide implementation detail and are correctly separated.

---

## 3. Violations

### Primitive Obsession (LOW SEVERITY)

The following fields use raw `String` where domain newtypes could be used:

| Location | Field | Smell |
|----------|-------|-------|
| `DocReconcileError::OutOfScopeChange` | `change_kind: String` | Primitive obsession |
| `DocReconcileError::OutOfScopeChange` | `path_or_operation: String` | Primitive obsession |
| `DocReconcileError::StaleCleanOnlyTaintText` | `phrase: String` | Primitive obsession |
| `DocReconcileError::UnsupportedEvidenceClaim` | `sentence: String` | Primitive obsession |
| `DocReconcileError::TaintVocabularyConflict` | `sentence: String` | Primitive obsession |
| `DocReconcileError::TaintVocabularyConflict` | `term: Option<String>` | Primitive obsession |
| `DocReconcileError::MissingTraceability` | `clause: String` | Primitive obsession |
| `DocReconcileError::ControlFlowTaintConflation` | `sentence: String` | Primitive obsession |
| `TaintVocabularyReport::lattice` | `Vec<String>` | Primitive obsession |
| `PatchPlan::forbidden_actions` | `Vec<String>` | Primitive obsession |

**Note**: These are in error types (`DocReconcileError`), which is somewhat acceptable since error messages often need flexibility. However, a stricter DDD approach would define domain-specific error detail types.

### Structural Observations

1. **Error variants are data-heavy** — `DocReconcileError` carries rich context in each variant, which is good for debugging but makes exhaustive matching harder. Consider `#[non_exhaustive]` on the enum (already present).

2. **No workflow state machine** — The module defines data types but doesn't expose explicit state-transition functions. If `DocPatchPlan` represents a workflow, consider adding `impl DocPatchPlan { pub fn transition(...) }` methods.

3. **Two submodules suggest split** — If `evidence` and `reconcile` grow, this lib.rs could become a pure re-export barrel. Current state is acceptable.

---

## 4. DDD Smell Assessment

| Smell | Present | Severity |
|-------|---------|----------|
| Primitive Obsession | Yes (in error variants) | LOW |
| Data Clumps | No | — |
| Feature Envy | No | — |
| God Objects | No | — |
| Parallel Inheritance | No | — |
| Shotgun Surgery | No | — |
| Incomplete Abstraction | No | — |

---

## 5. Priority

| Issue | Priority | Effort |
|-------|----------|--------|
| Primitive obsession in error types | LOW | Could be addressed with domain error detail types but not critical |
| No explicit state transitions | LOW | Would improve discoverability if patch plans are workflows |

**Overall Priority: LOW** — The module is well-structured, under line limit, and exhibits only minor DDD smells in error handling that are pragmatic for flexible error reporting.

---

## 6. Recommendation

No refactoring required. The file is production-ready as-is. The primitive obsession in error types is acceptable given that:
1. Error messages require flexibility
2. The domain types themselves (`MasterDocSnapshot`, `DocPatchPlan`, etc.) are properly modeled
3. Line count is well under threshold

**STATUS: PERFECT**
