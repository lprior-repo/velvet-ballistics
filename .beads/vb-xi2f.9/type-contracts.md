# Type Contracts: YAML Path and Source Span Diagnostics

**Bead:** vb-xi2f.9  
**Agent:** rust-contract (State 3)  
**Schema:** 2026-05-24

## Type Contract Checklist

- [x] Replace stringly IDs and primitive domain values with newtypes.
- [x] Replace boolean behavior flags with enums.
- [x] Replace `Option` lifecycle state with explicit state variants.
- [x] Parse external input once at the boundary.
- [x] Represent domain failures with semantic error variants.
- [x] Keep pure core free of I/O, time, network, storage, and randomness.

---

## TC-01: Enriched Span (vb_core)

### Specification

```rust
/// Byte-offset span with optional human-readable line/column coordinates.
///
/// # Invariants
/// - `start <= end`
/// - `line.is_some() == column.is_some()` (both present or both absent)
/// - `Span::ZERO` is the canonical empty/unknown span
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
    pub line: Option<u32>,    // ADDED: 1-indexed start line
    pub column: Option<u32>,  // ADDED: 1-indexed start column
}
```

### Smart Constructor Contract

```rust
impl Span {
    pub const ZERO: Self = Self { start: 0, end: 0, line: None, column: None };

    /// Byte-offset-only span (runtime core usage).
    /// Pre: start <= end
    pub const fn new(start: u32, end: u32) -> Self;

    /// Span with full location data (authoring usage).
    /// Pre: start <= end, line >= 1, column >= 1
    pub const fn with_location(
        start: u32, end: u32,
        line: u32, column: u32,
    ) -> Self;

    /// Returns true when the span covers no bytes.
    pub const fn is_empty(self) -> bool;

    /// Returns the human-readable location if available.
    pub const fn location(self) -> Option<(u32, u32)>;
}
```

### Contracts

- **TC-01a:** `Span::with_location(s, e, l, c)` produces a span where `line == Some(l)` and `column == Some(c)`.
- **TC-01b:** `Span::new(s, e)` produces a span where `line.is_none()` and `column.is_none()`.
- **TC-01c:** `Span::ZERO` equals `Span::new(0, 0)` and equals `Span::default()`.
- **TC-01d:** `Span::is_empty()` returns `true` iff `start == end`.
- **TC-01e:** Constructing `Span::with_location(s, e, 0, c)` is a compile-legal but semantically invalid state — callers must ensure `line >= 1, column >= 1`.
- **TC-01f:** `Located<T>` and `Spanned<T>` wrappers remain unchanged; they delegate to `Span` semantics.
- **TC-01g:** **Backward compat:** All existing callers using `Span::new()` continue to compile and produce spans with `line: None, column: None`. `Span::ZERO` is unchanged.

---

## TC-02: Enriched Diagnostic (vb_core)

### Specification

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub message: Box<str>,
    pub severity: Severity,
    pub span: Span,                         // now carries optional line/col
    pub source_file: Option<Box<str>>,      // ADDED: path to source file
}
```

### Contracts

- **TC-02a:** `Diagnostic::new()` with `Span::ZERO` and `source_file: None` produces a valid diagnostic (backward compat).
- **TC-02b:** `source_file: Some(path)` implies the diagnostic came from a YAML authoring source file.
- **TC-02c:** `source_file: None` implies the diagnostic was produced at runtime or from an un-named source.
- **TC-02d:** When `source_file` is `Some(s)`, `s` is never empty (enforced by `SourceFile` newtype at the builder boundary).

---

## TC-03: NonEmptyVec\<T\> (vb_core, New Type)

### Specification

```rust
/// A non-empty vector guaranteed to contain at least one element.
///
/// # Invariants
/// - `head` is always a valid `T`
/// - `len() >= 1`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmptyVec<T> {
    head: T,
    tail: Vec<T>,
}
```

### Smart Constructors

```rust
impl<T> NonEmptyVec<T> {
    /// Creates from a single element.
    pub fn new(head: T) -> Self;

    /// Creates from head + tail.
    pub fn with_tail(head: T, tail: Vec<T>) -> Self;

    /// Creates from a Vec; returns None if empty.
    pub fn from_vec(vec: Vec<T>) -> Option<Self>;
}

impl<T> From<NonEmptyVec<T>> for Vec<T> { ... }

impl<T> IntoIterator for NonEmptyVec<T> { ... }

impl<T> NonEmptyVec<T> {
    pub fn first(&self) -> &T;
    pub fn last(&self) -> &T;
    pub fn len(&self) -> usize; // >= 1
    pub fn is_empty(&self) -> bool; // always false
    pub fn push(&mut self, value: T);
    pub fn extend(&mut self, iter: impl IntoIterator<Item = T>);
}
```

### Contracts

- **TC-03a:** `NonEmptyVec::new(x).len() == 1`
- **TC-03b:** `NonEmptyVec::from_vec(vec![])` returns `None`
- **TC-03c:** `NonEmptyVec::from_vec(vec![x])` returns `Some(nev)` where `nev.first() == &x`
- **TC-03d:** `nev.is_empty()` always returns `false`
- **TC-03e:** `nev.first()` is guarantee-panics-free (no `Option` return)
- **TC-03f:** `nev.push(x)` preserves `len() >= 1`

---

## TC-04: SourceMark Enrichment (vb_compile)

### Specification

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMark {
    pub index: usize,          // unchanged
    pub end_index: usize,      // unchanged
    pub line: usize,           // unchanged (1-indexed)
    pub column: usize,         // unchanged (1-indexed)
    pub available: bool,       // unchanged
    pub source_file: Option<Box<str>>,  // ADDED
}
```

### Contracts

- **TC-04a:** `SourceMark::unavailable()` returns `available: false, source_file: None, index: 0, end_index: 0, line: 0, column: 0`.
- **TC-04b:** `SourceMark::from_parser_span(span)` returns `available: true, source_file: None` (file path set separately by the compiler when known).
- **TC-04c:** `From<vb_yaml::SourceSpan> for SourceMark` converts offsets/lines/cols and sets `available: true, source_file: None`.
- **TC-04d:** `SourceMark` with `available: true` has `line >= 1, column >= 1`.
- **TC-04e:** **Bridge:** `From<SourceMark> for vb_core::Span` extracts byte offsets and optional line/column.
- **TC-04f:** When `source_file` is `Some(s)`, `s` is non-empty.

---

## TC-05: YamlError Enrichment (vb_yaml)

### Specification

Each of the 17 `YamlError` variants gains an optional `span: Option<SourceSpan>` field.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum YamlError {
    UnsupportedTrigger { trigger: &'static str, span: Option<SourceSpan> },
    UnsupportedFeature { feature: &'static str, span: Option<SourceSpan> },
    DuplicateKey { key: Box<str>, span: Option<SourceSpan> },
    AnchorAliasMerge { span: Option<SourceSpan> },
    CustomTag { tag: Box<str>, span: Option<SourceSpan> },
    BinaryScalar { span: Option<SourceSpan> },
    MultipleDocuments { count: usize, span: Option<SourceSpan> },
    AmbiguousScalar { scalar: Box<str>, span: Option<SourceSpan> },
    SourceTooLarge { size: usize, max: usize },
    NestingTooDeep { depth: u16, max: u16 },
    NodeLimitExceeded { count: u32, max: u32 },
    ScalarTooLong { len: usize, max: usize },
    SequenceTooLong { len: usize, max: usize },
    MappingTooLarge { count: usize, max: usize },
    UnknownField { field: Box<str>, span: Option<SourceSpan> },
    EmptySource,
    MissingField { field: &'static str, span: Option<SourceSpan> },
    FieldShape { field: &'static str, expected: &'static str, span: Option<SourceSpan> },
    ParseError { line: usize, reason: Box<str>, span: Option<SourceSpan> },
    ForbiddenFeature { detail: &'static str, span: Option<SourceSpan> },
}
```

### Contracts

- **TC-05a:** `span: None` on any `YamlError` variant means no source location was available at the point of failure.
- **TC-05b:** `span: Some(s)` provides the `SourceSpan` that caused or is nearest to the error.
- **TC-05c:** Parse-level errors (`ParseError`, `AnchorAliasMerge`, `CustomTag`, etc.) are constructed with `span: Some(...)` from the parser event stream.
- **TC-05d:** Limit-exceeded errors (`SourceTooLarge`, `NestingTooDeep`, etc.) that occur at the whole-document level may have `span: None`.
- **TC-05e:** **Backward compat:** Constructing a `YamlError::DuplicateKey { key, span: None }` is still legal. Existing callers are updated to supply `span` where possible.

---

## TC-06: ValidationError Span Propagation (vb_validate)

### Specification

Each `ValidationError` variant gains an optional `span: Span` field where source location is meaningful:

```rust
pub enum ValidationError {
    // Variants that gain a span (where the error anchors to a source location):
    DuplicateKey { span: Span },
    ForbiddenYamlFeature { span: Span },
    UnknownTopLevelField { span: Span },
    UnknownStepField { span: Span },
    MissingRequiredField { field: String, span: Span },
    InvalidVersion { version: String, span: Span },
    InvalidId { id: String, span: Span },
    ReservedId { id: String, span: Span },
    DuplicateId { id: String, span: Span },
    MultipleStepPrimitives { span: Span },
    MissingStepPrimitive { span: Span },
    UnknownReference { reference: String, span: Span },
    FutureReference { reference: String, span: Span },
    SecretNotDeclared { secret: String, span: Span },
    DirectRuntimeReference { span: Span },
    // ... ~30 more variants each gain span: Span
    // Runtime-only errors (LimitRequired, LimitExceeded, PayloadTooLarge) also
    // gain span: Span but will typically carry Span::ZERO at runtime
}
```

### Contracts

- **TC-06a:** `diagnostic_from_error(error)` propagates `error.span` into `Diagnostic.span`.
- **TC-06b:** When `error.span == Span::ZERO`, the diagnostic falls back to `Span::ZERO` — same as current behavior, backward compatible.
- **TC-06c:** When `error.span` has line/column info, the diagnostic preserves it.
- **TC-06d:** **Backward compat:** Tests asserting `diagnostic.span == Span::ZERO` are updated to construct errors with `Span::ZERO` spans — tests now pass both zero and non-zero cases explicitly.
- **TC-06e:** All ~50 variants are covered by `error_diagnostic_parts()` — no new variant is added without a matching arm.

---

## TC-07: Canonical Yaml Bridge (vb_compile)

### Specification

```rust
pub(crate) fn canonical_yaml_error(error: vb_yaml::YamlError) -> CompileError {
    let span = extract_span_from_yaml_error(&error);  // NEW
    CompileError::CanonicalYaml {
        category: yaml_error_category(&error),
        message: error.to_string().into_boxed_str(),
        mark: span,  // NEW — was implicit SourceMark::unavailable()
    }
}
```

### Contracts

- **TC-07a:** When `YamlError` carries `span: Some(source_span)`, `canonical_yaml_error()` produces a `SourceMark` from that span.
- **TC-07b:** When `YamlError` carries `span: None`, `SourceMark::unavailable()` is used (backward compatible).
- **TC-07c:** `extract_span_from_yaml_error()` handles all 17 variants exhaustively.
- **TC-07d:** The `CompileError::CanonicalYaml` variant gains a `mark: SourceMark` field.

---

## TC-08: SourceMap Removal (vb_core)

### Specification

**REMOVE** the dead `SourceMap` placeholder from `vb_core::span`:

```rust
// REMOVED:
// pub struct SourceMap { _private: () }
```

### Contracts

- **TC-08a:** All public re-exports of `SourceMap` from `vb_core::lib.rs` are removed.
- **TC-08b:** All imports of `vb_core::span::SourceMap` in other crates are removed or replaced.
- **TC-08c:** `vb_yaml::SourceMap` is the canonical source map type — no naming collision remains.

---

## TC-09: Diagnostic Conversion Unification (vb_validate)

### Specification

Exactly ONE canonical function converts `ValidationError` → `Diagnostic`. The duplicate implementation in `diag_render.rs` is either:
- Removed entirely, OR
- Made a thin re-export of `diagnostic::diagnostic_from_error()`.

### Contracts

- **TC-09a:** There exists exactly one function containing the match arm for every `ValidationError` variant → `DiagnosticCode` mapping.
- **TC-09b:** `diag_render.rs` either re-exports `diagnostic::diagnostic_from_error()` or is removed.
- **TC-09c:** No divergence between the two files can persist.

---

## TC-10: Span Bridging — SourceSpan → Span (vb_compile)

### Specification

```rust
impl From<vb_yaml::SourceSpan> for vb_core::Span {
    fn from(ss: SourceSpan) -> Self {
        Self {
            start: clamp_u32(ss.start_offset),
            end: clamp_u32(ss.end_offset),
            line: Some(clamp_u32(ss.start_line)),
            column: Some(clamp_u32(ss.start_col)),
        }
    }
}

impl From<vb_yaml::SourceSpan> for SourceMark {
    fn from(ss: SourceSpan) -> Self {
        Self {
            index: ss.start_offset,
            end_index: ss.end_offset,
            line: ss.start_line,
            column: ss.start_col,
            available: true,
            source_file: None,
        }
    }
}

impl From<SourceMark> for vb_core::Span {
    fn from(mark: SourceMark) -> Self {
        if mark.available {
            Self {
                start: clamp_u32(mark.index),
                end: clamp_u32(mark.end_index),
                line: Some(clamp_u32(mark.line)),
                column: Some(clamp_u32(mark.column)),
            }
        } else {
            Self {
                start: clamp_u32(mark.index),
                end: clamp_u32(mark.end_index),
                line: None,
                column: None,
            }
        }
    }
}
```

### Contracts

- **TC-10a:** `SourceSpan → Span` conversion is lossy for offsets > `u32::MAX`; clamped to `u32::MAX`.
- **TC-10b:** `SourceSpan → SourceMark` does not truncate (both fields are `usize`).
- **TC-10c:** `SourceMark → Span` sets `line`/`column` to `Some(...)` only when `available == true`.
- **TC-10d:** `span_for_node()` and `span_for_path()` lookups are O(n) by construction and bounded.

---

## TC-11: SemanticSourceMap Integration (vb_compile)

### Specification

Error messages for validation/compilation errors include the YAML author path (from `SemanticSourceMap`) when available:

```rust
pub(crate) fn render_error_with_path(
    error: &CompileError,
    semantic_map: &SemanticSourceMap,
    source_file: Option<&str>,
) -> Diagnostic {
    // If a path is known for this error's SourceMark, include it in the message
    // Format: "unknown field: `inputs` at path $.inputs"
}
```

### Contracts

- **TC-11a:** When `SemanticSourceMap` contains a matching path for the error's location, the path is appended to the diagnostic message.
- **TC-11b:** When no matching path exists, the message is unchanged.
- **TC-11c:** Path inclusion is additive only; it never replaces the primary error message.
- **TC-11d:** `SemanticSourceMap` is optional at the conversion boundary — code does not panic if it is absent.

---

## TC-12: CompileErrors → NonEmptyVec Migration (vb_compile)

### Specification

Current `CompileErrors` is a wrapper around `Vec<CompileError>`. It is migrated to use `NonEmptyVec<CompileError>`.

```rust
pub struct CompileErrors {
    errors: NonEmptyVec<CompileError>,
}
```

### Contracts

- **TC-12a:** `CompileErrors::collect()` returns `Result<T, CompileErrors>`, not `Option`.
- **TC-12b:** A failed compilation produces at least one `CompileError` — enforced by type.
- **TC-12c:** `CompileErrors::is_empty()` is removed (always false).
- **TC-12d:** `CompileErrors::iter()` iterates over all errors.
- **TC-12e:** `CompileErrors::first()` returns the head error.

---

## TC-13: Backward Compatibility Contract

### Specification

All existing public APIs and test assertions remain valid or are updated minimally.

### Contracts

- **TC-13a:** `Span::new(start, end)` continues to compile and produce a span without line/column.
- **TC-13b:** `Span::ZERO` equals `Span::default()` and equals `Span::new(0, 0)`.
- **TC-13c:** `Diagnostic::new(code, msg, severity, Span::ZERO)` continues to compile.
- **TC-13d:** `diagnostic_from_error(&error)` returns a `Diagnostic` with `span: Span::ZERO` when the error carries no span — unchanged observable behavior.
- **TC-13e:** `SourceMap::new()` in `vb_core` is removed; any callers are migrated to `vb_yaml::SourceMap::new()` or removed.
- **TC-13f:** Public re-exports of `Span`, `Located`, `Spanned`, `Diagnostic`, `DiagnosticCode` from `vb_core` continue to compile. `SourceMap` re-export is removed.
- **TC-13g:** `ValidationError` variants that gain `span: Span` fields use `Span` as the last field (append-only), so pattern matches using `..` continue to compile.
- **TC-13h:** `CompileError::CanonicalYaml` gains a `mark: SourceMark` field (append-only).
