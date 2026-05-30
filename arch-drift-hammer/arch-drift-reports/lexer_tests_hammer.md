# Architectural Drift Report: `lexer/tests.rs`

**File:** `crates/vb_expr/src/lexer/tests.rs`  
**Total Lines:** 680 (VIOLATION: limit is 300)  
**Drift Score:** CRITICAL

---

## Executive Summary

The lexer test file is **380 lines over budget** and suffers from severe **primitive obsession** and **duplication** violations. Every test hand-crafts token trees from raw primitives instead of using domain-aligned test builders. This is a textbook Wlaschin "primitive obsession" anti-pattern: raw `Box`, raw `TokenSpan { start, end }`, raw `crate::lexer::MAX_TOKENS` instead of typed test factories.

---

## 1. Line Count Violation

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 680 | 300 | **+380 OVER** |
| Test functions | ~55 | — | — |
| Avg lines/test | ~12.4 | — | — |

**Verdict:** File must be split into at least **3 modules** (~220 lines each).

---

## 2. Primitive Obsession Violations

### 2.1 `TokenSpan` — Raw `usize` Fields Exposed Everywhere

**Every** spanned token test builds `TokenSpan` by hand:

```rust
// LINE 104 — primitive obsession in 50+ locations
span: TokenSpan { start: 0, end: 4 }
span: TokenSpan { start: 5, end: 6 }
span: TokenSpan { start: 7, end: 9 }
span: TokenSpan { start: 9, end: 9 }
// ... repeated 20+ more times
```

**Root Cause:** `TokenSpan` has no constructor, no `Span::new(start, end)` factory, no `Span::range()` helper. Tests reach into struct fields directly.

**Fix Required:**
```rust
// In types.rs — add:
impl TokenSpan {
    pub const fn new(start: usize, end: usize) -> Self { Self { start, end } }
    pub fn len(&self) -> usize { self.end.saturating_sub(self.start) }
    pub fn is_empty(&self) -> bool { self.start == self.end }
}
```

### 2.2 `SpannedToken` — Raw Struct Construction in Every Test

```rust
// LINE 102-117 — repeated ~10 times
SpannedToken {
    token: Token::Reference(Box::from("$foo")),
    span: TokenSpan { start: 0, end: 4 },
}
```

**Fix Required:** Add `SpannedToken::new(token, start, end)` constructor.

### 2.3 `Box<str>` Wrapping — `Box::from(...)` Scattered Everywhere

String tokens are built as `Box::from("literal")` in **30+ locations**:
- `Token::Reference(Box::from("$foo"))` — lines 48, 103, 248, 587, 595, 603, 614, 624
- `Token::Identifier(Box::from("contains"))` — lines 90, 91, 477, 485, 493, 502-507
- `Token::Literal(LiteralToken::Text(Box::from("hello")))` — lines 38, 454, 463, 465

**Fix Required:** Add typed constructors `Token::text_literal("hello")`, `Token::reference("$foo")`, `Token::identifier("name")`.

### 2.4 Raw Numeric Constants — `MAX_TOKENS` and `MAX_SOURCE_BYTES`

Lines 125-128, 135, 142 use raw internal constants in test assertions:

```rust
let source = "1 ".repeat(crate::lexer::MAX_TOKENS);
let tokens = lex_expr(&source)?;
assert_eq!(tokens.len(), crate::lexer::MAX_TOKENS.saturating_add(1));
```

**Fix Required:** Re-export typed constants: `pub use Self::MAX_TOKENS as MAX_TOKEN_COUNT;` so tests use `lexer::MAX_TOKEN_COUNT`.

### 2.5 Float Construction — `vb_core::FiniteF64::new(x)?` Inlined 7 Times

```rust
// Lines 315, 326, 337, 348, 350, 441 — 7 copies
Token::Literal(LiteralToken::F64(vb_core::FiniteF64::new(3.14)?))
```

**Fix Required:** `LiteralToken::f64(3.14)` constructor that returns `ExprResult<LiteralToken>`.

### 2.6 `Token::Literal(LiteralToken::I64(n))` — Raw Variant Construction

Appears on lines: 16, 110, 170, 183, 196, 209, 224, 226, 248, 363, 365, 376, 378, 410, 418, 427, 560, 562, 573, 586, 636, 638, 662, 670 — **24+ locations**.

**Fix Required:** `LiteralToken::i64(42)` constructor.

### 2.7 Error Matching — Raw `ExprError` Field Access

Lines 149-150, 163-164, 268-273 use raw error matching:

```rust
assert!(matches!(result, Err(ExprError::UnterminatedString)));
assert!(matches!(result, Err(ExprError::UnexpectedChar { ch: '@' })));
```

**Note:** This pattern is acceptable since `ExprError` variants are being exercised. However, no test helper exists to reduce `matches!` boilerplate.

---

## 3. Scott Wlaschin DDD Violations

### 3.1 No Value Objects for Token Construction

| What | Where | Primitive Obsession |
|------|-------|---------------------|
| `Token::Literal(LiteralToken::I64(n))` | 24+ tests | No `literal().i64()` builder |
| `Token::Literal(LiteralToken::Text(Box::from(s)))` | 8+ tests | No `literal().text()` builder |
| `Token::Reference(Box::from("$x"))` | 9+ tests | No `reference("$x")` builder |
| `Token::Identifier(Box::from("x"))` | 7+ tests | No `identifier("x")` builder |
| `SpannedToken { token, span }` | 10+ tests | No `spanned(token, start, end)` builder |
| `TokenSpan { start, end }` | 20+ tests | No `span(start, end)` builder |

**DDD Principle Violated:** "Make illegal states unrepresentable" — tests can construct malformed `TokenSpan` values with `start > end` because there is no factory to enforce invariants.

### 3.2 No Test Module Decomposition

The file is organized by **comment fences** instead of modules:

```
// --- BDD lexer tests ---      (line 166)
// --- F64 literal lexer tests ---  (line 309)
// --- Comma token ---           (line 357)
// --- Paren tokens individually ---  (line 387)
// --- Integer boundaries ---    (line 405)
// ... etc
```

**Wlaschin Principle:** Tests should mirror domain module structure. The lexer has `lexer/mod.rs` (types + logic) and tests should be split into:
- `tests/literal_tokens.rs` — I64, F64, Text, Bool, Null construction
- `tests/reference_tokens.rs` — Reference token construction  
- `tests/operator_tokens.rs` — Binary/Unary operator tokens
- `tests/spanned_tokens.rs` — Span preservation tests
- `tests/error_cases.rs` — Rejection tests
- `tests/adversarial.rs` — Already exists but is 14.3K

### 3.3 Duplication in Test Vectors

Every happy-path test builds `Vec<Token>` by hand:

```rust
let expected = vec![
    Token::Literal(LiteralToken::I64(3)),
    Token::Operator(BinaryOp::Add),
    Token::Literal(LiteralToken::I64(5)),
    Token::End,
];
```

**Pattern:** `token_vec![Token::Literal(LiteralToken::I64(3)), Token::Op(BinaryOp::Add), ...]` macro would reduce this significantly.

### 3.4 No Test Domain Language

The `lexer/tests.rs` has **no test-specific DSL**. Compare:

**Current (680 lines of raw construction):**
```rust
lex_expr("3 + 5")?,
vec![Token::Literal(LiteralToken::I64(3)), Token::Op(BinaryOp::Add), ...]
```

**Wlaschin ideal (typed test API):**
```rust
lexer_test("3 + 5").gives([
    lit().i64(3),
    op().add(),
    lit().i64(5),
    end(),
])
```

---

## 4. Module Dependency Graph

```
lexer/mod.rs (232 lines)
├── lexer/types.rs (105 lines)  ← Token, TokenSpan, SpannedToken, LiteralToken, BinaryOp, UnaryOp
└── lexer/tests.rs (680 lines)  ← VIOLATION: 3x over budget
    └── tests/adversarial.rs (14.3K)
```

**Note:** The adversarial submodule (14.3K) is a separate concern. The 680-line `tests.rs` is itself 2x the 300-line budget before counting `adversarial.rs`.

---

## 5. Refactoring Obligations

### 5.1 Immediate (Required for Compliance)

| Obligation | Target | Benefit |
|-----------|--------|---------|
| Split `tests.rs` into `tests/literal_tokens.rs`, `tests/operator_tokens.rs`, `tests/spanned_tokens.rs`, `tests/error_cases.rs` | 4 modules @ ~170 lines each | File size compliance |
| Add `TokenSpan::new(start, end)` constructor with validation | `types.rs` | No more raw field access |
| Add `SpannedToken::new(token, start, end)` constructor | `types.rs` | Eliminates 10+ inline struct constructions |
| Add `LiteralToken::i64(n)` and `LiteralToken::f64(f)` constructors | `types.rs` | Eliminates 30+ inline constructions |
| Add `Token::reference("$x")`, `Token::identifier("x")`, `Token::text_literal("x")` helpers | `types.rs` or `mod.rs` | Eliminates 20+ `Box::from` calls |

### 5.2 Short-term (DDD Hygiene)

| Obligation | Benefit |
|-----------|---------|
| Create `lexer/tests/lexer_test_helpers.rs` with `token_vec![]` macro | Reduces test vector boilerplate |
| Add `Token::dollar()` and `Token::lparen()`/`rparen()`/`comma()` variants | Consistent with other token constructors |
| Add `TokenSpan::range(start..end)` constructor using `Range<usize>` | More expressive span construction |
| Re-export `MAX_TOKENS` as typed `pub const MAX_TOKEN_COUNT` | Tests stop using internal constants |

---

## 6. Findings Summary

| Category | Severity | Count |
|----------|----------|-------|
| **File size** | CRITICAL | 680 lines (limit 300) |
| **Primitive obsession** — `TokenSpan { start, end }` | CRITICAL | 20+ violations |
| **Primitive obsession** — `Box::from` wrapping | HIGH | 30+ violations |
| **Primitive obsession** — `LiteralToken::I64(n)` | HIGH | 24+ violations |
| **Primitive obsession** — `vb_core::FiniteF64::new(x)?` | HIGH | 7 violations |
| **DDD violation** — No test value objects | HIGH | All 55 tests affected |
| **DDD violation** — Comment-section organization | MEDIUM | 9 comment fences instead of modules |
| **Dead import** — `#[allow(unused_imports)]` on ExprError | LOW | Lines 4-6 |

---

## 7. Impact Assessment

**If not fixed:**
- Test file will continue to bloat as lexer gains features
- Every new token variant requires copy-paste of 5-10 raw construction patterns  
- `TokenSpan` field exposure means invalid spans (`start > end`) are constructible in tests
- No coherent test domain language means adding BDD scenarios requires verbose raw construction

**If fixed:**
- New test coverage for a token variant = ~3 lines via `lit().i64(n)` instead of ~8 lines raw
- Span validation lives in one constructor, not repeated in every test
- File splits enable parallel test development across team members

---

## 8. Recommended Module Split

```
src/lexer/tests/
├── mod.rs              (20 lines — re-exports)
├── literal_tokens.rs   (160 lines — I64, F64, Text, Bool, Null tests)
├── operator_tokens.rs   (140 lines — BinaryOp, UnaryOp tests)
├── reference_tokens.rs (100 lines — Reference, Dollar tests)
├── spanned_tokens.rs   (120 lines — Span preservation tests)
├── identifier_tokens.rs (100 lines — Identifier, keyword classification tests)
├── error_cases.rs      (120 lines — Rejection tests)
└── adversarial.rs      (existing, separate)
```

**Total after split: ~760 lines across 7 modules (avg ~108 lines) — well under 300-line limit.**

---

## Verdict

**ARCHITECTURAL DRIFT CONFIRMED.** File is 2.27x over budget with pervasive primitive obsession. The `types.rs` module lacks the constructors and factories that would enable type-safe, DRY test construction. The comment-fence organization is a smell indicating the file grew beyond its original scope without structural refactoring.

**Required Actions:**
1. Split into `tests/*.rs` module tree
2. Add `TokenSpan`, `SpannedToken`, `LiteralToken`, `Token` constructors to `types.rs`
3. Create `lexer/tests/lexer_test_helpers.rs` with `token_vec![]` macro
4. Re-export `MAX_TOKEN_COUNT` as public constant
