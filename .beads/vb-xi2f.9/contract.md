# Contract: YAML Path and Source Span Diagnostics

**Bead:** vb-xi2f.9  
**Agent:** rust-contract (State 3)  
**Schema:** 2026-05-24  
**Version:** velvet-ballastics/v1

## Preamble

This contract defines the domain-level behavioral guarantees for enriching diagnostics in the velvet-ballistics compiler pipeline with YAML source paths, line/column spans, and non-empty error accumulation. It bridges three currently independent source-location subsystems to produce one unified diagnostic output.

---

## Clause 1: Span Enrichment (Clause ID: SPAN-ENRICH)

### C1.1 — Backward Compatibility

The `Span` type SHALL be extended with `line: Option<u32>` and `column: Option<u32>` fields. All existing constructors (`Span::new()`, `Span::ZERO`) SHALL continue to produce spans with `line: None, column: None`. All existing callers SHALL compile and function identically.

### C1.2 — Location Constructor

`Span::with_location(start, end, line, column)` SHALL produce a span where `self.line == Some(line)` and `self.column == Some(column)`. Both `line` and `column` SHALL be >= 1.

### C1.3 — Paired Invariant

For any `Span` produced by public constructors, `self.line.is_some() == self.column.is_some()` SHALL hold.

### C1.4 — Located/Spanned Compatibility

`Located<T>` and `Spanned<T>` SHALL function identically with enriched `Span`s. No changes to their API.

**Affected files:** `crates/vb_core/src/span.rs`  
**Risk tags:** `public API`  
**Behavior affecting:** YES

---

## Clause 2: Diagnostic File Path (Clause ID: DIAG-FILE)

### C2.1 — Optional Source File

`Diagnostic` SHALL gain an optional `source_file: Option<Box<str>>` field. `Diagnostic::new()` SHALL accept this as a parameter. Runtime-produced diagnostics SHALL set `source_file: None`.

### C2.2 — File Path Validity

When `source_file` is `Some(s)`, `s` SHALL be non-empty. For stdin sources, `source_file` SHALL be `Some("-".into())`.

### C2.3 — Backward Compatibility

Constructing `Diagnostic::new(code, message, severity, Span::ZERO)` SHALL produce a diagnostic with `source_file: None`.

**Affected files:** `crates/vb_core/src/diagnostic.rs`  
**Risk tags:** `public API`  
**Behavior affecting:** YES

---

## Clause 3: NonEmptyVec (Clause ID: NEVEC)

### C3.1 — Non-Empty Guarantee

`NonEmptyVec<T>` SHALL always contain at least one element. `is_empty()` SHALL always return `false`. `first()` SHALL never panic.

### C3.2 — Safe Construction

`NonEmptyVec::new(head)` SHALL produce a vec with `len() == 1`. `NonEmptyVec::with_tail(head, tail)` SHALL produce a vec with `len() == 1 + tail.len()`. `NonEmptyVec::from_vec(vec)` SHALL return `None` when `vec` is empty.

### C3.3 — Iteration

`NonEmptyVec<T>` SHALL implement `IntoIterator` yielding all elements (head first, then tail). Converting to `Vec<T>` via `into_vec()` or `From` SHALL preserve all elements and their order.

**Affected files:** `crates/vb_core/src/non_empty_vec.rs` (NEW) or `crates/vb_core/src/span.rs` (adjacent)  
**Risk tags:** `bounded-state`  
**Behavior affecting:** YES

---

## Clause 4: YamlError Span Enrichment (Clause ID: YERR-SPAN)

### C4.1 — Span Field Addition

Each `YamlError` variant that corresponds to a specific source location SHALL gain an `span: Option<SourceSpan>` field. Limit-exceeded variants that apply to the whole document (`SourceTooLarge`, `NestingTooDeep`, `NodeLimitExceeded`, `EmptySource`) MAY omit the span.

### C4.2 — Span Source

Parse-level errors (`ParseError`, `AnchorAliasMerge`, `CustomTag`, `BinaryScalar`, `AmbiguousScalar`) SHALL be constructed with `span: Some(...)` where the span is extracted from the parser event stream.

### C4.3 — Backward Compatibility

Constructing any `YamlError` variant with `span: None` SHALL be legal and equivalent to current behavior.

**Affected files:** `crates/vb_yaml/src/error.rs`  
**Risk tags:** `parser/codec`, `public API`  
**Behavior affecting:** YES

---

## Clause 5: Canonical YAML Span Preservation (Clause ID: CANON-SPAN)

### C5.1 — Span Extraction

`canonical_yaml_error(yaml_error)` SHALL extract the `SourceSpan` from `yaml_error` (if present) and convert it to a `SourceMark` embedded in the resulting `CompileError::CanonicalYaml`.

### C5.2 — CanonicalYaml Mark Field

`CompileError::CanonicalYaml` SHALL gain a `mark: SourceMark` field. When the source `YamlError` has no span, `mark` SHALL be `SourceMark::unavailable()`.

### C5.3 — Exhaustive Extraction

`extract_span_from_yaml_error()` SHALL handle all 19 `YamlError` variants exhaustively.

**Affected files:** `crates/vb_compile/src/mod_compile_validation/part_01.rs`, `crates/vb_compile/src/mod_compile_errors/kind.rs`  
**Risk tags:** `parser/codec`, `migration`  
**Behavior affecting:** YES

---

## Clause 6: ValidationError Span Propagation (Clause ID: VERR-SPAN)

### C6.1 — Span Field Addition

Every `ValidationError` variant SHALL gain a `span: Span` field (appended as the last field). Unit variants with no structured data become `VariantName { span: Span }`.

### C6.2 — Diagnostic Propagation

`diagnostic_from_error(error)` SHALL set `Diagnostic.span` to `error.span`. When `error.span == Span::ZERO`, the diagnostic SHALL have `Span::ZERO` — backward compatible.

### C6.3 — Exhaustive Coverage

`error_diagnostic_parts()` SHALL have a match arm for every `ValidationError` variant.

**Affected files:** `crates/vb_validate/src/lib.rs`, `crates/vb_validate/src/diagnostic.rs`  
**Risk tags:** `public API`, `migration`  
**Behavior affecting:** YES

---

## Clause 7: Diagnostic Conversion Unification (Clause ID: UNIFY-DIAG)

### C7.1 — Single Canonical Conversion

Exactly one function SHALL map `ValidationError` → `Diagnostic`. The duplicate in `diag_render.rs` SHALL be removed or converted to a re-export.

### C7.2 — Shared Code Constants

The error code constants (`CODE_DUPLICATE_KEY`, etc.) SHALL be defined in exactly one module. Either `diagnostic.rs` defines them directly (current state) or they live in `diag_codes.rs` and are imported by `diagnostic.rs`.

**Affected files:** `crates/vb_validate/src/diagnostic.rs`, `crates/vb_validate/src/diag_render.rs`, `crates/vb_validate/src/diag_codes.rs`  
**Risk tags:** `migration`  
**Behavior affecting:** NO (refactoring)

---

## Clause 8: SourceMap Dead Code Removal (Clause ID: RM-SRCMAP)

### C8.1 — Removal

The dead `SourceMap { _private: () }` placeholder in `vb_core::span` SHALL be removed.

### C8.2 — Re-export Cleanup

All public re-exports of `SourceMap` from `vb_core` SHALL be removed.

### C8.3 — Canonical Type

`vb_yaml::SourceMap` SHALL be the sole `SourceMap` type in the codebase.

**Affected files:** `crates/vb_core/src/span.rs`, `crates/vb_core/src/lib.rs`  
**Risk tags:** `public API`, `migration`  
**Behavior affecting:** NO (dead code removal)

---

## Clause 9: Span Bridging (Clause ID: SPAN-BRIDGE)

### C9.1 — SourceSpan → Span Conversion

A `From<vb_yaml::SourceSpan> for vb_core::Span` implementation SHALL exist in `vb_compile`. Byte offsets, line, and column SHALL be converted from `usize` to `u32` using a lossless-or-clamping strategy.

### C9.2 — SourceMark → Span Conversion

A `From<SourceMark> for vb_core::Span` implementation SHALL exist in `vb_compile`. When `available == true`, line/column SHALL be propagated. When `available == false`, line/column SHALL be `None`.

### C9.3 — Conversion Safety

No bridge conversion SHALL panic. Offsets exceeding `u32::MAX` SHALL be clamped to `u32::MAX`.

**Affected files:** `crates/vb_compile/src/` (NEW bridge module or inline in existing module)  
**Risk tags:** `bounded-state`, `parser/codec`  
**Behavior affecting:** YES

---

## Clause 10: Tree Validation Mark Backfilling (Clause ID: TREE-MARK)

### C10.1 — AstMarks Integration

Tree validation functions (`validate_strict_profile`, `validate_one_node`, etc.) SHALL consult `AstMarks` to obtain `SourceMark`s for errors, replacing `SourceMark::unavailable()` where a mark lookup succeeds.

### C10.2 — Graceful Degradation

When `AstMarks` cannot provide a mark for an error location, `SourceMark::unavailable()` SHALL be used as a fallback.

### C10.3 — Lookup Coverage

The following lookups SHALL be attempted before falling back to `unavailable()`:
- Step-level errors → `AstMarks::step(step_id)`
- Field-level errors → `AstMarks::nested_key(parent, key)`
- Trigger-level errors → `AstMarks::trigger(kind)`
- Document-level errors → `AstMarks::document()`

**Affected files:** `crates/vb_compile/src/mod_compile_validation/part_02.rs`, `crates/vb_compile/src/ast/marks.rs`  
**Risk tags:** `parser/codec`  
**Behavior affecting:** YES

---

## Clause 11: SemanticSourceMap in Error Messages (Clause ID: SEM-MAP-MSG)

### C11.1 — Path Annotation

When a `CompileError` or `ValidationError` can be associated with a YAML author path via `SemanticSourceMap`, the diagnostic message SHALL include the path (e.g., `"unknown field: inputs at path $.inputs"`).

### C11.2 — Additive Only

Path annotation SHALL be appended to the existing message. It SHALL NOT replace the primary error message.

### C11.3 — Optional Dependency

`SemanticSourceMap` SHALL be optional at the diagnostic conversion boundary. Absence of the map SHALL produce the un-annotated message.

**Affected files:** `crates/vb_compile/src/` (diagnostic rendering)  
**Risk tags:** `parser/codec`  
**Behavior affecting:** YES

---

## Clause 12: Backward Compatibility and Test Migration (Clause ID: BACK-COMPAT)

### C12.1 — Test Span::ZERO Assertions

All tests that assert `diagnostic.span == Span::ZERO` SHALL be updated to either:
- Accept both `Span::ZERO` and non-zero spans (if span is now propagated), OR
- Construct errors with explicit `Span::ZERO` and assert the zero.

### C12.2 — Pattern Match Compatibility

All pattern matches on `Span`, `Diagnostic`, `ValidationError`, and `CompileError` using `..` SHALL continue to compile. Exhaustive matches without `..` SHALL be updated.

### C12.3 — CI Gate

`moon ci` SHALL pass. All workspace tests SHALL pass. No new clippy warnings on affected files.

**Affected files:** All test files in `vb_validate`, `vb_compile`, `vb_yaml`, `vb_core`, and `workspace_tests`  
**Risk tags:** `public API`, `migration`  
**Behavior affecting:** YES

---

## Acceptance Gates

| Gate | Criterion | Measurement |
|---|---|---|
| AG1 | All 12 clauses satisfied | Type contracts + tests + evidence |
| AG2 | `Span::ZERO` backward compat | No runtime Span::ZERO behavior change |
| AG3 | Diagnostic shows file:line:col | Compile invalid YAML, check diagnostic output |
| AG4 | No vb_yaml → vb_core dependency | Check Cargo.toml for vb_yaml |
| AG5 | Single canonical diagnostic conversion | Grep `fn diagnostic_from_error` → 1 definition |
| AG6 | `SourceMap` removed from vb_core | No `SourceMap` in `crates/vb_core/src/` |
| AG7 | `NonEmptyVec` enforces len>=1 | Type system prevents empty construction |
| AG8 | YAML parse errors show correct line | Unit test: parse invalid YAML, assert error line |
| AG9 | Validation errors propagate span | Unit test: validation error with span → diagnostic has span |
| AG10 | CI passes | `moon ci` exit code 0 |
