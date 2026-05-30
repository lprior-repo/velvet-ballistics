# Architectural Drift Report: `vb_doc::reconcile::vocabulary`

**File**: `crates/vb_doc/src/reconcile/vocabulary.rs`  
**Lines**: 76  
**Status**: PERFECT (no refactoring required)

---

## 1. Line Count Check

| Metric | Value | Threshold | Result |
|--------|-------|-----------|--------|
| Total lines | 76 | < 300 | ✅ PASS |

---

## 2. DDD Cohesion Analysis

### Module Purpose
The `vocabulary` module encapsulates taint vocabulary validation — a single bounded context concerned with:
- Defining the canonical taint lattice (`Clean`, `DerivedFromSecret`, `Secret`)
- Rejecting control-flow conflation patterns
- Rejecting lattice ordering violations (wrong order, downgrades, unknown terms)

### Cohesion Score: HIGH

All 5 public/internal functions serve the vocabulary validation domain:

| Function | Responsibility | DDD Alignment |
|----------|----------------|---------------|
| `validate()` | Orchestrates validation pipeline | ✅ Entry workflow |
| `taint_vocabulary_report()` | Constructs canonical lattice report | ✅ Value object factory |
| `reject_control_flow_conflation()` | Guards against CF taint conflation | ✅ Business rule |
| `reject_lattice_conflicts()` | Guards against lattice violations | ✅ Business rule |
| `vocabulary_error()` | Error construction helper | ✅ Domain error factory |

### Ubiquitous Language Captured
- `TaintVocabularyReport` — the vocabulary artifact
- `TaintVocabularyRule::JoinedDataFlowTaint` — propagation semantics
- `PreservedNonGoal::ControlFlowTaintV1NonGoal` — explicit non-goal
- `ConflictKind` variants: `WrongOrder`, `Downgrade`, `UnknownTerm`

---

## 3. Violations

**None identified.** The module adheres to DDD principles:

- ✅ No primitive obsession (uses domain types `TaintVocabularyReport`, `ConflictKind`, `DocReconcileError`)
- ✅ Explicit state transitions (validation functions return `Result`)
- ✅ "Parse, don't validate" is respected (the `validate()` function parses and transforms, not just validates strings)
- ✅ Domain newtypes properly wrapping behavior
- ✅ Single responsibility per function

### Minor Observations (not violations)
1. `to_ascii_lowercase()` in `contains_control_flow_conflation()` is locale-aware but acceptable for ASCII-only pattern matching
2. Hardcoded lattice strings (`"Clean"`, `"DerivedFromSecret"`, `"Secret"`) are encapsulated in `taint_vocabulary_report()` factory — not a violation since they represent the canonical vocabulary definition

---

## 4. DDD Smell Assessment

| Smell | Present | Severity |
|-------|---------|----------|
| Primitive obsession | No | — |
| Data class | No | — |
| Feature envy | No | — |
| Shotgun surgery | No | — |
| Lazy element | No | — |
| Cross-module talk | No | — |

---

## 5. Priority

**NONE** — No architectural drift detected.

---

## Conclusion

This module is a textbook example of DDD cohesion:
- Single bounded context (taint vocabulary)
- Explicit business rules as functions
- Domain types drive behavior
- No primitive obsession

**STATUS: PERFECT**
