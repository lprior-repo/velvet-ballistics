# Workflow Model — Section 16 Symbolic Diagnostic Codes

**Bead**: vb-xi2f.10  
**Phase**: State 3 — Rust Contract

---

## 1. State Machine: Diagnostic Code Lifecycle

### States

```
┌──────────┐    register     ┌──────────┐    deprecate    ┌───────────┐
│ Proposed │ ──────────────→ │  Active  │ ──────────────→ │ Deprecated│
│          │                 │          │                 │           │
│ (not in  │                 │ (in      │                 │ (in       │
│ registry)│                 │ registry │                 │ registry, │
└──────────┘                 │ usable)  │                 │ not       │
                             └──────────┘                 │ emitted)  │
                                                          └───────────┘
```

| State | Description | Legal Operations |
|-------|-------------|-----------------|
| **Proposed** | A symbolic/numeric code pair has been defined in source but not yet in the registry. | Unit testing, internal use only. Cannot appear in public `SymbolicCode`. |
| **Active** | The code is registered, usable, and emitted by at least one error variant. | All operations: construct, match, display, serialize. |
| **Deprecated** | The code was previously active but is no longer emitted. Retained for backward compatibility in deserialization. | Deserialize, display (with deprecation note). Cannot be constructed from new errors. |

### Transitions

| Transition | Guard | Action |
|-----------|-------|--------|
| Proposed → Active | Code satisfies all registry invariants (unique symbolic, unique numeric, valid category). | Add to `CODE_REGISTRY`. `SymbolicCode::from_static()` now returns `Some`. |
| Active → Deprecated | No error variant in any crate still maps to this code. All former uses migrated to a new code. | Mark deprecated in registry. Deserialization still accepts. Construction from error variants fails. |

**Invariant**: A code may not transition Active → Deprecated if any public error variant still maps to it.

---

## 2. State Machine: Error → Diagnostic Resolution

### States

```
┌──────────┐   error occurs    ┌───────────┐   code() called   ┌──────────┐
│  NoError │ ────────────────→ │  ErrorSet  │ ────────────────→ │ Resolved │
│          │                   │ (variant   │                   │          │
│          │                   │  known)    │                   │ (Symbolic│
└──────────┘                   └───────────┘                   │ Code)    │
                                                                └──────────┘
```

| State | Description |
|-------|-------------|
| **NoError** | No error condition exists. |
| **ErrorSet** | An error variant has been constructed. Its symbolic code is determined but not yet extracted. |
| **Resolved** | The `code()` / `symbolic_code()` method has been called and a `SymbolicCode` returned. |

### Transition Contract

For every error variant `E`, the transition ErrorSet → Resolved must be:
1. **Deterministic**: `E.code()` always returns the same `SymbolicCode`.
2. **Total**: `E.code()` never panics, never returns `None`.
3. **Pure**: No allocation, no I/O, no side effects.

---

## 3. Workflow: Diagnostic Code Registration (Adding a New Code)

### Commands

```
Command: RegisterCode { symbolic: &'static str, numeric: u16, category: CodeCategory }
```

### Workflow

```
┌──────────────────┐
│ Validate symbolic │ ──→ Not unique? ──→ REJECT: DuplicateSymbolicCode
│ name uniqueness   │
└──────┬───────────┘
       │ unique
       ▼
┌──────────────────┐
│ Validate numeric  │ ──→ Not unique? ──→ REJECT: DuplicateNumericCode
│ value uniqueness  │
└──────┬───────────┘
       │ unique
       ▼
┌──────────────────┐
│ Validate numeric  │ ──→ Zero? ──→ REJECT: ZeroCodeNotAllowed
│ non-zero          │
└──────┬───────────┘
       │ valid
       ▼
┌──────────────────┐
│ Validate category │ ──→ Category mismatch with high-byte? ──→ REJECT: CategoryRangeMismatch
│ matches range     │
└──────┬───────────┘
       │ valid
       ▼
┌──────────────────┐
│ Add to CODE_      │
│ REGISTRY          │ ──→ Success: code is now Active
└──────────────────┘
```

### Guards

| Guard | Condition | Rejection |
|-------|-----------|-----------|
| G1 | Symbolic name not already in registry | `DuplicateSymbolicCode` |
| G2 | Numeric value not already in registry | `DuplicateNumericCode` |
| G3 | Numeric value != 0 | `ZeroCodeNotAllowed` |
| G4 | High byte of numeric matches category range | `CategoryRangeMismatch` |

### Category Range Guards

| Category | Valid High Byte Range |
|----------|----------------------|
| Schema | `0x01` |
| Reference | `0x02` |
| ControlFlow | `0x03` |
| TypeTaint | `0x04` |
| Gate | `0x05` |
| ContractDiscovery | `0x06` |
| Compilation | `0x10` |
| WorkflowIr | `0x11` |
| Expression | `0x12` |
| Accessor | `0x13` |
| Lowering | `0x14` |
| Storage | `0x20` |
| Runtime | `0x30` |
| RuntimeBoundary | `0x40` |

---

## 4. Workflow: Diagnostic Emission (Error → User-Facing Diagnostic)

### Commands

```
Command: EmitDiagnostic { error: impl HasSymbolicCode, span: Span, message: String }
```

### Workflow

```
┌──────────────────┐
│ error.symbolic_   │
│ code()            │ ──→ always returns Some(SymbolicCode)
└──────┬───────────┘
       │ SymbolicCode
       ▼
┌──────────────────┐
│ SymbolicCode::as_ │
│ diagnostic_code() │ ──→ DiagnosticCode(u16) (derived, infallible)
└──────┬───────────┘
       │
       ▼
┌──────────────────┐
│ Diagnostic::new   │
│ (code, message,   │
│  severity, span)  │ ──→ Diagnostic { code: SymbolicCode, numeric_code: DiagnosticCode, ... }
└──────────────────┘
```

**Outcome**: A `Diagnostic` record with both symbolic and numeric codes. The symbolic code is the primary identifier.

---

## 5. Workflow: Parsing Diagnostic Codes from External Input

### Commands

```
Command: ParseNumericCode("E0101")  → DiagnosticCode
Command: ParseSymbolicCode("DUPLICATE_KEY")  → SymbolicCode
```

### Numeric Parse Workflow

```
┌──────────────────┐
│ validate format   │ ──→ not "E" + 4 hex? ──→ DiagnosticCodeParseError::InvalidFormat
│ "E" + 4 hex       │
└──────┬───────────┘
       │ valid format
       ▼
┌──────────────────┐
│ pack digits       │ ──→ overflow? ──→ DiagnosticCodeParseError::InvalidFormat
│ to u16            │
└──────┬───────────┘
       │ packed
       ▼
┌──────────────────┐
│ is_supported_     │ ──→ false? ──→ DiagnosticCodeParseError::UnsupportedCode
│ code(u16)         │
└──────┬───────────┘
       │ true
       ▼
  Ok(DiagnosticCode)
```

### Symbolic Parse Workflow

```
┌──────────────────┐
│ lookup in CODE_   │ ──→ not found? ──→ SymbolicCodeParseError::UnknownCode
│ REGISTRY          │
└──────┬───────────┘
       │ found
       ▼
  Ok(SymbolicCode)
```

---

## 6. Workflow: Code Mapping Consistency Audit

### Command

```
Command: AuditCodeRegistry
```

### Workflow

```
┌──────────────────┐
│ Check symbolic    │ ──→ duplicates found? ──→ FAIL: Duplicate symbolic names
│ name uniqueness   │
└──────┬───────────┘
       │ unique
       ▼
┌──────────────────┐
│ Check numeric     │ ──→ duplicates found? ──→ FAIL: Duplicate numeric codes
│ value uniqueness  │
└──────┬───────────┘
       │ unique
       ▼
┌──────────────────┐
│ Check all in-use  │ ──→ missing? ──→ FAIL: Unregistered in-use code
│ codes registered  │
└──────┬───────────┘
       │ all registered
       ▼
┌──────────────────┐
│ Check all error   │ ──→ variant missing code? ──→ FAIL: Error variant without code
│ variants covered  │
└──────┬───────────┘
       │ all covered
       ▼
┌──────────────────┐
│ Check bijection:  │ ──→ round-trip fails? ──→ FAIL: Registry bijection broken
│ sym↔num           │
└──────┬───────────┘
       │ bijection holds
       ▼
     PASS
```

---

## 7. Cancellation, Retries, and Idempotence

| Aspect | Property |
|--------|----------|
| **Cancellation** | Diagnostic code resolution is pure and atomic — no cancellation points. |
| **Retries** | Code resolution is deterministic — retrying always produces the same result. |
| **Idempotence** | `error.code()` called N times always returns the same `SymbolicCode`. `SymbolicCode::from_static(s)` always returns the same `Option<SymbolicCode>`. |
| **Heap allocation** | Code resolution is zero-allocation. Diagnostic message creation allocates once. |

---

## 8. Temporal Hazards

| Hazard | Risk | Mitigation |
|--------|------|-----------|
| **Registry drift** | A new error variant is added but not registered. | Audit workflow catches this; CI gate. |
| **Symbolic renaming** | A symbolic name changes between releases. | Policy: symbolic names are append-only. Deprecate, never rename. |
| **Numeric reallocation** | A numeric code is reassigned to a different symbolic name. | Policy: numeric codes are append-only. Deprecate, never reassign. |
| **Concurrent registry writes** | Two crates independently define the same numeric code for different purposes. | Single source of truth in `vb_core`. All crates import from it. |
| **Deserialization of unknown codes** | A new version serializes codes unknown to an older version. | `Deserialize` for `SymbolicCode` rejects unknown codes. `Deserialize` for `DiagnosticCode` (numeric) accepts all valid-format codes. |

---

## 9. Concurrency Hazards

| Hazard | Risk | Mitigation |
|--------|------|-----------|
| **Concurrent error emission** | Two threads emit diagnostics simultaneously. | `SymbolicCode` is `Copy` + `Send` + `Sync`. `Diagnostic` construction is allocation-only; Box<str> is thread-safe. |
| **Registry reading** | Multiple threads read the registry simultaneously. | `CODE_REGISTRY` is a `const` slice — statically initialized, immutable at runtime. No synchronization needed. |

---

## 10. Error Recovery Paths

| Error Scenario | Recovery Path |
|---------------|---------------|
| `DiagnosticCodeParseError::InvalidFormat` | Return error to caller. Do not construct a `DiagnosticCode`. |
| `DiagnosticCodeParseError::UnsupportedCode` | Return error. The code range is valid format but not registered. |
| `SymbolicCodeParseError::UnknownCode` | Return error. The string is not in the registry. |
| Deserialization of unknown `SymbolicCode` | Deserialization error. Consumer must handle unknown-code gracefully (log, skip, fail). |
| `is_supported_code()` rejects a valid in-use code | CI audit gate failure. The registry and `is_supported_code()` must be kept in sync. |
