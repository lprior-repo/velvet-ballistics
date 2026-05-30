# Architectural Drift Report: `expression.rs`

**File:** `/home/lewis/src/velvet-ballistics/crates/vb_compile/src/expression.rs`  
**Status:** ⚠️ **DRIFT DETECTED**  
**Line Count:** 881 (violates ≤300 line rule by 581 lines)  
**Workspace:** `arch-drift-hammer` (JJ)

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 881 | 300 | ❌ OVER BY 581 |
| Data types (lines 1–151) | ~151 | — | ✅ |
| Lexer (lines 174–389) | ~216 | 300 | ✅ |
| Parser (lines 391–639) | ~249 | 300 | ✅ |
| Tests (lines 653–881) | 229 | 300 | ✅ |

**Verdict:** File exceeds limit by 193%. Requires split into 3+ modules.

---

## 2. RESPONSIBILITY MAP

```
expression.rs (881 lines)
├── DATA TYPES (lines 1–151)
│   ├── ParsedExpression enum (recursive AST node)
│   ├── ExpressionHelper enum (10 built-in helpers)
│   ├── ExpressionLiteral enum (Null, Bool, I64, F64, Text)
│   ├── UnaryOp enum (Not, Neg)
│   ├── BinaryOp enum (12 operators)
│   ├── TokenKind enum (internal lexer tokens)
│   ├── Token struct (kind + index position)
│   └── Constants (MAX_EXPRESSION_SOURCE_BYTES, MAX_EXPRESSION_TOKENS, etc.)
│
├── LEXER (lines 162–389)
│   ├── lex() — entry point with source length check
│   ├── Lexer struct (source, index, tokens)
│   └── 20+ impl methods (lex_all, lex_one, lex_integer_or_float, etc.)
│
├── PARSER (lines 391–639)
│   ├── parse_expression() — public entry point
│   ├── Parser struct (source, tokens, index)
│   └── 20+ impl methods (parse_complete, parse_precedence, etc.)
│
├── UTILITIES (lines 603–651)
│   ├── infix_binding_power()
│   ├── parse_helper()
│   ├── limit_error()
│   └── 3 character-class predicates
│
└── TESTS (lines 653–881)
    ├── 15 test functions (~228 lines)
    └── Test helpers (ensure, parse, parse_err, etc.)
```

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### 3.1 `usize` for Byte Index

```rust
// Line 152: Raw usize for position
struct Token {
    kind: TokenKind,
    index: usize,  // ❌ Primitive obsession: no SourceIndex newtype
}

// Line 176, 394: Raw usize throughout Lexer/Parser
struct Lexer<'a> {
    source: &'a str,
    index: usize,  // ❌ Should be SourceIndex
}
struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    index: usize,  // ❌ Should be TokenIndex
}
```

**Violation:** `usize` is used for byte offsets throughout. No `SourceIndex` or `ByteOffset` newtype.

**DDD Fix:** Create `SourceIndex(usize)` and `TokenIndex(usize)` newtypes.

### 3.2 Raw `u8` for Binding Power and Depth

```rust
// Line 414–418: Raw u8 for precedence climbing
fn parse_precedence(
    &mut self,
    min_bp: u8,   // ❌ Should be BindingPower
    depth: u8,    // ❌ Should be ParseDepth
) -> Result<ParsedExpression, CompileError>
```

**Violation:** No `BindingPower` or `ParseDepth` types. Values like `11` for unary precedence are magic numbers.

**DDD Fix:** Create `BindingPower(u8)` and `ParseDepth(u8)` newtypes with constants for precedence levels.

### 3.3 `&str` Source Passed Everywhere

```rust
// Line 156, 162, 175, 391: &str source repeated
pub fn parse_expression(source: &str) -> Result<ParsedExpression, CompileError>
fn lex(source: &str) -> Result<Vec<Token>, CompileError>
struct Lexer<'a> { source: &'a str, ... }
struct Parser<'a> { source: &'a str, ... }
```

**Violation:** Source text is passed as raw `&str` with no validation wrapper. Every error message reconstructs `Box::<str>::from(self.source)`.

**DDD Fix:** Create `SourceText<'a>` value object that owns its bounds and can produce proper error contexts.

### 3.4 Raw `usize` for Limits

```rust
// Lines 128–132: Raw usize constants
const MAX_EXPRESSION_SOURCE_BYTES: usize = 4096;
const MAX_EXPRESSION_TOKENS: usize = 256;
const MAX_EXPRESSION_DEPTH: u8 = 64;
const MAX_HELPER_ARGS: usize = 8;
```

**Violation:** Limits are scattered primitives with no `Limit` type to enforce them.

**DDD Fix:** Create `ExpressionLimits` struct bundling all limits with a single config point.

---

## 4. DDD STRUCTURAL ISSUES

### 4.1 Mixed Domain Layers

The file conflates three distinct DDD layers:

| Layer | Lines | Responsibility |
|-------|-------|----------------|
| Domain Types | 1–151 | `ParsedExpression`, operators, literals |
| Infrastructure (Lexer) | 162–389 | Tokenization |
| Application (Parser) | 391–639 | Tree building |
| Tests | 653–881 | Verification |

**Violation:** Scott Wlaschin DDD prescribes explicit layer separation. The lexer/parser are infrastructure, not domain.

### 4.2 No Value Objects for Parse Results

```rust
// Line 150: Token is a Anemic struct, not a value object
#[derive(Debug, Clone, PartialEq)]
struct Token {
    kind: TokenKind,
    index: usize,
}
```

**Violation:** `Token` bundles a `kind` with a raw `index`. Should be `Token { kind: TokenKind, position: SourceIndex }`.

### 4.3 Tests Not Separated

```rust
// Lines 653–881: 228-line tests module inline
#[cfg(test)]
mod tests { ... }
```

**Violation:** Tests are co-located with implementation instead of in `expression/tests.rs` or `expression/parser/tests.rs`.

---

## 5. PROPOSED REFACTOR

### 5.1 File Split (Target: 4 files, all <300 lines)

```
crates/vb_compile/src/expression/
├── mod.rs          (~50 lines)  — re-exports only
├── types.rs       (~160 lines)  — ParsedExpression, Literal, Op enums
├── lexer.rs        (~240 lines)  — Lexer struct + impl
├── parser.rs       (~260 lines)  — Parser struct + impl
└── tests.rs        (~230 lines)  — moved tests
```

### 5.2 Newtype Refactors

```rust
// types.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceIndex(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TokenIndex(usize);

#[derive(Debug, Clone, Copy)]
struct BindingPower(u8);

#[derive(Debug, Clone, Copy)]
struct ParseDepth(u8);

struct ExpressionLimits {
    max_source_bytes: usize,
    max_tokens: usize,
    max_depth: ParseDepth,
    max_helper_args: usize,
}
```

### 5.3 Module Structure After Refactor

```rust
// expression/mod.rs
pub mod types;
pub mod lexer;
pub mod parser;
#[cfg(test)] mod tests;

pub use types::*;
pub use parser::parse_expression;
```

---

## 6. RISK ASSESSMENT

| Risk | Level | Reason |
|------|-------|--------|
| Breaking change | HIGH | Any newtype addition changes public API |
| Test churn | MEDIUM | Tests reference internal details extensively |
| Performance | LOW | Split doesn't affect runtime behavior |

---

## 7. MANDATORY ACTIONS

1. **SPLIT FILE** into `types.rs`, `lexer.rs`, `parser.rs`, `tests.rs`
2. **CREATE newtypes**: `SourceIndex`, `TokenIndex`, `BindingPower`, `ParseDepth`
3. **BUNDLE limits** into `ExpressionLimits` struct
4. **MOVE tests** to `expression/tests.rs`
5. **UPDATE** `lib.rs` import to use `expression::{parse_expression, ...}`

---

## 8. COMPLIANCE CHECKLIST

- [ ] File split completed (all resulting files <300 lines)
- [ ] All `usize` position fields wrapped in newtypes
- [ ] All `u8` precedence/depth fields wrapped in newtypes
- [ ] `ExpressionLimits` struct created
- [ ] Tests moved to separate module
- [ ] `lib.rs` updated with new module path
- [ ] All public API signatures updated
- [ ] Existing tests pass after refactor

---

**REPORT STATUS:** `⚠️ ACTION REQUIRED`  
**NEXT STEP:** Execute proposed file split and newtype refactors  
**ESTIMATED CHURN:** Medium (228 test lines need review after structural change)
