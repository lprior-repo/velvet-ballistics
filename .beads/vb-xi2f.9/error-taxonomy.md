# Error Taxonomy: YAML Path and Source Span Diagnostics

**Bead:** vb-xi2f.9  
**Agent:** rust-contract (State 3)  
**Schema:** 2026-05-24

## Railway Error Architecture

```
YAML text
  │
  ▼
vb_yaml::parse_workflow_source() ──Err──► YamlError (+ SourceSpan)
  │                                            │
  ▼                                            ▼
WorkflowSource                           canonical_yaml_error()
  │                                            │
  ▼                                            ▼
vb_compile::parse_ast()                  CompileError::CanonicalYaml (+ SourceMark)
  │                                            │
  ▼                                            ▼
vb_compile::compile()                    CompileErrors (NonEmptyVec)
  │                                            │
  ▼                                            ▼
vb_compile::validate() ──Err──► ValidationError (+ Span)
  │                                    │
  ▼                                    ▼
CompiledWorkflow                  diagnostic_from_error()
                                       │
                                       ▼
                                  Diagnostic (+ Span + source_file)
```

All error paths converge to `Diagnostic` before surfacing to the user. Runtime errors (no YAML) produce `Diagnostic` with `Span::ZERO` and `source_file: None`.

---

## Error Categories

### EC-01: YAML Parse Errors (YamlError — 19 variants enriched)

| Subtype | Has Span After Enrichment? | Severity | Bridge Target |
|---|---|---|---|
| `ParseError` | **Yes** (from saphyr-parser event stream) | Error | `CanonicalYaml` with mark |
| `DuplicateKey` | **Yes** (where the duplicate occurred) | Error | `CanonicalYaml` with mark |
| `AnchorAliasMerge` | **Yes** (from event span) | Error | `CanonicalYaml` with mark |
| `CustomTag` | **Yes** (from event span) | Error | `CanonicalYaml` with mark |
| `BinaryScalar` | **Yes** (from event span) | Error | `CanonicalYaml` with mark |
| `AmbiguousScalar` | **Yes** (from event span) | Error | `CanonicalYaml` with mark |
| `UnknownField` | **Yes** (where the unknown field appears) | Error | `CanonicalYaml` with mark |
| `MissingField` | **Yes** (where the field is expected) | Error | `CanonicalYaml` with mark |
| `FieldShape` | **Yes** (where the shape violation occurs) | Error | `CanonicalYaml` with mark |
| `ForbiddenFeature` | **Yes** (from event span) | Error | `CanonicalYaml` with mark |
| `UnsupportedTrigger` | **Yes** (trigger declaration location) | Error | `CanonicalYaml` with mark |
| `UnsupportedFeature` | **Yes** (feature location) | Error | `CanonicalYaml` with mark |
| `MultipleDocuments` | **Maybe** (whole-document level) | Error | `CanonicalYaml` with mark |
| `SourceTooLarge` | No (whole-file) | Error | `CanonicalYaml` |
| `NestingTooDeep` | No (whole-file) | Error | `CanonicalYaml` |
| `NodeLimitExceeded` | No (whole-file) | Error | `CanonicalYaml` |
| `ScalarTooLong` | **Yes** (where the long scalar is) | Error | `CanonicalYaml` with mark |
| `SequenceTooLong` | **Yes** (sequence location) | Error | `CanonicalYaml` with mark |
| `MappingTooLarge` | **Yes** (mapping location) | Error | `CanonicalYaml` with mark |
| `EmptySource` | No (no source to point to) | Error | `CanonicalYaml` |

### EC-02: Validation Errors (ValidationError — 50 variants)

All ~50 variants gain an optional `span: Span` field. Grouped by diagnostic code prefix:

| Code Range | Category | Example Variants | Typical Span Source |
|---|---|---|---|
| E01xx | Schema | DuplicateKey, MissingRequiredField, InvalidId | AST node marks |
| E02xx | References | UnknownReference, FutureReference | Reference expression location |
| E03xx | Control Flow | ControlFlowCycle, UnreachableStep | Affected step location |
| E04xx | Type/Taint | TypeMismatch, SecretResultLeak | Expression/value location |
| E05xx | Gate Verifier | ExpressionStackExceeded, SlotDependencyCycle | Affected node location |
| E06xx | Contract | MissingSchemaVersion, CueVetFailed | Schema file location |

### EC-03: Compilation Errors (CompileError — 80 variants)

| Category | Count | Has SourceMark After Enrichment? | Enrichment Method |
|---|---|---|---|
| **Event-stream errors** (strict YAML) | 6 | Already present | `AstMarks` / `strict_yaml.rs` |
| **Tree validation errors** | ~25 | **Yes** (backfilled from `AstMarks`) | `AstMarks::nested_key()`, `step()`, `trigger()` |
| **Canonical YAML bridge** | 1 | **Yes** (from `YamlError.span`) | `extract_span_from_yaml_error()` |
| **AST → IR lowering errors** | ~40 | **Planned** (from AST node marks) | `mark` field on `WorkflowAst`, `StepAst`, etc. |
| **IR validation errors** | ~3 | Runtime-only (no mark) | `unavailable()` |
| **Expression errors** | ~8 | Partial (byte index available) | `index: usize` field on expression errors |

### EC-04: Runtime Diagnostics (no YAML)

| Scenario | Span | source_file |
|---|---|---|
| Budget error | `Span::ZERO` | `None` |
| Runtime admission error | `Span::ZERO` | `None` |
| IPC frame error | `Span::ZERO` | `None` |
| Storage error | `Span::ZERO` | `None` |
| Engine trap | `Span::ZERO` | `None` |

---

## Error Conversion Matrix

| From | To | Conversion Point | Span Preservation |
|---|---|---|---|
| `vb_yaml::YamlError` | `CompileError::CanonicalYaml` | `canonical_yaml_error()` | **NEW:** `SourceSpan → SourceMark` |
| `vb_yaml::YamlError` | `CompileError::Parse` | `impl From<saphyr::ScanError>` | N/A (parser-level) |
| `CompileError` | `Diagnostic` | `diagnostic_from_compile_error()` | **NEW:** `SourceMark → Span` |
| `ValidationError` | `Diagnostic` | `diagnostic_from_error()` | **NEW:** `error.span → Diagnostic.span` |
| `CompileErrors` | `DiagnosticBundle` | Compiler output boundary | All errors converted |
| `String` error | `Diagnostic` | Legacy paths | `Span::ZERO` (unchanged) |

---

## Error Code Stability

All `DiagnosticCode` values (E0101..E401B) remain stable. No new codes are added for span enrichment — this is purely a payload change within existing error variants.

| Code Prefix | Max Variants | Currently Used | Headroom |
|---|---|---|---|
| E01xx | 16 | 11 | 5 |
| E02xx | 16 | 4 | 12 |
| E03xx | 16 | 9 | 7 |
| E04xx | 16 | 12 | 4 |
| E05xx | 16 | 19 (overflow!) | **RISK** |
| E06xx | 16 | 3 | 13 |

**Note:** The E05xx range has 19 variants but supports only 16 distinct 4-bit suffixes (E0501..E0510 = 16 values; E0511+E0512+E0513 = 3 more, totaling 19). This overflow has already been handled by using upper nibble extension. No action needed for this bead.

---

## Error Severity Classification

| Severity | When Used |
|---|---|
| `Error` | All YAML parse, validation, and compilation failures. Blocks workflow admission. |
| `Warning` | Reserved for future non-blocking diagnostics (e.g., deprecated features). |
| `Info` | Reserved for future informational diagnostics (e.g., suggested improvements). |

Currently all diagnostics are `Severity::Error`. This bead does not introduce `Warning` or `Info` diagnostics.

---

## Railway Pattern Enforcement

All error-producing functions follow the railway pattern:
- **Success:** Continue to next processing stage.
- **Failure:** Return `Err(...)` with span-enriched error, short-circuit the pipeline.

The compiler pipeline is a sequence of `Result<T, E>` compositions where `E` accumulates all errors before returning:

```
parse() → validate() → lower() → admit()
   │          │          │          │
   ▼          ▼          ▼          ▼
 Ok/Err    Ok/Err     Ok/Err     Ok/Sink
```

Accumulation strategy:
1. **Parse phase:** Fail-fast — first `YamlError` aborts parsing.
2. **Validation phase:** Accumulate all `ValidationError`s — collect into `NonEmptyVec`.
3. **Lowering phase:** Accumulate all `CompileError`s — collect into `NonEmptyVec`.
4. **Diagnostic rendering:** Convert all errors to `Diagnostic` records — exhaustively map every variant.
