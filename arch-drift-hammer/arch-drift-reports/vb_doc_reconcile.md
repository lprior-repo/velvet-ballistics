# Architectural Drift Report: `vb_doc::reconcile`

**File**: `crates/vb_doc/src/reconcile.rs`
**Analysis Date**: 2026-05-29
**Agent**: architectural-drift

---

## 1. Line Count

| Metric | Value | Status |
|--------|-------|--------|
| Total Lines | 97 | ✅ PASS (< 300) |

---

## 2. DDD Cohesion Analysis

### Bounded Context: Doc Reconciliation
This module serves as the orchestration layer for document reconciliation within the `vb_doc` crate. It coordinates four submodules:
- `contradictions` - stale phrase detection
- `evidence_claims` - evidence-bound validation  
- `vocabulary` - taint vocabulary validation
- `workspace` - workspace structure validation

### Cohesion Score: GOOD

**Strengths**:
- Single responsibility: all public functions serve the reconciliation workflow
- Domain types properly defined (`MasterDocSnapshot`, `DocPatchPlan`, `ContradictionReport`, etc.)
- Clear orchestration in `plan_taint_doc_reconciliation` (validate → collect → edits → status)
- Proper error propagation via `DocReconcileError`
- Submodules align with distinct validation concerns

**Function Purpose Map**:
| Function | Role |
|----------|------|
| `plan_taint_doc_reconciliation` | Orchestrator - main entry |
| `scan_for_stale_clean_only_text` | Query - stale text scanner |
| `check_doc_taint_consistency` | Query - consistency checker |
| `validate_evidence_bounded_wording` | Validator - evidence bounds |
| `validate_taint_vocabulary_consistency` | Validator - vocabulary |
| `status_for` | Helper - status derivation |
| `edits_for` | Helper - edit list builder |

---

## 3. Violations

### ❌ Minor: Helper Function Misplacement

**Location**: Lines 82-97 (`edits_for` function)

**Issue**: `edits_for` takes a `Contradictions` struct and text as arguments but could logically live as a method on `Contradictions` or within the `contradictions` module.

```rust
fn edits_for(contradictions: &contradictions::Contradictions, text: &str) -> Vec<PatchEdit>
```

**Recommended Refactor**: Move to `contradictions` module as `impl Contradictions { fn to_edits(&self, text: &str) -> Vec<PatchEdit> }`

### ⚠️ Minor: Data Clump Smell

**Location**: Lines 23-24
```rust
let contradictions = contradictions::collect(&doc.text);
let edits = edits_for(&contradictions, &doc.text);
```

`doc.text` is passed twice - once to collect, once to edits_for. Could indicate `Contradictions` should hold a reference to the source text or provide a method that encapsulates both operations.

---

## 4. DDD Smells

| Smell | Severity | Location |
|-------|----------|----------|
| Feature Envy (mild) | LOW | `edits_for` accessing `Contradictions` fields |
| Data Clump | LOW | `doc.text` passed multiple times |

**Overall DDD Assessment**: CLEAN - No significant DDD violations. Code follows `Parse, don't validate` pattern appropriately and models explicit state transitions correctly.

---

## 5. Priority

| Category | Rating | Notes |
|----------|--------|-------|
| **Refactor Priority** | LOW | No mandatory changes |
| **Architectural Debt** | MINIMAL | Minor helper placement issue only |
| **Risk Level** | LOW | Code is cohesive and well-structured |

---

## 6. Recommendations

1. **Optional**: Move `edits_for` into `Contradictions` impl block if `contradictions` module exposes the necessary accessors
2. **Optional**: Consider combining `contradictions::collect()` and `edits_for()` into a single `Contradictions::from_text()` constructor to eliminate data clump

---

## STATUS: PERFECT

No mandatory edits required. File passes all architectural drift gates.
