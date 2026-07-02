# Verification Layers — vb-qi37.9.2

## Boundary
- **Verus-owned kernel**: Pure F64 arithmetic in `eval.rs` — `eval_add_op`, `eval_sub_op`, `eval_mul_op`, `eval_div_op`, `eval_neg_op`, and comparison ops. These are deterministic pure functions over f64/I64 inputs.
- **TLA+ temporal model**: None — no temporal/state-over-time behavior in F64 bytecode evaluation. Explicit non-applicability.
- **Theorem projection**: None — no algebraic kernels beyond Verus expressibility.
- **Runtime shell**: `eval_expr_program`, `eval_expr_program_with_store` — orchestrate stack, ops, ValueStore. Not pure; handled by integration tests.
- **External systems**: None — no I/O, network, DB, or FFI in F64 eval path.

## Layer Assignment

### INV-001: SlotValue::F64 always finite
- **Layer**: `verus` — `FiniteF64::new` constructor invariant; proptest for `FiniteF64` boundary values
- **Layer**: `proptest` — `vb_core/value.rs` already has `finite_f64_*` proptest cases (NaN, ±Inf, subnormals, signed zero)
- **Layer**: `cargo-careful` — compile vb_expr with `cargo careful` to verify no UB in F64 ops
- **Waiver**: No TLA+ needed (pure computation, no temporal behavior)

### INV-002: ConstValue::F64 always finite at deserialization
- **Layer**: `proptest` — serde roundtrip tests for FiniteF64 with adversarial f64 inputs
- **Layer**: `cargo-fuzz` — malformed constant pool inputs

### INV-003: F64 ops never produce non-finite SlotValue::F64
- **Layer**: `verus` — postcondition on each eval op: result is finite
- **Layer**: `proptest` — exhaustively test F64 ops with boundary inputs (0, -0, MIN, MAX, subnormals, very-small + very-large → overflow)
- **Layer**: `kani` — bounded model check of F64 arithmetic ops for non-finite detection
- **Waiver**: No TLA+ needed (pure computation)

### INV-004: Stack depth ≤ 64 at all eval points
- **Layer**: `verus` — `ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>` capacity is 64; overflow prevented by `push_value` capacity check
- **Layer**: `proptest` — stress test with deeply nested expressions

### POST-001 through POST-005: F64 arithmetic correctness
- **Layer**: `verus` — IEEE 754 correspondence: `eval_add_op` postcondition `result.get() ≈ lhs + rhs` (bit-exact for finite results)
- **Layer**: `proptest` — cross-validation: compile+eval produces same result as native Rust f64 arithmetic for finite inputs
- **Layer**: `kani` — bounded model check for overflow detection on `f64::MAX * 2.0`, `f64::MAX + f64::MAX`

### POST-004: F64/0 → NonFiniteFloat error (NOT DivisionByZero)
- **Layer**: `proptest` — explicit test: `eval_div_op(F64(1.0), F64(0.0))` returns `Err(ExprError::NonFiniteFloat)`, NOT `Err(ExprError::DivisionByZero)`
- **Layer**: `kani` — cover property: F64 division by zero does not produce `DivisionByZero`

### POST-006: NaN comparison returns false
- **Layer**: `proptest` — cover property: `eval_gt_op(F64(NaN), F64(x))` returns `false`; same for all comparison ops
- **Layer**: `verus` — postcondition: comparison with NaN yields false

### ERR-001: NonFiniteFloat error on NaN/Inf inputs
- **Layer**: `proptest` — `FiniteF64::new` rejects all NaN bit patterns and ±Inf
- **Layer**: `kani` — cover: `eval_add_op(F64(NaN-like), F64(x))` returns `Err(NonFiniteFloat)`

### ERR-002: DivisionByZero only for I64
- **Layer**: `proptest` — explicit test: I64/0 → `Err(DivisionByZero)`, F64/0 → `Err(NonFiniteFloat)`
- **Layer**: `verus` — postcondition distinguishing I64 and F64 division by zero behavior

### ERR-003: IntegerOverflow on I64 checked arithmetic
- **Layer**: `proptest` — `i64::MAX.checked_add(i64::MAX)` returns `None` → `Err(IntegerOverflow)`
- **Layer**: `kani` — bounded model check for overflow on `checked_add`, `checked_sub`, `checked_mul`

### TypeMismatch error taxonomy
- **Layer**: `proptest` — F64 op with I64 operand returns `TypeMismatch`
- **Layer**: `verus` — postcondition on mixed-type ops

### Performance: Stack overflow at >64 entries
- **Layer**: `proptest` — generate expression with 65+ stack entries → `Err(StackOverflow { max: 64 })`
- **Layer**: `cargo-careful` — verify no UB on ArrayVec overflow attempt

## Verus Scope (Core Pure Logic)
- **Rust target**: `crates/vb_expr/src/eval.rs` — `eval_add_op`, `eval_sub_op`, `eval_mul_op`, `eval_div_op`, `eval_neg_op`, `eval_gt_op`, `eval_gte_op`, `eval_lt_op`, `eval_lte_op`
- **Spec functions**: Postconditions expressing IEEE 754 result finiteness and correctness for finite inputs
- **Invariants**: Stack depth ≤ 64, SlotValue::F64 always finite
- **Trusted boundary**: `FiniteF64::new` constructor; `ArrayVec` capacity
- **Shell exclusions**: I/O, ValueStore, program loading, lexing/parsing

## TLA+ Scope
**Non-applicable**: F64 bytecode evaluation is pure deterministic Rust computation. No temporal properties, liveness, fairness, deadlock, workflow, protocol, scheduler, retry, claim/lease, concurrent, or distributed behavior. TLA+ provides no value for this bead's scope.

## Proptest Scope
- `vb_core/value.rs` already has comprehensive `FiniteF64` tests (NaN, Inf, subnormals, signed zero)
- `vb_expr/eval/tests/integration.rs` needs F64-specific eval tests covering:
  - F64 arithmetic with boundary values (0, -0, MIN, MAX, subnormals)
  - F64/0 → NonFiniteFloat (NOT DivisionByZero)
  - NaN propagation in comparisons
  - Mixed-type errors (TypeMismatch)
  - Stack overflow (expressions with 65+ stack entries)

## Kani Scope
- Bounded model check of F64 ops for overflow detection
- Cross-validate IEEE 754 correspondence for all five arithmetic ops
- Verify NonFiniteFloat vs DivisionByZero distinction

## Waivers
1. **TLA+ waiver**: No temporal/state-over-time behavior in F64 bytecode eval scope. Owner: contract phase. Reason: pure deterministic computation. Compensating evidence: proptest + kani + verus.
2. **Formal proof waiver for eval ops**: No Verus specs currently exist for eval ops (gap). Owner: vb-qi37.9.2 State 4 (proof-planner). Reason: first-pass contract; formal proof obligations to be planned. Compensating evidence: proptest coverage in vb_core + integration test gap analysis.
3. **Constant folding**: F64 constant folding not implemented (fold.rs returns None). Owner: separate bead. Not in scope.
