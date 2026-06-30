# Boundary Map — Section 16 Symbolic Diagnostic Codes

**Bead**: vb-xi2f.10  
**Phase**: State 3 — Rust Contract

---

## 1. Architecture Layers

```
┌───────────────────────────────────────────────────────────┐
│                    IMPERATIVE SHELL                        │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐ │
│  │ vb_cli      │  │ Serialize/    │  │ External Input   │ │
│  │ (display,   │  │ Deserialize   │  │ (YAML files,     │ │
│  │  format)    │  │ (serde, JSON) │  │  HTTP, CLI args) │ │
│  └──────┬──────┘  └──────┬───────┘  └────────┬─────────┘ │
│         │                │                    │           │
├─────────┼────────────────┼────────────────────┼───────────┤
│         │     PARSER/CODEC BOUNDARY            │           │
│         │  ┌───────────────────────────────────┤           │
│         │  │  FromStr for DiagnosticCode       │           │
│         │  │  FromStr for SymbolicCode         │           │
│         │  │  serde::Deserialize for Diagnostic│           │
│         │  │  serde::Deserialize for SymbolicCode│         │
│         │  └───────────────┬───────────────────┘           │
├─────────┼──────────────────┼───────────────────────────────┤
│         │                  │                               │
│         │    PURE FUNCTIONAL CORE                          │
│         │  ┌──────────────────────────────────────┐        │
│         │  │  vb_core::diagnostic                  │        │
│         │  │  ├── SymbolicCode (newtype)           │        │
│         │  │  ├── DiagnosticCode (numeric, evolved)│        │
│         │  │  ├── Diagnostic (record)              │        │
│         │  │  ├── Severity (enum)                  │        │
│         │  │  ├── CodeCategory (enum)              │        │
│         │  │  ├── CodeRegistry (const data)        │        │
│         │  │  └── is_supported_code()              │        │
│         │  └──────────────────────────────────────┘        │
│         │                                                  │
│         │  ┌──────────────────────────────────────┐        │
│         │  │  Error Domains (per crate)           │        │
│         │  │  ├── vb_validate::ValidationError    │        │
│         │  │  │   └── code() → SymbolicCode       │        │
│         │  │  ├── vb_compile::CompileError        │        │
│         │  │  │   └── code() → SymbolicCode       │        │
│         │  │  ├── vb_yaml::YamlError              │        │
│         │  │  │   └── code() → SymbolicCode       │        │
│         │  │  ├── vb_runtime::RuntimeError        │        │
│         │  │  │   └── symbolic_code() → SymbolicCode│      │
│         │  │  └── vb_storage::JournalError        │        │
│         │  │      └── symbolic_code() → SymbolicCode│      │
│         │  └──────────────────────────────────────┘        │
│         │                                                  │
│         │  ┌──────────────────────────────────────┐        │
│         │  │  Conversion Functions                │        │
│         │  │  ├── vb_validate::diagnostic::       │        │
│         │  │  │   diagnostic_from_error()         │        │
│         │  │  │   → Diagnostic (symbolic)         │        │
│         │  │  ├── SymbolicCode::as_diagnostic_code│        │
│         │  │  │   → DiagnosticCode (numeric)      │        │
│         │  │  └── DiagnosticCode::symbolic_code() │        │
│         │  │      → Option<SymbolicCode>          │        │
│         │  └──────────────────────────────────────┘        │
│         │                                                  │
├─────────┼──────────────────────────────────────────────────┤
│         │                                                  │
│    ASYNC SHELL (not in scope for this bead)                │
│    ┌────────────────────────────────────────┐              │
│    │  vb_runtime engine (tokio)             │              │
│    │  vb_storage journal (Fjall LSM)        │              │
│    └────────────────────────────────────────┘              │
└───────────────────────────────────────────────────────────┘
```

---

## 2. Boundary Definitions

### 2.1 Pure Core

**Location**: `vb_core/src/diagnostic.rs`, error type `code()` methods in each crate

**Contents**:
- `SymbolicCode` — newtype over `&'static str`, checked against registry
- `DiagnosticCode` — packed `u16`, internal encoding
- `Diagnostic` — record: symbolic code + message + severity + span
- `Severity` — error/warning/info enum
- `CodeCategory` — code grouping enum
- `CODE_REGISTRY` — const mapping of all known codes
- `is_supported_code()` — const validation function
- Error type `code()` / `symbolic_code()` methods

**Properties**:
- No I/O (no filesystem, network, stdin/stdout)
- No time (no `Instant`, `SystemTime`)
- No randomness (no `rand`)
- No storage (no database, no files)
- No threads or async
- No `unsafe` code
- No heap allocation in hot path (`SymbolicCode` is `Copy`; `DiagnosticCode` is `Copy`)
- `Diagnostic` allocates `Box<str>` for message (cold path only)

### 2.2 Parser/Codec Boundary

**Location**: `FromStr` impls, `serde::Deserialize` impls

**Contents**:
- `DiagnosticCode::from_str("E0101")` — parses E-hex format
- `SymbolicCode::from_static("DUPLICATE_KEY")` — validates against registry
- `serde::Deserialize for Diagnostic` — deserializes symbolic code string, validates
- `serde::Serialize for Diagnostic` — serializes symbolic code as string

**Properties**:
- Validates external input
- Returns `Result`, never panics
- Rejects malformed input with typed errors
- `is_supported_code()` gate ensures only registered numeric codes parse

### 2.3 Imperative Shell

**Location**: `vb_cli/src/app_impl.rs`

**Contents**:
- `explain_error()` — formats diagnostics for human display
- `explain_compile_repair_hint()` — repair suggestions
- Terminal output formatting

**Properties**:
- I/O is allowed (writes to stdout/stderr)
- Formats symbolic codes for user display
- Does NOT parse codes — delegates to parser boundary

### 2.4 Async Shell

**Location**: `vb_runtime`, `vb_storage`

**Contents**:
- Runtime engine (tokio-based)
- Storage journal (Fjall LSM)

**Properties**:
- Error types in these crates produce `SymbolicCode` values
- Code resolution is pure (no async in `symbolic_code()`)
- Diagnostic emission may happen in async context but is a pure operation

### 2.5 Unsafe/FFI Boundary

**None in scope**. Diagnostic code infrastructure contains no `unsafe` code. No FFI.

---

## 3. Data Flow

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│ Error occurs │ ──→ │ code() called│ ──→ │ SymbolicCode │ ──→ │ Diagnostic   │
│ (variant     │     │ (pure,       │     │ resolved     │     │ record       │
│  constructed)│     │  zero-alloc) │     │ (Copy type)  │     │ (allocates   │
└──────────────┘     └──────────────┘     └──────────────┘     │ Box<str> for │
                                                               │ message)     │
                                                               └──────┬───────┘
                                                                      │
                                                    ┌─────────────────┼─────┐
                                                    ▼                 ▼     ▼
                                              ┌───────────┐  ┌─────────┐ ┌─────┐
                                              │ CLI format │  │ JSON    │ │ Log │
                                              │ (human)    │  │ (serde) │ │     │
                                              └───────────┘  └─────────┘ └─────┘
```

---

## 4. Crate Dependency Graph (Post-Change)

```
vb_core ── contains SymbolicCode, DiagnosticCode, Diagnostic, CodeRegistry
    ↑
    ├── vb_validate ── ValidationError::code() → SymbolicCode
    │       ↑
    │       └── vb_compile ── CompileError::code() → SymbolicCode
    │                           (wraps ValidationError, adds own)
    │
    ├── vb_yaml ── YamlError::code() → SymbolicCode (NEW)
    │       ↑
    │       └── vb_compile ── (already depends on vb_yaml)
    │
    ├── vb_runtime ── RuntimeError::symbolic_code() → SymbolicCode
    │
    ├── vb_storage ── JournalError::symbolic_code() → SymbolicCode
    │
    └── vb_cli ── (consumes all error types, formats diagnostic output)
```

**Key**: All arrows point to `vb_core`. No circular dependencies. `vb_core` is the root of the diagnostic dependency tree.

---

## 5. Boundary Invariants

| Boundary | Invariant |
|----------|-----------|
| Pure Core → Parser | `SymbolicCode::from_static(s)` must not access any external resource. |
| Parser → Pure Core | Parsed `DiagnosticCode` must pass `is_supported_code()` before entering core. |
| Parser → Pure Core | Parsed `SymbolicCode` must be validated against `CODE_REGISTRY`. |
| Error → Diagnostic | `Diagnostic.code` is always a valid `SymbolicCode`; `Diagnostic.numeric_code` is always its derived numeric value. |
| CLI Display | CLI formats symbolic code for display but does not parse or construct codes. |
| Serialization | `Diagnostic` serializes symbolic code as a string; deserialization validates against registry. |

---

## 6. Separation of Concerns

| Concern | Owned By | Reason |
|---------|----------|--------|
| Code identity (symbolic) | `vb_core::SymbolicCode` | Single source of truth |
| Code identity (numeric) | `vb_core::DiagnosticCode` | Internal encoding, derived from symbolic |
| Code registry | `vb_core::CODE_REGISTRY` | Cross-crate consistency |
| Error semantics | Per-crate error enums | Domain-specific error meaning |
| Error → code mapping | Per-crate `code()` methods | Each error knows its own code |
| Diagnostic formatting | `vb_core::Diagnostic` | Record structure |
| User display | `vb_cli` | Human-readable output |
| Serialization format | `serde` impls on `vb_core` types | Wire format |
