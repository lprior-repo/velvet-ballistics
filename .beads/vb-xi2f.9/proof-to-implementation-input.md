# Proof-to-Implementation Bridge Input: YAML Path and Source Span Diagnostics

**Bead:** vb-xi2f.9
**Agent:** proof-planner (State 4)
**Schema:** proof-to-implementation-input/v1

This document maps each planned proof obligation to the Rust source files, test files, and harness locations that implementers, test writers, and proof writers must target. It is an input to the `proof-to-implementation` agent, not a bridge plan itself.

---

## 1. Span Enrichment (PS-001 / SPAN-ENRICH)

### Source Files
| File | Type | Role |
|---|---|---|
| `crates/vb_core/src/span.rs` | production | Span struct, constructors (`new`, `with_location`, `ZERO`), `Located<T>`, `Spanned<T>` |
| `crates/vb_core/src/lib.rs` | production | Public re-exports of Span, Located, Spanned |

### Proof Artifacts
| Obligation | Location | Target |
|---|---|---|
| PO-K01 | `crates/vb_core/proofs/span_kani.rs` | Kani harness for Span constructor invariants |
| PO-F01 | `crates/vb_core/src/span.rs` | Flux `#[refined_by]` annotation on Span struct |
| PO-P01 | `crates/vb_core/tests/proptest_span.rs` | Proptest properties for Span constructors |

### Test Artifacts
| Test File | What It Tests |
|---|---|
| `crates/vb_core/tests/span_enriched_tests.rs` | Unit tests: backward compat, Located<T>/Spanned<T> unchanged, Span::ZERO equality |
| `crates/vb_core/tests/proptest_span.rs` | Proptest: for-all (start,end,line,col), invariants hold |

### Implementation Bridge Notes
- `Span::ZERO` MUST remain `{start:0, end:0, line:None, column:None}` — never change this constant.
- `Span::new(start, end)` MUST set `line: None, column: None` (not default-derived from struct update syntax).
- `Span::with_location(start, end, line, col)` MUST set both `line` and `column` to `Some(...)` together.
- `Located<T>` and `Spanned<T>` MUST delegate to Span's semantics (they wrap Span without extra logic).
- Conservative: consider making `line` and `column` fields `pub(crate)` to prevent external construction that violates the paired invariant. If they must be `pub`, note that HA-04 (unpaired invariant) is accepted as a modeling hazard.

---

## 2. Diagnostic Source File (PS-003 / DIAG-FILE)

### Source Files
| File | Type | Role |
|---|---|---|
| `crates/vb_core/src/diagnostic.rs` | production | Diagnostic struct, `Diagnostic::new()`, `DiagnosticCode`, `Severity` |
| `crates/vb_core/src/lib.rs` | production | Public re-exports of Diagnostic, DiagnosticCode |

### Proof Artifacts
| Obligation | Location | Target |
|---|---|---|
| PO-K03 | `crates/vb_core/proofs/diagnostic_kani.rs` | Kani harness for Diagnostic source_file invariant |

### Test Artifacts
| Test File | What It Tests |
|---|---|
| `crates/vb_core/tests/diagnostic_file_tests.rs` | Unit tests: source_file: Some(non_empty), source_file: None for runtime, backward compat with Span::ZERO |

### Implementation Bridge Notes
- `Diagnostic::new()` gains `source_file: Option<Box<str>>` as NEW parameter (or builder method). Existing callers pass `None`.
- When `source_file: Some(s)`, `s` MUST be non-empty. Consider a `SourceFile` newtype that validates at construction.
- Runtime diagnostics (budget errors, IPC errors, etc.) always use `source_file: None` — no implementation change needed in `vb_runtime`, `vb_storage`, `vb_ipc`.

---

## 3. NonEmptyVec (PS-002 / NEVEC)

### Source Files
| File | Type | Role |
|---|---|---|
| `crates/vb_core/src/non_empty_vec.rs` | production (NEW) | NonEmptyVec struct, constructors, `IntoIterator`, `From<NonEmptyVec<T>> for Vec<T>` |
| `crates/vb_core/src/lib.rs` | production | Public re-export of NonEmptyVec |

### Proof Artifacts
| Obligation | Location | Target |
|---|---|---|
| PO-K02 | `crates/vb_core/proofs/non_empty_vec_kani.rs` | Kani harness for NonEmptyVec invariants |
| PO-P02 | `crates/vb_core/tests/proptest_non_empty_vec.rs` | Proptest round-trip properties |

### Test Artifacts
| Test File | What It Tests |
|---|---|
| `crates/vb_core/tests/non_empty_vec_tests.rs` | Unit tests: all constructors, edge cases (empty vec→None, single element, push/extend) |

### Implementation Bridge Notes
- Fields MUST be private (`head: T`, `tail: Vec<T>`). No `pub` on internal fields.
- `from_vec(vec)` MUST return `None` when `vec.is_empty()`.
- `is_empty()` MUST always return `false` (can be `const fn` returning `false`).
- `first()` returns `&T`, `last()` returns `&T` (or `&T` if tail is empty).
- Do NOT derive `Deserialize` (HA-12).
- `CompileErrors` in `vb_compile/src/mod_compile_errors/collection.rs` should be migrated to use `NonEmptyVec<CompileError>`.

---

## 4. YamlError Span Enrichment (PS-004 / YERR-SPAN)

### Source Files
| File | Type | Role |
|---|---|---|
| `crates/vb_yaml/src/error.rs` | production | YamlError enum — add `span: Option<SourceSpan>` to each variant |
| `crates/vb_yaml/src/events_conv.rs` | production | Where YamlError variants are constructed from event stream — supply span |
| `crates/vb_yaml/src/strict_yaml.rs` (? or equivalent) | production | Event-level profile check — preserve spans for alias/anchor/merge/tag errors |

### Proof Artifacts
| Obligation | Location | Target |
|---|---|---|
| PO-K04 | `crates/vb_yaml/proofs/yaml_error_kani.rs` | Kani harness for YamlError construction with None span |
| PO-P03 | `crates/vb_yaml/tests/proptest_yaml_error.rs` | Proptest: event-stream errors produce matching spans |

### Test Artifacts
| Test File | What It Tests |
|---|---|
| `crates/vb_yaml/tests/yaml_error_span_tests.rs` | Unit tests: each variant with Some(span), each variant with None, backward compat |

### Implementation Bridge Notes
- `span: Option<SourceSpan>` MUST be added as the LAST field on each variant (append-only for pattern match compat).
- Whole-document limit errors (`SourceTooLarge`, `NestingTooDeep`, `NodeLimitExceeded`, `EmptySource`) SHOULD have `span: None` (or no span field).
- Parse-level errors (`ParseError`, `AnchorAliasMerge`, `CustomTag`, `BinaryScalar`, `AmbiguousScalar`) MUST be constructed with `span: Some(...)` from the parser event stream.
- `#[non_exhaustive]` on YamlError may already protect some consumers. Verify.

---

## 5. Canonical YAML Span Preservation (PS-005 / CANON-SPAN)

### Source Files
| File | Type | Role |
|---|---|---|
| `crates/vb_compile/src/mod_compile_validation/part_01.rs` | production | `canonical_yaml_error()`, `extract_span_from_yaml_error()` (NEW), `yaml_error_category()` |
| `crates/vb_compile/src/mod_compile_errors/kind.rs` | production | `CompileError::CanonicalYaml` — add `mark: SourceMark` field |
| `crates/vb_compile/src/mod_compile_errors/source_mark.rs` | production | `SourceMark` — add `source_file: Option<Box<str>>`, implement `From<SourceSpan> for SourceMark` |

### Proof Artifacts
| Obligation | Location | Target |
|---|---|---|
| PO-K05 | `crates/vb_compile/proofs/canonical_yaml_kani.rs` | Kani harness for exhaustive span extraction |
| PO-G04 | (workspace tests) | Exhaustive match unit test for all 19 variants |

### Test Artifacts
| Test File | What It Tests |
|---|---|
| `crates/vb_compile/tests/canonical_span_tests.rs` | Unit tests: each YamlError variant → extract_span → correct SourceMark or unavailable() |

### Implementation Bridge Notes
- `extract_span_from_yaml_error()` MUST have a match arm for EVERY YamlError variant. Use `#[deny(non_exhaustive_match)]` or a compile-time variant count check.
- `CompileError::CanonicalYaml` gains `mark: SourceMark` as the LAST field (append-only).
- `canonical_yaml_error()` calls `extract_span_from_yaml_error()` to get the mark, passes it into `CompileError::CanonicalYaml`.
- Existing callers that construct `CanonicalYaml { category, message }` MUST add `mark: SourceMark::unavailable()` or the real mark.

---

## 6. ValidationError Span Propagation (PS-006 / VERR-SPAN)

### Source Files
| File | Type | Role |
|---|---|---|
| `crates/vb_validate/src/lib.rs` | production | ValidationError enum — add `span: Span` to EVERY variant |
| `crates/vb_validate/src/diagnostic.rs` | production | `diagnostic_from_error()` — propagate `error.span` into `Diagnostic.span` |
| `crates/vb_validate/src/diag_render.rs` | production | Either REMOVED or re-export of `diagnostic::diagnostic_from_error()` |
| `crates/vb_validate/src/diag_codes.rs` | production | Error code constants — absorbed into diagnostic.rs or imported |

### Proof Artifacts
| Obligation | Location | Target |
|---|---|---|
| PO-K06 | `crates/vb_validate/proofs/validation_error_kani.rs` | Kani harness for span propagation |
| PO-P04 | `crates/vb_validate/tests/proptest_validation_error.rs` | Proptest: for-all (variant, span), diagnostic.span == error.span |
| PO-G02 | (static check) | Grep for single `fn diagnostic_from_error` definition |
| PO-G04 | (workspace tests) | Exhaustive match unit test for all ~50 variants |

### Test Artifacts
| Test File | What It Tests |
|---|---|
| `crates/vb_validate/tests/validation_span_tests.rs` | Unit tests: each variant with span, diagnostic_from_error propagation, Span::ZERO fallback |
| `crates/vb_validate/src/diag_tests.rs` | Updated BDD tests — construct errors with Span::ZERO for new assertions |

### Implementation Bridge Notes
- `span: Span` MUST be the LAST field in every variant (append-only for `..` pattern matches).
- Unit variants (no structured data) become `VariantName { span: Span }`.
- `diagnostic_from_error()` currently hardcodes `Span::ZERO` at line 92 of diagnostic.rs — REMOVE the hardcode, use `error.span` instead.
- `error_diagnostic_parts()` MUST have a match arm for EVERY variant.
- Consolidate `diag_render.rs` — either remove it or make it re-export `diagnostic::diagnostic_from_error`.
- Tests asserting `diagnostic.span == Span::ZERO` MUST be updated to construct errors with explicit `Span::ZERO`.

---

## 7. Span Bridging (PS-007 / SPAN-BRIDGE)

### Source Files
| File | Type | Role |
|---|---|---|
| `crates/vb_compile/src/bridge/spans.rs` (NEW) or inline | production | `From<SourceSpan> for Span`, `From<SourceSpan> for SourceMark`, `From<SourceMark> for Span`, `clamp_u32()` |
| `crates/vb_compile/src/mod_compile_errors/source_mark.rs` | production | SourceMark struct — ensure `From<SourceSpan>` impl or bridge module |

### Proof Artifacts
| Obligation | Location | Target |
|---|---|---|
| PO-K07 | `crates/vb_compile/proofs/span_bridge_kani.rs` | Kani harness for no-panic usize→u32 conversion |
| PO-M01 | `crates/vb_compile/tests/miri_bridge.rs` | Miri test for UB in usize→u32 casts |
| PO-P05 | `crates/vb_compile/tests/proptest_span_bridge.rs` | Proptest: round-trip and available flag properties |

### Test Artifacts
| Test File | What It Tests |
|---|---|
| `crates/vb_compile/tests/span_bridge_tests.rs` | Unit tests: clamping behavior, available flag, conversion correctness |

### Implementation Bridge Notes
- `From<SourceSpan> for Span`: Byte offsets from `usize` to `u32` via `clamp_u32(x)`. Line/col set to `Some(clamp_u32(x))`. NEVER panics.
- `From<SourceSpan> for SourceMark`: Direct conversion — both use `usize`. `available: true`. `source_file: None`.
- `From<SourceMark> for Span`: When `available==true`, line/col are `Some(clamp_u32(...))`. When `available==false`, line/col are `None`.
- `clamp_u32(x: usize) -> u32`: `u32::try_from(x).unwrap_or(u32::MAX)`. Must not use `as` cast (truncation). Use `TryFrom` or `min(u32::MAX as usize) as u32`.
- Bridge conversions MUST NOT panic — verified by PO-K07 and PO-M01.
- The bridge lives in `vb_compile` because `vb_yaml` does not depend on `vb_core`.

---

## 8. Tree Validation Mark Backfilling (PS-008 / TREE-MARK)

### Source Files
| File | Type | Role |
|---|---|---|
| `crates/vb_compile/src/mod_compile_validation/part_02.rs` | production | `validate_strict_profile()`, `validate_one_node()`, `push_mapping()`, `validate_mapping_key()` — instrument with AstMarks lookups |
| `crates/vb_compile/src/ast/marks.rs` | production | `AstMarks` — existing step/nested_key/trigger/document lookups |

### Proof Artifacts
| Obligation | Location | Target |
|---|---|---|
| PO-K08 | `crates/vb_compile/proofs/tree_mark_kani.rs` | Kani harness for AstMarks lookup → available mark |
| PO-P06 | `crates/vb_compile/tests/proptest_ast_marks.rs` | Proptest: validate generated YAML, assert marks on errors |

### Test Artifacts
| Test File | What It Tests |
|---|---|
| `crates/vb_compile/tests/tree_mark_tests.rs` | Unit tests: step error gets mark, nested_key error gets mark, trigger error gets mark, document error gets mark, graceful degradation |

### Implementation Bridge Notes
- Before constructing a `CompileError` in tree validation, try `AstMarks::step(step_id)`, `AstMarks::nested_key(parent, key)`, `AstMarks::trigger(kind)`, or `AstMarks::document()`. If lookup returns `Some(mark)`, use it. Otherwise use `SourceMark::unavailable()`.
- This is a best-effort backfill — `SourceMark::unavailable()` is still legal (C10.2).
- Current code in part_02.rs uses `saphyr::Yaml` tree which discards parser spans. AstMarks compensates for this loss.

---

## 9. SourceMap Removal (PS-009 / RM-SRCMAP)

### Source Files
| File | Type | Role |
|---|---|---|
| `crates/vb_core/src/span.rs` | production | REMOVE `pub struct SourceMap { _private: () }` |
| `crates/vb_core/src/lib.rs` | production | REMOVE `SourceMap` from public re-exports |

### Static Verification
| Obligation | Command |
|---|---|
| PO-G01 | `grep -r 'SourceMap' crates/vb_core/src/` — must return no matches |

### Implementation Bridge Notes
- If any external crate imports `vb_core::SourceMap`, those imports must be migrated to `vb_yaml::SourceMap` or removed.
- `vb_yaml::SourceMap` is the canonical type. No new type with the same name should be created in vb_core.

---

## 10. Diagnostic Conversion Unification (PS-010 / UNIFY-DIAG)

### Source Files
| File | Type | Role |
|---|---|---|
| `crates/vb_validate/src/diagnostic.rs` | production | Canonical `diagnostic_from_error()`, `error_diagnostic_parts()`, code constants |
| `crates/vb_validate/src/diag_render.rs` | production | EITHER removed OR made thin re-export of diagnostic.rs |
| `crates/vb_validate/src/diag_codes.rs` | production | May be absorbed into diagnostic.rs or stay as import module |

### Static Verification
| Obligation | Command |
|---|---|
| PO-G02 | `grep -rn 'fn diagnostic_from_error' crates/vb_validate/src/` — must return exactly 1 definition |

### Implementation Bridge Notes
- `diagnostic.rs` becomes the canonical home.
- `diag_render.rs` either deleted or contains only `pub use super::diagnostic::diagnostic_from_error;`.
- `diag_codes.rs` constants accessible from both if `diagnostic.rs` imports them. Or absorbed into `diagnostic.rs` directly.
- All BDD tests in `diag_tests.rs` must continue to pass after consolidation.

---

## 11. SemanticSourceMap Message Annotation (PS-011 / SEM-MAP-MSG)

### Source Files
| File | Type | Role |
|---|---|---|
| `crates/vb_compile/src/diagnostic_render.rs` (NEW or existing) | production | `render_error_with_path()` — append YAML path to diagnostic message |
| `crates/vb_yaml/src/source_map_types.rs` | production | `SemanticSourceMap` — existing path→span mapping |
| `crates/vb_compile/src/mod_compile_core.rs` | production | Compile orchestration — wire SemanticSourceMap into diagnostic rendering |

### Proof Artifacts
| Obligation | Location | Target |
|---|---|---|
| PO-P07 | `crates/vb_compile/tests/proptest_semantic_map.rs` | Proptest: path annotation in diagnostic messages |

### Test Artifacts
| Test File | What It Tests |
|---|---|
| `crates/vb_compile/tests/semantic_map_msg_tests.rs` | Unit tests: path appended to message, no replacement, un-annotated when map absent |

### Implementation Bridge Notes
- Path annotation MUST be appended to existing message: `format!("{message} at path {path}")`.
- Path annotation MUST NOT replace the primary error message.
- `SemanticSourceMap` is optional at the conversion boundary — code MUST not panic if `None`.
- The path format should be consistent: use `$.path.to.field` notation from SemanticSourceMap.

---

## 12. Backward Compatibility and CI Gate (PS-012 / BACK-COMPAT)

### Source Files
| File | Type | Role |
|---|---|---|
| All test files in `vb_validate`, `vb_compile`, `vb_yaml`, `vb_core`, `workspace_tests` | test | Updated Span::ZERO assertions, new span propagation tests |

### CI Verification
| Obligation | Command |
|---|---|
| PO-G03 | `moon ci` |
| PO-G04 | `cargo test --workspace` |

### Implementation Bridge Notes
- All tests asserting `diagnostic.span == Span::ZERO` MUST be updated to either accept both zero and non-zero spans or construct errors with explicit `Span::ZERO` and assert exactly that.
- Pattern matches on `Span`, `Diagnostic`, `ValidationError`, `CompileError` using `..` SHALL continue to compile (fields are append-only).
- Exhaustive matches without `..` MUST be updated.
- `SourceMap` in `vb_core` is a breaking API removal — any downstream consumers of the public re-export must be migrated.
- `moon ci` MUST exit 0. No new clippy warnings.

---

## 13. Cross-Cutting: Kani Arbitrary Implementations

All Kani proofs require `kani::Arbitrary` implementations for core types. These implementations live in the proof files, not in production code.

| Type | Required For | Location |
|---|---|---|
| `Span` | PO-K01, PO-K06 | `crates/vb_core/proofs/span_kani.rs` |
| `DiagnosticCode` | PO-K03 | `crates/vb_core/proofs/diagnostic_kani.rs` |
| `SourceSpan` | PO-K07 | `crates/vb_compile/proofs/span_bridge_kani.rs` |
| `ValidationError` (all variants) | PO-K06 | `crates/vb_validate/proofs/validation_error_kani.rs` |
| `YamlError` (all variants) | PO-K04, PO-K05 | `crates/vb_yaml/proofs/yaml_error_kani.rs` |

**CRITICAL:** Per GOD RULE 1 — Kani harnesses MUST NOT hardcode structural inputs with fixed dummy data. Use `kani::Arbitrary` or `kani::any()` for core structures. Proving a function doesn't panic on one hardcoded data structure proves nothing.

---

## 14. Proptest Arbitrary Implementations

| Type | Required For | Location |
|---|---|---|
| `Span` (including line/column variants) | PO-P01, PO-P04 | `crates/vb_core/tests/proptest_span.rs` |
| `SourceSpan` | PO-P05 | `crates/vb_compile/tests/proptest_span_bridge.rs` |
| `ValidationError` (all variants) | PO-P04 | `crates/vb_validate/tests/proptest_validation_error.rs` |

---

## File Creation Summary

| New File | Type | Purpose |
|---|---|---|
| `crates/vb_core/src/non_empty_vec.rs` | production | NonEmptyVec type |
| `crates/vb_core/proofs/span_kani.rs` | proof | Kani harness for Span |
| `crates/vb_core/proofs/non_empty_vec_kani.rs` | proof | Kani harness for NonEmptyVec |
| `crates/vb_core/proofs/diagnostic_kani.rs` | proof | Kani harness for Diagnostic |
| `crates/vb_core/tests/proptest_span.rs` | test | Proptest for Span |
| `crates/vb_core/tests/proptest_non_empty_vec.rs` | test | Proptest for NonEmptyVec |
| `crates/vb_core/tests/span_enriched_tests.rs` | test | Unit tests for Span enrichment |
| `crates/vb_core/tests/diagnostic_file_tests.rs` | test | Unit tests for Diagnostic source_file |
| `crates/vb_core/tests/non_empty_vec_tests.rs` | test | Unit tests for NonEmptyVec |
| `crates/vb_yaml/proofs/yaml_error_kani.rs` | proof | Kani harness for YamlError |
| `crates/vb_yaml/tests/proptest_yaml_error.rs` | test | Proptest for YamlError spans |
| `crates/vb_yaml/tests/yaml_error_span_tests.rs` | test | Unit tests for YamlError spans |
| `crates/vb_compile/src/bridge/spans.rs` | production (NEW) | Bridge conversion implementations |
| `crates/vb_compile/proofs/canonical_yaml_kani.rs` | proof | Kani harness for canonical_yaml_error |
| `crates/vb_compile/proofs/span_bridge_kani.rs` | proof | Kani harness for bridge conversions |
| `crates/vb_compile/proofs/tree_mark_kani.rs` | proof | Kani harness for AstMarks backfill |
| `crates/vb_compile/tests/miri_bridge.rs` | test | Miri test for usize→u32 UB |
| `crates/vb_compile/tests/proptest_span_bridge.rs` | test | Proptest for bridge conversions |
| `crates/vb_compile/tests/proptest_ast_marks.rs` | test | Proptest for AstMarks backfill |
| `crates/vb_compile/tests/proptest_semantic_map.rs` | test | Proptest for SemanticSourceMap messages |
| `crates/vb_compile/tests/canonical_span_tests.rs` | test | Unit tests for canonical_yaml_error span |
| `crates/vb_compile/tests/span_bridge_tests.rs` | test | Unit tests for bridge conversions |
| `crates/vb_compile/tests/tree_mark_tests.rs` | test | Unit tests for tree validation marks |
| `crates/vb_compile/tests/semantic_map_msg_tests.rs` | test | Unit tests for path annotation |
| `crates/vb_validate/proofs/validation_error_kani.rs` | proof | Kani harness for ValidationError |
| `crates/vb_validate/tests/proptest_validation_error.rs` | test | Proptest for ValidationError span propagation |
| `crates/vb_validate/tests/validation_span_tests.rs` | test | Unit tests for ValidationError spans |

---

## Dependency Order for Implementation

1. **vb_core** — `Span` enrichment, `NonEmptyVec` (no deps on other changes)
2. **vb_yaml** — `YamlError` enrichment (depends on nothing except `SourceSpan`, already exists)
3. **vb_validate** — `ValidationError` enrichment + diagnostic consolidation (depends on vb_core for Span)
4. **vb_compile** — Bridge conversions, canonical_yaml_error, AstMarks backfill, SemanticSourceMap (depends on vb_core + vb_yaml + vb_validate)
5. **All crates** — Backward compat test updates, CI gate
