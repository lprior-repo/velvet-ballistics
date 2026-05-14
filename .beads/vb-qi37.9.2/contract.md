# Contract Specification — vb-qi37.9.2

## Context
- **Feature**: F64 bytecode execution semantics in vb_expr
- **Bead**: vb-qi37.9.2
- **Title**: expr: Execute F64 bytecode semantics
- **Phase**: State 3 — Contract

## Domain Terms
- `SlotValue::F64(FiniteF64)` — runtime F64 value, always finite
- `ConstValue::F64(FiniteF64)` — compile-time F64 constant
- `FiniteF64` — newtype wrapper; constructor rejects NaN and infinities
- `ExprError::NonFiniteFloat` — returned when F64 arithmetic yields NaN or ±Inf
- `ExprError::DivisionByZero` — returned for I64 division by zero (F64/0 yields ±Inf, not error)
- `eval_binary_op` — public binary op dispatcher
- `eval_unary_op` — public unary op dispatcher
- Bytecode pipeline: lex → parse → typecheck → compile → eval

## Assumptions
- F64 arithmetic follows IEEE 754 double-precision rules natively
- `f64::add`, `f64::sub`, `f64::mul`, `f64::div` produce deterministic results for finite inputs
- F64 division by zero produces ±Inf per IEEE 754 (no exception raised in Rust)
- NaN comparisons yield `false` per IEEE 754 (e.g., `NaN > x` is always false)
- The `FiniteF64` wrapper is the sole gatekeeper: if `new(result)` succeeds, the value is finite
- I64 arithmetic uses `checked_add`, `checked_sub`, `checked_mul`, `checked_div` — overflow returns `None`
- Stack is bounded to 64 entries (`MAX_EXPRESSION_STACK_USIZE`)
- Bytecode program is bounded to 256 ops (`MAX_EXPRESSION_OPS`)

## Open Questions
- **Q1**: Should F64 constant folding be implemented in `fold.rs`? Currently returns `None`. Classification: **missing functionality**, not in bead scope per delivery-scope.
- **Q2**: Are there any F64 helper ops (sum, avg, min, max on F64 lists) needed? Currently not in bead scope.
- **Q3**: Should the comparison semantics for NaN be explicitly documented? Currently: raw f64 comparison, NaN comparisons yield false.

---

## Preconditions
- **PRE-001**: Input values to F64 arithmetic must be constructed via `FiniteF64::new` (enforced at type level for SlotValue::F64)
- **PRE-002**: Stack depth must not exceed `MAX_EXPRESSION_STACK` (64) before any eval op
- **PRE-003**: Program op count must not exceed `MAX_EXPRESSION_OPS` (256)
- **PRE-004**: For I64 division, divisor must not be zero

## Postconditions
- **POST-001**: `eval_add_op` on two `SlotValue::F64` returns `Ok(SlotValue::F64(finite))` where `finite` is the IEEE 754 sum of the inputs, or `Err(ExprError::NonFiniteFloat)` if the result is NaN or infinite
- **POST-002**: `eval_sub_op` on two `SlotValue::F64` returns `Ok(SlotValue::F64(finite))` where `finite` is the IEEE 754 difference, or `Err(ExprError::NonFiniteFloat)` if the result is NaN or infinite
- **POST-003**: `eval_mul_op` on two `SlotValue::F64` returns `Ok(SlotValue::F64(finite))` where `finite` is the IEEE 754 product, or `Err(ExprError::NonFiniteFloat)` if the result is NaN or infinite
- **POST-004**: `eval_div_op` on two `SlotValue::F64` returns `Ok(SlotValue::F64(finite))` for all inputs including division by zero (which yields ±Inf → error), or `Err(ExprError::NonFiniteFloat)` if the result is NaN or infinite
  - **NOTE**: F64/0 returns ±Inf → `FiniteF64::new(±Inf)` fails → `Err(ExprError::NonFiniteFloat)`. This is a design decision distinguishing F64 from I64.
- **POST-005**: `eval_neg_op` on `SlotValue::F64` returns `Ok(SlotValue::F64(finite))` where `finite` is the IEEE 754 negation, or `Err(ExprError::NonFiniteFloat)` if the result is NaN or infinite
- **POST-006**: F64 comparison ops (`eval_gt_op`, `eval_gte_op`, `eval_lt_op`, `eval_lte_op`) return `Ok(SlotValue::Bool(...))` with IEEE 754 semantics (NaN comparisons yield false)
- **POST-007**: After any successful F64 binary op, the result `SlotValue::F64` contains a value that passes `FiniteF64::new(result.get())` (i.e., is finite)
- **POST-008**: Stack underflow/overflow errors return exact `ExprError` variants: `StackUnderflow`, `StackOverflow { max: 64 }`
- **POST-009**: Type mismatch on F64 ops with wrong type returns `Err(ExprError::TypeMismatch { expected: "number", found: ... })`

## Invariants
- **INV-001**: `SlotValue::F64` always contains a `FiniteF64` — construction via `FiniteF64::new` is the only way to create one; invalid values are impossible to represent (Scott Wlaschin: illegal states unrepresentable)
- **INV-002**: `ConstValue::F64` always contains a `FiniteF64` — serde deserialization validates at boundary
- **INV-003**: F64 ops never produce a `SlotValue::F64` containing NaN or infinity — the `FiniteF64::new(result)?` pattern is the enforcement point
- **INV-004**: Stack depth is always ≤ 64 at all evaluation points (bounded by `ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>`)
- **INV-005**: The bytecode evaluator processes ops sequentially without mutation of the program itself (pure function over program state)

## Error Taxonomy
- `ExprError::NonFiniteFloat` — F64 arithmetic yields NaN or ±Inf (covers: overflow, 0/0, x/0 for F64)
- `ExprError::DivisionByZero` — I64 division by zero only; F64/0 yields ±Inf → `NonFiniteFloat`
- `ExprError::IntegerOverflow` — I64 checked arithmetic overflow
- `ExprError::TypeMismatch { expected, found }` — F64 op received non-F64 type
- `ExprError::StackOverflow { max: 64 }` — more than 64 values on stack
- `ExprError::StackUnderflow` — pop on empty stack
- `ExprError::UnexpectedEof` — bytecode program truncated

## Contract Signatures
```rust
// Public API under contract
pub fn eval_expr_program(program: &ExprProgram, slots: &[Option<SlotValue>], constants: &[ConstValue]) -> ExprResult<SlotValue>
pub fn eval_expr_program_with_store(program: &ExprProgram, slots: &[Option<SlotValue>], constants: &[ConstValue], store: &mut ValueStore) -> ExprResult<SlotValue>
pub fn eval_binary_op(op: BinaryOp, left: SlotValue, right: SlotValue) -> ExprResult<SlotValue>
pub fn eval_unary_op(op: UnaryOp, value: SlotValue) -> ExprResult<SlotValue>

// F64-specific op semantics (internal, public for testing)
fn eval_add_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue>
fn eval_sub_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue>
fn eval_mul_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue>
fn eval_div_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue>
fn eval_neg_op(value: SlotValue) -> ExprResult<SlotValue>
fn eval_gt_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue>
fn eval_gte_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue>
fn eval_lt_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue>
fn eval_lte_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue>
```

## F64 Arithmetic Policy Summary
| Operation | Inputs | Result | Non-finite outcome |
|-----------|--------|--------|--------------------|
| F64 + F64 | finite, finite | IEEE 754 sum | `Err(NonFiniteFloat)` if overflow to ±Inf |
| F64 - F64 | finite, finite | IEEE 754 diff | `Err(NonFiniteFloat)` if overflow to ±Inf |
| F64 * F64 | finite, finite | IEEE 754 prod | `Err(NonFiniteFloat)` if overflow to ±Inf |
| F64 / F64 | finite, 0 | ±Inf | `Err(NonFiniteFloat)` (Inf fails FiniteF64) |
| F64 / F64 | finite, non-zero finite | IEEE 754 div | `Err(NonFiniteFloat)` if overflow to ±Inf |
| -F64 | finite | IEEE 754 neg | `Err(NonFiniteFloat)` (negating -f64::MAX does not overflow) |
| F64 cmp F64 | any | IEEE 754 cmp | Returns `false` for NaN comparisons |

## Non-goals
- F64 constant folding in `fold.rs` — returns `None`, not implemented
- F64 helper ops (sum/avg/min/max on F64 lists)
- Verus/Flux/Kani formal proofs — gap noted; `missing functionality` risk tag
- vb_runtime build repair — classified DEFERRED_GLOBAL

## TLA+-Owned Clauses
- **None** — F64 bytecode evaluation is pure deterministic Rust computation with no temporal/state-over-time behavior. No workflow, protocol, scheduler, retry, claim/lease, concurrent, or distributed behavior is in scope. TLA+ not applicable per `tla-spec.md` explicit non-applicability.

## Verus-Owned Clauses
- INV-001 (SlotValue::F64 always finite): Verus spec for `FiniteF64::new` guarantees finite value at construction; no Verus spec currently exists for eval ops (gap)
- INV-003 (F64 ops never produce non-finite): Can be expressed as Verus postconditions on eval ops; currently unwritten (gap)

## Theorem-Owned Clauses
- None — no algebraic theorem kernels beyond Verus expressibility needed for F64 arithmetic
