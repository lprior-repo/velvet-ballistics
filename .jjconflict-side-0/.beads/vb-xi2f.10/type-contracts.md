# Type Contracts — Section 16 Symbolic Diagnostic Codes

**Bead**: vb-xi2f.10  
**Phase**: State 3 — Rust Contract

---

## 1. `SymbolicCode` — Primary Stable Diagnostic Identifier

### Purpose

The first-class, stable, symbolic error code type. Replaces stringly codes and makes invalid states unrepresentable.

### Type Contract

```rust
/// Stable symbolic diagnostic code.
/// Wraps a `&'static str` from the canonical registry.
/// Cannot represent unregistered codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SymbolicCode(&'static str);
```

### Smart Constructors

| Constructor | Signature | Contract |
|------------|-----------|----------|
| `from_static` | `const fn from_static(s: &'static str) -> Option<Self>` | Accepts only strings listed in the canonical registry. Returns `None` for unregistered strings. |
| `from_str` | `fn from_str(s: &str) -> Option<Self>` | Delegates to `from_static`; returns `None` for non-registry strings. |

### Methods

| Method | Signature | Contract |
|--------|-----------|----------|
| `as_str` | `fn as_str(&self) -> &'static str` | Returns the symbolic name (e.g., `"DUPLICATE_KEY"`). |
| `numeric_code` | `fn numeric_code(&self) -> u16` | Returns the packed numeric encoding. Deterministic, bijective with the symbolic name. |
| `as_diagnostic_code` | `fn as_diagnostic_code(&self) -> DiagnosticCode` | Returns the equivalent `DiagnosticCode(u16)` for backward-compatible consumers. |
| `category` | `fn category(&self) -> CodeCategory` | Returns the high-level category (Schema, Reference, ControlFlow, TypeTaint, Gate, ContractDiscovery, Storage, Runtime, Compilation). |

### Trait Implementations

| Trait | Contract |
|-------|----------|
| `Display` | Formats as the symbolic name (e.g., `"DUPLICATE_KEY"`). |
| `Serialize` / `Deserialize` | Serializes as the symbolic string `"DUPLICATE_KEY"`. During deserialization, validates against the registry and rejects unknown codes. |
| `FromStr` | Parses from symbolic name `"DUPLICATE_KEY"`. Rejects unknown names. |
| `Copy` | Trivially copyable — zero allocation. |

### Type-Safety Properties

- **No invalid codes**: A `SymbolicCode` value is always a registered diagnostic code. This is enforced by the smart constructor.
- **No stringly errors**: Consumers never parse error display strings to extract codes.
- **Bijection**: Each `SymbolicCode` maps to exactly one numeric `u16` and vice versa.

### CodeChecklist Compliance

- [x] Replaces stringly IDs with newtype: `SymbolicCode` replaces bare `&'static str`.
- [x] No boolean behavior flags: uses `CodeCategory` enum.
- [x] No `Option` lifecycle state: `SymbolicCode` has no lifecycle.
- [x] Parse external input once: `FromStr` validates against registry.
- [x] Semantic error variants: registry lookup failure is a parse error.
- [x] Pure core: no I/O, time, network, storage, or randomness.

---

## 2. `CodeCategory` — Code Grouping Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CodeCategory {
    /// Schema validation: E01xx
    Schema,
    /// Reference validation: E02xx
    Reference,
    /// Control-flow validation: E03xx
    ControlFlow,
    /// Type/taint/resource: E04xx
    TypeTaint,
    /// Gate verifier: E05xx
    Gate,
    /// Contract discovery: E06xx
    ContractDiscovery,
    /// Internal compilation: E10xx
    Compilation,
    /// Workflow IR: E11xx
    WorkflowIr,
    /// Expression: E12xx
    Expression,
    /// Accessor/Path: E13xx
    Accessor,
    /// Lowering: E14xx
    Lowering,
    /// Storage: E20xx
    Storage,
    /// Runtime core: E30xx
    Runtime,
    /// Runtime boundary: E40xx
    RuntimeBoundary,
}
```

---

## 3. `DiagnosticCode` — Numeric Encoding (Evolved)

### Purpose

Retained internal encoding type. The packed `u16` is suitable for wire protocols and storage. Extended with symbolic knowledge.

### Type Contract

```rust
/// Packed numeric diagnostic code (e.g., 0x0101 = "E0101").
/// Internal encoding detail. The symbolic `SymbolicCode` is the stable public contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct DiagnosticCode(u16);
```

### Methods (existing retained plus new)

| Method | Signature | Contract |
|--------|-----------|----------|
| `new` (existing) | `const fn new(code: u16) -> Self` | Constructs from raw `u16`. No validity check at construction time (caller must ensure correctness). |
| `code` (existing) | `const fn code(self) -> u16` | Returns the packed `u16`. |
| `symbolic_code` (new) | `fn symbolic_code(self) -> Option<SymbolicCode>` | Reverse-lookup from numeric to symbolic. Returns `None` if the numeric code is not in the registry. |
| `category` (new) | `fn category(self) -> Option<CodeCategory>` | Returns the code's category, derived from the high byte. |

### `FromStr` Contract (updated)

```rust
impl FromStr for DiagnosticCode {
    type Err = DiagnosticCodeParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        // Parse format: exactly "E" + 4 hex digits.
        // Validate: pack_digits produces a u16.
        // Guard: is_supported_code(code) must return true.
        //          Updated: includes E05xx, E06xx, and codes above 0x401B.
    }
}
```

### `is_supported_code()` Contract (updated)

```rust
const fn is_supported_code(code: u16) -> bool {
    matches!(
        code,
        // Schema: E01xx
        0x0101..=0x010B
        // Reference: E02xx
        | 0x0201..=0x0204
        // Control Flow: E03xx
        | 0x0301..=0x0309
        // Type/Taint: E04xx
        | 0x0401..=0x040C
        // Gate Verifier: E05xx (NEW)
        | 0x0501..=0x0513
        // Contract Discovery: E06xx (NEW)
        | 0x0601..=0x0603
        // Compilation Internal: E10xx
        | 0x1001..=0x1002
        | 0x1011..=0x1013
        // Workflow IR: E11xx
        | 0x1101..=0x1104
        // Expression: E12xx
        | 0x1201..=0x1202
        // Accessor/Path: E13xx
        | 0x1301..=0x130D
        | 0x1311..=0x1314
        // Lowering: E14xx
        | 0x1401..=0x1407
        // Storage: E20xx
        | 0x2001..=0x200F
        // Runtime Core: E30xx
        | 0x3001..=0x300E
        // Runtime Boundary: E40xx
        | 0x4001..=0x401C   // Extended from 0x401B to include JournalError codes
    )
}
```

**Gate**: Every numeric code constant defined in `vb_validate/src/diagnostic.rs` (lines 16–83) and `vb_storage/src/error/codes.rs` must be accepted by `is_supported_code()`.

---

## 4. `Diagnostic` — User-Facing Record (Evolved)

### Type Contract

```rust
/// User-facing diagnostic with stable symbolic code and source span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Symbolic diagnostic code (primary identifier).
    pub code: SymbolicCode,
    /// Owned human-readable message.
    pub message: Box<str>,
    /// Diagnostic severity.
    pub severity: Severity,
    /// Source span for the diagnostic.
    pub span: Span,
    /// Derived numeric code (for backward-compatible consumers).
    /// This is deterministic: code.symbolic_code().numeric_code() == self.numeric_code.code().
    pub numeric_code: DiagnosticCode,
}
```

### Type-Safety Properties

- `code` is `SymbolicCode` — always a valid registered code.
- `numeric_code` is derived, not independently settable.
- Invariant: `numeric_code.symbolic_code() == Some(code)`.
- The `message` field is `Box<str>` (not `String`) — capacity not exposed.

### Constructor Contract

| Constructor | Contract |
|------------|----------|
| `Diagnostic::new(code: SymbolicCode, message: Box<str>, severity: Severity, span: Span) -> Self` | Constructs a diagnostic. Derives `numeric_code` from `code`. Panic-free: `SymbolicCode::as_diagnostic_code()` is infallible. |

### Backward Compatibility

Existing code that accesses `.code.code()` (numeric) must migrate to `.numeric_code.code()`. The `.code` field changes type from `DiagnosticCode` to `SymbolicCode`.

---

## 5. `ValidationError` — `code()` Method (New)

### Contract

```rust
impl ValidationError {
    /// Returns the symbolic diagnostic code for this validation error.
    /// Every variant maps to exactly one Section 16/17 code.
    #[must_use]
    pub fn code(&self) -> SymbolicCode;
}
```

### Mapping Table (excerpt — all 58+ variants)

| ValidationError Variant | SymbolicCode | Numeric |
|------------------------|-------------|---------|
| `DuplicateKey` | `DUPLICATE_KEY` | 0x0101 |
| `ForbiddenYamlFeature` | `FORBIDDEN_YAML_FEATURE` | 0x0102 |
| `UnknownTopLevelField` | `UNKNOWN_TOP_LEVEL_FIELD` | 0x0103 |
| `UnknownStepField` | `UNKNOWN_STEP_FIELD` | 0x0104 |
| `MissingRequiredField` | `MISSING_REQUIRED_FIELD` | 0x0105 |
| `InvalidVersion` | `INVALID_VERSION` | 0x0106 |
| `InvalidId` | `INVALID_ID` | 0x0107 |
| `ReservedId` | `RESERVED_ID` | 0x0108 |
| `DuplicateId` | `DUPLICATE_ID` | 0x0109 |
| `MultipleStepPrimitives` | `MULTIPLE_STEP_PRIMITIVES` | 0x010A |
| `MissingStepPrimitive` | `MISSING_STEP_PRIMITIVE` | 0x010B |
| `UnknownReference` | `UNKNOWN_REFERENCE` | 0x0201 |
| `FutureReference` | `FUTURE_REFERENCE` | 0x0202 |
| `SecretNotDeclared` | `SECRET_NOT_DECLARED` | 0x0203 |
| `DirectRuntimeReference` | `DIRECT_RUNTIME_REFERENCE` | 0x0204 |
| `InvalidThenTarget` | `INVALID_THEN_TARGET` | 0x0301 |
| `ControlFlowCycle` | `CONTROL_FLOW_CYCLE` | 0x0302 |
| `UnreachableStep` | `UNREACHABLE_STEP` | 0x0303 |
| `InvalidChoose` | `INVALID_CHOOSE` | 0x0304 |
| `InvalidForEach` | `INVALID_FOR_EACH` | 0x0305 |
| `InvalidTogether` | `INVALID_TOGETHER` | 0x0306 |
| `InvalidCollect` | `INVALID_COLLECT` | 0x0307 |
| `InvalidReduce` | `INVALID_REDUCE` | 0x0308 |
| `InvalidRepeat` | `INVALID_REPEAT` | 0x0309 |
| `InvalidWait` | `INVALID_WAIT` | 0x0401 |
| `InvalidAsk` | `INVALID_ASK` | 0x0402 |
| `InvalidFinish` | `INVALID_FINISH` | 0x0403 |
| `InvalidRetry` | `INVALID_RETRY` | 0x0404 |
| `InvalidOnError` | `INVALID_ON_ERROR` | 0x0405 |
| `SecretResultLeak` | `SECRET_RESULT_LEAK` | 0x0406 |
| `TypeMismatch` | `TYPE_MISMATCH` | 0x0407 |
| `PayloadTooLarge` | `PAYLOAD_TOO_LARGE` | 0x0408 |
| `LimitRequired` | `LIMIT_REQUIRED` | 0x0409 |
| `LimitExceeded` | `LIMIT_EXCEEDED` | 0x040A |
| `UnsupportedTrigger` | `UNSUPPORTED_TRIGGER` | 0x040B |
| `HttpTriggerOutOfCore` | `HTTP_TRIGGER_OUT_OF_CORE` | 0x040C |
| *Gate verifier variants (19)* | *E05xx codes* | 0x0501–0x0513 |
| *Contract discovery variants (3)* | *E06xx codes* | 0x0601–0x0603 |

**Full mapping**: The internal `error_diagnostic_parts()` function in `vb_validate/src/diagnostic.rs` is the source of truth. The `code()` method delegates to this mapping. The symbolic name is derived from the numeric code via the registry.

---

## 6. `YamlError` — `code()` Method (New)

### Contract

```rust
impl YamlError {
    /// Returns the symbolic diagnostic code for this YAML error.
    #[must_use]
    pub fn code(&self) -> SymbolicCode;
}
```

### Mapping Table

| YamlError Variant | SymbolicCode | Rationale |
|------------------|-------------|-----------|
| `DuplicateKey { key }` | `DUPLICATE_KEY` | Same semantic as Section 16 DUPLICATE_KEY |
| `ForbiddenFeature { .. }` | `FORBIDDEN_YAML_FEATURE` | Direct match |
| `AnchorAliasMerge` | `FORBIDDEN_YAML_FEATURE` | Anchors/aliases/tags are forbidden YAML features |
| `CustomTag { .. }` | `FORBIDDEN_YAML_FEATURE` | Tags are forbidden |
| `BinaryScalar` | `FORBIDDEN_YAML_FEATURE` | Binary scalars are forbidden |
| `AmbiguousScalar { .. }` | `FORBIDDEN_YAML_FEATURE` | YAML 1.1 ambiguity is rejected |
| `UnsupportedTrigger { .. }` | `UNSUPPORTED_TRIGGER` | Direct match |
| `UnsupportedFeature { .. }` | `FORBIDDEN_YAML_FEATURE` | Unsupported features are forbidden |
| `MultipleDocuments { .. }` | `FORBIDDEN_YAML_FEATURE` | Multiple documents rejected |
| `SourceTooLarge { .. }` | `PAYLOAD_TOO_LARGE` | Size constraints |
| `NestingTooDeep { .. }` | `LIMIT_EXCEEDED` | Depth limits |
| `NodeLimitExceeded { .. }` | `LIMIT_EXCEEDED` | Node counts |
| `ScalarTooLong { .. }` | `LIMIT_EXCEEDED` | Scalar limits |
| `SequenceTooLong { .. }` | `LIMIT_EXCEEDED` | Sequence limits |
| `MappingTooLarge { .. }` | `LIMIT_EXCEEDED` | Mapping limits |
| `UnknownField { .. }` | `UNKNOWN_TOP_LEVEL_FIELD` | Unknown fields |
| `EmptySource` | `MISSING_REQUIRED_FIELD` | Empty source is missing content |
| `MissingField { .. }` | `MISSING_REQUIRED_FIELD` | Direct match |
| `FieldShape { .. }` | `TYPE_MISMATCH` | Field shape is a type mismatch |
| `ParseError { .. }` | `FORBIDDEN_YAML_FEATURE` | Parse errors at YAML level |

---

## 7. `CompileError` — `code()` Method (Existing, Extended)

### Contract

Existing contract preserved. The return type is updated from `&'static str` to `SymbolicCode`:

```rust
impl CompileError {
    /// Stable machine-readable validation diagnostic code.
    #[must_use]
    pub fn code(&self) -> SymbolicCode;
}
```

### Additional Symbolic Codes Used by CompileError

Beyond the 36 Section 16 codes, CompileError currently emits these additional symbolic strings that must be registered:

| String | Proposed CodeCategory | Notes |
|--------|----------------------|-------|
| `"UNKNOWN_INPUT_SCHEMA_FIELD"` | Compilation | Not in Section 16; compiler-specific |
| `"UNSUPPORTED_TOP_LEVEL_DECLARATION"` | Compilation | Compiler-specific |
| `"UNKNOWN_OUTPUT_NAME"` | Compilation | Compiler-specific |
| `"UNSUPPORTED_ACCESSOR_REFERENCE"` | Compilation | Compiler-specific |
| `"INVALID_EXPRESSION"` | Expression | Compiler-specific |
| `"IDEMPOTENCY_VIOLATION"` | Compilation | Compiler-specific |
| `"INVALID_COMPILED_WORKFLOW"` | WorkflowIr | Compiler-specific |
| `"CONST_OUT_OF_BOUNDS"` | WorkflowIr | Compiler-specific |

**All 8 must be added to the registry.**

---

## 8. `CoreError`, `RuntimeError`, `JournalError` — Symbolic Code Methods

### Contract

Each error type gains a `symbolic_code() -> SymbolicCode` method (or `code() -> SymbolicCode` if naming conventions allow). The existing `diagnostic_code() -> DiagnosticCode` methods are retained.

| Error Type | Existing Method | New Method | Notes |
|-----------|----------------|-----------|-------|
| `CoreError` | `diagnostic_code() -> DiagnosticCode` | `symbolic_code() -> SymbolicCode` | Has `runtime_code() -> Option<&'static str>` for Section 17 — both can now use `SymbolicCode` |
| `RuntimeError` | `diagnostic_code() -> DiagnosticCode` | `symbolic_code() -> SymbolicCode` | Has `runtime_code() -> Option<&'static str>` |
| `JournalError` | `diagnostic_code() -> DiagnosticCode` | `symbolic_code() -> SymbolicCode` | 28 numeric constants in `codes.rs` need symbolic equivalents |

---

## 9. `Severity` — Unchanged

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Severity {
    Error,
    Warning,
    Info,
}
```

No changes. `Severity` does not carry diagnostic codes.

---

## 10. `DiagnosticCodeParseError` — Unchanged Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum DiagnosticCodeParseError {
    #[error("diagnostic code must use format E0101")]
    InvalidFormat,
    #[error("diagnostic code is outside the supported ranges")]
    UnsupportedCode,
}
```

The `UnsupportedCode` variant's doc/message should clarify it applies to numeric E-style codes; symbolic codes use a separate parse error type or `SymbolicCodeParseError`.

---

## 11. Code Registry — `vb_core::diagnostic::registry`

### Purpose

A const-validated, compile-time checked mapping of all diagnostic codes.

### Shape

```rust
/// Registry entry: symbolic name + numeric encoding + category.
pub struct CodeEntry {
    pub symbolic: &'static str,
    pub numeric: u16,
    pub category: CodeCategory,
}

/// Canonical code registry. All diagnostic codes used anywhere in the workspace
/// MUST appear here.
pub const CODE_REGISTRY: &[CodeEntry] = &[
    CodeEntry { symbolic: "DUPLICATE_KEY",               numeric: 0x0101, category: CodeCategory::Schema },
    CodeEntry { symbolic: "FORBIDDEN_YAML_FEATURE",       numeric: 0x0102, category: CodeCategory::Schema },
    // ... all 90+ entries ...
];

/// Lookup: symbolic → numeric.
pub const fn symbolic_to_numeric(symbolic: &str) -> Option<u16>;

/// Lookup: numeric → symbolic.
pub const fn numeric_to_symbolic(numeric: u16) -> Option<&'static str>;
```

### Compile-Time Validation

A `const` assertion verifies:
- No duplicate symbolic names.
- No duplicate numeric codes.
- Every entry has a valid category.
- Numeric codes are non-zero.

---

## 12. Trait: `HasSymbolicCode`

```rust
/// Trait for error types that carry a symbolic diagnostic code.
pub trait HasSymbolicCode {
    /// Returns the symbolic diagnostic code for this error.
    fn symbolic_code(&self) -> SymbolicCode;
}
```

Implementers: `ValidationError`, `CompileError`, `YamlError`, `CoreError`, `RuntimeError`, `JournalError`.

---

## Checklist Compliance

- [x] Replace stringly IDs: `SymbolicCode` newtype replaces bare `&'static str`.
- [x] Replace boolean flags: `CodeCategory` enum replaces range-based flag checks.
- [x] Replace `Option` lifecycle: Not applicable (codes are stateless).
- [x] Parse external input at boundary: `FromStr` validates against registry; `SymbolicCode` smart constructor.
- [x] Semantic error variants: Registry lookup failure, parse failure, unsupported code.
- [x] Pure core: Registry, `SymbolicCode`, `DiagnosticCode` — no I/O/time/network/storage.
