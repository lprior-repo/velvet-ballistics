---
section: 27
title: "Mandatory Function Surface: `vb_expr`"
parent: velvet-ballistics-MASTER.md
---

## 27. Mandatory Function Surface: `vb_expr`


**Source of truth:** `crates/vb_expr/src/`.

Required coverage areas:

| Area | Required public surface |
|------|------------------------|
| Lexer | `lex_expr` (bounded token stream, 256 max tokens). |
| Parser | `parse_expr` (Pratt parser, AST output, 64 max depth). |
| Typechecker | `typecheck_expr` (type propagation, mismatch detection). |
| Bytecode compiler | `compile_expr_to_bytecode`, `compile_expr_with_pool`, `compile_expr_with_resolver`, `const_fold_expr`, `check_expr_stack_bound`. |
| Evaluator | `eval_expr_program`, `eval_binary_op`, `eval_unary_op`, `eval_helper`. |

---
