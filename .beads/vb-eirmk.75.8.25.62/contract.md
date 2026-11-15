# Contract: return constants from compile_expr_to_bytecode

## Behavior

`vb_expr::compile_expr_to_bytecode` compiles the parsed AST with the same rejecting reference resolver, lowering order, constant indexing, opcode validation, stack-bound validation, and typed error behavior as today. On success it returns `(ExprProgram, Vec<ConstValue>)`; the vector is the exact ordered constant pool referenced by the returned bytecode. `compile_expr_with_pool` and `compile_expr` retain their existing behavior.

## Public boundary

Keep the existing helper name and both existing re-export paths. The return type change is intentional and visible through `vb_expr` and `vb_compile`. Update in-repository callers. Do not add a compatibility wrapper with the old signature under this name.

## Invariants and exclusions

- For `1 + 2`, the returned program references the returned ordered `[I64(1), I64(2)]` pool, and evaluating that exact pair yields `I64(3)`.
- Preserve error precedence and variants for unsupported literals, unresolved references, bytecode limits, constant-pool limits, and stack validation.
- Do not re-lower or recompile the AST to obtain the test pool.
- Do not add constants to `ExprProgram`, add fallback evaluation state, add a type/API/dependency, alter runtime evaluation, or change IR serialization/postcard bytes.
- Keep the separate compiler-owned `vb_compile` lowering helper and its explicit pool handling unchanged.

## Evidence scope

A pinned-nightly public-API reproduction on baseline `4f7e70e57b753cef07632b854029a5a77a7f634a` observed `compile_expr_to_bytecode("1 + 2")` returning `LoadConst(0), LoadConst(1), Add` while evaluation with the caller-available empty pool returned `UnexpectedEof`; `compile_expr_with_pool` returned the two constants and evaluated to `I64(3)`. The regression test must consume only the exact returned program and pool from `compile_expr_to_bytecode`.
