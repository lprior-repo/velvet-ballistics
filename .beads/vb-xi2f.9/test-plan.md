# Test Plan: vb-xi2f.9 — YAML Path and Source Span Diagnostics

**Bead:** vb-xi2f.9 (child of vb-engine-yaml)
**Scope:** Diagnostic source-span enrichment — Span, Diagnostic, NonEmptyVec, YamlError, ValidationError,
SourceMark, span bridge, canonical YAML bridge, AstMarks, SemanticSourceMap
**Agent:** test-planner (State 8)
**Date:** 2026-05-25
**Input artifacts:** contract.md, domain-model.md, type-contracts.md, error-taxonomy.md,
boundary-map.md, hazard-analysis.md, proof-to-rust-map.md, rust-refinement-obligations.jsonl

## Summary

| Metric | Count |
|---|---|
| Behaviors identified | 78 |
| Contract clauses | 12 (C1–C12) |
| Affected crates | vb_core, vb_yaml, vb_compile, vb_validate |
| Public API surfaces | Span, Diagnostic, NonEmptyVec, YamlError, ValidationError, SourceMark, span bridge |
| Trophy allocation | 36 unit / 28 integration / 6 e2e / 8 static + verification |
| Proptest invariants | 9 |
| Fuzz targets | 4 |
| Kani harnesses (planned for test-writer visibility) | 8 groups (PO-K01–PO-K08) |
| Mutation threshold target | >= 90% kill rate |

### Trophy Allocation Rationale

```
         [E2E: 6]          ← Full compilation pipeline + diagnostic rendering
    [Integration: 28]      ← Cross-crate bridges (span_bridge, canonical_yaml,
    /                    \     AstMarks validation, SemanticSourceMap rendering,
   /                      \    diagnostic_from_error with real spans)
  /    [Unit: 36]          \  ← Pure functions: Span constructors, NonEmptyVec,
 /--------------------------\    Diagnostic::new, clamp_u32, YamlError::span(),
/  [Static + Verification: 8]\   Severity, DiagnosticCode parsing
```

**Deviation from 60/30/5/5 target:** ~46% unit, ~36% integration, ~8% e2e, ~10% verification.
Justification: This bead is a *type enrichment* operation — the majority of new behaviors
are pure data transformations (construct newtypes, propagate fields, clamp values).
Every pure function is trivially unit-testable. Integration tests focus on the 3 bridge
boundaries (YamlError→CanonicalYaml, SourceSpan→Span, ValidationError→Diagnostic).
E2E tests verify the full compilation-to-diagnostic-rendering pipeline. Formal verification
(Kani/Miri/Flux) accounts for the static layer.

---

## 1. Behavior Inventory

### Clause 1: Span Enrichment (SPAN-ENRICH) — vb_core::span

| # | Behavior |
|---|---|
| B01 | Span::ZERO maintains backward compatibility: start=0, end=0, line=None, column=None |
| B02 | Span::ZERO is empty (is_empty() returns true) |
| B03 | Span::ZERO equals Span::new(0, 0) |
| B04 | Span::new(start, end) produces a span with byte offsets only; line=None, column=None |
| B05 | Span::new(start, end) preserves start and end values exactly |
| B06 | Span::new(start, end) is_empty when start==end |
| B07 | Span::with_location(start, end, line, column) produces line=Some(line), column=Some(column) |
| B08 | Span::with_location preserves all four fields exactly |
| B09 | Span::location() returns Some((l,c)) when both line and column are Some |
| B10 | Span::location() returns None when line is None (or column is None) |
| B11 | Span::default() equals Span::ZERO |
| B12 | Span paired invariant: line.is_some() == column.is_some() for all public constructors |
| B13 | Span equality considers line and column fields, not just offsets |
| B14 | Span Debug format includes offsets and optional line/column |
| B15 | Span Clone and Copy preserve all fields |
| B16 | Span is Serialize and Deserialize round-trip safe |
| B17 | Located<T> and Spanned<T> hold enriched Span identically |
| B18 | Span::new at max offsets (u32::MAX) does not panic |

### Clause 2: Diagnostic File Path (DIAG-FILE) — vb_core::diagnostic

| # | Behavior |
|---|---|
| B19 | Diagnostic::new with Span::ZERO and source_file:None produces valid diagnostic |
| B20 | Diagnostic::new with source_file:Some(path) preserves path exactly |
| B21 | Diagnostic::new with source_file:None sets source_file.is_none() |
| B22 | Diagnostic backward compat: constructing with Span::ZERO and None works identically to pre-enrichment |
| B23 | DiagnosticCode::new(code) preserves packed u16 value |
| B24 | DiagnosticCode Display formats as EXXXX hex |
| B25 | DiagnosticCode FromStr parses valid hex codes in supported ranges |
| B26 | DiagnosticCode FromStr rejects malformed input: missing E prefix → InvalidFormat |
| B27 | DiagnosticCode FromStr rejects too-short input → InvalidFormat |
| B28 | DiagnosticCode FromStr rejects too-long input → InvalidFormat |
| B29 | DiagnosticCode FromStr rejects non-hex digits → InvalidFormat |
| B30 | DiagnosticCode FromStr rejects unsupported code ranges → UnsupportedCode |
| B31 | DiagnosticCode FromStr rejects empty input → InvalidFormat |
| B32 | Severity has three variants: Error, Warning, Info |
| B33 | Diagnostic carries source_file for authoring-time diagnostics |

### Clause 3: NonEmptyVec (NEVEC) — vb_core::non_empty_vec

| # | Behavior |
|---|---|
| B34 | NonEmptyVec::new(head) produces len() == 1 |
| B35 | NonEmptyVec::new(head).first() returns &head |
| B36 | NonEmptyVec::new(head).is_empty() returns false |
| B37 | NonEmptyVec::with_tail(head, tail) produces len() == 1 + tail.len() |
| B38 | NonEmptyVec::with_tail preserves head + tail order |
| B39 | NonEmptyVec::from_vec(empty) returns None |
| B40 | NonEmptyVec::from_vec(non_empty) returns Some with correct first(), last(), len() |
| B41 | NonEmptyVec::from_vec preserves element order |
| B42 | NonEmptyVec::push(value) increases len and value becomes last element |
| B43 | NonEmptyVec::extend(iter) appends all elements preserving order |
| B44 | NonEmptyVec::into_vec() preserves all elements and order |
| B45 | NonEmptyVec::into_iter() yields head first, then tail in order |
| B46 | NonEmptyVec From trait conversion into Vec preserves all elements |
| B47 | NonEmptyVec Display format renders elements comma-separated |
| B48 | NonEmptyVec::into_vec() on single-element does not double-allocate head |

### Clause 4: YamlError Span Enrichment (YERR-SPAN) — vb_yaml::error

| # | Behavior |
|---|---|
| B49 | Every YamlError variant constructible with span: None (backward compat) |
| B50 | YamlError::span() returns None for limit-only variants: SourceTooLarge, NestingTooDeep, NodeLimitExceeded, EmptySource |
| B51 | YamlError::span() returns Some for span-carrying variants (16 of 20 variants) |
| B52 | YamlError::span() is exhaustive — all 20 variants have a match arm |
| B53 | YamlError::span() returns the exact SourceSpan supplied at construction |
| B54 | YamlError with span: None is Display/Eq-compatible with pre-enrichment |
| B55 | YamlError parse-level variants (ParseError, AnchorAliasMerge, CustomTag, BinaryScalar, AmbiguousScalar) carry span from event stream |

### Clause 5: Canonical YAML Span Preservation (CANON-SPAN) — vb_compile

| # | Behavior |
|---|---|
| B56 | canonical_yaml_error with span-carrying YamlError produces CompileError::CanonicalYaml with correct mark |
| B57 | canonical_yaml_error with span-less YamlError produces mark as SourceMark::unavailable() |
| B58 | canonical_yaml_error never panics for any YamlError variant |
| B59 | yaml_error_category classifies all 20 YamlError variants into 9 categories |
| B60 | CompileError::CanonicalYaml structural stability: category and message fields preserved |

### Clause 6: ValidationError Span Propagation (VERR-SPAN) — vb_validate

| # | Behavior |
|---|---|
| B61 | diagnostic_from_error propagates error.span into Diagnostic.span exactly |
| B62 | diagnostic_from_error with error.span == Span::ZERO produces Diagnostic.span == Span::ZERO |
| B63 | diagnostic_from_error with location-bearing span preserves line and column in output |
| B64 | diagnostic_from_error produces Severity::Error for all variants |
| B65 | diagnostic_from_error produces unique diagnostic codes for all variants (no duplicates) |
| B66 | diagnostic_from_error produces non-empty message for all variants |
| B67 | error_diagnostic_parts covers all ~55 ValidationError variants exhaustively |
| B68 | error_code returns correct DiagnosticCode for each variant |
| B69 | ValidationError variants with structured data preserve all fields through to diagnostic message |
| B70 | ValidationError pattern matches with .. continue to compile (append-only field additions) |

### Clause 7: Diagnostic Conversion Unification (UNIFY-DIAG) — vb_validate

| # | Behavior |
|---|---|
| B71 | Exactly one public canonical diagnostic_from_error function exists |
| B72 | diag_render.rs either removed or re-exports from diagnostic.rs (no duplicate match) |
| B73 | Error code constants defined in exactly one module, imported by diagnostic.rs |

### Clause 8: SourceMap Dead Code Removal (RM-SRCMAP) — vb_core

| # | Behavior |
|---|---|
| B74 | No SourceMap definition or re-export in vb_core/src/ |
| B75 | vb_yaml::SourceMap is the sole canonical SourceMap type project-wide |

### Clause 9: Span Bridging (SPAN-BRIDGE) — vb_compile::span_bridge

| # | Behavior |
|---|---|
| B76 | clamp_u32(0) returns 0 |
| B77 | clamp_u32(x) returns x when x <= u32::MAX |
| B78 | clamp_u32(u32::MAX as usize) returns u32::MAX |
| B79 | clamp_u32(u32::MAX as usize + 1) returns u32::MAX (saturation) |
| B80 | clamp_u32(usize::MAX) returns u32::MAX (no panic) |
| B81 | span_from_source_span converts byte offsets, line, column correctly |
| B82 | span_from_source_span clamps oversized usize values to u32::MAX |
| B83 | span_from_source_span always produces Some for line and column |
| B84 | span_from_source_span never panics for extreme values |
| B85 | SourceMark available=true → Span with Some(line) and Some(column) |
| B86 | SourceMark available=false → Span with None line and column |
| B87 | SourceMark unavailable ignores line/col fields; byte offsets still converted |
| B88 | Span from SourceMark with large values clamps without panic |
| B89 | SourceMark::from_parser_span preserves index, end_index, line, column |
| B90 | SourceMark::from_parser_span always sets available: true |
| B91 | SourceMark::unavailable() has available: false, all fields zero |

### Clause 10: AstMarks (TREE-MARK) — vb_compile::ast::marks

| # | Behavior |
|---|---|
| B92 | AstMarks::empty().document() returns None |
| B93 | AstMarks::empty().nested_key("parent", "key") returns None for any input |
| B94 | AstMarks::empty().trigger("kind") returns None for any input |
| B95 | AstMarks::empty().step("id") returns None for any input |
| B96 | AstMarks::new(valid YAML) successfully parses and populates lookup tables |
| B97 | AstMarks backfills document mark from DocumentStart event |
| B98 | AstMarks backfills step marks from id fields within steps[] sequence |
| B99 | AstMarks backfills nested key marks from parent-key pairs |
| B100 | AstMarks backfills trigger marks from keys under "when" parent |
| B101 | AstMarks lookup returns SourceMark with available: true when match found |
| B102 | AstMarks lookup returns None when no match exists (graceful degradation) |

### Clause 11: SemanticSourceMap in Error Messages (SEM-MAP-MSG) — vb_compile

| # | Behavior |
|---|---|
| B103 | Diagnostic message includes YAML author path when SemanticSourceMap available |
| B104 | Author path is appended to existing message (not a replacement) |
| B105 | Absence of SemanticSourceMap produces un-annotated message |
| B106 | Path annotation function never panics when SemanticSourceMap is None |

### Clause 12: Backward Compatibility (BACK-COMPAT) — workspace

| # | Behavior |
|---|---|
| B107 | All existing tests with Span::ZERO assertions pass after enrichment |
| B108 | Pattern matches with .. on Span, Diagnostic, ValidationError, CompileError continue to compile |
| B109 | moon ci exits 0 on the full workspace |
| B110 | No new clippy warnings on affected files |
| B111 | All crate-level tests pass individually (vb_core, vb_yaml, vb_compile, vb_validate) |

---

## 2. Trophy Allocation

### Unit Tests (36)

All pure Calc-layer functions — no I/O, no cross-crate deps (except within same crate):

| Group | Count | Crate |
|---|---|---|
| Span constructors and invariants | 10 | vb_core |
| Diagnostic construction and source_file | 5 | vb_core |
| DiagnosticCode parsing | 8 | vb_core |
| NonEmptyVec construction and iteration | 10 | vb_core |
| YamlError span() | 3 | vb_yaml |

### Integration Tests (28)

Cross-crate bridges and pipeline components — use REAL dependencies:

| Group | Count | Crates involved |
|---|---|---|
| Span bridge (clamp_u32, SourceSpan→Span, SourceMark→Span) | 8 | vb_compile ↔ vb_core, vb_yaml |
| SourceMark construction and conversion | 4 | vb_compile |
| YamlError span construction and extraction | 3 | vb_yaml ↔ vb_core (via vb_compile) |
| canonical_yaml_error span preservation | 3 | vb_compile ↔ vb_yaml |
| diagnostic_from_error span propagation | 5 | vb_validate ↔ vb_core |
| AstMarks backfill from YAML parsing | 3 | vb_compile ↔ vb_yaml |
| SemanticSourceMap path annotation | 2 | vb_compile ↔ vb_yaml |

### E2E Tests (6)

Full compilation pipeline from YAML text to diagnostic output:

| Scenario | Count |
|---|---|
| Compile invalid YAML → diagnostic shows file:line:col | 2 |
| Compile YAML with validation error → diagnostic has correct span | 2 |
| Compile YAML with known error → rendered output includes YAML author path | 2 |

### Static Analysis + Verification (8)

| Check | Tool |
|---|---|
| SourceMap absent from vb_core | grep |
| Single diagnostic_from_error definition | grep |
| Error code duplicate detection | compile-time test |
| Pattern match .. compatibility | compilation |
| moon ci passes | moon-ci |
| Span paired invariant | Kani (PO-K01) |
| NonEmptyVec invariants | Kani (PO-K02) |
| Bridge panic-freedom | Kani (PO-K07) |

---

## 3. BDD Scenarios

### 3.1 Span Enrichment Scenarios (vb_core)

#### Behavior B01: Span::ZERO maintains backward compatibility

```
fn span_zero_is_backward_compatible()
Given: no preconditions
When: Span::ZERO is referenced
Then: start == 0, end == 0, line == None, column == None
```

#### Behavior B02: Span::ZERO is empty

```
fn span_zero_is_empty()
Given: Span::ZERO
When: is_empty() is called
Then: returns true
```

#### Behavior B03: Span::ZERO equals Span::new(0, 0)

```
fn span_zero_equals_new_zero_zero()
Given: Span::ZERO and Span::new(0, 0)
When: compared for equality
Then: they are equal
```

#### Behavior B04: Span::new produces byte-offset-only span

```
fn span_new_produces_no_location()
Given: Span::new(10, 20)
When: line, column, and location() are inspected
Then: line == None, column == None, location() == None
```

#### Behavior B05: Span::new preserves start and end

```
fn span_new_preserves_offsets()
Given: Span::new(2, 5)
When: start and end are inspected
Then: start == 2, end == 5, is_empty() == false
```

#### Behavior B06: Span::new is_empty when start==end

```
fn span_new_is_empty_when_start_equals_end()
Given: Span::new(100, 100)
When: is_empty() is called
Then: returns true
```

#### Behavior B07: Span::with_location produces paired fields

```
fn span_with_location_produces_paired_fields()
Given: Span::with_location(1, 10, 3, 5)
When: line, column, and location() are inspected
Then: line == Some(3), column == Some(5), location() == Some((3, 5))
```

#### Behavior B08: Span::with_location preserves all fields

```
fn span_with_location_preserves_all_fields()
Given: Span::with_location(1, 10, 3, 5)
When: all fields inspected
Then: start == 1, end == 10, line == Some(3), column == Some(5)
```

#### Behavior B09: Span::location returns Some when both present

```
fn span_location_returns_some_when_both_present()
Given: Span::with_location(0, 10, 42, 99)
When: location() is called
Then: returns Some((42, 99))
```

#### Behavior B10: Span::location returns None when fields absent

```
fn span_location_returns_none_when_no_line_column()
Given: Span::new(0, 10) and Span::ZERO
When: location() is called
Then: returns None for both
```

#### Behavior B11: Span::default equals Span::ZERO

```
fn span_default_equals_zero()
Given: Span::default()
When: compared to Span::ZERO
Then: they are equal, is_empty() is true
```

#### Behavior B12: Span paired invariant

```
fn span_paired_invariant_holds_across_constructors()
Given: all valid constructor combinations (new, with_location, ZERO, default)
When: line.is_some() and column.is_some() are inspected
Then: line.is_some() == column.is_some() for all
```

#### Behavior B13: Span equality considers line and column

```
fn span_equality_considers_line_and_column()
Given: spans with same offsets but different line/column
When: compared for equality
Then: different line → not equal; different column → not equal; same all → equal
```

#### Behavior B14: Span Debug includes offsets

```
fn span_debug_format_contains_offsets()
Given: Span::new(10, 20)
When: Debug formatted
Then: contains "Span"
```

#### Behavior B15: Span Clone preserves equality

```
fn span_clone_preserves_equality()
Given: Span::new(5, 15)
When: cloned
Then: clone equals original
```

#### Behavior B17: Located and Spanned hold enriched Span

```
fn located_and_spanned_hold_enriched_span()
Given: Located::new(value, enriched_span)
When: value and span accessed on both Located and Spanned
Then: both carry the enriched span with line/column intact
```

### 3.2 Diagnostic Scenarios (vb_core)

#### Behavior B19: Diagnostic backward compat with Span::ZERO and source_file None

```
fn diagnostic_backward_compat_span_zero_source_none()
Given: Diagnostic::new(code, "runtime error", Severity::Warning, Span::ZERO, None)
When: span and source_file are inspected
Then: span == Span::ZERO, source_file.is_none()
```

#### Behavior B20: Diagnostic preserves source_file when Some

```
fn diagnostic_preserves_source_file_when_some()
Given: Diagnostic::new(code, "test", Severity::Error, Span::ZERO, Some("workflow.yaml".into()))
When: source_file inspected
Then: source_file.as_deref() == Some("workflow.yaml")
```

#### Behavior B21: Diagnostic source_file absent for runtime

```
fn diagnostic_source_file_none_for_runtime_error()
Given: Diagnostic::new(code, "error", Severity::Error, Span::ZERO, None)
When: source_file inspected
Then: source_file.is_none()
```

#### Behavior B22: Diagnostic carries all fields correctly

```
fn diagnostic_record_owns_message_and_code_and_severity()
Given: Diagnostic::new(code, message, severity, span, source_file)
When: all fields inspected
Then: code, message, severity, span, source_file match inputs exactly
```

### 3.3 DiagnosticCode Scenarios (vb_core)

#### Behavior B23: DiagnosticCode preserves packed value

```
fn diagnostic_code_preserves_packed_value()
Given: DiagnosticCode::new(0x0101)
When: code() is called and Display formatted
Then: code() == 0x0101, Display == "E0101"
```

#### Behavior B25: DiagnosticCode parses valid codes

```
fn diagnostic_code_parses_supported_ranges()
Given: "E0101", "E010B", "E0409", "E040C", "E1314", "E4015"
When: parsed via FromStr
Then: all return Ok with correct packed values
```

#### Behavior B26: DiagnosticCode rejects missing E prefix

```
fn diagnostic_code_rejects_missing_e_prefix()
Given: "0101"
When: parsed via FromStr
Then: Err(DiagnosticCodeParseError::InvalidFormat)
```

#### Behavior B27: DiagnosticCode rejects too-short input

```
fn diagnostic_code_rejects_too_short_input()
Given: "E01"
When: parsed via FromStr
Then: Err(DiagnosticCodeParseError::InvalidFormat)
```

#### Behavior B28: DiagnosticCode rejects too-long input

```
fn diagnostic_code_rejects_too_long_input()
Given: "E010101"
When: parsed via FromStr
Then: Err(DiagnosticCodeParseError::InvalidFormat)
```

#### Behavior B29: DiagnosticCode rejects non-hex digits

```
fn diagnostic_code_rejects_non_hex_digits()
Given: "E010G"
When: parsed via FromStr
Then: Err(DiagnosticCodeParseError::InvalidFormat)
```

#### Behavior B30: DiagnosticCode rejects unsupported ranges

```
fn diagnostic_code_rejects_unsupported_code_ranges()
Given: "E010C", "E0410", "E9999"
When: parsed via FromStr
Then: Err(DiagnosticCodeParseError::UnsupportedCode) for each
```

#### Behavior B31: DiagnosticCode rejects empty input

```
fn diagnostic_code_rejects_empty_input()
Given: ""
When: parsed via FromStr
Then: Err(DiagnosticCodeParseError::InvalidFormat)
```

### 3.4 NonEmptyVec Scenarios (vb_core)

#### Behavior B34: NonEmptyVec::new has len 1

```
fn non_empty_vec_new_has_len_one()
Given: NonEmptyVec::new(42)
When: len() and is_empty() are called
Then: len() == 1, is_empty() == false
```

#### Behavior B35: NonEmptyVec::new first returns head

```
fn non_empty_vec_new_first_returns_head()
Given: NonEmptyVec::new("hello")
When: first() and last() are called
Then: first() == &"hello", last() == &"hello"
```

#### Behavior B37: NonEmptyVec::with_tail correct len

```
fn non_empty_vec_with_tail_correct_len_and_order()
Given: NonEmptyVec::with_tail(1, vec![2, 3, 4])
When: len(), first(), last() are called
Then: len() == 4, first() == &1, last() == &4
```

#### Behavior B39: NonEmptyVec::from_vec empty returns None

```
fn non_empty_vec_from_vec_empty_returns_none()
Given: Vec::<i32>::new()
When: NonEmptyVec::from_vec is called
Then: returns None
```

#### Behavior B40: NonEmptyVec::from_vec non-empty returns Some

```
fn non_empty_vec_from_vec_non_empty_returns_some()
Given: vec![10, 20, 30]
When: NonEmptyVec::from_vec is called
Then: returns Some, first() == &10, len() == 3, last() == &30
```

#### Behavior B42: NonEmptyVec::push increases len

```
fn non_empty_vec_push_increases_len_and_appends()
Given: NonEmptyVec::new(1)
When: push(2) is called
Then: len() == 2, last() == &2
```

#### Behavior B44: NonEmptyVec::into_vec round-trip

```
fn non_empty_vec_into_vec_round_trip_preserves_all()
Given: vec![1, 2, 3, 4, 5]
When: from_vec → into_vec round-trip
Then: result equals original vec
```

#### Behavior B45: NonEmptyVec::into_iter yields head first

```
fn non_empty_vec_into_iter_exhaustive_order()
Given: NonEmptyVec::with_tail(10, vec![20, 30])
When: collected via into_iter()
Then: result == vec![10, 20, 30]
```

#### Behavior B46: NonEmptyVec From trait into Vec

```
fn non_empty_vec_from_trait_into_vec_preserves_elements()
Given: NonEmptyVec::with_tail(7, vec![8, 9])
When: converted via Into<Vec<i32>>
Then: result == vec![7, 8, 9]
```

#### Behavior B47: NonEmptyVec Display renders elements

```
fn non_empty_vec_display_renders_comma_separated()
Given: NonEmptyVec::with_tail(1, vec![2, 3])
When: Display formatted
Then: string contains "1" and "2, 3" (exact format implementation-dependent)
```

### 3.5 YamlError Span Scenarios (vb_yaml)

#### Behavior B49: YamlError variants constructible with span None

```
fn yaml_error_all_variants_constructible_with_none_span()
Given: all 20 YamlError variants
When: each constructed with span: None where applicable
Then: no panic, construction succeeds
```

#### Behavior B50: YamlError::span returns None for limit-only variants

```
fn yaml_error_span_returns_none_for_limit_variants()
Given: SourceTooLarge { size: 100, max: 50 }, NestingTooDeep { depth: 20, max: 16 },
       NodeLimitExceeded { count: 5000, max: 1000 }, EmptySource
When: span() is called
Then: returns None for all four
```

#### Behavior B51: YamlError::span returns Some for span-carrying variants

```
fn yaml_error_span_returns_some_for_span_carrying_variants()
Given: each of the 16 span-carrying YamlError variants constructed with a known SourceSpan
When: span() is called
Then: returns Some(known_source_span) for each
```

#### Behavior B52: YamlError::span is exhaustive

```
fn yaml_error_span_is_exhaustive_all_20_variants()
Given: a YamlError constructed for all 20 variants
When: span() is called on each
Then: no compile error due to non-exhaustive match; each returns Some or None
```

### 3.6 Canonical YAML Scenarios (vb_compile)

#### Behavior B56: canonical_yaml_error preserves span

```
fn canonical_yaml_error_preserves_span_into_mark()
Given: a YamlError::DuplicateKey { key: "x", span: Some(known_source_span) }
When: canonical_yaml_error is called
Then: result is CompileError::CanonicalYaml { mark } where mark.available == true
      and mark.line == known_source_span.start_line
```

#### Behavior B57: canonical_yaml_error produces unavailable for span-less errors

```
fn canonical_yaml_error_produces_unavailable_mark_when_no_span()
Given: a YamlError::SourceTooLarge { size: 100, max: 50 }
When: canonical_yaml_error is called
Then: result is CompileError::CanonicalYaml { mark } where mark == SourceMark::unavailable()
```

#### Behavior B58: canonical_yaml_error never panics

```
fn canonical_yaml_error_never_panics_for_any_variant()
Given: each of the 20 YamlError variants
When: canonical_yaml_error is called
Then: no panic for any variant
```

#### Behavior B59: yaml_error_category covers all variants

```
fn yaml_error_category_classifies_all_20_variants()
Given: each of the 20 YamlError variants
When: yaml_error_category is called
Then: returns a non-empty category string; all 9 expected categories are represented
```

### 3.7 ValidationError Span Propagation Scenarios (vb_validate)

#### Behavior B61: diagnostic_from_error propagates span

```
fn diagnostic_from_error_propagates_span_exactly()
Given: ValidationError::ControlFlowCycle { span: Span::with_location(10, 20, 3, 5) }
When: diagnostic_from_error is called
Then: diagnostic.span == Span::with_location(10, 20, 3, 5)
```

#### Behavior B62: diagnostic_from_error backward compat with Span::ZERO

```
fn diagnostic_from_error_produces_zero_span_for_zero_span_error()
Given: ValidationError::DuplicateKey { span: Span::ZERO }
When: diagnostic_from_error is called
Then: diagnostic.span == Span::ZERO
```

#### Behavior B64: diagnostic_from_error produces Severity::Error

```
fn diagnostic_from_error_severity_is_always_error()
Given: all ~55 ValidationError variants
When: diagnostic_from_error is called for each
Then: severity == Severity::Error for all
```

#### Behavior B65: diagnostic_from_error produces unique codes

```
fn diagnostic_from_error_all_variants_have_unique_codes()
Given: all ~55 ValidationError variants
When: error_code is called for each
Then: all codes are non-zero and unique (no duplicates)
```

#### Behavior B66: diagnostic_from_error produces non-empty message

```
fn diagnostic_from_error_all_variants_have_non_empty_message()
Given: all ~55 ValidationError variants
When: diagnostic_from_error is called for each
Then: message is non-empty for all
```

#### Behavior B67: error_diagnostic_parts covers all variants

```
fn error_diagnostic_parts_is_exhaustive_over_all_variants()
Given: all ~55 ValidationError variants enumerated
When: error_diagnostic_parts is called (indirectly via diagnostic_from_error)
Then: no compile error due to non-exhaustive match; all produce (code, message, span)
```

#### Behavior B69: Diagnostic message includes structured data

```
fn diagnostic_message_includes_variant_specific_data()
Given: ValidationError::InvalidId { id: "bad-id", span: Span::ZERO }
When: diagnostic_from_error is called
Then: message contains "bad-id"
```

### 3.8 Span Bridge Scenarios (vb_compile)

#### Behavior B76: clamp_u32 zero returns zero

```
fn clamp_u32_zero_returns_zero()
Given: clamp_u32(0)
When: result is inspected
Then: returns 0_u32
```

#### Behavior B77: clamp_u32 identity within range

```
fn clamp_u32_identity_for_values_within_u32_range()
Given: clamp_u32(42), clamp_u32(u32::MAX as usize)
When: results are inspected
Then: returns 42 and u32::MAX respectively
```

#### Behavior B79: clamp_u32 saturates above u32::MAX

```
fn clamp_u32_saturates_values_above_u32_max()
Given: clamp_u32(u32::MAX as usize + 1), clamp_u32(usize::MAX)
When: results are inspected
Then: both return u32::MAX
```

#### Behavior B81: span_from_source_span converts correctly

```
fn span_from_source_span_converts_typical_values()
Given: SourceSpan::new(10, 20, 3, 5, 3, 9)
When: span_from_source_span is called
Then: start == 10, end == 20, line == Some(3), column == Some(5)
```

#### Behavior B82: span_from_source_span clamps oversized values

```
fn span_from_source_span_clamps_oversized_values()
Given: SourceSpan with all fields set to u32::MAX as usize + 100
When: span_from_source_span is called
Then: all u32 fields equal u32::MAX
```

#### Behavior B85: SourceMark available produces Some line/column

```
fn source_mark_available_produces_some_line_column_when_converted_to_span()
Given: SourceMark { index: 5, end_index: 15, line: 3, column: 8, available: true }
When: converted to Span via From
Then: start == 5, end == 15, line == Some(3), column == Some(8)
```

#### Behavior B86: SourceMark unavailable produces None line/column

```
fn source_mark_unavailable_produces_none_line_column_when_converted_to_span()
Given: SourceMark::unavailable()
When: converted to Span via From
Then: start == 0, end == 0, line == None, column == None
```

#### Behavior B87: SourceMark unavailable ignores stored line/col values

```
fn source_mark_unavailable_ignores_line_col_values()
Given: SourceMark { index: 100, end_index: 200, line: 5, column: 10, available: false }
When: converted to Span via From
Then: line == None, column == None (values ignored)
```

#### Behavior B89: SourceMark::from_parser_span preserves data

```
fn source_mark_from_parser_span_preserves_index_line_column()
Given: a saphyr_parser::Span with known index, line, col
When: SourceMark::from_parser_span is called
Then: index, end_index, line, column match input; available == true
```

### 3.9 AstMarks Scenarios (vb_compile)

#### Behavior B92-B95: AstMarks::empty returns None for all lookups

```
fn ast_marks_empty_document_returns_none()
Given: AstMarks::empty()
When: document() is called
Then: returns None

fn ast_marks_empty_nested_key_returns_none_for_any_input()
Given: AstMarks::empty()
When: nested_key("parent", "key") is called
Then: returns None

fn ast_marks_empty_trigger_returns_none_for_any_input()
Given: AstMarks::empty()
When: trigger("cron") is called
Then: returns None

fn ast_marks_empty_step_returns_none_for_any_input()
Given: AstMarks::empty()
When: step("step1") is called
Then: returns None
```

#### Behavior B96: AstMarks::new parses valid YAML

```
fn ast_marks_new_parses_valid_yaml_successfully()
Given: valid minimal workflow YAML text
When: AstMarks::new(source) is called
Then: returns Ok(AstMarks), result is not empty
```

#### Behavior B97-B100: AstMarks backfills marks for known structures

```
fn ast_marks_backfills_document_mark_from_document_start()
Given: valid YAML with a document
When: AstMarks::new is called and document() queried
Then: returns Some(SourceMark) with available == true

fn ast_marks_backfills_step_marks_from_steps_array()
Given: valid YAML with steps each having an id
When: AstMarks::new is called and step("step_id") queried
Then: returns Some(SourceMark) with available == true
```

#### Behavior B101: AstMarks lookup returns available mark when match found

```
fn ast_marks_lookup_returns_available_mark_when_match_found()
Given: AstMarks populated from YAML with a known step "build"
When: step("build") is called
Then: returns Some(mark) where mark.available == true
```

#### Behavior B102: AstMarks lookup degrades gracefully

```
fn ast_marks_lookup_returns_none_when_no_match()
Given: AstMarks populated from YAML without step "nonexistent"
When: step("nonexistent") is called
Then: returns None
```

### 3.10 SemanticSourceMap Scenarios (vb_compile)

#### Behavior B103: Diagnostic message includes YAML author path

```
fn diagnostic_message_includes_yaml_author_path_when_map_available()
Given: a CompileError with known SourceMark matching a SemanticSourceMap entry
When: the error is rendered to a Diagnostic with the semantic map
Then: the diagnostic message contains the YAML author path (e.g., "$.inputs" or "$.steps.build")
```

#### Behavior B105: Diagnostic message un-annotated when map absent

```
fn diagnostic_message_unannotated_when_semantic_map_absent()
Given: a CompileError with known SourceMark
When: the error is rendered to a Diagnostic without a semantic map
Then: the diagnostic message does NOT contain path decoration
```

#### Behavior B106: Path annotation never panics with absent map

```
fn path_annotation_never_panics_when_semantic_map_is_none()
Given: None as SemanticSourceMap
When: render_error_with_path is called with any error
Then: no panic, message is unchanged
```

### 3.11 Backward Compatibility Scenarios (workspace)

#### Behavior B108: Pattern matches with .. compile

```
fn pattern_match_backward_compat_continues_to_compile()
Given: code matching Span { start, end, .. } or ValidationError::ControlFlowCycle { .. }
When: compiled
Then: compilation succeeds (verified by static test)
```

---

## 4. Proptest Invariants

### 4.1 Span Paired Invariant (PO-P01) — vb_core

**Invariant:** For any (start, end, line, col) in u32 range with start <= end:
`Span::with_location(start, end, line, col)` produces
`line.is_some() == column.is_some()` and both are `Some`.

**Strategy:** Arbitrary u32 values for all four fields. Filter: `start <= end`.
**Anti-invariant:** `Span::new(start, end)` always produces `line.is_none() && column.is_none()`.

**Test ref:** `crates/vb_core/tests/proptest_span.rs`

### 4.2 Span Serialization Round-trip — vb_core

**Invariant:** Any valid `Span` (constructed via new, with_location, ZERO, default)
serializes to JSON/Postcard and deserializes back to the identical Span.

**Strategy:** Arbitrary Spans from all constructors, serde round-trip.
**Anti-invariant:** (none — round-trip always succeeds for valid spans)

### 4.3 NonEmptyVec Round-trip (PO-P02) — vb_core

**Invariant:** For any non-empty `Vec<T>`, `NonEmptyVec::from_vec(v)` produces `Some(nev)`
where `nev.into_vec() == v` and `nev.len() == v.len()`.

**Strategy:** Arbitrary non-empty Vec<i32> with size 1..100.
**Anti-invariant:** `from_vec(empty_vec)` always returns `None`.

**Test ref:** `crates/vb_core/tests/proptest_non_empty_vec.rs`

### 4.4 NonEmptyVec Iteration Order — vb_core

**Invariant:** For any `NonEmptyVec<T>`: `nev.into_iter().collect::<Vec<T>>()` equals
`nev.into_vec()` and preserves order head first, then tail.

**Strategy:** Arbitrary NonEmptyVec<i32> constructed from random head + tail of size 0..50.
**Anti-invariant:** (none — iteration always yields at least head)

### 4.5 YamlError Span Round-trip (PO-P03) — vb_yaml

**Invariant:** For any span-carrying YamlError variant, constructing with
`span: Some(source_span)` and calling `span()` returns `Some(source_span)`.

**Strategy:** Arbitrary SourceSpan values, variant selection via weighted strategy.
**Anti-invariant:** Limit-only variants always return `None` from `span()`.

**Test ref:** `crates/vb_yaml/tests/proptest_yaml_error.rs`

### 4.6 ValidationError Span Propagation (PO-P04) — vb_validate

**Invariant:** For any ValidationError variant and any Span value:
`diagnostic_from_error(error).span == error.span`.

**Strategy:** Arbitrary span, arbitrary variant from weighted strategy.
**Anti-invariant:** `diagnostic_from_error` never panics for any variant.

**Test ref:** `crates/vb_validate/tests/proptest_validation_error.rs`

### 4.7 Span Bridge Conversion Round-trip (PO-P05) — vb_compile

**Invariant:** For any SourceSpan: `span_from_source_span(ss)` produces a Span where:
- start == clamp_u32(ss.start_offset)
- end == clamp_u32(ss.end_offset)
- line == Some(clamp_u32(ss.start_line))
- column == Some(clamp_u32(ss.start_col))

**Strategy:** Arbitrary usize values for offset, line, column (including values > u32::MAX).
**Anti-invariant:** Values > u32::MAX are clamped, not truncated.

**Test ref:** `crates/vb_compile/tests/proptest_span_bridge.rs`

### 4.8 SourceMark → Span Flag Behavior — vb_compile

**Invariant:** For any SourceMark: converting to Span via `From`:
- When `available == true`: line/column are `Some(clamp_u32(mark.line))` and `Some(clamp_u32(mark.column))`
- When `available == false`: line/column are `None` regardless of mark.line/mark.column values

**Strategy:** Arbitrary SourceMark with random available flag and random line/col/offset.
**Anti-invariant:** (none — flag behavior is deterministic)

### 4.9 AstMarks Backfill Coverage (PO-P06) — vb_compile

**Invariant:** For valid YAML workflow text with steps and nested keys, AstMarks::new
produces a marks structure where step(), nested_key(), trigger(), and document()
lookups for entities present in the YAML return `Some(mark)` with `mark.available == true`.

**Strategy:** Generated valid YAML text fragments, parse, verify marks.
**Anti-invariant:** Lookups for entities not present return None.

**Test ref:** `crates/vb_compile/tests/proptest_ast_marks.rs`

---

## 5. Fuzz Targets

### 5.1 Fuzz: diagnostic_from_error (vb_validate)

**Input type:** Fuzzed `ValidationError` struct (arbitrary bytes deserialized as structured input).
**Risk:** Missing match arm causing panic in `error_diagnostic_parts`. If a new variant is
added without a match arm, the compiler only catches it if exhaustive matching is used.
Fuzzing handles deserialized/constructed variants at the boundary.
**Corpus seeds:**
- All 55 known variants with Span::ZERO
- All 55 known variants with Span::with_location(0, 10, 1, 1)
- `ValidationError` with all-zero struct (if applicable)

### 5.2 Fuzz: DiagnosticCode::from_str (vb_core)

**Input type:** Arbitrary `&str` (fuzzed bytes as UTF-8).
**Risk:** Panic on invalid UTF-8, stack overflow on long input, incorrect code range
classification allowing invalid codes through.
**Corpus seeds:**
- "E0101" (valid)
- "E010C" (valid format, unsupported range)
- "E401B" (valid, top of range)
- "E0000" (all zeros)
- "" (empty)
- "G0101" (wrong prefix)
- "E" followed by 4MB of hex digits (length attack)

### 5.3 Fuzz: clamp_u32 / span_from_source_span (vb_compile)

**Input type:** Arbitrary `usize` byte sequences reinterpreted as SourceSpan fields.
**Risk:** `usize → u32` truncation causing silent data loss, arithmetic overflow in
saturation logic, panic in `u32::try_from` edge cases.
**Corpus seeds:**
- SourceSpan { 0, 0, 0, 0, 0, 0 }
- SourceSpan { u32::MAX, u32::MAX, u32::MAX, u32::MAX, u32::MAX, u32::MAX }
- SourceSpan { u32::MAX+1, u32::MAX+1, ... }
- SourceSpan { usize::MAX, usize::MAX, ... }

### 5.4 Fuzz: AstMarks::new (vb_compile)

**Input type:** Arbitrary `&str` (fuzzed YAML text).
**Risk:** Parser panic on malformed input, OOM on deeply nested structures, infinite loop
in MarkBuilder state machine, incorrect SourceMark offsets for edge-case input.
**Corpus seeds:**
- Minimal valid workflow YAML
- Deeply nested mappings (near but within YAML limits)
- YAML with unicode keys and values
- YAML with empty document
- YAML with BOM
- YAML with trailing whitespace / comments

---

## 6. Kani Harnesses (Verification Checkpoints)

These harnesses exist (or must exist) for formal verification. The test-writer coordinates
with the proof-writer to ensure harness coverage:

| Harness Group | Property | Bound | Status (from proof-to-rust-map.md) |
|---|---|---|---|
| PO-K01: span_paired_invariant | line.is_some() == column.is_some() for all constructors | u32::any(), unwind 3 | VERIFIED (5/5) |
| PO-K02: nev_invariants | NonEmptyVec len>=1, is_empty==false, from_vec empty→None, first never panics | tail size 0..15 | PARTIAL (6/7, 1 timeout) |
| PO-K03: diag_source_file_invariant | Diagnostic source_file preserved exactly | unwind 2 | VERIFIED (4/4) |
| PO-K04: yaml_error_none_span_legal | All 20 variants constructible with span:None | unwind 3 | VERIFIED (5/5) |
| PO-K05: canonical_yaml_no_panic | canonical_yaml_error never panics, category exhaustive | unwind 5 | BLOCKED (span propagation unimplemented) |
| PO-K06: validation_error_span_propagation | diagnostic_from_error propagates span exactly, exhaustive ~55 variants | unwind 5 | VERIFIED (1/1) |
| PO-K07: span_bridge_no_panic | clamp_u32 never panics, SourceSpan→Span safe for all usize | unwind 5 | VERIFIED (9/9) |
| PO-K08: ast_marks_empty_invariants | AstMarks::empty() returns None for all lookups | unwind 10 | VERIFIED (7/7) |

**Gap:** PO-K02 into_vec_round_trip harness times out due to unbounded Vec generation.
Remediation: add `kani::assume(tail.len() <= 15)` bounds.

---

## 7. Mutation Testing Checkpoints

**Threshold:** >= 90% mutation kill rate across affected modules.

### Critical Mutations to Survive

| Mutation | File | Must Be Caught By |
|---|---|---|
| `clamp_u32`: replace `unwrap_or(u32::MAX)` with `unwrap()` | span_bridge.rs:25 | `clamp_u32_boundary_values` |
| `span_from_source_span`: swap line/column fields | span_bridge.rs:44-45 | `source_span_to_span_typical` |
| `SourceMark → Span`: always set available branch to true | span_bridge.rs:64-74 | `source_mark_unavailable_produces_none_line_col` |
| `Span::with_location`: swap line/column | span.rs:58-59 | `with_location_produces_paired_fields` |
| `Span::location`: remove line/column check | span.rs:72-75 | `span_location_returns_none_when_no_line_column` |
| `NonEmptyVec::from_vec`: remove `if vec.is_empty()` guard | non_empty_vec.rs:41-42 | `from_vec_returns_none_for_empty` |
| `diagnostic_from_error`: ignore error.span, always use Span::ZERO | diagnostic.rs:95-96 | `diagnostic_from_error_propagates_span_exactly` |
| `YamlError::span()`: remove a match arm (partial match) | error.rs:148-171 | `yaml_error_span_is_exhaustive_all_20_variants` |
| `error_diagnostic_parts`: map DuplicateKey to wrong code | diagnostic.rs:111-115 | `duplicate_key_maps_to_e0101` |
| `AstMarks::empty().step()`: return Some(unavailable_mark) | marks.rs:77-79 | `ast_marks_empty_step_returns_none_for_any_input` |

### Mutation Resistant Design

- **Type-driven:** `NonEmptyVec`'s invariant is enforced by the struct — no mutation can
  create an empty NonEmptyVec from public constructors
- **Exhaustive match:** All match arms on enums use exhaustive patterns, making missing-arm
  mutations compile-time errors
- **Assertion strength:** All tests assert exact values (e.g., `assert_eq!(span.line, Some(3))`)
  rather than existence checks (e.g., `assert!(span.line.is_some())`)

---

## 8. Combinatorial Coverage Matrix

### 8.1 Span Constructors (vb_core)

| Scenario | Input | Expected Output | Test Layer |
|---|---|---|---|
| ZERO construct | `Span::ZERO` | `start=0, end=0, line=None, column=None, is_empty()=true` | unit |
| new minimal | `Span::new(0, 0)` | `start=0, end=0, line=None, column=None` | unit |
| new typical | `Span::new(5, 10)` | `start=5, end=10, line=None, column=None, is_empty=false` | unit |
| new max offsets | `Span::new(u32::MAX, u32::MAX)` | `start=u32::MAX, end=u32::MAX, is_empty=true` | unit |
| with_location min | `with_location(0, 5, 1, 1)` | `line=Some(1), column=Some(1), location=Some((1,1))` | unit |
| with_location max | `with_location(0, 5, u32::MAX, u32::MAX)` | `line=Some(u32::MAX), column=Some(u32::MAX)` | unit |
| location present | Span with line/col set | `location() = Some((l, c))` | unit |
| location absent | Span::new or Span::ZERO | `location() = None` | unit |
| default | `Span::default()` | equals `Span::ZERO` | unit |
| equality same | two identical spans | `==` true | unit |
| equality different offsets | two spans different start | `!=` true | unit |
| equality different line | two spans different line | `!=` true | unit |
| clone preserves | `span.clone()` | `clone == original` | unit |
| is_empty zero span | `Span::ZERO` | `true` | unit |
| is_empty single byte | `Span::new(5, 6)` | `false` | unit |
| serialization round-trip | any valid Span | deserialized equals original | unit |

### 8.2 DiagnosticCode Parsing (vb_core)

| Scenario | Input | Expected | Layer |
|---|---|---|---|
| happy: E0101 | `"E0101"` | `Ok(0x0101)` | unit |
| happy: E010B | `"E010B"` | `Ok(0x010B)` | unit |
| happy: E401B | `"E401B"` | `Ok(0x401B)` | unit |
| happy: E4015 | `"E4015"` | `Ok(0x4015)` | unit |
| error: no prefix | `"0101"` | `Err(InvalidFormat)` | unit |
| error: wrong prefix | `"G0101"` | `Err(InvalidFormat)` | unit |
| error: too short | `"E01"` | `Err(InvalidFormat)` | unit |
| error: too long | `"E010101"` | `Err(InvalidFormat)` | unit |
| error: non-hex | `"E010G"` | `Err(InvalidFormat)` | unit |
| error: empty | `""` | `Err(InvalidFormat)` | unit |
| error: unsupported range low | `"E010C"` | `Err(UnsupportedCode)` | unit |
| error: unsupported range gap | `"E0410"` | `Err(UnsupportedCode)` | unit |
| error: unsupported range high | `"E9999"` | `Err(UnsupportedCode)` | unit |

### 8.3 NonEmptyVec Construction (vb_core)

| Scenario | Input | Expected | Layer |
|---|---|---|---|
| new single | `new(42)` | `len=1, first=42, last=42, is_empty=false` | unit |
| with_tail typical | `with_tail(1, vec![2,3,4])` | `len=4, first=1, last=4` | unit |
| with_tail empty tail | `with_tail(1, vec![])` | `len=1, first=1, last=1` | unit |
| with_tail max tail | `with_tail(0, vec of size 10_000)` | `len=10_001, first=0, last=9_999` | unit |
| from_vec empty | `from_vec(vec![])` | `None` | unit |
| from_vec single | `from_vec(vec![42])` | `len=1, first=42` | unit |
| from_vec multi | `from_vec(vec![10,20,30])` | `len=3, first=10, last=30` | unit |
| push on single | `new(0); push(1)` | `len=2, last=1` | unit |
| push on multi | `with_tail(1, vec![2]); push(3)` | `len=3, last=3` | unit |
| extend | `new(0); extend(1..3)` | `len=3, last=2` | unit |
| into_vec round-trip | `from_vec(vec![1,2,3,4,5]).into_vec()` | `== vec![1,2,3,4,5]` | unit |
| into_iter order | `with_tail(10, vec![20,30]).into_iter().collect()` | `== vec![10,20,30]` | unit |
| from trait | `NonEmptyVec::with_tail(7, [8,9]): Vec<_>` | `== vec![7,8,9]` | unit |

### 8.4 YamlError::span() (vb_yaml)

| Scenario | Input Variant | Expected span() | Layer |
|---|---|---|---|
| span-carrying: DuplicateKey | `DuplicateKey { key, span: Some(ss) }` | `Some(ss)` | unit |
| span-carrying: ParseError | `ParseError { line, reason, span: Some(ss) }` | `Some(ss)` | unit |
| span-carrying: UnknownField | `UnknownField { field, span: Some(ss) }` | `Some(ss)` | unit |
| span-carrying: ForbiddenFeature | `ForbiddenFeature { detail, span: Some(ss) }` | `Some(ss)` | unit |
| span-absent: SourceTooLarge | `SourceTooLarge { size, max }` | `None` | unit |
| span-absent: NestingTooDeep | `NestingTooDeep { depth, max }` | `None` | unit |
| span-absent: NodeLimitExceeded | `NodeLimitExceeded { count, max }` | `None` | unit |
| span-absent: EmptySource | `EmptySource` | `None` | unit |
| span as None | any variant with `span: None` | `None` | unit |
| all variants exhaustive | all 20 variants | no compile error | unit |

### 8.5 Span Bridge (vb_compile)

| Scenario | Input | Expected Output | Layer |
|---|---|---|---|
| clamp_u32 zero | `0` | `0` | integration |
| clamp_u32 normal | `42` | `42` | integration |
| clamp_u32 boundary u32::MAX | `u32::MAX as usize` | `u32::MAX` | integration |
| clamp_u32 boundary +1 | `u32::MAX as usize + 1` | `u32::MAX` | integration |
| clamp_u32 extreme usize::MAX | `usize::MAX` | `u32::MAX` | integration |
| span_from_source_span typical | SourceSpan(10, 20, 3, 5, 3, 9) | Span(10, 20, Some(3), Some(5)) | integration |
| span_from_source_span minimal | SourceSpan(0, 0, 1, 1, 1, 3) | Span(0, 0, Some(1), Some(1)), is_empty=true | integration |
| span_from_source_span clamped | SourceSpan(>u32::MAX, ...) | Span(u32::MAX, u32::MAX, Some(u32::MAX), Some(u32::MAX)) | integration |
| SourceMark available → Span | mark { idx=5, end=15, line=3, col=8, avail=true } | Span(5, 15, Some(3), Some(8)) | integration |
| SourceMark unavailable → Span | SourceMark::unavailable() | Span(0, 0, None, None) | integration |
| SourceMark unavailable ignores fields | { idx=100, end=200, line=5, col=10, avail=false } | Span(100, 200, None, None) | integration |
| SourceMark available clamped | { huge vals, avail=true } | Span(u32::MAX, u32::MAX, Some(u32::MAX), Some(u32::MAX)) | integration |
| SourceMark::from_parser_span | saphyr parser Span | { line, col, index, end_index preserved, avail=true } | integration |
| SourceMark::unavailable | SourceMark::unavailable() | avail=false, all fields 0 | integration |
| bridge extreme values no panic | all boundary values | no panic | integration |

### 8.6 ValidationError → Diagnostic (vb_validate)

| Scenario | Input | Expected | Layer |
|---|---|---|---|
| span propagation: location | `ControlFlowCycle { span: with_location(10, 20, 3, 5) }` | `diag.span == with_location(10, 20, 3, 5)` | integration |
| span propagation: ZERO | `DuplicateKey { span: Span::ZERO }` | `diag.span == Span::ZERO` | integration |
| code: DuplicateKey | `DuplicateKey {..}` | `diag.code == E0101 (0x0101)` | integration |
| code: InvalidVersion | `InvalidVersion {..}` | `diag.code == E0106 (0x0106)` | integration |
| code: UnknownReference | `UnknownReference {..}` | `diag.code == E0201 (0x0201)` | integration |
| code: SecretResultLeak | `SecretResultLeak {..}` | `diag.code == E0406 (0x0406)` | integration |
| msg: MissingRequiredField | `MissingRequiredField { field: "steps" }` | message contains "steps" | integration |
| msg: InvalidId | `InvalidId { id: "bad-id" }` | message contains "bad-id" | integration |
| msg: TypeMismatch | `TypeMismatch { expected: "bool", found: "num" }` | message contains "bool" AND "num" | integration |
| severity all variants | all ~55 variants | `diag.severity == Severity::Error` for all | integration |
| unique codes | all ~55 variants | no duplicate DiagnosticCode values | integration |
| non-empty message | all ~55 variants | `!diag.message.is_empty()` for all | integration |

### 8.7 AstMarks (vb_compile)

| Scenario | Input | Expected | Layer |
|---|---|---|---|
| empty document | `AstMarks::empty().document()` | `None` | integration |
| empty nested_key | `AstMarks::empty().nested_key("x", "y")` | `None` | integration |
| empty trigger | `AstMarks::empty().trigger("z")` | `None` | integration |
| empty step | `AstMarks::empty().step("s")` | `None` | integration |
| parse valid YAML | valid workflow YAML | `Ok(AstMarks)` | integration |
| document mark present | YAML with content | `document() → Some(mark)` | integration |
| step mark present | YAML with `steps[0].id: "build"` | `step("build") → Some(mark), mark.avail=true` | integration |
| nested key present | YAML with parent.key | `nested_key("parent", "key") → Some(mark)` | integration |
| trigger present | YAML with `when.cron: "..."` | `trigger("cron") → Some(mark)` | integration |
| lookup miss | non-existent step id | `step("gone") → None` | integration |

### 8.8 E2E: Compilation Pipeline → Diagnostic Output

| Scenario | Input | Expected | Layer |
|---|---|---|---|
| parse error shows line | YAML with syntax error at line 3 | diagnostic span.line == Some(3) | e2e |
| duplicate key shows location | YAML with `steps: ... steps:` | diagnostic span.line == Some(line_of_duplicate) | e2e |
| validation error has span | YAML with unknown field `$.steps[0].badfield` | diagnostic.span carries non-zero offsets | e2e |
| missing field shows path | YAML missing `version` | diagnostic message contains path `$.version` | e2e |
| backward compat | runtime diagnostic (no YAML input) | `span == Span::ZERO`, `source_file.is_none()` | e2e |
| pattern match compat | code matching `Span { .. }` | compilation succeeds | e2e |

---

## 9. Static Analysis Gates

These are verified by build/analysis tools, not by runtime tests:

| Gate ID | Check | Tool | Expected |
|---|---|---|---|
| PO-G01 | No `SourceMap` in `crates/vb_core/src/` | grep | zero matches |
| PO-G02 | Single `pub fn diagnostic_from_error` in `crates/vb_validate/src/` | grep | exactly 1 definition (not counting tests) |
| PO-G03 | `moon ci` passes | moon-ci | exit code 0, all tests pass |
| PO-G04 | `cargo test --workspace` passes | cargo test | exit code 0 |
| STA-01 | No `unsafe` in enriched modules | clippy | `#![forbid(unsafe_code)]` holds |
| STA-02 | No `unwrap`, `expect`, `panic` in new code | clippy | zero occurrences |
| STA-03 | Pattern matches using `..` compile on all enriched types | cargo check | compilation succeeds |
| STA-04 | `cargo clippy --workspace` no new warnings | clippy | exit code 0 |

---

## 10. Known Gaps and Test Debt

| Gap ID | Description | Test Impact |
|---|---|---|
| GAP-DIAG-001 | PO-K02 Kani into_vec_round_trip harness times out; needs `kani::assume` bounds | `into_vec_round_trip` regression test must exercise large (10k+ elements) NonEmptyVec |
| GAP-DIAG-002 | Span propagation from YamlError into CanonicalYaml not yet implemented | Tests for B56-B58 must be written AFTER implementation completes |
| GAP-DIAG-003 | Flux RS annotations (PO-F01) are planned but not yet written; Span paired invariant verified by Kani | No test impact — Kani is the canonical proof for the paired invariant |
| GAP-DIAG-004 | moon ci not yet executed in this workspace | B109, B111 must be verified once CI gate is run |
| GAP-DIAG-009 | PO-K02 harness design defect (unbounded vec generation) | Proptest (PO-P02) already covers the into_vec round-trip with bounded inputs; Kani remediation is defense-in-depth |

---

## 11. Test Execution Order (Recommended for test-writer)

1. **vb_core unit tests** — Span, Diagnostic, NonEmptyVec (pure, no dependencies)
2. **vb_yaml unit tests** — YamlError::span() (depends on SourceSpan, no vb_core dep)
3. **vb_compile integration tests** — span_bridge, SourceMark, AstMarks (depends on vb_core + vb_yaml)
4. **vb_validate integration tests** — diagnostic_from_error (depends on vb_core)
5. **Proptest suites** — property invariants across all crates
6. **Fuzz targets** — parsing boundaries
7. **E2E tests** — full compilation pipeline
8. **moon ci** — workspace-wide gate

---

## 12. Exit Criteria Checklist

- [x] Every public API behavior has at least one BDD scenario (78 behaviors, 78+ scenarios)
- [x] Every pure function with multiple inputs has at least one proptest invariant (9 invariants)
- [x] Every parsing/deserialization boundary has a fuzz target (4 fuzz targets)
- [x] Every error variant in YamlError and ValidationError has an explicit test scenario
- [x] Mutation threshold target (≥90%) is stated
- [x] No test asserts only `is_ok()` or `is_err()` without specifying the exact value
- [x] Gaps and test debt explicitly documented

---

## Open Questions

1. **Should the E2E diagnostic rendering tests use `insta` snapshot testing or explicit field assertions?**
   Recommendation: explicit field assertions for the span/line/column fields; insta snapshots for the
   full text rendering output if the rendering format is stable.

2. **Should `NonEmptyVec` gain `Deserialize` with validation?** The hazard analysis (HA-12) notes it does
   not currently derive `Deserialize`. If serialization is added, a validation test for the deserialize
   boundary must be added to this plan.

3. **Are there any existing tests that assert `span == Span::ZERO` and will break?** The backward
   compatibility clause (C12.1) requires these to be updated. The test-writer should run the full
   test suite before implementing new tests to identify all such assertions.

4. **What is the rust-toolchain version for this workspace?** The Miri obligation (PO-M01) requires
   nightly. The test-writer must ensure toolchain compatibility before running Miri tests.

---

*STATUS: test-plan complete — ready for test-reviewer review*
