# Architectural Drift Report: vb_doc_api.rs

## File Analysis

| Metric | Value |
|--------|-------|
| **File** | `crates/vb_doc/tests/vb_doc_api.rs` |
| **Total Lines** | 1284 |
| **File Size** | 39.1 KB |
| **Test Count** | 53 tests (49 unit + 4 proptest) |

## Test Breakdown

| Category | Count |
|----------|-------|
| Unit tests (`#[test]`) | 49 |
| Proptest property-based tests | 4 |
| **Total** | **53** |

## Test Coverage by Module

| Module Under Test | Test Count |
|-------------------|------------|
| `MasterDocSnapshot` | 6 |
| `EvidencePolicy` | 2 |
| `EvidenceIndex` | 3 |
| `EvidenceSupport` | 2 |
| `validate_taint_vocabulary_consistency` | 7 |
| `validate_evidence_bounded_wording` | 7 |
| `scan_for_stale_clean_only_text` | 10 |
| `check_doc_taint_consistency` | 2 |
| `plan_taint_doc_reconciliation` | 13 |
| Data structure equality/Debug | 5 |
| Boundary/edge cases | 4 |

## Size Assessment

| Guideline | Threshold | Actual | Status |
|-----------|-----------|--------|--------|
| Max lines per file | 300 | 1284 | ❌ VIOLATION (4.3× over) |

## DDD Cohesion Assessment

The test file demonstrates **high cohesion**:
- Single domain focus: `vb_doc` document reconciliation API
- Clear Given/When/Then structure
- Exhaustive coverage of Error variants (every `DocReconcileError` variant tested)
- Property-based tests validate determinism

## Architectural Drift Findings

### Violation: File Size

**Issue**: 1284 lines exceeds the 300-line guideline by 4.3×.

**Recommendation**: Split into logical test modules:

```
crates/vb_doc/tests/
├── vb_doc_api.rs           # MasterDocSnapshot, EvidencePolicy constructors
├── vb_doc_vocabulary.rs    # validate_taint_vocabulary_consistency tests
├── vb_doc_evidence.rs      # validate_evidence_bounded_wording tests
├── vb_doc_stale.rs         # scan_for_stale_clean_only_text tests
├── vb_doc_reconcile.rs     # plan_taint_doc_reconciliation tests
├── vb_doc_eq.rs            # equality/Debug tests
└── vb_doc_proptest.rs      # property-based tests
```

### Positive Observations

1. **Test density**: 24.2 lines/test — adequate for comprehensive API testing
2. **No panics in tests**: All error paths properly handled with `is_ok()`/`is_err()`
3. **No `unwrap()`/`expect()` in test assertions**: Production code called safely
4. **Determinism verified**: Proptest validates `check_doc_taint_consistency` and `validate_taint_vocabulary_consistency` are deterministic
5. **Unicode/edge case coverage**: Unicode text, empty text, very long text all tested

## Recommendation

**ACTION REQUIRED**: Split `vb_doc_api.rs` into 6-7 focused test modules. The current monolithic test file violates the <300 line guideline. Domain cohesion is good, but structural enforcement requires modular decomposition.

---
*Generated: 2026-05-29*
*Tool: architectural-drift*
