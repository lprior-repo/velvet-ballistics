# Boundary Map: YAML Path and Source Span Diagnostics

**Bead:** vb-xi2f.9  
**Agent:** rust-contract (State 3)  
**Schema:** 2026-05-24

## Core Principle

**Functional core / imperative shell.** The core is pure data transformation (parsing, validation, lowering, error conversion). The shell handles I/O (file reading, diagnostic output, YAML text ingestion).

---

## Crate Boundaries

```
┌─────────────────────────────────────────────────────────────┐
│                      vb_yaml (PURE)                         │
│  ┌────────────────┐  ┌──────────────┐  ┌─────────────────┐ │
│  │ events_types   │  │ source_map   │  │ error (enriched)│ │
│  │ EventSpan      │  │ SourceSpan   │  │ YamlError +span │ │
│  │ YamlEvent      │  │ SourceMap    │  └─────────────────┘ │
│  └────────────────┘  │ SemanticMap  │                        │
│  ┌────────────────┐  └──────────────┘                        │
│  │ events_conv    │  ┌──────────────┐                        │
│  │ (saphyr-parser)│  │ source_map_  │                        │
│  └────────────────┘  │ build        │                        │
│                      └──────────────┘                        │
│  DEPENDS ON: saphyr-parser, saphyr                           │
│  DOES NOT DEPEND ON: vb_core                                 │
└─────────────────────────────────────────────────────────────┘

                              │ (no dep)
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     vb_compile (BRIDGE)                      │
│  ┌────────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │ ast/marks      │  │ errors/kind  │  │ errors/source_  │  │
│  │ AstMarks       │  │ CompileError │  │ mark (enriched) │  │
│  │ mark:SourceMark│  │ +mark fields │  │ SourceMark+file │  │
│  └────────────────┘  └──────────────┘  └─────────────────┘  │
│  ┌────────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │ bridge:        │  │ validation/  │  │ diagnostic_     │  │
│  │ SourceSpan→Span│  │ part_01/02   │  │ compile (NEW)   │  │
│  │ SourceMark→Span│  │ (enriched)   │  └─────────────────┘  │
│  └────────────────┘  └──────────────┘                        │
│                                                              │
│  DEPENDS ON: vb_core, vb_yaml, vb_validate, saphyr-parser    │
│  ROLE: Bridge vb_yaml spans → vb_core spans                  │
└─────────────────────────────────────────────────────────────┘

                              │
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     vb_validate (PURE)                       │
│  ┌────────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │ lib (enriched) │  │ diagnostic   │  │ diag_render     │  │
│  │ ValidationError│  │ (consolidat.)│  │ (→ re-export)   │  │
│  │ +span fields    │  └──────────────┘  └─────────────────┘  │
│  └────────────────┘                                           │
│                                                              │
│  DEPENDS ON: vb_core                                         │
└─────────────────────────────────────────────────────────────┘

                              │
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     vb_core (PURE CORE)                      │
│  ┌────────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │ span (enriched)│  │ diagnostic   │  │ non_empty_vec   │  │
│  │ Span+line/col  │  │ +source_file │  │ (NEW)           │  │
│  │ Located<T>     │  │ Diagnostic   │  │ NonEmptyVec<T>  │  │
│  │ Spanned<T>     │  │ Code+Severity│  └─────────────────┘  │
│  │ SourceMap RMVD │  └──────────────┘                        │
│  └────────────────┘                                           │
│                                                              │
│  DEPENDS ON: core, serde, thiserror (diagnostic only)        │
│  DOES NOT DEPEND ON: vb_yaml, vb_compile, saphyr             │
└─────────────────────────────────────────────────────────────┘

                              │
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              vb_runtime / vb_storage / vb_ipc                │
│              (IMPERATIVE SHELL — NO YAML)                    │
│                                                              │
│  Diagnostics use: Span::new(start, end) → line/col = None    │
│                   source_file: None                          │
│                   Diagnostic::new(code, msg, sev, Span::ZERO)│
│                                                              │
│  NO CHANGES NEEDED for span enrichment.                      │
└─────────────────────────────────────────────────────────────┘
```

---

## Pure Core vs Imperative Shell

### Pure Core (No I/O)

| Crate | Pure Components | Notes |
|---|---|---|
| `vb_core` | `Span`, `Diagnostic`, `DiagnosticCode`, `NonEmptyVec`, `Located`, `Spanned` | All pure data types |
| `vb_yaml` | `EventSpan`, `SourceSpan`, `SourceMap`, `SemanticSourceMap`, `YamlEvent`, `YamlError` | Typed event stream, source maps |
| `vb_validate` | `ValidationError` (all 50 variants), `diagnostic_from_error()` | Gate validation logic |
| `vb_compile` | `SourceMark`, `CompileError` (80 variants), `AstMarks`, bridge conversions | AST → IR lowering, error conversion |

### Imperative Shell (I/O at Boundary)

| Crate | Shell Components | Notes |
|---|---|---|
| `vb_yaml` | `parse_workflow_source()` — reads YAML text from string | Parser boundary; I/O is string input |
| `vb_compile::YamlCompiler` | Reads file or `&str`, orchestrates pipeline | File I/O happens here or at caller |
| `vb_runtime` | Diagnostic output to logs/JSON | Never constructs spans from YAML |
| `vb_storage` | Persists `CompiledWorkflow` | Storage boundary |

---

## Parser Boundaries

| Boundary | Input | Output | Span Tracking |
|---|---|---|---|
| **saphyr-parser event stream** | `&str` (YAML text) | `(Event, Span)` pairs | `Span` carries index, line, col |
| **vb_yaml event collection** | saphyr-parser events | `Vec<YamlEvent>` with `EventSpan` | `EventSpan::from_parser_span()` |
| **vb_yaml source map** | `&str` + re-parsed event stream | `SourceMap`, `SemanticSourceMap` | `build_source_map()` |
| **vb_compile AstMarks** | saphyr-parser event stream | `AstMarks` (document, nested, trigger, step) | `MarkBuilder::accept()` |
| **vb_compile tree validation** | `saphyr::Yaml` tree | `CompileError`s | **CURRENT GAP:** `unavailable()` |

---

## Bridge Conversions (NEW)

| Conversion | Location | Direction | Lossiness |
|---|---|---|---|
| `EventSpan → SourceSpan` | `vb_yaml::source_map_build::event_span_to_source_span()` | Within vb_yaml | None |
| `SourceSpan → Span` | `vb_compile` (NEW) | vb_yaml → vb_core | `usize → u32` clamp |
| `SourceSpan → SourceMark` | `vb_compile` (NEW) | vb_yaml → vb_compile | None |
| `SourceMark → Span` | `vb_compile` (NEW) | vb_compile → vb_core | `usize → u32` clamp, optional line/col based on `available` |
| `Span → diagnostic_from_error()` | `vb_validate::diagnostic` (MODIFIED) | vb_validate → vb_core | Span propagated as-is |
| `CompileError → Diagnostic` | `vb_compile` (NEW) | vb_compile → vb_core | New conversion function |

---

## Storage Boundary

- `Diagnostic` is `Serialize + Deserialize` — adding `source_file: Option<Box<str>>` does not break serde (new optional field).
- `Span` is `Serialize + Deserialize` — adding `line: Option<u32>`, `column: Option<u32>` does not break serde (new optional fields).
- `CompiledWorkflow` IR does **not** contain diagnostic or span data — no migration needed.
- `ValidationError` is NOT `Serialize` — no storage impact.

---

## Unsafe Boundary

- `#![forbid(unsafe_code)]` is declared on all affected modules: `vb_core::span`, `vb_core::diagnostic`, `vb_yaml::events_types`, `vb_yaml::source_map_types`, `vb_yaml::error`, `vb_validate`, `vb_compile::ast::marks`.
- No new `unsafe` code is introduced.
- No FFI boundary is crossed.

---

## Time / Random / Network Boundaries

- **NONE.** YAML parsing and compilation are deterministic and side-effect-free.
- `Diagnostic` construction is pure — no timestamps, no random IDs.
- `DiagnosticCode` generation is deterministic from error variant matching.

---

## Dependency Direction Validation

```
vb_yaml ──no dep──► vb_core       ✓ (correct — runtime never parses YAML)
vb_compile ──────► vb_yaml        ✓ (existing)
vb_compile ──────► vb_core        ✓ (existing)
vb_compile ──────► vb_validate    ✓ (existing)
vb_validate ─────► vb_core        ✓ (existing)
vb_core ──no dep──► vb_yaml       ✓ (correct — core is YAML-free)
```

**No new dependencies are added.** The bridge implementation `From<SourceSpan> for Span` lives in `vb_compile`, which already depends on both crates.
