# Architecture Refactor: vb_expr Split

## Status: REFACTORED

## Summary

Split all four vb_expr source files that exceeded 300 lines into directory-based modules with proper submodules.

## Files Created/Modified

### lexer/
| File | Lines | Purpose |
|------|-------|---------|
| `lexer/types.rs` | 96 | Token, LiteralToken, BinaryOp, UnaryOp, TokenSpan, SpannedToken |
| `lexer/mod.rs` | 218 | LogosToken, lexing functions, conversion, binding power |
| `lexer/tests.rs` | 300 | Basic + BDD lexer tests |
| `lexer/tests/adversarial.rs` | 140 | Adversarial lexer tests |

### parser/
| File | Lines | Purpose |
|------|-------|---------|
| `parser/types.rs` | 77 | ExprAst, ExprLiteral, ExprHelper |
| `parser/mod.rs` | 255 | Parser struct, parsing functions, helper utilities |
| `parser/tests.rs` | 272 | Basic + BDD parser tests |
| `parser/tests/adversarial.rs` | 139 | Adversarial parser tests |

### bytecode/
| File | Lines | Purpose |
|------|-------|---------|
| `bytecode/mod.rs` | 294 | ReferenceResolver, compile functions, lower_* functions |
| `bytecode/fold.rs` | 84 | Constant folding utilities (fold_literal, fold_unary, fold_binary, etc.) |
| `bytecode/tests.rs` | 239 | Basic + BDD bytecode tests |
| `bytecode/tests/adversarial.rs` | 148 | Adversarial bytecode tests |

### typecheck/
| File | Lines | Purpose |
|------|-------|---------|
| `typecheck/mod.rs` | 227 | ExprType, TypeContext, type inference functions |
| `typecheck/tests.rs` | 198 | Basic + BDD typecheck tests |
| `typecheck/tests/adversarial.rs` | 104 | Adversarial typecheck tests |

## Original Files Removed
- `lexer.rs` (827 lines) → replaced by `lexer/` directory
- `parser.rs` (782 lines) → replaced by `parser/` directory
- `bytecode.rs` (805 lines) → replaced by `bytecode/` directory
- `typecheck.rs` (568 lines) → replaced by `typecheck/` directory

## Line Count Evidence

```
96 lexer/types.rs
218 lexer/mod.rs
300 lexer/tests.rs
140 lexer/tests/adversarial.rs
77 parser/types.rs
255 parser/mod.rs
272 parser/tests.rs
139 parser/tests/adversarial.rs
294 bytecode/mod.rs
84 bytecode/fold.rs
239 bytecode/tests.rs
148 bytecode/tests/adversarial.rs
227 typecheck/mod.rs
198 typecheck/tests.rs
104 typecheck/tests/adversarial.rs
Σ 2791 lines across 15 files (all ≤ 300 lines)
```

## Module Declaration

The `lib.rs` declarations `pub mod lexer;`, `pub mod parser;`, `pub mod bytecode;`, `pub mod typecheck;` automatically resolve to the directory modules via Rust 2018 module conventions.

## DDD Compliance

- Types (ExprAst, Token, ExprType) are properly separated into NewType modules
- Parser implements Parse, don't validate - illegal states unrepresentable
- No unsafe, unwrap, expect, panic, or unchecked operations
- All error handling uses Result-based types (ExprError)
