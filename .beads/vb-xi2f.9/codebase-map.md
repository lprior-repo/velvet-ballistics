# Codebase Map: YAML Path and Source Span Diagnostic Enrichment

**Bead:** vb-xi2f.9  
**Agent:** explore (State 2)  
**Date:** 2026-05-24

## Executive Summary

The codebase has **three independent source-location subsystems** with **no bridge** between them.
Diagnostics in `vb_core` use a byte-offset-only `Span` type with no file path or line/column info.
The `vb_yaml` crate has rich `SourceSpan`/`SourceMap`/`SemanticSourceMap` tracking but none of it
surfaces into diagnostics. Validation errors (`vb_validate`) always emit `Span::ZERO`.
Compilation errors (`vb_compile`) carry their own `SourceMark` type but many paths discard spans.
The `SourceMap` in `vb_core` is a dead placeholder.

## Crate Dependency Graph (relevant subset)

```
vb_yaml  (no dep on vb_core)
  ├── saphyr-parser  (provides event-level Span with byte offset + line + col)
  └── saphyr         (tree API; DISCARDED spans)

vb_validate
  └── vb_core        (uses Diagnostic, Span)

vb_compile
  ├── vb_core        (uses ConstValue, SlotIdx, StepIdx, etc. — NOT Diagnostic/Span)
  ├── vb_validate    (uses ValidationError)
  ├── vb_yaml        (uses YamlError, WorkflowSource, parse_workflow_source)
  └── saphyr / saphyr-parser
```

**Critical architectural observation:** `vb_yaml` does NOT depend on `vb_core`, so `span.rs`
cannot hold YAML-level source-map types without creating a dependency cycle or relocating types.

## Key Files

### 1. Core Diagnostic Infrastructure (`vb_core`)
| File | Key Symbols | Notes |
|---|---|---|
| `crates/vb_core/src/diagnostic.rs` | `Diagnostic`, `DiagnosticCode`, `Severity`, `DiagnosticCodeParseError` | `Diagnostic.span` is `Span` — byte offsets only |
| `crates/vb_core/src/span.rs` | `Span`, `Located<T>`, `Spanned<T>`, `SourceMap` | `Span` has only `start: u32`, `end: u32`. **No line, column, file path.** `SourceMap` is `struct SourceMap { _private: () }` — dead placeholder. |
| `crates/vb_core/src/lib.rs` (line 105) | Re-exports `Span`, `SourceMap`, `Located`, `Spanned` | Public API surface |

### 2. YAML Parsing & Source Maps (`vb_yaml`)
| File | Key Symbols | Notes |
|---|---|---|
| `crates/vb_yaml/src/events_types.rs` | `YamlEvent`, `EventSpan` | `EventSpan` has `start`, `end` (byte offsets), `line`, `column` — from saphyr-parser |
| `crates/vb_yaml/src/source_map_types.rs` | `SourceSpan`, `SourceMap`, `SemanticSourceMap` | `SourceSpan` has byte offsets + start_line/start_col/end_line/end_col. `SemanticSourceMap` maps JSONPath-like paths (`$.steps.build.input`) to spans. |
| `crates/vb_yaml/src/events_conv.rs` | `collect_events()`, `convert_event()` | Converts saphyr-parser events to `YamlEvent` — preserves spans |
| `crates/vb_yaml/src/source_map_build.rs` | `build_source_map()`, `build_semantic_source_map()` | Build `SourceMap`/`SemanticSourceMap` from YAML text by re-parsing event stream |
| `crates/vb_yaml/src/error.rs` | `YamlError` (17 variants) | **No span fields** — variant `ParseError` has `line: usize` only |
| `crates/vb_yaml/src/lib.rs` | `parse_yaml_events()`, `parse_workflow_source()`, `build_source_map()` | Public API |

### 3. Compilation Error Infrastructure (`vb_compile`)
| File | Key Symbols | Notes |
|---|---|---|
| `crates/vb_compile/src/mod_compile_errors/kind.rs` | `CompileError` (~80 variants) | Own error type, separate from `vb_core::Diagnostic`. **String diagnostic codes** (not `DiagnosticCode`). Only 6 variants carry `SourceMark`. |
| `crates/vb_compile/src/mod_compile_errors/source_mark.rs` | `SourceMark` | Has `index`, `end_index`, `line`, `column`, `available: bool`. `unavailable()` default is all zeros. **Richer than `Span` but not bridged.** |
| `crates/vb_compile/src/mod_compile_errors/collection.rs` | `CompileErrors`, `collect()`, `diagnostic_code()` | Error collection type. `diagnostic_code()` returns `&'static str` codes. |
| `crates/vb_compile/src/ast/marks.rs` | `AstMarks`, `MarkBuilder` | Builds a mark lookup from saphyr-parser event stream. Tracks document, nested keys, triggers, steps. **Already working span capture!** |
| `crates/vb_compile/src/ast/types.rs` | `WorkflowAst`, `AstMapEntry<T>`, `StepAst`, `TriggerAst` | AST types all carry `mark: Option<SourceMark>` — hooks exist |
| `crates/vb_compile/src/strict_yaml.rs` | `reject_unsupported_profile_events()` | Profile gate — preserves spans for alias/anchor/merge/tag errors |
| `crates/vb_compile/src/mod_compile_validation/part_01.rs` | `canonical_yaml_error()`, `reject_known_canonical_text_gaps()` | **Strips all span info** — converts `YamlError` to `CompileError::CanonicalYaml` with only category/message |
| `crates/vb_compile/src/mod_compile_validation/part_02.rs` | `validate_strict_profile()`, `validate_one_node()` | Tree-based validation using `saphyr::Yaml` — **discards saphyr-parser span info**. Many errors use `SourceMark::unavailable()`. |
| `crates/vb_compile/src/mod_compile_core.rs` | `YamlCompiler`, `compile()`, `parse_ast()` | Orchestration — compiles YAML to `CompiledWorkflow`. Two code paths: `compile()` via `vb_yaml` and `parse_ast()` via `saphyr` directly. |

### 4. Validation Error Infrastructure (`vb_validate`)
| File | Key Symbols | Notes |
|---|---|---|
| `crates/vb_validate/src/lib.rs` (lines 99-362) | `ValidationError` (~50 variants) | **None carry span or source info** |
| `crates/vb_validate/src/diagnostic.rs` | `diagnostic_from_error()`, `error_code()` | ALWAYS uses `Span::ZERO` (line 92) |
| `crates/vb_validate/src/diag_render.rs` | `diagnostic_from_error()` (DUPLICATE) | Same logic as `diagnostic.rs`, always `Span::ZERO` (line 17) |
| `crates/vb_validate/src/diag_codes.rs` | Error code constants | Shared code table used by `diag_render.rs` |
| `crates/vb_validate/src/diag_convert.rs` | `all_variants()` | Test helper for full variant coverage |

## Gap Analysis

### Gap 1: `Span` is byte-offset-only — no line, column, or file path
- **File:** `vb_core/src/span.rs`
- **Issue:** `Span { start: u32, end: u32 }` cannot express `file:path/to/workflow.yaml:42:10`.
  Users see only byte ranges, not human-readable locations.
- **Blocked by:** `vb_core` is the hot runtime core — file paths don't belong there.
  A separate "rich span" type may need to live in `vb_compile` or a new shared crate.

### Gap 2: `SourceMap` in `vb_core` is a dead placeholder
- **File:** `vb_core/src/span.rs` (lines 53-65)
- **Issue:** `struct SourceMap { _private: () }` is publicly exported but entirely non-functional.
  Either fill it or remove it.
- **Risk:** Public API change if removed.

### Gap 3: No bridge between `vb_yaml::SourceSpan` and `vb_core::Span`
- **Issue:** `vb_yaml` has rich `SourceSpan` (offsets + line/col pairs) and `SemanticSourceMap`
  (JSONPath author paths). None of this feeds into `vb_core::Diagnostic.span`.
- **Root cause:** `vb_yaml` does not depend on `vb_core` (by design — runtime never parses YAML).
  Bridging requires either making `vb_yaml` depend on `vb_core` or lifting source-map types to a shared crate.

### Gap 4: `ValidationError` has ZERO span fields — all diagnostics use `Span::ZERO`
- **Files:** `vb_validate/src/lib.rs`, `vb_validate/src/diagnostic.rs`, `vb_validate/src/diag_render.rs`
- **Issue:** All ~50 `ValidationError` variants lack any source position. `diagnostic_from_error()`
  hardcodes `Span::ZERO`. Tests explicitly assert this.
- **How to fix:** Add optional span fields to `ValidationError` variants and thread the span
  through from the caller.

### Gap 5: `CompileError` uses `SourceMark::unavailable()` heavily — saphyr tree API loses spans
- **File:** `vb_compile/src/mod_compile_validation/part_02.rs`
- **Issue:** `validate_strict_profile()` uses `saphyr::Yaml` tree API which discards the
  parser's span data. Many errors get `SourceMark::unavailable()`.
- **Fix path:** Use `saphyr-parser` event streaming instead of the tree API, or backfill
  marks from `AstMarks`.

### Gap 6: `CanonicalYaml` error variant strips all span info
- **File:** `vb_compile/src/mod_compile_validation/part_01.rs` (line 25-30)
- **Issue:** When `vb_yaml::YamlError` is converted to `CompileError::CanonicalYaml`, all
  position data is discarded. Only `category` and `message` survive.
- **Fix path:** `YamlError` itself lacks span fields — enrich `YamlError` first, or
  propagate the span from the point of failure.

### Gap 7: Duplicate diagnostic conversion implementations
- **Files:** `vb_validate/src/diagnostic.rs` (pub API) and `vb_validate/src/diag_render.rs` (re-exports via diag_tests)
- **Issue:** Two identical implementations of the same mapping from `ValidationError` to `Diagnostic`.
  Both always use `Span::ZERO`. Changes must be synchronized.
- **Recommendation:** Consolidate to one canonical conversion point before enriching with spans.

### Gap 8: `Diagnostic` type uses `vb_core::Span` but compilation errors use `SourceMark`
- **Issue:** Two parallel span/source-location systems: `Span` in the core and `SourceMark` in the compiler.
  `Diagnostic` carries `Span` but `CompileError` carries `SourceMark` — they don't connect.
  `CompileError::diagnostic_code()` returns string codes, not `DiagnosticCode`.

## What Already Works — Span Infrastructure We Can Build On

1. **`vb_yaml::EventSpan`** — Every `YamlEvent` carries `EventSpan { start, end, line, column }` from saphyr-parser. Parsing preserves spans end-to-end.

2. **`vb_yaml::SourceMap`** — Maps node indices to `SourceSpan` with full line/column pairs. `build_source_map()` works and is tested.

3. **`vb_yaml::SemanticSourceMap`** — Maps JSONPath-like author paths (`$.steps.build.input`) to `SourceSpan`. `build_semantic_source_map()` works and has tests.

4. **`vb_compile::AstMarks`** — Already builds a mark lookup from the saphyr-parser event stream. Tracks document, nested keys, triggers, and step IDs by name. This is the primary bridge that should feed into diagnostics.

5. **AST `mark` fields** — `WorkflowAst.mark`, `AstMapEntry.mark`, `StepAst.mark`, `TriggerAst.mark` all carry `Option<SourceMark>`. The hooks for span propagation exist.

6. **`SourceMark::from_parser_span()`** — Converts from saphyr-parser spans with full line/column data. Used in `strict_yaml.rs` and `ast/marks.rs` — pattern to follow.

## Recommended Attack Plan

| Phase | What | Where |
|---|---|---|
| 1 | Add file path and line/col to `Span` (or create `RichSpan`) | `vb_core/src/span.rs` or new shared type |
| 2 | Enrich `YamlError` with optional `SourceSpan`/span fields | `vb_yaml/src/error.rs` |
| 3 | Thread `SourceSpan` from `vb_yaml` errors into `CompileError::CanonicalYaml` | `vb_compile/src/mod_compile_validation/part_01.rs` |
| 4 | Replace `SourceMark::unavailable()` with real marks from event stream | `vb_compile/src/mod_compile_validation/part_02.rs` (use `AstMarks`) |
| 5 | Add span fields to `ValidationError` variants and to `diagnostic_from_error()` | `vb_validate/src/lib.rs`, `vb_validate/src/diagnostic.rs` |
| 6 | Bridge `SourceMark`/`SourceSpan` into `vb_core::Diagnostic.span` | Consolidation point |
| 7 | Wire `SemanticSourceMap` into error messages (show YAML key path in messages) | `vb_yaml` + `vb_compile` bridge |

## Risk Tags

- **parser/codec** — YAML event stream and tree parsing, two parsing paths (saphyr-parser events vs saphyr tree)
- **public API** — `Span`, `SourceMap`, `Diagnostic` are publicly exported; changes are breaking
- **dependency** — `vb_yaml` does not depend on `vb_core`; bridging requires dependency changes
- **migration** — Duplicate conversion code in `diagnostic.rs` vs `diag_render.rs`

## Open Questions

1. Should `Span` be enriched with line/column/file, or should a separate `RichSpan`/`DiagnosticSpan` type live in `vb_compile` with `vb_core::Span` remaining minimal?
2. Should `vb_yaml` gain a dependency on `vb_core`, or should source-map types move to a shared crate?
3. Should the unused `SourceMap` in `vb_core` be removed, repurposed, or bridged to `vb_yaml::SourceMap`?
4. Should `CompileError` be converted to `vb_core::Diagnostic` at the output boundary, or should diagnostic conversion be a separate pass?
