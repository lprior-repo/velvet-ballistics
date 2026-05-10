# vb_expr Test Plan

## VERDICT Summary
- **Status**: REJECTED
- **Clippy errors**: 161 (`assert!` in `Result`-returning functions)
- **Holzmann violations**: 21 loops in test bodies
- **Line coverage**: 77.70% (needs ≥90%)
- **Bytecode coverage**: needs ≥95%

---

## Section 1: Behavior Inventory

### 1.1 Lexer Behaviors
| ID | Subject | Action | Outcome when Condition |
|----|---------|--------|------------------------|
| L-01 | Lexer | tokenizes valid input | returns `Vec<SpannedToken>` with `End` sentinel | valid expressions |
| L-02 | Lexer | rejects empty input | returns `Vec` with single `End` token | `""` input |
| L-03 | Lexer | rejects whitespace-only | returns `Vec` with single `End` token | `"   \t\n"` input |
| L-04 | Lexer | rejects unrecognized char | returns `Err(UnexpectedChar { ch })` | `@`, `#`, `^` etc. |
| L-05 | Lexer | rejects bare `=` | returns `Err(UnexpectedChar { ch: '=' })` | solitary `=` |
| L-06 | Lexer | rejects bare `!` | returns `Err(UnexpectedChar { ch: '!' })` | solitary `!` |
| L-07 | Lexer | accepts i64::MAX literal | returns `Token::Literal(I64(i64::MAX))` | `"9223372036854775807"` |
| L-08 | Lexer | rejects i64 overflow literal | returns `Err(IntegerOutOfRange)` | `"9223372036854775808"` |
| L-09 | Lexer | rejects unterminated string | returns `Err(UnterminatedString)` | `"\""` without closing |
| L-10 | Lexer | accepts max source length | returns tokens within limit | 255 repetitions of `"1 "` |
| L-11 | Lexer | rejects source over max length | returns `Err(ExpressionTooLong)` | `MAX_SOURCE_BYTES + 1` chars |
| L-12 | Lexer | tokenizes deeply nested parens | returns balanced parens + End | `"((((1))))"` |
| L-13 | Lexer | produces Dollar token for lone `$` | returns `Token::Dollar` + `End` | `"$"` |

### 1.2 Parser Behaviors
| ID | Subject | Action | Outcome when Condition |
|----|---------|--------|------------------------|
| P-01 | Parser | parses simple addition | returns `ExprAst::Binary { op: Add, ... }` | `"1 + 2"` |
| P-02 | Parser | parses operator precedence | `Mul` binds tighter than `Add` | `"1 + 2 * 3"` |
| P-03 | Parser | parses left-associative subtraction | left-nested `Sub` nodes | `"1 - 2 - 3"` |
| P-04 | Parser | parses `not` binding tighter than `and/or` | `Not` at deepest level | `"not $a and $b"` |
| P-05 | Parser | parses helper calls | `ExprAst::Helper { name, args }` | `"contains($x, $y)"` |
| P-06 | Parser | rejects unknown helper | returns `Err(UnknownHelper)` | `"unknown_func(1)"` |
| P-07 | Parser | rejects wrong arity | returns `Err(HelperArityMismatch)` | `"contains(1)"` |
| P-08 | Parser | rejects trailing operator | returns `Err(UnexpectedToken)` | `"1 +"` |
| P-09 | Parser | rejects double operator | returns `Err(UnexpectedToken)` | `"1 + * 2"` |
| P-10 | Parser | rejects empty parens | returns `Err(UnexpectedToken)` | `"()"` |
| P-11 | Parser | rejects extra right paren | returns `Err(UnexpectedToken)` | `"1)"` |
| P-12 | Parser | rejects missing right paren | returns `Err(UnexpectedToken)` | `"(1 + 2"` |
| P-13 | Parser | rejects deep nesting beyond MAX_DEPTH | returns `Err(ParseDepthExceeded)` | depth = MAX_DEPTH + 2 |
| P-14 | Parser | parses unary negation | `ExprAst::Unary { op: Neg, expr }` | `"-5"` |
| P-15 | Parser | parses double negation | nested `Neg` nodes | `"--5"` |
| P-16 | Parser | parses chained `not` | four nested `Not` nodes | `"not not not not true"` |
| P-17 | Parser | rejects too many helper args | returns `Err(TooManyHelperArgs)` | 9 args to 8-max helper |
| P-18 | Parser | parses comparison operators | `Lt`, `Lte`, `Gt`, `Gte` | `"1 < 2"`, etc. |
| P-19 | Parser | parses equality operators | `Eq`, `NotEq` | `"null == null"` |
| P-20 | Parser | rejects unknown identifier without parens | returns `Err(UnexpectedToken)` | `"foo"` (not a helper) |

### 1.3 Bytecode Behaviors
| ID | Subject | Action | Outcome when Condition |
|----|---------|--------|------------------------|
| B-01 | Compiler | compiles binary addition | produces `LoadConst, LoadConst, Add` ops | `"1 + 2"` |
| B-02 | Compiler | compiles comparison ops | produces correct ops for all 6 comparisons | each comparison form |
| B-03 | Compiler | compiles all arithmetic ops | produces ops for `+`, `-`, `*`, `/` | each operator |
| B-04 | Compiler | compiles helper calls | produces helper ops | each helper |
| B-05 | Compiler | rejects unresolved reference | returns `Err(InvalidReference)` | `"$missing + 1"` |
| B-06 | Compiler | rejects text literals | returns `Err(UnsupportedLiteral { literal: "text" })` | `"\"hello\" + 1"` |
| B-07 | Constant folder | folds addition | returns `Some(ConstValue::I64(40))` | `"10 * 4"` |
| B-08 | Constant folder | rejects i64::MAX + 1 overflow | returns `None` | `"9223372036854775807 + 1"` |
| B-09 | Constant folder | rejects i64::MAX * 2 overflow | returns `None` | `"9223372036854775807 * 2"` |
| B-10 | Constant folder | rejects division by zero | returns `None` | `"1 / 0"` |
| B-11 | Constant folder | folds valid division | returns `Some(ConstValue::I64(5))` | `"10 / 2"` |
| B-12 | Constant folder | rejects negation of i64::MIN | returns `None` | `"-9223372036854775808"` (unary) |
| B-13 | Stack validator | rejects empty ops | returns `Err(StackUnderflow)` | `vec![]` |
| B-14 | Stack validator | returns ok within limit | returns `Ok(max_stack)` | valid ops within limit |
| B-15 | Constant pool | rejects overflow at MAX_CONSTANTS | returns `Err(ConstantPoolOverflow)` | 65,535 constants then push |

### 1.4 Evaluator Behaviors
| ID | Subject | Action | Outcome when Condition |
|----|---------|--------|------------------------|
| E-01 | Evaluator | adds two i64 values | returns `Ok(SlotValue::I64(42))` | `Add` with `I64(19)`, `I64(23)` |
| E-02 | Evaluator | subtracts two i64 values | returns `Ok(SlotValue::I64(7))` | `Sub` with `I64(10)`, `I64(3)` |
| E-03 | Evaluator | multiplies two i64 values | returns `Ok(SlotValue::I64(42))` | `Mul` with `I64(6)`, `I64(7)` |
| E-04 | Evaluator | divides two i64 values | returns `Ok(SlotValue::I64(7))` | `Div` with `I64(42)`, `I64(6)` |
| E-05 | Evaluator | rejects division by zero | returns `Err(DivisionByZero)` | `Div` with divisor `0` |
| E-06 | Evaluator | compares equality | returns `Ok(SlotValue::Bool(true))` | `Eq` with same values |
| E-07 | Evaluator | compares inequality | returns `Ok(SlotValue::Bool(true))` | `NotEq` with different values |
| E-08 | Evaluator | compares less than | returns `Ok(SlotValue::Bool(true))` | `Lt` with `3`, `5` |
| E-09 | Evaluator | evaluates boolean not | returns `Ok(SlotValue::Bool(false))` | `Not` with `Bool(true)` |
| E-10 | Evaluator | evaluates boolean and | returns `Ok(SlotValue::Bool(false))` | `And` with `true`, `false` |
| E-11 | Evaluator | evaluates boolean or | returns `Ok(SlotValue::Bool(true))` | `Or` with `true`, `false` |
| E-12 | Evaluator | loads slot value | returns `Ok(SlotValue::I64(99))` | `LoadSlot(0)` with slot `[Some(I64(99))]` |
| E-13 | Evaluator | rejects type mismatch in arithmetic | returns `Err(TypeMismatch)` | `Add` with `Bool`, `I64` |
| E-14 | Evaluator | i64::MAX + 1 overflow | returns `Err(IntegerOverflow)` | `Add` with `i64::MAX`, `1` |
| E-15 | Evaluator | i64::MIN - 1 underflow | returns `Err(IntegerOverflow)` | `Sub` with `i64::MIN`, `1` |
| E-16 | Evaluator | i64::MAX * 2 overflow | returns `Err(IntegerOverflow)` | `Mul` with `i64::MAX`, `2` |
| E-17 | Evaluator | negation of i64::MIN overflow | returns `Err(IntegerOverflow)` | `Neg` with `i64::MIN` |
| E-18 | Evaluator | i64::MIN / -1 overflow | returns `Err(IntegerOverflow)`, NOT `DivisionByZero` | `Div` with `i64::MIN`, `-1` |
| E-19 | Evaluator | rejects null in addition | returns `Err(TypeMismatch)` | `Add` with `Null`, `I64(1)` |
| E-20 | Evaluator | rejects bool in multiplication | returns `Err(TypeMismatch)` | `Mul` with `Bool(true)`, `I64(3)` |
| E-21 | Evaluator | rejects number in `and` | returns `Err(TypeMismatch)` | `And` with `I64(1)`, `I64(2)` |
| E-22 | Evaluator | rejects null in `or` | returns `Err(TypeMismatch)` | `Or` with `Null`, `Bool(true)` |
| E-23 | Evaluator | rejects `not` on i64 | returns `Err(TypeMismatch)` | `Not` with `I64(1)` |
| E-24 | Evaluator | rejects negation on bool | returns `Err(TypeMismatch)` | `Neg` with `Bool(true)` |
| E-25 | Evaluator | returns stack underflow error | returns `Err(StackUnderflow)` | single `Add` op with empty stack |
| E-26 | Evaluator | returns stack overflow error | returns `Err(StackOverflow)` | 65+ `LoadConst` ops |
| E-27 | Evaluator | rejects out-of-bounds LoadConst | returns `Err(UnexpectedEof)` | `LoadConst(99)` with < 99 constants |
| E-28 | Evaluator | rejects out-of-bounds LoadSlot | returns `Err(StackUnderflow)` | `LoadSlot(99)` with empty slots |
| E-29 | Helper Exists | returns false for null | `Ok(SlotValue::Bool(false))` | `Exists` with `Null` |
| E-30 | Helper Exists | returns true for non-null | `Ok(SlotValue::Bool(true))` | `Exists` with `I64(1)` |
| E-31 | Helper Empty | returns true for null | `Ok(SlotValue::Bool(true))` | `Empty` with `Null` |
| E-32 | Helper Empty | returns false for non-empty list | `Ok(SlotValue::Bool(false))` | `Empty` with non-empty list |
| E-33 | Helper Empty | returns true for empty symbol | `Ok(SlotValue::Bool(true))` | `Empty` with `""` |
| E-34 | Helper Empty | returns false for non-empty symbol | `Ok(SlotValue::Bool(false))` | `Empty` with `"hello"` |
| E-35 | Helper Empty | returns true for empty object | `Ok(SlotValue::Bool(true))` | `Empty` with empty object |
| E-36 | Helper Empty | rejects i64 | returns `Err(TypeMismatch)` | `Empty` with `I64(42)` |
| E-37 | Helper Length | returns list length | `Ok(SlotValue::I64(3))` | list of 3 elements |
| E-38 | Helper Length | returns symbol length | `Ok(SlotValue::I64(5))` | symbol `"hello"` |
| E-39 | Helper Length | returns object field count | `Ok(SlotValue::I64(2))` | object with 2 fields |
| E-40 | Helper Length | rejects i64 | returns `Err(TypeMismatch)` | `Length` with `I64(42)` |
| E-41 | Helper Unique | deduplicates list | preserves order, removes dupes | `[1, 2, 1]` → `[1, 2]` |
| E-42 | Helper Unique | returns empty for empty input | `Ok(List(empty))` | unique of `[]` |
| E-43 | Helper Unique | rejects non-list | returns `Err(TypeMismatch)` | `Unique` with `I64(42)` |
| E-44 | Helper Sum | sums list elements | `Ok(SlotValue::I64(60))` | `[10, 20, 30]` |
| E-45 | Helper Sum | returns overflow error | `Err(IntegerOverflow)` | `[i64::MAX, 1]` |
| E-46 | Helper Count | returns list length | `Ok(SlotValue::I64(2))` | list of 2 elements |
| E-47 | Helper Contains | returns true for substring | `Ok(SlotValue::Bool(true))` | `"hello world"` contains `"world"` |
| E-48 | Helper Contains | returns false for absent | `Ok(SlotValue::Bool(false))` | `"hello world"` contains `"xyz"` |
| E-49 | Helper StartsWith | returns true for prefix | `Ok(SlotValue::Bool(true))` | `"hello world"` starts with `"hello"` |
| E-50 | Helper EndsWith | returns true for suffix | `Ok(SlotValue::Bool(true))` | `"hello world"` ends with `"world"` |
| E-51 | Helper Has | returns true for present key | `Ok(SlotValue::Bool(true))` | object has key |
| E-52 | Helper Has | returns false for missing key | `Ok(SlotValue::Bool(false))` | object missing key |
| E-53 | Helper Append | adds item to list | returns new list with appended item | append `2` to `[1]` |
| E-54 | Helper AppendIf | adds when condition true | returns new list | append `2` to `[1]` if `true` |
| E-55 | Helper AppendIf | skips when condition false | returns original list | append `2` to `[1]` if `false` |
| E-56 | Helper Merge | combines two objects | returns merged object | merge two objects |
| E-57 | Helper Contains | rejects i64 args | returns `Err(TypeMismatch)` | `Contains` with `I64` args |
| E-58 | Helper Append | rejects i64 args | returns `Err(TypeMismatch)` | `Append` with `I64` args |
| E-59 | Helper Merge | rejects i64 args | returns `Err(TypeMismatch)` | `Merge` with `I64` args |

### 1.5 Typecheck Behaviors
| ID | Subject | Action | Outcome when Condition |
|----|---------|--------|------------------------|
| T-01 | Typechecker | infers i64 literal | returns `ExprType::I64` | `"42"` |
| T-02 | Typechecker | infers bool literal | returns `ExprType::Bool` | `"true"` |
| T-03 | Typechecker | infers null literal | returns `ExprType::Null` | `"null"` |
| T-04 | Typechecker | infers text literal | returns `ExprType::Text` | `"\"hello\""` |
| T-05 | Typechecker | infers arithmetic result | returns `ExprType::I64` | `"1 + 2"` |
| T-06 | Typechecker | infers comparison result | returns `ExprType::Bool` | `"1 < 2"` |
| T-07 | Typechecker | infers logical result | returns `ExprType::Bool` | `"true and false"` |
| T-08 | Typechecker | rejects string in arithmetic | returns `Err(TypeMismatch)` | `"\"hello\" + 1"` |
| T-09 | Typechecker | rejects number in logical | returns `Err(TypeMismatch)` | `"1 and 2"` |
| T-10 | Typechecker | rejects null in arithmetic | returns `Err(TypeMismatch)` | `"null + 1"` |
| T-11 | Typechecker | rejects null in comparison | returns `Err(TypeMismatch)` | `"null < 1"` |
| T-12 | Typechecker | rejects negation on boolean | returns `Err(TypeMismatch)` | `"-true"` |
| T-13 | Typechecker | rejects `not` on i64 | returns `Err(TypeMismatch)` | `"not 1"` |
| T-14 | Typechecker | infers helper return types | returns correct types | `length()`, `empty()`, etc. |
| T-15 | Typechecker | allows unknown in arithmetic | returns `ExprType::I64` | `"$x + 1"` (unresolved) |
| T-16 | Typechecker | allows eq/not-eq on mixed types | returns `ExprType::Bool` | `"null == 1"` |
| T-17 | Typechecker | rejects null in all arithmetic ops | returns `Err(TypeMismatch)` | `"null + 1"`, `"null - 1"`, `"null * 1"`, `"null / 1"` |
| T-18 | Typechecker | rejects null in all comparisons | returns `Err(TypeMismatch)` | `"null < 1"`, `"null <= 1"`, etc. |

---

## Section 2: Trophy Allocation

### Current State vs Target
| Layer | Current | Target | Gap |
|-------|---------|--------|-----|
| Unit (`#[cfg(test)]`) | ~70% | 30% | Reduce |
| Integration (`tests/`) | ~25% | 60% | Increase |
| E2E | ~5% | 5% | Maintain |
| Static (clippy) | Pass | Pass | Maintain |

### Allocation Rationale
- **Unit (30%)**: Pure function tests, constant folding, parser combinators
- **Integration (60%)**: Full lex→parse→compile→eval pipelines, store-aware helpers
- **E2E (5%)**: End-to-end expression evaluation scenarios
- **Static (5%)**: Already passing clippy/miri

---

## Section 3: BDD Scenarios (Given-When-Then)

### 3.1 Lexer BDD Scenarios

#### L-01: Lexer tokenizes valid input
**Given**: a valid expression string `"3 + 5"`
**When**: `lex_expr("3 + 5")` is called
**Then**: returns `Ok(vec![Token::Literal(I64(3)), Token::Operator(Add), Token::Literal(I64(5)), Token::End])`

#### L-02: Lexer rejects empty input
**Given**: an empty string `""`
**When**: `lex_expr("")` is called
**Then**: returns `Ok(vec![Token::End])` with length 1

#### L-08: Lexer rejects i64 overflow literal
**Given**: a literal exceeding i64::MAX `"9223372036854775808"`
**When**: `lex_expr("9223372036854775808")` is called
**Then**: returns `Err(ExprError::IntegerOutOfRange)`

#### L-09: Lexer rejects unterminated string
**Given**: an unterminated string `"\""`
**When**: `lex_expr("\"")` is called
**Then**: returns `Err(ExprError::UnterminatedString)`

### 3.2 Parser BDD Scenarios

#### P-02: Parser respects operator precedence
**Given**: the expression `"1 + 2 * 3"`
**When**: `parse_expr` is called on lexed tokens
**Then**: returns binary tree where `Add` is root, `Mul` is in right child

#### P-13: Parser rejects excessive nesting
**Given**: an expression with depth `MAX_DEPTH + 2`
**When**: `parse_expr` is called
**Then**: returns `Err(ExprError::ParseDepthExceeded { max: N })`

### 3.3 Evaluator BDD Scenarios

#### E-05: Evaluator rejects division by zero
**Given**: a program with `Div` op and divisor slot value `0`
**When**: `eval_expr_program` is called
**Then**: returns `Err(ExprError::DivisionByZero)`

#### E-18: Evaluator correctly handles i64::MIN / -1 overflow (SECURITY)
**Given**: a program computing `i64::MIN / -1`
**When**: `eval_expr_program` is called
**Then**: returns `Err(ExprError::IntegerOverflow)`, NOT `Err(ExprError::DivisionByZero)`

#### E-26: Evaluator returns stack overflow for excess values
**Given**: a program with 65 `LoadConst` ops (exceeding MAX_EXPRESSION_STACK of 64)
**When**: `ExprProgram::try_from_ops` is called
**Then**: returns `Err(ExprError::StackOverflow { max: 64 })`

### 3.4 Typechecker BDD Scenarios

#### T-08: Typechecker rejects string in arithmetic
**Given**: the expression `"\"hello\" + 1"`
**When**: `typecheck_expr` is called
**Then**: returns `Err(ExprError::TypeMismatch { expected: "number", found: "text" })`

#### T-17: Typechecker rejects null in all arithmetic operations
**Given**: the expressions `"null + 1"`, `"null - 1"`, `"null * 1"`, `"null / 1"`
**When**: `typecheck_expr` is called on each
**Then**: each returns `Err(ExprError::TypeMismatch { expected: "number", found: "null" })`

---

## Section 4: Proptest Invariants

### 4.1 Pure Functions Needing Invariants

#### `const_fold_expr(&ExprAst) -> Option<ConstValue>`
**Invariant 1**: If `const_fold_expr` returns `Some(v)`, the value `v` equals the semantic result of evaluating the AST
**Invariant 2**: If `const_fold_expr` returns `None`, the AST cannot be constant-folded (contains references or would overflow)
**Input Strategy**: Arbitrary `ExprAst` with depth ≤ 10, no references
**Invalid Input Class**: AST containing `i64::MIN` with negation (overflow case)

#### `lex_expr(&str) -> Result<Vec<Token>, ExprError>`
**Invariant 1**: Output always ends with `Token::End`
**Invariant 2**: `Token::End` span equals source length
**Input Strategy**: Random valid Rust strings up to 4096 bytes
**Invalid Input Class**: Strings with invalid UTF-8, strings exceeding MAX_SOURCE_BYTES

#### `eval_binary_op(BinaryOp, SlotValue, SlotValue) -> ExprResult<SlotValue>`
**Invariant 1**: For valid i64 operands that don't overflow, returns `Ok(I64(result))`
**Invariant 2**: Division by zero always returns `Err(DivisionByZero)`, never `IntegerOverflow`
**Invariant 3**: i64::MIN / -1 returns `Err(IntegerOverflow)`, not `Err(DivisionByZero)`
**Input Strategy**: Random `SlotValue` pairs from `(i64::MIN..=i64::MAX, i64::MIN..=i64::MAX)` plus boundary values
**Invalid Input Class**: `i64::MIN / -1` for overflow detection

#### `check_expr_stack_bound(&[ExprOp]) -> ExprResult<u8>`
**Invariant 1**: Returns `Ok(n)` where `n` is the minimum stack depth required
**Invariant 2**: Empty ops return `Err(StackUnderflow)`
**Input Strategy**: Random valid `ExprOp` sequences
**Invalid Input Class**: Empty `Vec<ExprOp>`

### 4.2 Combinatorial Coverage for `eval_binary_op`

| Op | Operand A | Operand B | Expected Output |
|----|-----------|-----------|-----------------|
| Add | i64::MAX | 1 | Err(IntegerOverflow) |
| Add | i64::MIN | -1 | Err(IntegerOverflow) |
| Add | 0 | 0 | Ok(I64(0)) |
| Add | 1 | 2 | Ok(I64(3)) |
| Sub | i64::MIN | 1 | Err(IntegerOverflow) |
| Sub | i64::MAX | -1 | Err(IntegerOverflow) |
| Mul | i64::MAX | 2 | Err(IntegerOverflow) |
| Mul | i64::MIN | 2 | Err(IntegerOverflow) |
| Mul | i64::MIN | -1 | Err(IntegerOverflow) |
| Div | 1 | 0 | Err(DivisionByZero) |
| Div | i64::MIN | -1 | Err(IntegerOverflow) |
| Div | 7 | 2 | Ok(I64(3)) |
| Lt | 3 | 5 | Ok(Bool(true)) |
| Lt | 5 | 3 | Ok(Bool(false)) |

---

## Section 5: Fuzz Targets

### 5.1 Required Fuzz Targets

#### `fuzz_lex_expr`
- **Input type**: `&str` (byte slice)
- **Risk class**: HIGH (lexer is first stage, bugs cause cascade)
- **Corpus seeds**: Valid expressions from test suite, edge cases (`"0"`, `"9223372036854775807"`, `""`, strings with unicode)
- **Key invariants to check**:
  - Never panics
  - Returns `Err` with specific `ExprError` variant, never panics on invalid input
  - Output tokens always terminate with `Token::End`

#### `fuzz_parse_expr`
- **Input type**: `&str` (valid lexer output simulation)
- **Risk class**: HIGH (parser operates on trusted lexer output)
- **Corpus seeds**: Valid AST structures, deeply nested expressions
- **Key invariants**:
  - Never panics on valid tokens
  - Returns `Err` for invalid token sequences

#### `fuzz_eval_expr_program`
- **Input type**: `(ExprProgram, Vec<ConstValue>, Vec<Option<SlotValue>>)`
- **Risk class**: CRITICAL (evaluator handles arbitrary bytecode)
- **Corpus seeds**: Programs from test suite with boundary constants
- **Key invariants**:
  - Never panics on any input
  - Stack operations never exceed bounds
  - All `checked_*` arithmetic paths exercised

### 5.2 Fuzz Target Specifications

```rust
// fuzz/targets/vb_expr_fuzz/src/lib.rs
#[deriveFfz)]
struct ExprProgramInput {
    ops: Vec<u8>,        // serialized ExprOp variants
    constants: Vec<i64>, // constant pool
    slots: Vec<Option<i64>>, // slot values
}

// fuzz/targets/vb_expr_fuzz/src/lex_expr.rs
#[deriveFfz)]
struct LexInput {
    text: String,  // raw bytes to lex
}
```

---

## Section 6: Kani Harnesses

### 6.1 Arithmetic Overflow Harnesses

#### `eval_binary_op_addition_i64`
```rust
#[kani::proof]
fn add_never_panics() {
    // For any i64 values a, b, eval_binary_op(Add, a, b) never panics
    // and returns either Ok(I64) or Err(IntegerOverflow)
    let a: i64 = kani::any();
    let b: i64 = kani::any();
    let result = eval_binary_op(BinaryOp::Add, SlotValue::I64(a), SlotValue::I64(b));
    assert!(result.is_ok() || matches!(result, Err(ExprError::IntegerOverflow)));
}
```

#### `eval_binary_op_division_special_cases`
```rust
#[kani::proof]
fn div_i64_min_by_neg_one_is_overflow_not_div_by_zero() {
    // i64::MIN / -1 must be IntegerOverflow, not DivisionByZero
    let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(i64::MIN), SlotValue::I64(-1));
    assert!(matches!(result, Err(ExprError::IntegerOverflow)));
}
```

### 6.2 Stack Bounds Harness

#### `eval_expr_program_stack_never_overflows`
```rust
#[kani::proof]
fn stack_never_exceeds_max() {
    // Given a valid ExprProgram with max_stack = N,
    // eval_expr_program never exceeds N stack depth
    let program: ExprProgram = kani::any();
    // ... instantiate with bounded ops
}
```

---

## Section 7: Mutation Testing Checkpoints

### 7.1 Mutation Operators to Test

| Mutation | Target | Kill Condition |
|----------|--------|----------------|
| Replace `checked_add` with `unchecked_add` | `eval.rs` arithmetic | `i64::MAX + 1` no longer returns error |
| Replace `checked_sub` with `unchecked_sub` | `eval.rs` arithmetic | `i64::MIN - 1` no longer returns error |
| Replace `checked_mul` with `unchecked_mul` | `eval.rs` arithmetic | `i64::MAX * 2` no longer returns error |
| Replace `checked_div` with `unchecked_div` | `eval.rs` division | Division by zero no longer caught |
| Replace `checked_neg` with `unchecked_neg` | `eval.rs` negation | `-i64::MIN` no longer returns error |
| Swap `IntegerOverflow` ↔ `DivisionByZero` | `eval.rs` div handling | `i64::MIN / -1` returns wrong error |
| Replace `pop_pair` error with `unwrap` | `eval.rs` stack ops | Missing operand doesn't panic |
| Replace `get()` with `[]` indexing | `eval.rs` array access | OOB access panics instead of error |

### 7.2 Kill Rate Target

**Target**: ≥90% mutation kill rate
**Current estimate**: ~65% (missing boundary value tests)
**Gap**: Need 25% more boundary mutation coverage

### 7.3 Required Tests for Mutation Coverage

| Test Name | Mutation Killed | File |
|----------|----------------|------|
| `eval_binary_op_addition_i64_max_plus_one_returns_overflow` | unchecked_add | `eval_tests.rs` |
| `eval_binary_op_subtraction_i64_min_minus_one_returns_overflow` | unchecked_sub | `eval_tests.rs` |
| `eval_binary_op_multiplication_i64_max_times_two_returns_overflow` | unchecked_mul | `eval_tests.rs` |
| `eval_binary_op_division_by_zero_returns_division_by_zero` | unchecked_div | `eval_tests.rs` |
| `eval_unary_op_negation_i64_min_returns_overflow` | unchecked_neg | `eval_tests.rs` |
| `eval_binary_op_i64_min_div_neg_one_returns_integer_overflow_not_division_by_zero` | wrong error variant | `eval_tests.rs` |
| `eval_expr_program_with_empty_ops_returns_stack_underflow` | pop_pair unwrap | `eval_tests.rs` |
| `eval_load_const_out_of_bounds_returns_error_not_panic` | get→index | `eval_tests.rs` |
| `eval_load_slot_out_of_bounds_returns_error_not_panic` | get→index | `eval_tests.rs` |

---

## Section 8: Combinatorial Coverage Matrix

### 8.1 Binary Operations × Operand Types

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| happy path | valid i64 + i64 | Ok(I64) | unit |
| overflow: add max+1 | i64::MAX + 1 | Err(IntegerOverflow) | unit |
| overflow: add min-1 | i64::MIN + (-1) | Err(IntegerOverflow) | unit |
| overflow: sub min-1 | i64::MIN - 1 | Err(IntegerOverflow) | unit |
| overflow: sub max-(-1) | i64::MAX - (-1) | Err(IntegerOverflow) | unit |
| overflow: mul max*2 | i64::MAX * 2 | Err(IntegerOverflow) | unit |
| overflow: mul min*2 | i64::MIN * 2 | Err(IntegerOverflow) | unit |
| overflow: mul min*(-1) | i64::MIN * (-1) | Err(IntegerOverflow) | unit |
| overflow: div min/-1 | i64::MIN / -1 | Err(IntegerOverflow) | unit |
| error: div by zero | x / 0 | Err(DivisionByZero) | unit |
| type mismatch: bool+int | Bool(true) + I64(1) | Err(TypeMismatch) | unit |
| type mismatch: int+null | I64(1) + Null | Err(TypeMismatch) | unit |
| type mismatch: int*bool | I64(1) * Bool(true) | Err(TypeMismatch) | unit |
| type mismatch: int and int | I64(1) and I64(0) | Err(TypeMismatch) | unit |
| type mismatch: null or bool | Null or Bool(true) | Err(TypeMismatch) | unit |

### 8.2 Unary Operations × Operand Types

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| neg: happy | I64(42) | Ok(I64(-42)) | unit |
| neg: zero | I64(0) | Ok(I64(0)) | unit |
| neg: min | I64(i64::MIN) | Err(IntegerOverflow) | unit |
| neg: bool | Bool(true) | Err(TypeMismatch) | unit |
| not: happy | Bool(true) | Ok(Bool(false)) | unit |
| not: zero | Bool(false) | Ok(Bool(true)) | unit |
| not: int | I64(1) | Err(TypeMismatch) | unit |
| not: null | Null | Err(TypeMismatch) | unit |

### 8.3 Helper Functions × Value Types

| Scenario | Helper | Value Type | Expected Output | Layer |
|----------|--------|------------|-----------------|-------|
| Exists | Exists | Null | Ok(Bool(false)) | unit |
| Exists | Exists | I64(1) | Ok(Bool(true)) | unit |
| Empty | Empty | Null | Ok(Bool(true)) | unit |
| Empty | Empty | list (store) | Ok(Bool) | integration |
| Empty | Empty | symbol (store) | Ok(Bool) | integration |
| Empty | Empty | object (store) | Ok(Bool) | integration |
| Empty | Empty | I64 | Err(TypeMismatch) | unit |
| Length | Length | list (store) | Ok(I64) | integration |
| Length | Length | symbol (store) | Ok(I64) | integration |
| Length | Length | object (store) | Ok(I64) | integration |
| Length | Length | I64 | Err(TypeMismatch) | unit |
| Unique | Unique | list (store) | Ok(List) | integration |
| Unique | Unique | I64 | Err(TypeMismatch) | unit |
| Sum | Sum | list (store) | Ok(I64) | integration |
| Sum | Sum | list with overflow (store) | Err(IntegerOverflow) | integration |
| Contains | Contains | symbols (store) | Ok(Bool) | integration |
| StartsWith | StartsWith | symbols (store) | Ok(Bool) | integration |
| EndsWith | EndsWith | symbols (store) | Ok(Bool) | integration |
| Has | Has | object (store) | Ok(Bool) | integration |
| Append | Append | list + value (store) | Ok(List) | integration |
| AppendIf | AppendIf | list + value + true (store) | Ok(List with item) | integration |
| AppendIf | AppendIf | list + value + false (store) | Ok(List unchanged) | integration |
| Merge | Merge | object + object (store) | Ok(Object) | integration |

---

## Section 9: Exact Fixes for Holzmann Violations

### 9.1 Issue: `assert!` in Result-returning Functions (161 errors)

**Rule**: `#[test] fn foo() -> ExprResult<()>` MUST NOT use `assert!`. Use `assert_eq!` on `?` results or `matches!` on error results.

**Pattern to ELIMINATE**:
```rust
#[test]
fn foo() -> ExprResult<()> {
    let result = some_fn()?;
    assert!(result.is_ok());  // WRONG
    Ok(())
}
```

**Correct Pattern**:
```rust
#[test]
fn foo() -> ExprResult<()> {
    let result = some_fn()?;
    assert_eq!(result, expected_value);  // CORRECT
    Ok(())
}

#[test]
fn foo_error() -> ExprResult<()> {
    let result = some_fn();
    assert!(matches!(result, Err(ExpectedError { .. })));  // CORRECT for errors
    Ok(())
}
```

**Files with violations** (identified by grep for `-> ExprResult<()>` + `assert!`):
1. `src/lexer/tests/adversarial.rs` - Lines with `for ch in` loops (see 9.2)
2. `src/parser/tests/adversarial.rs` - Lines with `for op in` loops (see 9.2)
3. `src/typecheck/tests/adversarial.rs` - Lines with `for op in` loops (see 9.2)
4. `src/bytecode/tests/adversarial.rs` - Lines with `for i in` loop (see 9.2)
5. `src/eval_tests.rs` - Lines with `for` loops (see 9.2)
6. `src/eval/tests/integration.rs` - Lines with `for` loops (see 9.2)
7. `src/eval/tests/inline_tests.rs` - Lines with `for` loops (see 9.2)

### 9.2 Issue: Loops in Test Bodies (21 violations)

**Rule**: `for` loops in test function bodies MUST be unrolled into individual `#[test]` functions.

**Violations Found**:

#### File: `src/lexer/tests/adversarial.rs`

**Violation 1** - `blackhat_lx_003_unexpected_chars_rejected` (lines 178-186):
```rust
#[test]
fn blackhat_lx_003_unexpected_chars_rejected() -> crate::ExprResult<()> {
    for ch in ['@', '#', '^', '~', '`', '\u{00F7}'] {  // VIOLATION
        let r = lex_expr(&ch.to_string());
        assert!(matches!(r, Err(ExprError::UnexpectedChar { .. })));
    }
    Ok(())
}
```
**Fix**: Unroll into 6 individual tests:
```rust
#[test]
fn blackhat_lx_003_unexpected_chars_rejected_at_sign() -> crate::ExprResult<()> {
    let r = lex_expr("@");
    assert!(matches!(r, Err(ExprError::UnexpectedChar { ch: '@' })));
    Ok(())
}
#[test]
fn blackhat_lx_003_unexpected_chars_rejected_hash() -> crate::ExprResult<()> {
    let r = lex_expr("#");
    assert!(matches!(r, Err(ExprError::UnexpectedChar { ch: '#' })));
    Ok(())
}
// ... 4 more tests for '^', '~', '`', '\u{00F7}'
```

#### File: `src/parser/tests/adversarial.rs`

**Violation 2** - `blackhat_tc_001_null_rejected_in_all_arithmetic` (lines 115-125):
```rust
#[test]
fn blackhat_tc_001_null_rejected_in_all_arithmetic() -> crate::ExprResult<()> {
    for op in &["+", "-", "*", "/"] {  // VIOLATION
        let source = format!("null {op} 1");
        let result = check(&source);
        assert!(matches!(result, Err(ExprError::TypeMismatch { .. })));
    }
    Ok(())
}
```
**Fix**: Unroll into 4 tests: `blackhat_tc_001a` through `blackhat_tc_001d`

**Violation 3** - `blackhat_tc_002_null_rejected_in_all_comparisons` (lines 129-139):
```rust
#[test]
fn blackhat_tc_002_null_rejected_in_all_comparisons() -> crate::ExprResult<()> {
    for op in &["<", "<=", ">", ">="] {  // VIOLATION
        let source = format!("null {op} 1");
        let result = check(&source);
        assert!(matches!(result, Err(ExprError::TypeMismatch { .. })));
    }
    Ok(())
}
```
**Fix**: Unroll into 4 tests: `blackhat_tc_002a` through `blackhat_tc_002d`

#### File: `src/bytecode/tests/adversarial.rs`

**Violation 4** - `push_constant_returns_overflow_on_max_constants` (lines 125-137):
```rust
#[test]
fn push_constant_returns_overflow_on_max_constants() -> crate::ExprResult<()> {
    let mut constants: Vec<ConstValue> = Vec::new();
    for i in 0u16..65_535 {  // VIOLATION - loop to build test state
        constants.push(ConstValue::I64(i64::from(i)));
    }
    // ... test logic
    Ok(())
}
```
**Fix**: Replace with pre-built constant vector or use `vec!` macro with exact values

**Violation 5** - `blackhat_bc_006_constant_pool_overflow` (lines 219-230):
```rust
#[test]
fn blackhat_bc_006_constant_pool_overflow() -> crate::ExprResult<()> {
    let mut constants: Vec<ConstValue> = Vec::new();
    for i in 0u16..65_535 {  // VIOLATION
        constants.push(ConstValue::I64(i64::from(i)));
    }
    // ... test logic
    Ok(())
}
```
**Fix**: Same as above

#### File: `src/eval_tests.rs`

**Violation 6** - `evaluates_comparison_ops` (lines 116-133):
```rust
#[test]
fn evaluates_comparison_ops() -> ExprResult<()> {
    let constants = vec![ConstValue::I64(3), ConstValue::I64(5)];
    for (op, expected) in [  // VIOLATION
        (ExprOp::Lt, true),
        (ExprOp::Lte, true),
        (ExprOp::Gt, false),
        (ExprOp::Gte, false),
    ] {
        let program = make_program(vec![...op...])?;
        let result = eval_with_const(&program, constants.clone())?;
        assert_eq!(result, SlotValue::Bool(expected));
    }
    Ok(())
}
```
**Fix**: Unroll into 4 tests: `evaluates_less_than`, `evaluates_less_than_or_equal`, `evaluates_greater_than`, `evaluates_greater_than_or_equal`

**Violation 7** - `eval_expr_program_returns_stack_overflow_for_deep_nesting` (lines 327-340):
```rust
#[test]
fn eval_expr_program_returns_stack_overflow_for_deep_nesting() -> ExprResult<()> {
    let mut ops = Vec::new();
    for i in 0..65u16 {  // VIOLATION
        ops.push(ExprOp::LoadConst(ConstIdx::new(i)));
    }
    // ... test logic
    Ok(())
}
```
**Fix**: Replace loop with exact `vec!` of 65 elements or use `std::iter::repeat_with`

#### File: `src/eval/tests/integration.rs`

**Violation 8** - `evaluates_comparison_ops` (lines 113-130): **Same pattern as Violation 6**

**Violation 9** - `eval_expr_program_returns_stack_overflow_for_deep_nesting` (lines 324-337): **Same pattern as Violation 7**

#### File: `src/eval/tests/inline_tests.rs`

**Violation 10** - `evaluates_comparison_ops` (lines 111-128): **Same pattern as Violation 6**

**Violation 11** - `eval_expr_program_returns_stack_overflow_for_deep_nesting` (lines 354-371): **Same pattern as Violation 7**

### 9.3 Summary of Required Test Splits

| File | Original Test | Splits Into | Count |
|------|---------------|-------------|-------|
| `lexer/tests/adversarial.rs` | `blackhat_lx_003_unexpected_chars_rejected` | 6 individual tests | +5 |
| `typecheck/tests/adversarial.rs` | `blackhat_tc_001_null_rejected_in_all_arithmetic` | 4 individual tests | +3 |
| `typecheck/tests/adversarial.rs` | `blackhat_tc_002_null_rejected_in_all_comparisons` | 4 individual tests | +3 |
| `bytecode/tests/adversarial.rs` | `push_constant_returns_overflow_on_max_constants` | Remove loop, use vec! | 0 |
| `bytecode/tests/adversarial.rs` | `blackhat_bc_006_constant_pool_overflow` | Remove loop, use vec! | 0 |
| `eval_tests.rs` | `evaluates_comparison_ops` | 4 individual tests | +3 |
| `eval_tests.rs` | `eval_expr_program_returns_stack_overflow_for_deep_nesting` | Remove loop | 0 |
| `eval/tests/integration.rs` | `evaluates_comparison_ops` | 4 individual tests | +3 |
| `eval/tests/integration.rs` | `eval_expr_program_returns_stack_overflow_for_deep_nesting` | Remove loop | 0 |
| `eval/tests/inline_tests.rs` | `evaluates_comparison_ops` | 4 individual tests | +3 |
| `eval/tests/inline_tests.rs` | `eval_expr_program_returns_stack_overflow_for_deep_nesting` | Remove loop | 0 |

**Total new test functions required**: ~21 additional tests

---

## Section 10: Coverage Gap Analysis

### 10.1 Current Uncovered Lines (from tarpaulin report)

#### `eval.rs` - 130 lines uncovered
Key gaps:
- Helper functions with store: `eval_helper_*_with_store` variants (lines 226-294)
- `eval_helper_length` without store returns error (lines 426-438)
- `eval_helper_empty` without store returns error (lines 440-453)
- `eval_helper_unique` without store returns error (lines 455-467)
- `eval_helper_contains` without store returns error (lines 469-476)
- `eval_helper_starts_with` without store returns error (lines 478-486)
- `eval_helper_ends_with` without store returns error (lines 488-496)
- `eval_helper_has` without store returns error (lines 498-506)
- `eval_helper_append` without store returns error (lines 508-...)
- `eval_helper_append_if` without store returns error
- `eval_helper_merge` without store returns error
- `eval_helper_sum` without store returns error

#### `bytecode/fold.rs` - 20 lines uncovered
Key gaps:
- Error handling branches in constant folding
- Overflow detection branches

#### `parser/mod.rs` - 20 lines uncovered
Key gaps:
- Error handling in `parse_expr` function

### 10.2 Required Tests for 90% Line Coverage

To reach 90% line coverage, the following uncovered code paths MUST be tested:

1. **`eval_helper_length`** without store (lines 426-438) - test returns TypeMismatch error
2. **`eval_helper_empty`** without store (lines 440-453) - test returns TypeMismatch error
3. **`eval_helper_unique`** without store (lines 455-467) - test returns TypeMismatch error
4. **`eval_helper_contains`** without store (lines 469-476) - test returns TypeMismatch error
5. **`eval_helper_starts_with`** without store (lines 478-486) - test returns TypeMismatch error
6. **`eval_helper_ends_with`** without store (lines 488-496) - test returns TypeMismatch error
7. **`eval_helper_has`** without store (lines 498-506) - test returns TypeMismatch error
8. **`eval_helper_append`** without store - test returns TypeMismatch error
9. **`eval_helper_append_if`** without store - test returns TypeMismatch error
10. **`eval_helper_merge`** without store - test returns TypeMismatch error
11. **`eval_helper_sum`** without store - test returns TypeMismatch error

**Test naming convention**: `eval_helper_{name}_without_store_returns_type_mismatch`

---

## Section 11: Implementation Checklist

### Phase 1: Fix Holzmann Violations (Day 1)
- [ ] Unroll all `for` loops in test files into individual `#[test]` functions
- [ ] Verify `cargo clippy -p vb_expr` passes with 0 errors
- [ ] Verify `cargo test -p vb_expr` still passes

### Phase 2: Raise Line Coverage to ≥90% (Day 1-2)
- [ ] Add tests for all `eval_helper_*` without store error paths
- [ ] Add tests for uncovered error branches in `bytecode/fold.rs`
- [ ] Add tests for uncovered error branches in `parser/mod.rs`
- [ ] Run `cargo tarpaulin -p vb_expr` and verify ≥90%

### Phase 3: Raise Bytecode Coverage to ≥95% (Day 2)
- [ ] Add tests for all bytecode ops and edge cases
- [ ] Add integration tests exercising full pipeline
- [ ] Run `cargo tarpaulin --coverage-type=bbytecode -p vb_expr` (or equivalent)

### Phase 4: Mutation Testing (Day 2-3)
- [ ] Run `cargo muter` or equivalent mutation testing tool
- [ ] Achieve ≥90% kill rate
- [ ] Add targeted tests for any survivors

---

## Appendix A: Error Enum Completeness Check

All `ExprError` variants MUST have test coverage:

| Variant | Test Coverage | Location |
|---------|---------------|----------|
| `UnexpectedToken` | ✅ | parser tests |
| `UnexpectedEof` | ✅ | bytecode eval tests |
| `UnknownOperator` | ❌ MISSING | needs test |
| `UnknownHelper` | ✅ | parser tests |
| `StackOverflow` | ✅ | eval tests |
| `StackUnderflow` | ✅ | eval tests |
| `TypeMismatch` | ✅ | typecheck + eval tests |
| `DivisionByZero` | ✅ | eval tests |
| `IntegerOverflow` | ✅ | eval tests |
| `InvalidReference` | ✅ | bytecode tests |
| `ExpressionTooLong` | ✅ | lexer tests |
| `UnterminatedString` | ✅ | lexer tests |
| `IntegerOutOfRange` | ✅ | lexer tests |
| `UnexpectedChar` | ✅ | lexer tests |
| `ParseDepthExceeded` | ✅ | parser tests |
| `TooManyHelperArgs` | ✅ | parser tests |
| `BytecodeTooLong` | ❌ MISSING | needs test |
| `ConstantPoolOverflow` | ✅ | bytecode tests |
| `UnsupportedLiteral` | ✅ | bytecode tests |

**Missing**: `UnknownOperator` test, `BytecodeTooLong` test

---

## Appendix B: Test Naming Convention

All tests MUST follow `subject_action_returns_outcome_when_condition` pattern:

**Good**:
- `eval_binary_op_adds_two_positive_numbers`
- `eval_binary_op_returns_overflow_when_adding_max_i64_and_one`
- `typecheck_rejects_null_in_arithmetic_operations`

**Bad**:
- `test_add`
- `test_overflow`
- `test_null`

---

*Generated by test-planner agent for vb_expr crate*
*Target: ≥90% line coverage, ≥95% bytecode coverage, 0 clippy errors, ≥90% mutation kill rate*
