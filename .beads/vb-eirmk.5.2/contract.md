# vb-eirmk.5.2 — Numeric Operator Parity (Frontend vs Live Core)

## Scope
Integration test proving vb_expr frontend (lexer/parser/compiler/evaluator) produces
identical numeric results to direct core engine execution, via the live public path.

## Public Path
`vb_expr::compile_expr(source, resolver)` → `(ExprProgram, Vec<ConstValue>)`
`vb_expr::eval_expr_program(&program, &[], &constants)` → `SlotValue`

Same program/constants driven through core via `CompiledWorkflow` with
`EvalExpr` + `Finish` node sequence.

## Test Cases

### Arithmetic (4 ops × 4 type pairings = 16)
Values: 6/6.0 + 2/2.0 → 8/8.0 (I64 if both integers, else F64)
- add, sub, mul, div with I64/I64, F64/F64, I64/F64, F64/I64 pairings

### Ordered Comparisons — All True (4 ops × 4 type pairings = 16)
- gt: 6 > 2 → true (all pairings)
- gte: 4 >= 4 → true (all pairings)
- lt: 2 < 6 → true (all pairings)
- lte: 4 <= 4 → true (all pairings)

### Ordered Comparisons Including Equal (3 relations × 4 ops × 4 type pairings = 48)
- Greater: 6/6.0 vs 2/2.0 → Gt true, Gte true, Lt false, Lte false
- Less: 2/2.0 vs 6/6.0 → Gt false, Gte false, Lt true, Lte true
- Equal: 2/2.0 vs 2/2.0 → Gt false, Gte true, Lt false, Lte true

### Error Cases — Frontend and Core Separately (24)
- Integer overflow (4): MAX+1, MIN-1, MAX*2, MIN/-1
  - Frontend: `ExprError::IntegerOverflow`
  - Core: `EngineError::InvalidCompiledWorkflow { reason }`
- Division by zero (1): 1/0
  - Frontend: `ExprError::DivisionByZero`
  - Core: `EngineError::DivisionByZero`
- Non-finite float (3): 1.0/0, 1/0.0, 1.0/0.0
  - Frontend: `ExprError::NonFiniteFloat`
  - Core: `EngineError::NonFiniteNumber`
- Type mismatch (16): 8 ops × 2 operand orderings (true OP 2, 2 OP true)
  - Frontend: `ExprError::TypeMismatch { expected: "number", found: "boolean" }`
  - Core: `EngineError::TypeMismatch { expected: "number", found: "boolean" }`

## Not Tested
- Eq/NotEq comparison operators
- Inverse operations
- Unary negation

## Invariants
- Each positive case: actual compile + eval frontend + drive core vs independent expected
- Compiler produces real binary ops in bytecode (not just LoadConst)
- No unwrap/expect/panic in test code
- Typed error cases compared separately for frontend and core APIs