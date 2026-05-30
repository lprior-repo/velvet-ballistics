# Architectural Drift Report: vb_compile/error_variant_tests.rs

**File:** `crates/vb_compile/src/tests/error_variant_tests.rs`  
**Analyzed:** 2026-05-29  
**Rule Set:** `<300 line files` / DDD cohesion / boundary integrity

---

## Summary

| Metric | Value |
|--------|-------|
| **Total Lines** | 2210 |
| **Test Count** | 79 |
| **Location Category** | `tests/` — integration/completeness test suite |
| **Size Gate** | ❌ EXCEEDS 300-line threshold (2210 lines) |

---

## Size Analysis

- **2210 lines** vs. **300-line recommended maximum**
- **Ratio:** 7.37× over the recommended limit
- **Severity:** HIGH — monolithic test file exceeds safe structural limit by >7×

---

## Cohesion Review

This file contains:
1. **Error variant completeness tests** — tests for each `CompileError` variant (lines 24–346)
2. **CompileErrors collection tests** — iterator, len, is_empty, first() coverage (lines 371–453)
3. **Expression parser tests** — literals, helpers (lines 481–678)
4. **YamlLimits tests** — boundary configuration (lines 714–848)
5. **Digest determinism tests** — PO-009, PO-010, PO-018 coverage (lines 850–958)
6. **Together digest tests** — GAP-1 through GAP-12 covering canonical names, branch variants, sub-step hashing (lines 960–2210)

**Cohesion Concern:** The file mixes:
- Error variant completeness
- Expression parsing
- Digest/hashing behavior
- Workflow compilation

These are distinct domain concerns that should live in separate test files.

---

## Recommendation

**Action:** SPLIT this file into focused test modules

### Split Proposal:

| New File | Content | Approx. Size |
|----------|---------|--------------|
| `error_variant_tests.rs` | CompileError variant tests only | ~350 lines |
| `expression_tests.rs` | Expression parsing, helpers, literals | ~200 lines |
| `digest_tests.rs` | Digest determinism, Together hashing | ~400 lines |
| `limits_tests.rs` | YamlLimits configuration | ~150 lines |
| `compile_errors_tests.rs` | CompileErrors collection API | ~100 lines |

### Rationale:
1. **Single Responsibility** — each file tests one domain concept
2. **Parallel CI** — test files can run in parallel
3. **Maintainability** — targeted changes reduce risk
4. **Locators** — developers find relevant tests faster

---

## Drift Verdict

| Check | Status |
|-------|--------|
| File size ≤300 lines | ❌ FAIL (2210 lines) |
| DDD cohesion | ⚠️ MIXED (multi-domain) |
| Test count appropriate | ✅ OK (79 tests, 36 lines/test avg) |
| Test isolation | ✅ OK (no cross-cutting deps) |

**Overall:** Structural drift detected — file MUST be refactored to comply with architectural constraints.
