# Workflow Model: YAML Path and Source Span Diagnostics

**Bead:** vb-xi2f.9  
**Agent:** rust-contract (State 3)  
**Schema:** 2026-05-24

## 1. Legal States

### 1.1 Span Lifecycle

```
┌──────────────────────────────────────────────────────────────┐
│                        Span States                            │
├──────────────┬────────────────────┬──────────────────────────┤
│ State        │ Description        │ Transition Trigger        │
├──────────────┼────────────────────┼──────────────────────────┤
│ ZERO         │ Unknown/empty span │ Default, runtime diag     │
│ BYTE_ONLY    │ Byte offsets only  │ Span::new(start, end)     │
│ RICH         │ + line + column    │ Span::with_location(...)  │
│ FILE_BOUND   │ + source file path │ Diagnostic.source_file    │
│              │                    │   or SourceMark.source_file│
└──────────────┴────────────────────┴──────────────────────────┘
```

States are cumulative: RICH implies BYTE_ONLY; FILE_BOUND implies RICH.

### 1.2 Diagnostic Lifecycle

```
┌──────────────────────────────────────────────────────────────┐
│                    Diagnostic States                          │
├──────────────┬──────────────────────┬────────────────────────┤
│ State        │ Description          │ Transition              │
├──────────────┼──────────────────────┼────────────────────────┤
│ CONSTRUCTED  │ code + msg + sev OK  │ Diagnostic::new()       │
│ LOCATED      │ + Span (may be ZERO) │ Always present          │
│ FILED        │ + source_file path   │ Set by compiler bridge  │
│ RENDERED     │ Display-formatted    │ diagnostic.display()    │
└──────────────┴──────────────────────┴────────────────────────┘
```

### 1.3 ValidationError Lifecycle

```
┌──────────────────────────────────────────────────────────────┐
│                ValidationError States                         │
├──────────────┬──────────────────────┬────────────────────────┤
│ State        │ Description          │ Transition              │
├──────────────┼──────────────────────┼────────────────────────┤
│ DETACHED     │ No span info         │ Constructed from IR     │
│ ANCHORED     │ Has Span             │ Constructed from AST +  │
│              │                      │   SourceMap/SemanticMap │
│ DIAGNOSED    │ Converted to Diag    │ diagnostic_from_error() │
└──────────────┴──────────────────────┴────────────────────────┘
```

### 1.4 CompileError Lifecycle

```
┌──────────────────────────────────────────────────────────────┐
│                CompileError States                            │
├──────────────┬──────────────────────┬────────────────────────┤
│ State        │ Description          │ Transition              │
├──────────────┼──────────────────────┼────────────────────────┤
│ MARKED       │ Has SourceMark       │ Constructed from AST +  │
│              │ (6 variants)         │   event stream          │
│ UNMARKED     │ No source info       │ Constructed from IR     │
│              │ (74 variants)        │   or tree validation    │
│ BRIDGED      │ CanonicalYaml w/mark │ canonical_yaml_error()  │
│ COLLECTED    │ In CompileErrors     │ CompileErrors::collect()│
│ DIAGNOSED    │ Converted to Diag    │ diagnostic_from_...()   │
└──────────────┴──────────────────────┴────────────────────────┘
```

Target state post-refactor: ALL 80 variants become MARKED (carry `SourceMark`).

### 1.5 CompileErrors Lifecycle

```
┌──────────────────────────────────────────────────────────────┐
│               CompileErrors States                            │
├──────────────┬──────────────────────┬────────────────────────┤
│ State        │ Description          │ Transition              │
├──────────────┼──────────────────────┼────────────────────────┤
│ EMPTY (BAD)  │ Vec with 0 errors    │ Current code allows     │
│ NON_EMPTY    │ NonEmptyVec with ≥1  │ Enforced by type        │
│ TERMINAL     │ Returned from fn     │ ? operator / return     │
└──────────────┴──────────────────────┴────────────────────────┘
```

EMPTY is **forbidden** — the type system enforces NON_EMPTY via `NonEmptyVec`.

## 2. State Transition Diagram (End-to-End Compilation)

```
                               parse_workflow_source()
YAML text ──────────────────────────────────────────► WorkflowSource
   │                                                       │
   │ build_source_map()                              AstMarks::new()
   ▼                                                       ▼
SourceMap + SemanticSourceMap ──────────────► validate_ast() ──────►
                                                        │
                                               ┌────────┴────────┐
                                               ▼                 ▼
                                        Ok(WorkflowAst)    Err(NonEmptyVec<ValidationError>)
                                               │                 │
                                         compile_to_ir()    diagnostic_from_error()
                                               │                 │
                                        ┌──────┴──────┐         ▼
                                        ▼              ▼    DiagnosticBundle
                                 Ok(IR)    Err(NonEmptyVec<CompileError>)
                                                       │
                                              diagnostic_from_compile_error()
                                                       │
                                                       ▼
                                               DiagnosticBundle
```

## 3. Guards

### 3.1 Span Construction Guards

| Guard | Condition | Violation Behavior |
|---|---|---|
| G-S01 | `start <= end` | Clamp or reject at construction |
| G-S02 | `line.is_some() ⇔ column.is_some()` | Both set together or both None |
| G-S03 | `line >= 1, column >= 1` when Some | Caller responsibility; documented invariant |
| G-S04 | `Span::ZERO` has `line`/`column` both None | Hardcoded constant |

### 3.2 Diagnostic Construction Guards

| Guard | Condition | Violation Behavior |
|---|---|---|
| G-D01 | `code` is in supported range | Enforced by `DiagnosticCode::from_str` |
| G-D02 | `message` is non-empty | Caller responsibility |
| G-D03 | `source_file` is non-empty when Some | Enforced by `SourceFile` newtype |

### 3.3 NonEmptyVec Guards

| Guard | Condition | Violation Behavior |
|---|---|---|
| G-N01 | `head` is always a valid `T` | Enforced by constructor signature |
| G-N02 | `len() >= 1` | Invariant maintained by all mutation ops |
| G-N03 | `from_vec(empty)` → None | Reject at boundary |

### 3.4 Error Conversion Guards

| Guard | Condition | Violation Behavior |
|---|---|---|
| G-E01 | `YamlError → CompileError` preserves span | `canonical_yaml_error()` extracts span |
| G-E02 | `ValidationError → Diagnostic` preserves span | `diagnostic_from_error()` propagates `error.span` |
| G-E03 | `CompileError → Diagnostic` includes SourceMark | New `diagnostic_from_compile_error()` |
| G-E04 | No diagnostic conversion loses available span data | If span is available it appears in Diagnostic |

### 3.5 Bridge Guards

| Guard | Condition | Violation Behavior |
|---|---|---|
| G-B01 | `SourceSpan → Span` does not panic on overflow | Clamp to `u32::MAX` |
| G-B02 | `SourceMark(available=false) → Span` sets `line=None, column=None` | Explicit check |
| G-B03 | `usize → u32` conversion is checked | `TryFrom` or `saturating` |
| G-B04 | vb_yaml depends on vb_core only if architecture permits | Bridge in vb_compile |

## 4. Terminal Outcomes

| Outcome | Precondition | Result |
|---|---|---|
| **Success** | Parsing + validation + compilation all pass | `CompiledWorkflow` produced |
| **Parse Failure** | YAML text is malformed or violates strict profile | `YamlError` with span → `CompileError::CanonicalYaml` with span |
| **Validation Failure** | Schema/reference/control-flow/type error | `NonEmptyVec<ValidationError>` with spans → DiagnosticBundle |
| **Compilation Failure** | Lowering/IR construction error | `NonEmptyVec<CompileError>` with marks → DiagnosticBundle |
| **Runtime Diagnostic** | Error during execution (no YAML) | `Diagnostic` with `Span::ZERO`, `source_file: None` |

## 5. Retries and Cancellation

- **No retries for YAML errors:** YAML parsing is deterministic; re-parsing the same input produces the same errors.
- **No cancellation during parsing:** YAML parsing is CPU-bound and non-interruptible (no async).
- **Cancellation at the compiler boundary:** The `YamlCompiler` can be dropped mid-compilation if the caller cancels. No cleanup of intermediate AST state required.
- **Idempotent diagnostic rendering:** Calling `diagnostic_from_error()` on the same error produces the same `Diagnostic` every time.

## 6. Temporal Workflow Hazards

### H-01: Span Propagation Gap (Validation Phase)
- **Scenario:** Validator receives span info from the parser but it isn't threaded through to `ValidationError`.
- **Risk:** Diagnostics show `Span::ZERO` even when source location is known.
- **Mitigation:** Every `ValidationError` variant gains a `span: Span` field; `diagnostic_from_error()` propagates it.

### H-02: Span Stripping at Canonical Bridge
- **Scenario:** `canonical_yaml_error()` converts `YamlError` to `CompileError::CanonicalYaml` and discards all span info.
- **Risk:** YAML parse errors get `SourceMark::unavailable()` everywhere.
- **Mitigation:** Enrich `YamlError` with `Option<SourceSpan>`, then thread through `canonical_yaml_error()`.

### H-03: Tree Validation Loses Parser Spans
- **Scenario:** `validate_strict_profile()` uses the `saphyr::Yaml` tree API which discards event-level span data.
- **Risk:** All tree-based validation errors get `SourceMark::unavailable()`.
- **Mitigation:** Backfill marks from `AstMarks` (which is built from the event stream). Phase 1: use `AstMarks` lookups. Phase 2: consider event-stream-driven validation.

### H-04: Duplicate Conversion Drift
- **Scenario:** `diagnostic.rs` and `diag_render.rs` both map `ValidationError` → `Diagnostic`. A change to one without the other creates inconsistent diagnostics.
- **Risk:** Two different `Diagnostic` formats for the same error, depending on which path is used.
- **Mitigation:** Consolidate to one canonical conversion; remove or delegate the duplicate.

### H-05: Breaking Test Assertions
- **Scenario:** Tests assert `diagnostic.span == Span::ZERO`. After enriching spans, these tests fail.
- **Risk:** False-positive test failures; tests that don't actually verify correct behavior.
- **Mitigation:** Update tests to construct errors with `Span::ZERO` and add new tests that verify span propagation.

### H-06: Public API Breakage
- **Scenario:** Adding fields to `Span`, `Diagnostic`, `ValidationError`, `CompileError` breaks pattern matches and exhaustiveness checks.
- **Risk:** Compile errors in downstream crates and integration tests.
- **Mitigation:** Add fields as the last position (append-only); use `..` in pattern matches; `SourceMap` removal requires explicit migration.

### H-07: Dependency Cycle Risk
- **Scenario:** If `vb_yaml` is modified to depend on `vb_core`, an unwanted dependency is created (runtime core → YAML authoring).
- **Risk:** Architectural violation — the runtime could inadvertently import YAML parsing code.
- **Mitigation:** All bridging lives in `vb_compile`; `vb_yaml` never depends on `vb_core`.

### H-08: usize → u32 Truncation
- **Scenario:** `SourceSpan` uses `usize` for offsets and line numbers; `Span` uses `u32`. On 64-bit systems, large values overflow.
- **Risk:** Incorrect span offsets, misleading line/column numbers.
- **Mitigation:** Use `u32::try_from(...)` or clamp at `u32::MAX`. A `Span` with clamped values is still useful for the range it covers.

### H-09: SemanticSourceMap Lookup Performance
- **Scenario:** `span_for_path()` does O(n) linear scan. Used in error rendering for every error.
- **Risk:** Slow diagnostic rendering for multi-error compilations.
- **Mitigation:** Acceptable for cold authoring path. If perf becomes an issue, migrate `SemanticSourceMap` to a `HashMap` internally but this is out of scope for this bead.

### H-10: NonEmptyVec Integration Risk
- **Scenario:** `CompileErrors::collect()` callers currently build a `Vec` and push errors. Migrating to `NonEmptyVec` requires changing collection patterns.
- **Risk:** Callers that build an empty `CompileErrors` and then return `Ok(())` need restructuring.
- **Mitigation:** Pattern: accumulate into `Vec<CompileError>` during processing, then convert to `NonEmptyVec` at the return boundary via `NonEmptyVec::from_vec(vec).map(CompileErrors::new).ok_or(...)`.
