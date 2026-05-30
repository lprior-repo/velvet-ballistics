# Architectural Drift Report: `vb_expr/src/lib.rs`

## File Summary
| Metric | Value |
|--------|-------|
| **Total Lines** | 152 |
| **Line Limit** | 300 |
| **Status** | ✅ UNDER LIMIT |

## DDD Cohesion Analysis

### Module Structure
```
vb_expr/lib.rs (152 lines)
├── bytecode module (external)
├── eval module (external)
├── lexer module (external)
├── parser module (external)
├── typecheck module (external)
├── property_tests (test, conditional)
├── proofs (kani, conditional)
├── kani_expr_stack (kani, conditional)
├── ExprError (135→47 lines - see violations)
├── From<vb_core::CoreError> impl (15 lines)
└── ExprResult type alias (1 line)
```

### Cohesion Assessment
**Cohesion**: LOW - `lib.rs` acts as a re-export facade and error definition hub. The actual domain logic lives in submodules. This is a facade pattern, which is acceptable for crate roots.

**Domain Boundary**: The file correctly exposes the public API boundary of the expression engine (lexer → parser → typecheck → bytecode → eval pipeline).

---

## Violations

### 🔴 HIGH: Primitive Obsession in `ExprError`

**Location**: Lines 47–134 (89 lines, ~59% of file)

**Problem**: The `ExprError` enum uses raw `String` types for all descriptive fields:

| Variant | Primitive Fields | Should Be |
|---------|------------------|-----------|
| `UnexpectedToken { token: String }` | `String` | `Token(String)` or newtype |
| `UnknownOperator { op: String }` | `String` | `Operator(String)` |
| `UnknownHelper { helper: String }` | `String` | `HelperName(String)` |
| `TypeMismatch { expected: String, found: String }` | `String` (×2) | `TypeName(String)` newtypes |
| `InvalidReference { reference: String }` | `String` | `Reference(String)` |
| `ExpressionTooLong { len: usize, max: usize }` | `usize` (×2) | Acceptable (bounded values) |
| `HelperArityMismatch { helper, expected, actual }` | `String`, `usize` (×2) | `HelperName(String)` |
| `BytecodeTooLong { len: usize, max: usize }` | `usize` (×2) | Acceptable |
| `UnsupportedLiteral { literal: String }` | `String` | `Literal(String)` |

**Scott Wlaschin Violation**: "Make illegal states unrepresentable." Using raw `String` allows any arbitrary string to be stored as a token, operator, or helper name.

### 🟡 MEDIUM: God Error Enum

**Location**: Lines 47–134

**Problem**: 20 error variants is excessive. Many represent the same conceptual failure mode (validation failure) and could be consolidated using refinement types or parameterized variants.

**Recommendation**: Consider a 3–5 variant enum with structured data:
```rust
pub enum ExprError {
    UnexpectedToken { token: Token },
    UnknownIdentifier { name: Identifier },
    StackFault { kind: StackFaultKind }, // overflow/underflow
    TypeMismatch { expected: TypeName, found: TypeName },
    ArithmeticFault { kind: ArithmeticFaultKind }, // div-by-zero, overflow, NaN
    // ... validation variants
}
```

### 🟢 LOW: `From<vb_core::CoreError>` Mapping Loses Information

**Location**: Lines 136–150

**Problem**: The catch-all `_ => ExprError::UnexpectedEof` discards 7+ `CoreError` variants, mapping them to a misleading error.

**Recommendation**: Exhaustively match all `CoreError` variants or use a `#[non_exhaustive]` approach.

---

## DDD Smell Summary

| Smell | Severity | Lines Affected |
|-------|----------|----------------|
| Primitive Obsession | HIGH | 47–134 |
| God Error Enum | MEDIUM | 47–134 |
| Information Loss in Conversion | LOW | 136–150 |
| Facade-Only lib.rs | INFO | 1–45 |

---

## Priority

**MEDIUM**

**Rationale**:
- ✅ File is well under 300 lines (152)
- ✅ Facade pattern for crate root is architecturally sound
- 🔴 `ExprError` primitive obsession is a real DDD violation
- 🟡 God error enum reduces domain clarity
- The errors represent domain invariants that should be enforced at the type level

---

## Recommendations

1. **Create newtypes** in `types.rs` or inline:
   - `pub struct Token(pub String);`
   - `pub struct Operator(pub String);`
   - `pub struct HelperName(pub String);`
   - `pub struct TypeName(pub String);`
   - `pub struct Reference(pub String);`

2. **Consolidate error variants** into 5–8 conceptual categories with structured data.

3. **Exhaustively map `CoreError`** variants instead of using catch-all.

---

*Report generated: 2026-05-29*
*Analyzer: architectural-drift skill*
