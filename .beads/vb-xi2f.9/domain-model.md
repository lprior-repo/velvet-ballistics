# Domain Model: YAML Path and Source Span Diagnostics

**Bead:** vb-xi2f.9  
**Agent:** rust-contract (State 3)  
**Schema:** 2026-05-24  
**Ubiquitous Language:** velvet-ballastics/v1

## 1. Ubiquitous Language

| Term | Definition | Current Representation |
|---|---|---|
| **Source Span** | A contiguous region in a source document, identified by byte offsets and optionally line/column coordinates. | `vb_core::Span` (byte offsets only) |
| **Rich Span** | A Span that also carries line, column, and optionally a file path — enables human-readable diagnostics. | NOT YET REPRESENTED |
| **Source Location** | The combination of a source file path and a Rich Span, fully identifying a point in YAML source text. | NOT YET REPRESENTED (`SourceMark` in `vb_compile` is partial) |
| **YAML Event Span** | The line+column+byte-offset span attached to every YAML parser event in the event stream. | `vb_yaml::EventSpan` — `start`, `end`, `line`, `column` |
| **Source Map** | A lookup from YAML node index to `SourceSpan` (line+col+offset). | `vb_yaml::SourceMap` (working) |
| **Semantic Source Map** | A lookup from JSONPath-like author path (e.g., `$.steps.build.input`) to `SourceSpan`. | `vb_yaml::SemanticSourceMap` (working) |
| **Source Mark** | A lightweight span record in the compiler carrying index, line, column, and an `available` flag. | `vb_compile::SourceMark` — 6 CompileError variants carry one |
| **AST Mark** | A `SourceMark` captured from the saphyr-parser event stream and stored on AST nodes. | `vb_compile::AstMarks` — document, nested keys, triggers, steps |
| **Diagnostic** | A stable error code + message + severity + source span, rendered for the user. | `vb_core::Diagnostic` |
| **Error Accumulator** | A non-empty collection of errors produced by a compilation or validation pass. | `vb_compile::CompileErrors` (Vec-based, can be empty) |
| **Canonical YAML Bridge** | The conversion point where `vb_yaml::YamlError` is turned into `CompileError::CanonicalYaml`. | `canonical_yaml_error()` in `vb_compile` — strips span info |
| **Validation Error** | A gate-level validation failure with structured fields but no source position. | `vb_validate::ValidationError` (~50 variants, zero span fields) |

## 2. Entities

### 2.1 Span (Core Primitive — ENRICHED)

**Current state:** `Span { start: u32, end: u32 }` — byte offsets only.  
**Target state:** Add optional line/column fields. File path lives on `Diagnostic` or a `SourceLocation` wrapper, not on `Span` itself (keeping `Span` usable in the runtime core without file I/O baggage).

```
Span {
    start: u32,        // inclusive byte offset (unchanged)
    end: u32,          // exclusive byte offset (unchanged)
    line: Option<u32>, // 1-indexed start line (add)
    column: Option<u32>, // 1-indexed start column (add)
}
```

**Invariants:**
- I1: `start <= end` (or `start == end == 0` for `Span::ZERO`)
- I2: If `line.is_some()`, then `column.is_some()` (and vice versa)
- I3: `Span::ZERO` has all fields at zero/None — backward compatible
- I4: Byte offsets `start`/`end` are in-bounds for their source document when meaningful

### 2.2 Diagnostic (ENRICHED)

**Current state:** Carries `Span` (byte offsets only).  
**Target state:** Add optional `source_file` field for file path.

```
Diagnostic {
    code: DiagnosticCode,       // unchanged
    message: Box<str>,          // unchanged
    severity: Severity,         // unchanged
    span: Span,                 // now carries optional line/column
    source_file: Option<Box<str>>, // add: path to the YAML source file
}
```

**Invariants:**
- I5: `source_file` is `None` for runtime-generated diagnostics, `Some(path)` for compile-time authoring diagnostics
- I6: `Diagnostic` is always constructible with `Span::ZERO` and `source_file: None` — backward compatible

### 2.3 NonEmptyVec\<T\> (New Value Object)

**Current state:** No such type exists.  
**Purpose:** Guarantee that diagnostic/error accumulators contain at least one element.

```
NonEmptyVec<T> {
    head: T,
    tail: Vec<T>,
}
```

**Invariants:**
- I7: `len() >= 1` always
- I8: Construction requires at least one element (smart constructor, not public fields)
- I9: `first()` never returns `None`
- I10: `into_vec()` preserves all elements

### 2.4 SourceSpan (vb_yaml — UNCHANGED)

**Status:** Already correct and working. No domain changes needed.

```
SourceSpan {
    start_offset: usize,
    end_offset: usize,
    start_line: usize,    // 1-indexed
    start_col: usize,     // 1-indexed
    end_line: usize,      // 1-indexed
    end_col: usize,       // 1-indexed
}
```

### 2.5 EventSpan (vb_yaml — UNCHANGED)

**Status:** Already correct and working. Source of truth for parser-level spans.

```
EventSpan {
    start: usize,    // byte offset
    end: usize,      // byte offset
    line: usize,     // 1-indexed
    column: usize,   // 1-indexed
}
```

### 2.6 SourceMark (vb_compile — ENRICHED)

**Current state:** Carries `index`, `end_index`, `line`, `column`, `available`.  
**Target state:** Add `source_file` and implement `From<SourceSpan>`.

```
SourceMark {
    index: usize,
    end_index: usize,
    line: usize,          // 1-indexed
    column: usize,        // 1-indexed
    available: bool,
    source_file: Option<Box<str>>, // add
}
```

### 2.7 ValidationError (ENRICHED)

**Current state:** ~50 variants, zero span fields.  
**Target state:** Add optional `span: Span` to variants where source location is meaningful. Unit variants get no span (or a zero span).

```
ValidationError::MissingRequiredField {
    field: String,
    span: Span,            // add — where the missing field was expected
}

ValidationError::DuplicateKey {
    span: Span,            // add — where the duplicate occurred
}
// ... etc for ~30 variants, remaining ~20 unit variants don't need span
```

**Invariant:** `diagnostic_from_error()` propagates the error's span into `Diagnostic.span` when present, falls back to `Span::ZERO` otherwise.

### 2.8 YamlError (ENRICHED)

**Current state:** 17 variants, `ParseError` has `line: usize` only.  
**Target state:** Add optional `span: Option<SourceSpan>` to error-producing variants.

```
YamlError::DuplicateKey {
    key: Box<str>,
    span: Option<SourceSpan>,  // add
}
YamlError::ParseError {
    line: usize,
    reason: Box<str>,
    span: Option<SourceSpan>,  // add
}
// ... etc
```

## 3. Value Objects

| Value Object | Wraps | Invariant |
|---|---|---|
| `DiagnosticCode` | `u16` | Must match supported range in `is_supported_code()` |
| `NonEmptyVec<T>` | `(T, Vec<T>)` | `head` is always a valid `T`; `tail` may be empty |
| `SourceFile` (new) | `Box<str>` | Non-empty path string, format: `path/to/file.yaml` or `-` (stdin) |
| `LineNumber` (new) | `NonZeroU32` | 1-indexed line number |
| `ColumnNumber` (new) | `NonZeroU32` | 1-indexed column number |

## 4. Aggregates

### 4.1 Compilation Pipeline (Aggregate Root)

The compilation pipeline produces a `CompileOutcome` which is either:
- `Ok(CompiledWorkflow)` — success
- `Err(NonEmptyVec<CompileError>)` — failure with at least one error, each carrying a `SourceMark`

### 4.2 Validation Pipeline (Aggregate Root)

The validation pipeline produces a `ValidationOutcome` which is either:
- `Ok(())` — no errors
- `Err(NonEmptyVec<ValidationError>)` — failure with at least one error

### 4.3 Diagnostic Bundle (Aggregate Root)

The set of diagnostics produced for a single compilation unit:
```
DiagnosticBundle {
    source_file: SourceFile,
    diagnostics: NonEmptyVec<Diagnostic>,
}
```

## 5. Commands

| Command | Trigger | Produces |
|---|---|---|
| `parse_workflow_source(file_path, text)` | Author invokes compiler | `WorkflowSource` + `SourceMap` + `SemanticSourceMap` |
| `validate_ast(workflow_ast, source_map)` | After parsing | `Vec<ValidationError>` (with spans) |
| `compile_to_ir(workflow_ast)` | After validation | `Result<CompiledWorkflow, NonEmptyVec<CompileError>>` |
| `render_diagnostics(errors, source_map)` | On any failure | `DiagnosticBundle` |

## 6. Events

| Event | When |
|---|---|
| `YamlParsed { source_map, semantic_map }` | After `parse_workflow_source` succeeds |
| `ValidationFailed { errors: NonEmptyVec<ValidationError> }` | When validation finds errors |
| `CompilationFailed { errors: NonEmptyVec<CompileError> }` | When compilation fails |
| `DiagnosticsRendered { bundle: DiagnosticBundle }` | After converting errors to diagnostics |

## 7. Policies

| Policy | Description |
|---|---|
| **Backward Compatibility:** | All existing callers of `diagnostic_from_error()` continue to compile. `Span::ZERO` is produced when no source span is available. Tests asserting `Span::ZERO` are updated to accept both zero and non-zero spans or to supply spans explicitly. |
| **Span Propagation:** | Errors produced at parse/validate time MUST carry their source location if available. Errors produced at runtime (never from YAML) carry `Span::ZERO`. |
| **No File in Core:** | `Span` does NOT carry a file path. File path lives on `Diagnostic.source_file` or on `SourceMark`. |
| **Non-Empty Errors:** | Failed compilations produce at least one error. The type system enforces this via `NonEmptyVec`. |
| **One Canonical Conversion:** | Exactly ONE function maps `ValidationError` → `Diagnostic`. The duplicate in `diag_render.rs` is removed or consolidated. |
| **Source Map Optional:** | Source maps are built eagerly but may be absent (e.g., for programmatically constructed ASTs). Errors still produce valid `Span::ZERO` fallbacks. |

## 8. Forbidden States

- `Span` with `line: Some(n)` and `column: None` (or vice versa) — I2 forbids this
- `NonEmptyVec` constructed with zero elements — impossible by type
- `Diagnostic.source_file: Some("")` — empty file path
- `SourceMark` with `available: true` but all fields zero
- `ValidationError` with a `span` that points past the end of the source text (enforced at span construction)
- `CompileErrors` (Vec form) being empty after a failed compilation — migrate to `NonEmptyVec`
- `canonical_yaml_error()` discarding span info when `YamlError` carries a `SourceSpan` — new code must preserve it

## 9. Cross-crate Bridge Architecture

```
vb_yaml (source spans) ──no dep──→ vb_core (diagnostic types)
        │                                ↑
        │                                │
        └────vb_compile (bridge)─────────┘
```

`vb_compile` depends on both `vb_yaml` and `vb_core`. It implements the conversion:
- `From<vb_yaml::SourceSpan> for vb_core::Span` — lossy (line/col preserved, offsets clamped to u32)
- `From<vb_yaml::SourceSpan> for SourceMark` — preserves all data
- `From<SourceMark> for vb_core::Span` — extracts byte offsets and line/col

## 10. Open Domain Decisions

1. **Should `Span` gain optional fields or should a separate `RichSpan` live in `vb_compile`?**  
   Decision: `Span` gains `Option<u32>` for line/column. This keeps a single span type. Runtime callers ignore the new fields. Diagnostic carry-rs use them.

2. **Should `vb_yaml` gain a dependency on `vb_core`?**  
   Decision: NO. The bridge lives in `vb_compile`. `vb_yaml` stays pure YAML parsing with its own `SourceSpan` type.

3. **Should the dead `SourceMap` placeholder in `vb_core` be removed or bridged?**  
   Decision: REMOVE the dead `vb_core::SourceMap`. The live `vb_yaml::SourceMap` is the canonical source map.

4. **Should `CompileError` be converted to `vb_core::Diagnostic`?**  
   Decision: YES — a `diagnostic_from_compile_error()` function in `vb_compile` that produces `vb_core::Diagnostic` from `CompileError`. This unifies diagnostic output.

5. **Should `CompileErrors` migrate from `Vec<CompileError>` to `NonEmptyVec<CompileError>`?**  
   Decision: YES — `NonEmptyVec` is defined in `vb_core` and used uniformly.

## 11. Illegal-State Risks That Remain Representable

| Risk | Mitigation |
|---|---|
| `Span` with negative offsets (offset types are `u32`/`usize` — type-safe) | Already prevented |
| `NonEmptyVec` could be constructed manually via destructuring if fields are pub | Smart constructor + private fields |
| `SourceMark::unavailable()` can still be used where real data is available | Code review; type doesn't prevent misuse |
| `usize` → `u32` truncation when bridging `SourceSpan` to `Span` | Saturated/clamped conversion or `TryFrom` with error |
| Duplicate `diagnostic_from_error()` could persist if `diag_render.rs` is not deleted | Consolidated in single file, other becomes re-export |
