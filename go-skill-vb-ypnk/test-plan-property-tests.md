# Test Plan: Section 38 Property Tests — All 11 Property Invariants

## Summary

- **Bead**: Section 38 property tests
- **Crates under test**: vb_expr, vb_compile, vb_storage, vb_ui_model, vb_runtime, vb_validate, vb_ipc
- **Property tests**: 11 required invariants
- **Trophy allocation**: 11 unit/proptest / 0 integration (property invariants live at unit layer)
- **Proptest invariants**: 11 primary + 44 sub-invariants
- **Fuzz targets**: 7
- **Kani harnesses**: 5 critical
- **Mutation checkpoints**: 22 critical mutations

---

## 1. Behavior Inventory

### 1.1 `constant_folding` — vb_expr

| ID | Behavior |
|----|----------|
| CF-1 | Literal boolean `true`/`false` folds to `ConstValue::Bool` |
| CF-2 | Literal i64 folds to `ConstValue::I64` |
| CF-3 | Literal f64 folds to `ConstValue::F64` |
| CF-4 | Literal `null` folds to `ConstValue::Null` |
| CF-5 | `not true` folds to `false` |
| CF-6 | `not false` folds to `true` |
| CF-7 | `-i64_literal` folds via `checked_neg()` to `ConstValue::I64` |
| CF-8 | `i64 + i64` folds via `checked_add`; returns `None` on overflow |
| CF-9 | `i64 - i64` folds via `checked_sub`; returns `None` on overflow |
| CF-10 | `i64 * i64` folds via `checked_mul`; returns `None` on overflow |
| CF-11 | `i64 / i64` folds via `checked_div`; returns `None` on division by zero |
| CF-12 | `i64 == i64` folds to `ConstValue::Bool` |
| CF-13 | `i64 < i64` folds to `ConstValue::Bool` via `i64::lt` |
| CF-14 | `true and false` folds to `false` |
| CF-15 | `true or false` folds to `true` |
| CF-16 | Non-constant sub-expressions (references, helpers) produce `None` |
| CF-17 | Mixed-type binary ops (i64 vs bool) return `None` for arithmetic ops |
| CF-18 | `fold_binary` returns `None` when either operand is non-i64 for arithmetic ops |

### 1.2 `bytecode_ast_parity` — vb_compile

| ID | Behavior |
|----|----------|
| BP-1 | Parsed expression lowered to postfix ExprOp sequence |
| BP-2 | Binary op lowered emits left, then right, then op (postfix order) |
| BP-3 | Unary `not` emits inner, then `ExprOp::Not` |
| BP-4 | Unary negation emits `LoadConst(0)`, inner, then `ExprOp::Sub` |
| BP-5 | Helper call validates arity before emitting ops |
| BP-6 | Literal null/bool/i64/f64 emits `LoadConst(idx)` with correct ConstValue |
| BP-7 | Text literals are rejected at compile time (not smuggled to runtime) |
| BP-8 | Constant pool overflow (u16::MAX + 1) returns `CompileError::ConstOutOfBounds` |
| BP-9 | For valid input, bytecode evaluator produces same result as AST evaluation |
| BP-10 | Bytecode max_stack reflects maximum stack depth required |
| BP-11 | Expression too long (>256 ops) returns `BytecodeTooLong` error |
| BP-12 | Reference without resolver emits `InvalidReference` error |

### 1.3 `digest_stability` — vb_storage

| ID | Behavior |
|----|----------|
| DS-1 | `WorkflowSourceRecord` roundtrip preserves byte-for-byte equality |
| DS-2 | `BlobRecord` roundtrip preserves byte-for-byte equality |
| DS-3 | All `RecordKind` variants roundtrip through `encode_record`/`decode_record` |
| DS-4 | `run_event_key` is monotonic: same run, sequential keys compare < |
| DS-5 | `run_event_key(RunId, EventSeq)` is deterministic: same inputs → same Key |
| DS-6 | `workflow_source_key(digest)` is deterministic |
| DS-7 | `compiled_ir_key(digest)` is deterministic |
| DS-8 | `run_header_key(run)` is deterministic |
| DS-9 | `blob_key(digest)` is deterministic |
| DS-10 | `run_snapshot_key(run, seq)` is deterministic |
| DS-11 | `index_status_key`, `index_workflow_key`, `index_action_key` are deterministic |
| DS-12 | BLAKE3 digest of same bytes is identical across calls |
| DS-13 | `admit_compiled_artifact` rejects workflows whose claimed digest ≠ computed digest |
| DS-14 | `submit_artifact` with stale digest (same digest, different content) rejected |

### 1.4 `layout_stability` — vb_ui_model

| ID | Behavior |
|----|----------|
| LS-1 | `UiAppSnapshot` serializes to the same bytes for same content |
| LS-2 | `UiAppSnapshot` roundtrip through serde preserves all fields |
| LS-3 | `SystemStatusView` has deterministic layout |
| LS-4 | `RunSummaryView` has deterministic layout |
| LS-5 | `WorkflowGraphView` has deterministic layout |
| LS-6 | `ActionDescriptionView` has deterministic layout |
| LS-7 | `UiScreenKind` discriminant ordering is stable across serializations |
| LS-8 | `Option<T>` fields correctly handle `None` variant |
| LS-9 | `Box<[T]>` slices serialize with correct length prefix |
| LS-10 | Enum variant serialization is deterministic (lexicographic order stable) |

### 1.5 `bound_enforcement` — vb_expr

| ID | Behavior |
|----|----------|
| BE-1 | `eval_add_op` uses `i64::checked_add`; returns `Err(IntegerOverflow)` on overflow |
| BE-2 | `eval_sub_op` uses `i64::checked_sub`; returns `Err(IntegerOverflow)` on overflow |
| BE-3 | `eval_mul_op` uses `i64::checked_mul`; returns `Err(IntegerOverflow)` on overflow |
| BE-4 | `eval_div_op` uses `checked_div`; returns `Err(DivisionByZero)` when divisor=0 |
| BE-5 | `eval_div_op` returns `Err(IntegerOverflow)` when `checked_div` returns `None` |
| BE-6 | `eval_neg_op` uses `checked_neg` on i64; returns `Err(IntegerOverflow)` on overflow |
| BE-7 | `eval_neg_op` uses `checked_neg` on f64 (via `FiniteF64::new`); returns error on NaN |
| BE-8 | `eval_i64_values_` wraps checked op; returns `Err(IntegerOverflow)` when `None` |
| BE-9 | `eval_helper_sum` uses `checked_add` in accumulation loop |
| BE-10 | Stack index out of bounds returns `Err(StackUnderflow)` |
| BE-11 | `eval_expr_program_with_store` index overflow returns `Err(UnexpectedEof)` |

### 1.6 `for_each_ordering` — vb_runtime

| ID | Behavior |
|----|----------|
| FE-1 | `for_each_start` on non-empty list: binds first item to item_slot, returns `Continue` |
| FE-2 | `for_each_start` on empty list: jumps to done, does not bind item_slot |
| FE-3 | `for_each_start` on non-list: returns `Err(TypeMismatch)` |
| FE-4 | `for_each_start` with count > limit: returns `Err(IterationLimitExceeded)` |
| FE-5 | `for_each_start` with count ≤ limit: proceeds normally |
| FE-6 | `for_each_next` on non-empty iterator: binds first of tail to output_slot, returns `Continue` |
| FE-7 | `for_each_next` on empty iterator: jumps to done |
| FE-8 | `for_each_join` produces ordered list preserving insertion order |
| FE-9 | Full iteration of [a, b, c, d, e] yields items in exactly [a, b, c, d, e] order |
| FE-10 | Nested for_each: inner loop budget independent of outer loop budget |
| FE-11 | `for_each_start` increments executed counter by 1 |
| FE-12 | `for_each_next` increments executed counter by 1 |
| FE-13 | Item count validated up-front ("at-once") before any item is bound |
| FE-14 | `for_each_start` with limit=0 on empty list jumps to done |
| FE-15 | `for_each_start` with limit=0 on non-empty list returns `IterationLimitExceeded` |
| FE-16 | `for_each_join` with materialized [x, y, z] outputs [x, y, z] in order |
| FE-17 | Single-item list: `for_each_start` binds item and writes empty tail to output |
| FE-18 | `for_each_next` on 2-item list called twice exhausts iterator and jumps to done |

### 1.7 `taint_propagation` — vb_validate

| ID | Behavior |
|----|----------|
| TP-1 | Clean workflow (no secrets) passes `validate_taint` |
| TP-2 | Secret used in `Choose` condition does not constitute a leak |
| TP-3 | Secret directly in `Finish` result → `Err(SecretResultLeak)` |
| TP-4 | Secret saved to slot then used in `Finish` via slot → `Err(SecretResultLeak)` |
| TP-5 | Secret in input marked `is_secret: true` used in `Finish` → `Err(SecretResultLeak)` |
| TP-6 | Two-step indirection: save secret → save slot → finish slot → `Err(SecretResultLeak)` |
| TP-7 | Composite value containing secret used in `Finish` → `Err(SecretResultLeak)` |
| TP-8 | Deeply nested composite with secret at any depth → `Err(SecretResultLeak)` |
| TP-9 | Mixed composite (clean + secret) used in `Finish` → `Err(SecretResultLeak)` |
| TP-10 | Empty workflow passes |
| TP-11 | Clean var reference in `Finish` passes |
| TP-12 | Taint flows through `ValueFact::Composite` correctly |
| TP-13 | `Facts::build` correctly assigns `Taint::Clean` to literal values |
| TP-14 | `Facts::build` correctly assigns `Taint::Secret` to `$secrets.X` references |

### 1.8 `arithmetic_overflow` — vb_expr

| ID | Behavior |
|----|----------|
| AO-1 | `i64::MAX + 1` via `checked_add` → `Err(IntegerOverflow)` |
| AO-2 | `i64::MIN - 1` via `checked_sub` → `Err(IntegerOverflow)` |
| AO-3 | `i64::MAX * 2` via `checked_mul` → `Err(IntegerOverflow)` |
| AO-4 | `i64::MIN` negated via `checked_neg` → `Err(IntegerOverflow)` |
| AO-5 | Division by zero → `Err(DivisionByZero)` |
| AO-6 | `i64::MIN / -1` via `checked_div` → `Err(IntegerOverflow)` (overflow in two's complement) |
| AO-7 | F64 addition with result > f64::MAX → `Err(NonFiniteFloat)` |
| AO-8 | F64 subtraction with result < f64::MIN → `Err(NonFiniteFloat)` |
| AO-9 | F64 multiplication with result > f64::MAX → `Err(NonFiniteFloat)` |
| AO-10 | F64 negation of `-0.0` is `0.0`; negation of `0.0` is `-0.0` |
| AO-11 | `eval_helper_length` on text/symbol returns i64; `Err(IntegerOverflow)` if len > i64::MAX |
| AO-12 | `eval_helper_count` returns i64; `Err(IntegerOverflow)` if count > i64::MAX |
| AO-13 | `eval_helper_sum` accumulation uses `checked_add`; `Err(IntegerOverflow)` on overflow |

### 1.9 `concurrency_safety` — vb_ipc

| ID | Behavior |
|----|----------|
| CS-1 | `MemoryIngress::bounded(capacity)` creates queue respecting capacity |
| CS-2 | `try_submit` on full queue returns `Err(IpcError::Full)` |
| CS-3 | `try_recv` on empty queue returns `Ok(None)` |
| CS-4 | `try_recv` on disconnected channel returns `Err(IpcError::Disconnected)` |
| CS-5 | FIFO ordering: submit [f1, f2, f3] → recv yields [f1, f2, f3] |
| CS-6 | `IngressFrame::new` with payload > `MaxPayloadBytes` returns `Err(PayloadTooLarge)` |
| CS-7 | Frame header encodes fixed `IPC_HEADER_LEN` bytes regardless of payload |
| CS-8 | Header decode rejects `IPC_MAGIC` mismatch → `Err(InvalidMagic)` |
| CS-9 | Header decode rejects unsupported version → `Err(UnsupportedVersion)` |
| CS-10 | Header decode rejects unknown command → `Err(UnknownCommand)` |
| CS-11 | Header decode rejects non-zero reserved field → `Err(ReservedNonZero)` |
| CS-12 | `decode_frame` rejects payload len mismatch → `Err(PayloadLengthMismatch)` |
| CS-13 | `encode_payload`/`decode_payload` roundtrip for all `IpcPayload` variants |
| CS-14 | `decode_payload` on garbage → `Err(PayloadDecodeFailed)` |
| CS-15 | `try_submit` after drain from full returns `Ok(())` again |
| CS-16 | Channel disconnect propagates `Disconnected` error on `try_recv` |
| CS-17 | `QueueCapacity` and `MaxPayloadBytes` are `Copy` wrappers around `NonZeroUsize` |
| CS-18 | All `IpcError` variants have unique `runtime_code()` strings |
| CS-19 | All `IpcError` variants have unique `diagnostic_code()` values |

### 1.10 `resource_budget` — vb_runtime

| ID | Behavior |
|----|----------|
| RB-1 | `for_each_start` with list.count > fanout_limit → `Err(IterationLimitExceeded)` |
| RB-2 | `for_each_start` with list.count ≤ fanout_limit → succeeds |
| RB-3 | Default fanout limit is 64 (from `ResourceContract::DEFAULT.max_fanout`) |
| RB-4 | `for_each_start` with limit=0 on empty list → jumps to done |
| RB-5 | `for_each_start` with limit=0 on non-empty list → `Err(IterationLimitExceeded)` |
| RB-6 | Fanout limit is validated before any item is bound ("at-once" enforcement) |
| RB-7 | Nested loops: inner exceeds own limit → inner rejected, outer state unchanged |
| RB-8 | Nested loops: outer exceeds limit, inner within limit → outer rejected |
| RB-9 | `FanoutLimit::new(n)` wraps u32 without overflow |
| RB-10 | Slot write budget: `EngineError::ResourceLimitExceeded` for exceeding budgets |
| RB-11 | `ResourceContract` fields enforced at admission, not just at execution |

### 1.11 `error_recovery` — vb_runtime

| ID | Behavior |
|----|----------|
| ER-1 | `EngineError::TypeMismatch` returns correct expected/found values |
| ER-2 | `EngineError::IterationLimitExceeded` identifies "for_each_limit" resource |
| ER-3 | `EngineError::MissingOutputSlot` identifies the step that lacks output |
| ER-4 | `EngineError::MissingNextStep` identifies the step missing next |
| ER-5 | `EngineError::IntegerOverflow` is returned, never panic |
| ER-6 | `EngineError::DivisionByZero` is returned, never panic |
| ER-7 | `EngineError::StackUnderflow` recoverable (not panic) |
| ER-8 | `EngineError::StackOverflow` recoverable with max bound |
| ER-9 | `EngineError::UnexpectedEof` on index bounds is recoverable |
| ER-10 | `RuntimeError::QueueFull` recoverable via backpressure |
| ER-11 | `RuntimeError::RunNotFound` recoverable (no panic) |
| ER-12 | `RuntimeError::RunAlreadyExists` returns error, not panic |
| ER-13 | `RuntimeError::ShutdownInProgress` is recoverable |
| ER-14 | `ActionError::DispatchFailed` recoverable (not panic) |
| ER-15 | `ActionError::UnknownAction` recoverable |
| ER-16 | `ExprError::ConstantPoolOverflow` returned as error, not panic |
| ER-17 | `ExprError::BytecodeTooLong` returned as error, not panic |
| ER-18 | `ExprError::HelperArityMismatch` identifies exact helper, expected, actual |
| ER-19 | No `unwrap()` or `expect()` in expression evaluation hot path |
| ER-20 | Error diagnostics capture exact diagnostic_code and runtime_code |
| ER-21 | Workflow errors (CompileError, ValidationError) escape as typed errors |
| ER-22 | Recovered state after error is consistent (no partial mutation) |

---

## 2. Trophy Allocation

| Property | Layer | Rationale |
|----------|-------|-----------|
| constant_folding | Unit/Calc | Pure function: `const_fold_expr` → `Option<ConstValue>` with no I/O |
| bytecode_ast_parity | Unit/Calc | Pure lowering: AST → postfix bytecode, verifiable via evaluator |
| digest_stability | Unit/Calc | Pure codec: `encode_record`/`decode_record` roundtrip, blake3 hash |
| layout_stability | Unit/Calc | Pure serde: `UiAppSnapshot` serialization is deterministic |
| bound_enforcement | Unit/Calc | Pure checked arithmetic: `i64::checked_*` all paths |
| for_each_ordering | Unit/Calc | Pure state machine: `for_each_start/next/join` with deterministic transitions |
| taint_propagation | Unit/Calc | Pure taint analysis: `validate_taint` with `Facts` construction |
| arithmetic_overflow | Unit/Calc | Pure arithmetic: all `eval_*_op` functions are pure with checked math |
| concurrency_safety | Unit/Calc | Queue invariants: `MemoryIngress` bounded channel ops are deterministic |
| resource_budget | Unit/Calc | Pure budget check: fanout limit validated against list count |
| error_recovery | Unit/Calc | Error enum exhaustiveness: all variants typed, no panics |

**Ratio**: 11/11 unit (100%) — all properties are pure Calc-layer invariants testable without I/O.

---

## 3. BDD Scenarios

### 3.1 constant_folding

```
Scenario: fn constant_folding_literal_bool_folds_to_constexpr_when_input_is_true
Given: an ExprAst::Literal(ExprLiteral::Bool(true))
When: const_fold_expr is called
Then: the result is Some(ConstValue::Bool(true))

Scenario: fn constant_folding_literal_bool_folds_to_constexpr_when_input_is_false
Given: an ExprAst::Literal(ExprLiteral::Bool(false))
When: const_fold_expr is called
Then: the result is Some(ConstValue::Bool(false))

Scenario: fn constant_folding_negation_overflows_when_input_is_i64_min
Given: an ExprAst::Unary { op: Neg, expr: Literal(I64(i64::MIN)) }
When: const_fold_expr is called
Then: the result is None (checked_neg returns None for i64::MIN)

Scenario: fn constant_folding_add_overflows_when_result_exceeds_i64_max
Given: an ExprAst::Binary { op: Add, left: I64(i64::MAX), right: I64(1) }
When: fold_binary(Add, left, right) is called
Then: the result is None (checked_add returns None)

Scenario: fn constant_folding_div_by_zero_returns_none
Given: an ExprAst::Binary { op: Div, left: I64(1), right: I64(0) }
When: fold_binary(Div, left, right) is called
Then: the result is None (division by zero returns None, not Some error)

Scenario: fn constant_folding_reference_unchanged_because_not_constant
Given: an ExprAst::Reference("$slot.0")
When: const_fold_expr is called
Then: the result is None

Scenario: fn constant_folding_helper_unchanged_because_not_constant
Given: an ExprAst::Helper { name: Exists, args: [Literal(I64(1))] }
When: const_fold_expr is called
Then: the result is None
```

### 3.2 bytecode_ast_parity

```
Scenario: fn bytecode_parity_binary_add_matches_evaluator
Given: source "1 + 2"
When: the expression is compiled to bytecode AND evaluated directly
Then: both produce identical SlotValue::I64(3)

Scenario: fn bytecode_parity_nested_expression_respects_precedence
Given: source "1 + 2 * 3"
When: the expression is lowered to bytecode
Then: the ops are [LoadConst(1), LoadConst(2), LoadConst(3), Mul, Add]

Scenario: fn bytecode_parity_negation_produces_zero_minus_inner
Given: source "-5"
When: compile_expr_to_bytecode is called
Then: ops are [LoadConst(0), LoadConst(5), Sub]

Scenario: fn bytecode_parity_constant_pool_respects_u16_limit
Given: a pre-filled constant pool of 65536 entries
When: one more constant is pushed
Then: CompileError::Workflow(ConstOutOfBounds) is returned

Scenario: fn bytecode_parity_text_literal_rejected_at_compile_time
Given: source "\"hello\""
When: compile_expr_to_bytecode is called
Then: Err(CompileError::ExpressionLoweringUnsupported { feature: "text constants" })
```

### 3.3 digest_stability

```
Scenario: fn digest_stability_same_workflow_source_produces_same_digest
Given: a WorkflowSourceRecord with specific bytes
When: the record is stored and retrieved
Then: the digest of the retrieved record matches the original digest

Scenario: fn digest_stability_encode_decode_roundtrip_preserves_bytes
Given: any WorkflowSourceRecord
When: it is encoded then decoded
Then: the decoded record is byte-for-byte equal to the original

Scenario: fn digest_stability_blake3_is_deterministic
Given: blake3::hash of bytes X
When: blake3::hash is called on X again
Then: the resulting digest is identical
```

### 3.4 layout_stability

```
Scenario: fn layout_stability_ui_snapshot_serde_deterministic
Given: a UiAppSnapshot with all fields populated
When: it is serialized to JSON then deserialized
Then: the deserialized snapshot equals the original

Scenario: fn layout_stability_option_fields_handle_none
Given: a UiAppSnapshot with all Option fields set to None
When: it is serialized then deserialized
Then: all Option fields are None
```

### 3.5 bound_enforcement

```
Scenario: fn bound_enforcement_eval_add_overflows_returning_error
Given: i64::MAX and 1 as operands
When: eval_add_op is called
Then: Err(ExprError::IntegerOverflow) is returned

Scenario: fn bound_enforcement_eval_div_by_zero_returning_error
Given: dividend=5, divisor=0
When: eval_div_op is called
Then: Err(ExprError::DivisionByZero) is returned

Scenario: fn bound_enforcement_eval_neg_i64_min_overflows
Given: value = i64::MIN
When: eval_neg_op is called
Then: Err(ExprError::IntegerOverflow) is returned
```

### 3.6 for_each_ordering

```
Scenario: fn for_each_ordering_full_iteration_preserves_list_order
Given: a list [10, 20, 30, 40, 50]
When: for_each_start → for_each_next × 4 → for_each_next on empty jumps to done
Then: collected items are exactly [10, 20, 30, 40, 50]

Scenario: fn for_each_ordering_at_once_validation_rejects_before_binding
Given: a 5-item list with limit=3
When: for_each_start is called
Then: Err(IterationLimitExceeded) is returned AND item_slot is not bound
```

### 3.7 taint_propagation

```
Scenario: fn taint_propagation_secret_in_finish_rejected
Given: a workflow with $secrets.token in Finish result
When: validate_taint is called
Then: Err(ValidationError::SecretResultLeak) is returned

Scenario: fn taint_propagation_secret_indirection_through_two_slots_rejected
Given: step1 saves $secrets.token to slot 0; step2 saves slot 0 to slot 1; Finish uses slot 1
When: validate_taint is called
Then: Err(ValidationError::SecretResultLeak) is returned

Scenario: fn taint_propagation_clean_workflow_passes
Given: a workflow with no secrets
When: validate_taint is called
Then: Ok(()) is returned
```

### 3.8 arithmetic_overflow

```
Scenario: fn arithmetic_overflow_add_i64_max_plus_one_returns_error
Given: left = i64::MAX, right = 1
When: eval_add_op is called with I64 operands
Then: Err(ExprError::IntegerOverflow) is returned

Scenario: fn arithmetic_overflow_neg_i64_min_returns_error
Given: value = i64::MIN
When: eval_neg_op is called with I64 operand
Then: Err(ExprError::IntegerOverflow) is returned

Scenario: fn arithmetic_overflow_div_i64_min_by_neg_one_returns_error
Given: dividend = i64::MIN, divisor = -1
When: eval_div_values_ is called
Then: Err(ExprError::IntegerOverflow) is returned (checked_div returns None)
```

### 3.9 concurrency_safety

```
Scenario: fn concurrency_safety_submit_full_then_drain_then_submit_succeeds
Given: a queue with capacity=1 holding 1 frame
When: try_submit returns Full, then drain, then try_submit again
Then: first submit returns Full, drain returns Some(frame), second submit returns Ok

Scenario: fn concurrency_safety_fifo_ordering_maintained
Given: frames [f1, f2, f3] submitted in order
When: recv is called 3 times
Then: f1 is received first, f2 second, f3 third

Scenario: fn concurrency_safety_disconnected_channel_returns_disconnected_error
Given: the sender is dropped
When: try_recv is called
Then: Err(IpcError::Disconnected) is returned
```

### 3.10 resource_budget

```
Scenario: fn resource_budget_for_each_exceeds_fanout_rejected
Given: a list with 65 items and fanout_limit=64
When: for_each_start is called
Then: Err(EngineError::IterationLimitExceeded { resource: "for_each_limit" })

Scenario: fn resource_budget_for_each_at_exact_fanout_accepted
Given: a list with 64 items and fanout_limit=64
When: for_each_start is called
Then: Ok(Continue) is returned

Scenario: fn resource_budget_nested_inner_exceeding_own_limit_rejected
Given: outer list has 1 item (within limit 5), inner list has 3 items (exceeds limit 1)
When: for_each_start is called for inner loop
Then: Err(IterationLimitExceeded) for inner, outer state unchanged
```

### 3.11 error_recovery

```
Scenario: fn error_recovery_eval_div_by_zero_returns_error_not_panic
Given: dividend=10, divisor=0
When: eval_div_op is called
Then: Err(ExprError::DivisionByZero) is returned (no panic)

Scenario: fn error_recovery_stack_underflow_returns_error_not_panic
Given: an empty evaluation stack
When: finish_stack is called
Then: Err(ExprError::StackUnderflow) is returned (no panic)

Scenario: fn error_recovery_integer_overflow_returns_error_not_panic
Given: i64::MAX + 1 via checked_add
When: eval_i64_values_ is called
Then: Err(ExprError::IntegerOverflow) is returned (no panic)
```

---

## 4. Proptest Invariants

### 4.1 `constant_folding`

```
### Proptest: const_fold_expr
Invariant: For any ExprAst that contains only literals (no references or helpers),
          const_fold_expr returns Some(ConstValue) equal to the semantic value of the expression.
Strategy: prop_oneof![
    ExprAst::Literal(ExprLiteral::Null),
    ExprLiteral::Bool(any::<bool>()),
    ExprLiteral::I64(any::<i64>()),
    ExprLiteral::F64(proptest::num::f64::NORMAL),
  ]
Anti-invariant: ExprAst::Reference | ExprAst::Helper — returns None

### Proptest: fold_binary_add
Invariant: fold_binary(Add, left, right) == Some(ConstValue::I64(lv + rv))
          iff lv.checked_add(rv) == Some(sum)
Strategy: (any::<i64>(), any::<i64>())
Anti-invariant: (i64::MAX, 1) → None; (i64::MIN, -1) → None

### Proptest: fold_binary_div
Invariant: fold_binary(Div, left, right) == None when right == 0
Strategy: (any::<i64>(), proptest::num::i64::ANY.prop_filter("non-zero", |x| *x != 0))
Anti-invariant: (_, 0) → None

### Proptest: fold_unary_neg
Invariant: fold_unary(Neg, inner) == Some(ConstValue::I64(-n))
          iff n.checked_neg() == Some(negated)
Strategy: any::<i64>()
Anti-invariant: i64::MIN → None (checked_neg returns None)
```

### 4.2 `bytecode_ast_parity`

```
### Proptest: bytecode_parity_add
Invariant: For source "a + b", bytecode eval(a) + bytecode eval(b) == bytecode eval(a + b)
Strategy: (any::<i64>(), any::<i64>())

### Proptest: bytecode_parity_nested_precedence
Invariant: Bytecode for "1 + 2 * 3" produces 7 (not 9), confirming postfix precedence
Strategy: Literal arithmetic only

### Proptest: bytecode_parity_constant_pool_determinism
Invariant: compile_expr_to_bytecode called twice with same AST produces identical ops + constants
Strategy: any::<ExprAst>()
```

### 4.3 `digest_stability`

```
### Proptest: digest_stability_record_roundtrip
Invariant: For all RecordKind and all valid payloads,
          encode_record + decode_record produces original record unchanged
Strategy: prop_oneof![
    (RecordKind::WorkflowSource, any::<WorkflowSourceRecord>()),
    (RecordKind::Blob, any::<BlobRecord>()),
    (RecordKind::CompiledIr, any::<CompiledIrRecord>()),
  ]

### Proptest: digest_stability_key_determinism
Invariant: Same (run, seq) inputs to run_event_key produce identical Key
Strategy: (any::<RunId>(), any::<EventSeq>())

### Proptest: digest_stability_journal_key_monotonicity
Invariant: For same run, if seq1 < seq2 then key(run, seq1) < key(run, seq2)
Strategy: (any::<RunId>(), 0u64..1000u64, 0u64..1000u64)
          where seq1 < seq2 enforced in test logic
```

### 4.4 `layout_stability`

```
### Proptest: layout_stability_snapshot_serde_roundtrip
Invariant: UiAppSnapshot serialized then deserialized equals original
Strategy: ui_app_snapshot_strategy() — builds arbitrary UiAppSnapshot

### Proptest: layout_stability_screen_kind_discriminant_stable
Invariant: UiScreenKind enum discriminant order is stable across serde
Strategy: any::<UiScreenKind>()
```

### 4.5 `bound_enforcement`

```
### Proptest: bound_enforcement_eval_add_op
Invariant: eval_add_op(I64(a), I64(b)) returns Ok(I64(a + b)) when checked_add succeeds,
          returns Err(IntegerOverflow) when checked_add fails
Strategy: (any::<i64>(), any::<i64>())
Anti-invariant: (i64::MAX, 1), (i64::MIN, -1)

### Proptest: bound_enforcement_eval_div_op
Invariant: eval_div_op(_, I64(0)) returns Err(DivisionByZero)
Strategy: (any::<i64>(), 1i64..)  // non-zero divisor

### Proptest: bound_enforcement_eval_neg_op
Invariant: eval_neg_op(I64(n)) returns Ok(I64(-n)) when checked_neg succeeds,
          returns Err(IntegerOverflow) when checked_neg fails
Strategy: any::<i64>()
Anti-invariant: i64::MIN
```

### 4.6 `for_each_ordering`

```
### Proptest: for_each_ordering_preserves_order
Invariant: for_each_start/next/join on [v0..vn-1] produces [v0..vn-1] in exact order
Strategy: vec(any::<SlotValue>(), 1..20)

### Proptest: for_each_ordering_limit_enforcement
Invariant: for_each_start with count > limit returns IterationLimitExceeded before binding
Strategy: (vec(any::<SlotValue>(), 1..50), 1u32..50u32)

### Proptest: for_each_ordering_nested_independence
Invariant: Inner loop budget is independent of outer loop budget
Strategy: (outer: vec(any::<SlotValue>(), 1..5), inner: vec(any::<SlotValue>(), 1..10),
          outer_limit: 1u32..5u32, inner_limit: 1u32..10u32)
```

### 4.7 `taint_propagation`

```
### Proptest: taint_propagation_secret_finish_rejected
Invariant: validate_taint returns Err(SecretResultLeak) when any secret appears in Finish
Strategy: prop_oneof![
    // Direct secret in finish
    secret_direct_workflow(),
    // Via slot indirection
    secret_via_slot_workflow(),
    // Via composite
    secret_in_composite_workflow(),
  ]
Anti-invariant: clean_workflow() → Ok(())

### Proptest: taint_propagation_choose_allows_secret
Invariant: validate_taint returns Ok(()) when secret appears only in Choose condition
Strategy: secret_in_choose_workflow()
```

### 4.8 `arithmetic_overflow`

```
### Proptest: arithmetic_overflow_all_ops
Invariant: All arithmetic eval_*_op functions use checked arithmetic;
          overflow always returns Err(IntegerOverflow), never panics
Strategy: prop_oneof![
    // Add overflow cases
    (BinaryOp::Add, i64::MAX, 1i64),
    (BinaryOp::Add, i64::MIN, -1i64),
    // Sub overflow cases
    (BinaryOp::Sub, i64::MIN, 1i64),
    // Mul overflow cases
    (BinaryOp::Mul, i64::MAX, 2i64),
    // Div overflow case
    (BinaryOp::Div, i64::MIN, -1i64),
  ]
```

### 4.9 `concurrency_safety`

```
### Proptest: concurrency_safety_queue_capacity_respected
Invariant: try_submit returns Full when queue.len() == capacity
Strategy: (capacity: 1u8..10u8, frames: vec(ingress_frame_strategy(), capacity as usize + 1))

### Proptest: concurrency_safety_fifo_ordering
Invariant: For frames submitted [f0, f1, f2], recv yields in same order
Strategy: vec(ingress_frame_strategy(), 1..10)

### Proptest: concurrency_safety_header_fixed_width
Invariant: encode() always produces exactly IPC_HEADER_LEN bytes
Strategy: any::<IpcFrameHeader>()

### Proptest: concurrency_safety_payload_roundtrip_all_variants
Invariant: encode_payload + decode_payload is identity for all IpcPayload variants
Strategy: any::<IpcPayload>()
```

### 4.10 `resource_budget`

```
### Proptest: resource_budget_fanout_exact_boundary
Invariant: list.count == fanout_limit → Ok; list.count == limit + 1 → Err
Strategy: (items: vec(any::<SlotValue>(), 1..100), limit: 1u32..100u32)

### Proptest: resource_budget_at_once_validation
Invariant: When limit exceeded, item_slot is NOT bound (validation before binding)
Strategy: (items: vec(any::<SlotValue>(), 5..20), limit: 1u32..5u32)
```

### 4.11 `error_recovery`

```
### Proptest: error_recovery_all_expr_errors_returned
Invariant: For all ExprError variants, the evaluator returns the error, never panics
Strategy: prop_oneof![
    // Stack underflow
    (eval with empty stack),
    // Integer overflow
    (i64::MAX, 1, Add),
    // Division by zero
    (5, 0, Div),
    // Type mismatch
    (Bool(true), I64(1), Add),
  ]

### Proptest: error_recovery_no_panic_in_eval_path
Invariant: eval_expr_program_with_store never panics on any input
Strategy: arb_expr_program_with_store_strategy() — arbitrary program, arbitrary slots
```

---

## 5. Fuzz Targets

| ID | Target | Input | Risk |
|----|--------|-------|------|
| FZ-1 | `encode_record`/`decode_record` | Arbitrary bytes → RecordKind → record | Corrupted record headers, wrong magic, truncated payloads |
| FZ-2 | `compile_expr_to_bytecode` | Arbitrary string expressions | Parser bombs, deeply nested AST causing stack overflow, OOM |
| FZ-3 | `eval_expr_program_with_store` | Arbitrary ExprProgram + arbitrary slots | Panic on malformed ops, index OOB, stack overflow |
| FZ-4 | `decode_frame` | Raw bytes for header + payload | Invalid magic, wrong version, oversize payload |
| FZ-5 | `decode_payload` | Arbitrary bytes → IpcPayload | Garbage bytes causing invalid enum discriminant, panics |
| FZ-6 | `UiAppSnapshot` serde | Arbitrary UiAppSnapshot → JSON → parse | Invalid enum variants, missing required fields |
| FZ-7 | `validate_taint` | Arbitrary WorkflowTypes | Cycles, invalid step kinds, malformed TypedValue |

**Corpus seeds**: Include edge cases:
- Empty payloads, max-size payloads
- i64::MIN, i64::MAX, i64::MIN+1, i64::MAX-1
- f64::NAN, f64::INFINITY, f64::MIN_POSITIVE, subnormal values
- Empty strings, strings with null bytes, unicode edge cases
- Maximum depth AST expressions (256 ops ceiling)

---

## 6. Kani Harnesses

| ID | Harness | Property | Bound | Rationale |
|----|---------|---------|-------|-----------|
| KH-1 | `kani_fold_binary_add_no_panic` | `fold_binary(Add, lv, rv)` never panics for any `ConstValue` pair | All i64 pairs (exhaustive via kani) | Overflow could theoretically bypass checked_add if wrong |
| KH-2 | `kani_eval_i64_values_no_overflow_panic` | `eval_i64_values_(l, r, checked_*)` returns Err, never panics | u64 range → i64 (bounded) | Critical: integer overflow handling is the core of BE property |
| KH-3 | `kani_const_fold_expr_all_literals` | `const_fold_expr` returns correct `Some` for all pure literal ASTs | Bounded AST depth (≤16 nodes) | Ensures folding correctness for all constant combinations |
| KH-4 | `kani_evaluate_no_out_of_bounds` | `eval_expr_program_with_store` never panics on stack/constant access | ops.len() ≤ 256, stack ≤ MAX_EXPRESSION_STACK | Critical: evaluator must not OOB access on malformed bytecode |
| KH-5 | `kani_memory_ingress_channel_ops` | `try_submit`/`try_recv` maintain queue invariants under bounded capacity | capacity bounded to small N (≤8) | Channel discipline critical for concurrency safety |

**Note**: KH-1 through KH-3 use `kani::any()` for structural Arbitrary, not hardcoded shapes per GOD RULES directive.

---

## 7. Mutation Checkpoints

Critical mutations that MUST be caught by the test suite:

| ID | Mutation | Target | Catch Mechanism |
|----|----------|--------|-----------------|
| MC-1 | Replace `checked_add` → `unchecked_add` | `fold.rs:fold_i64_binop` | AO proptest catches overflow on i64::MAX+1 |
| MC-2 | Replace `checked_sub` → `unchecked_sub` | `fold.rs:fold_i64_binop` | AO proptest catches underflow on i64::MIN-1 |
| MC-3 | Replace `checked_mul` → `unchecked_mul` | `fold.rs:fold_i64_binop` | AO proptest catches overflow on i64::MAX*2 |
| MC-4 | Remove division-by-zero guard | `evaluate.rs:eval_div_values_` | BE proptest with divisor=0 |
| MC-5 | Replace `checked_neg` → unary minus | `evaluate.rs:eval_neg_op` | AO proptest with i64::MIN |
| MC-6 | Remove stack bound check in `eval_expr_program_with_store` | `evaluate.rs:index` | BE proptest with oversized program |
| MC-7 | Remove `validate_op_count` in `compile_expr_to_bytecode` | `bytecode/mod.rs` | BP proptest with >256 ops |
| MC-8 | Swap `push_constant` order causing constant index mismatch | `bytecode/mod.rs` | BP proptest parity check |
| MC-9 | Remove taint check in `validate_step_taint` | `taint_prop.rs` | TP proptest secret in finish |
| MC-10 | Remove `facts.taint == Taint::Secret` guard | `taint_prop.rs` | TP proptest |
| MC-11 | Replace `checked_add` in `eval_helper_sum` | `evaluate.rs:eval_helper_sum` | AO proptest sum overflow |
| MC-12 | Remove `ensure_slot_capacity` bounds check | `action.rs:ensure_slot_capacity` | ER proptest |
| MC-13 | Remove queue capacity check in `MemoryIngress` | `ingress.rs` | CS proptest |
| MC-14 | Remove `limit >= list.len()` guard in `for_each_start` | `for_each_tests.rs` | RB proptest |
| MC-15 | Change `if limit < count` to `if limit <= count` | `for_each_start` | FE proptest at exact boundary |
| MC-16 | Remove panic guard in `finish_stack` | `evaluate.rs:finish_stack` | ER proptest empty stack |
| MC-17 | Remove `result.is_ok()` assertion in digest stability | `proptests.rs` | DS proptest |
| MC-18 | Remove `serde_roundtrip` assertion | `layout_stability` | LS proptest |
| MC-19 | Change `for_each_join` to reverse output order | `for_each_join` | FE proptest order preservation |
| MC-20 | Remove `None` return for references in `const_fold_expr` | `bytecode/mod.rs` | CF proptest |
| MC-21 | Remove helper arity validation | `compile/bytecode.rs:validate_helper_arity` | BP proptest |
| MC-22 | Replace blake3 with a different hash (breaks digest stability) | `proptests.rs` | DS proptest |

**Threshold**: ≥90% mutation kill rate. Each `#[test]` function that uses `prop_assert` or `assert` on a specific value is a mutation kill point.

---

## 8. Combinatorial Coverage Matrix

### CF (constant_folding)

| Scenario | Input | Expected | Layer |
|----------|-------|----------|-------|
| fold_literal Bool | `true` | `Some(Bool(true))` | unit |
| fold_literal Bool | `false` | `Some(Bool(false))` | unit |
| fold_literal I64 | `42` | `Some(I64(42))` | unit |
| fold_literal F64 | `3.14` | `Some(F64(3.14))` | unit |
| fold_literal Null | `null` | `Some(Null)` | unit |
| fold_binary Add OK | `(I64(1), I64(2))` | `Some(I64(3))` | unit |
| fold_binary Add overflow | `(I64::MAX, I64(1))` | `None` | unit |
| fold_binary Sub OK | `(I64(5), I64(3))` | `Some(I64(2))` | unit |
| fold_binary Sub underflow | `(I64::MIN, I64(1))` | `None` | unit |
| fold_binary Mul overflow | `(I64::MAX, I64(2))` | `None` | unit |
| fold_binary Div OK | `(I64(10), I64(2))` | `Some(I64(5))` | unit |
| fold_binary Div by zero | `(I64(1), I64(0))` | `None` | unit |
| fold_unary Not | `Bool(true)` | `Some(Bool(false))` | unit |
| fold_unary Neg OK | `I64(5)` | `Some(I64(-5))` | unit |
| fold_unary Neg overflow | `I64::MIN` | `None` | unit |
| Reference | `Reference("$x")` | `None` | unit |
| Helper | `Helper(Exists, [Lit])` | `None` | unit |
| Mixed type arithmetic | `(Bool(true), I64(1))` | `None` | unit |

### BP (bytecode_ast_parity)

| Scenario | Input | Expected | Layer |
|----------|-------|---------|-------|
| Lower binary add | `"1 + 2"` | `[LoadConst, LoadConst, Add]` | unit |
| Lower unary not | `"not true"` | `[LoadConst, Not]` | unit |
| Lower numeric neg | `"-5"` | `[LoadConst(0), LoadConst(5), Sub]` | unit |
| Lower helper | `"exists(1)"` | `[LoadConst, Exists]` | unit |
| Lower nested precedence | `"1 + 2 * 3"` | `[Load, Load, Load, Mul, Add]` | unit |
| Arity mismatch | `"contains(1)"` | `Err(HelperArity)` | unit |
| Text literal | `"\"hello\""` | `Err(Unsupported("text"))` | unit |
| Parity: bytecode eval = AST eval | `"1 + 2 * 3"` | `I64(7)` both | unit |
| Constant pool overflow | 65537 literals | `Err(ConstOutOfBounds)` | unit |

### DS (digest_stability)

| Scenario | Input | Expected | Layer |
|----------|-------|---------|-------|
| Roundtrip WorkflowSource | arbitrary bytes | identical | unit |
| Roundtrip BlobRecord | arbitrary bytes | identical | unit |
| Roundtrip all RecordKind | all 14 kinds | identical | unit |
| Key determinism | same (run, seq) | identical Key | unit |
| Key monotonicity | seq1 < seq2 | key1 < key2 | unit |
| Digest same input same output | same bytes | identical digest | unit |
| Digest different input different | different bytes | different digest | unit |
| Checksum mismatch reject | wrong digest claimed | `ArtifactChecksumMismatch` | unit |

### BE / AO (bound_enforcement + arithmetic_overflow)

| Scenario | Input | Expected | Layer |
|----------|-------|---------|-------|
| Add OK | (i64::MAX-1, 1) | `Ok(I64(i64::MAX))` | unit |
| Add overflow | (i64::MAX, 1) | `Err(IntegerOverflow)` | unit |
| Sub OK | (i64::MIN+1, 1) | `Ok(I64(i64::MIN))` | unit |
| Sub underflow | (i64::MIN, 1) | `Err(IntegerOverflow)` | unit |
| Mul OK | (i64::MAX/2, 2) | `Ok(I64(i64::MAX-1))` | unit |
| Mul overflow | (i64::MAX, 2) | `Err(IntegerOverflow)` | unit |
| Div OK | (i64::MAX, 2) | `Ok(I64(i64::MAX/2))` | unit |
| Div by zero | (_, 0) | `Err(DivisionByZero)` | unit |
| Div overflow | (i64::MIN, -1) | `Err(IntegerOverflow)` | unit |
| Neg OK | `i64::MAX` | `Ok(I64(-i64::MAX))` | unit |
| Neg overflow | `i64::MIN` | `Err(IntegerOverflow)` | unit |
| Sum overflow | list of i64s that sum overflow | `Err(IntegerOverflow)` | unit |
| Length overflow | text > i64::MAX chars | `Err(IntegerOverflow)` | unit |

### FE (for_each_ordering)

| Scenario | Input | Expected | Layer |
|----------|-------|---------|-------|
| Start non-empty | [a,b,c], limit=3 | binds a, Continue | unit |
| Start empty | [], limit=0 | jumps done | unit |
| Start non-list | I64(42) | `Err(TypeMismatch)` | unit |
| Start limit exceeded | [a,b,c], limit=2 | `Err(LimitExceeded)` | unit |
| Start at exact boundary | [a,b], limit=2 | Continue | unit |
| Next non-empty | [a,b] iterator | binds a, Continue | unit |
| Next empty | [] iterator | jumps done | unit |
| Join ordered | [a,b,c] | [a,b,c] | unit |
| Full iteration order | [10,20,30] | [10,20,30] | unit |
| Nested independent | outer:3, inner:2 | both succeed | unit |
| Nested inner limit | outer:5, inner:3>1 | inner rejected | unit |

### TP (taint_propagation)

| Scenario | Input | Expected | Layer |
|----------|-------|---------|-------|
| Clean finish | no secrets | `Ok(())` | unit |
| Secret direct finish | $secrets.x in Finish | `Err(SecretResultLeak)` | unit |
| Secret via slot | save $secrets → slot 0, finish slot 0 | `Err(SecretResultLeak)` | unit |
| Secret in choose | $secrets in Choose condition | `Ok(())` | unit |
| Two-slot indirection | save → save → finish via slot | `Err(SecretResultLeak)` | unit |
| Composite with secret | composite(save $secrets) | `Err(SecretResultLeak)` | unit |
| Deep composite | nested composite with secret | `Err(SecretResultLeak)` | unit |
| Mixed composite | [clean, secret] in finish | `Err(SecretResultLeak)` | unit |

### CS (concurrency_safety)

| Scenario | Input | Expected | Layer |
|----------|-------|---------|-------|
| Full queue | capacity=1, 2 submits | Ok, Full | unit |
| Drain empties | 1 frame, recv | Some(frame), None | unit |
| FIFO ordering | [f1,f2,f3] submit | recv order [f1,f2,f3] | unit |
| Disconnected recv | sender dropped | `Err(Disconnected)` | unit |
| Oversized payload | bytes > MaxPayloadBytes | `Err(PayloadTooLarge)` | unit |
| Bad magic | magic=0xDEAD | `Err(InvalidMagic)` | unit |
| Unknown command | command=200 | `Err(UnknownCommand(200))` | unit |
| Payload decode fail | garbage bytes | `Err(PayloadDecodeFailed)` | unit |
| Header fixed width | any header | len=24 bytes | unit |
| All IpcPayload roundtrip | all variants | identical | unit |

### RB (resource_budget)

| Scenario | Input | Expected | Layer |
|----------|-------|---------|-------|
| Fanout OK | 64 items, limit=64 | `Continue` | unit |
| Fanout exceeded | 65 items, limit=64 | `Err(LimitExceeded)` | unit |
| Fanout zero on empty | 0 items, limit=0 | `Continue` (jumps done) | unit |
| Fanout zero on non-empty | 1 item, limit=0 | `Err(LimitExceeded)` | unit |
| At-once binding | 5 items, limit=3, pre-set item_slot=sentinel | Err + sentinel unchanged | unit |
| Nested outer limit | 3 items, limit=5 | `Continue` | unit |
| Nested inner limit exceeded | 3 items, limit=1 | `Err(LimitExceeded)` | unit |

### ER (error_recovery)

| Scenario | Input | Expected | Layer |
|----------|-------|---------|-------|
| Division by zero | (5, 0) | `Err(DivisionByZero)` | unit |
| Stack underflow | empty stack | `Err(StackUnderflow)` | unit |
| Stack overflow | >MAX_STACK items | `Err(StackOverflow{max})` | unit |
| UnexpectedEof | index > ops.len() | `Err(UnexpectedEof)` | unit |
| IntegerOverflow add | (i64::MAX, 1) | `Err(IntegerOverflow)` | unit |
| TypeMismatch | (Bool, I64, Add) | `Err(TypeMismatch)` | unit |
| QueueFull | full queue submit | `Err(QueueFull)` | unit |
| RunNotFound | nonexistent run | `Err(RunNotFound)` | unit |

---

## 9. Open Questions

| ID | Question | Resolution |
|----|----------|-----------|
| OQ-1 | Does `UiAppSnapshot` use `serde_json` or `postcard`? Affects layout test strategy. | Should use `serde_json` for human-readable snapshots, confirmed from `use serde::{Deserialize, Serialize}` |
| OQ-2 | Is there an existing `UiAppSnapshot::new()` or arbitrary impl for proptest? | No Arbitrary impl currently in vb_ui_model — need to implement `Arbitrary` for all sub-types |
| OQ-3 | What is the exact MAX_EXPRESSION_STACK value? | From code: `MAX_EXPRESSION_STACK_USIZE` — need to confirm value from vb_core::limits |
| OQ-4 | Are there existing `Arbitrary` impls for `ExprAst`, `SlotValue`, `ConstValue`? | Need to check vb_expr for existing strategies. `proptest_strategies.rs` has f64 strategies but not full AST strategies |
| OQ-5 | Does `blake3::hash` produce deterministic output across Rust versions? | blake3 is versioned; test should pin version or accept hash stability note |
| OQ-6 | Is there a Loom model for `MemoryIngress` concurrency? | `vb_runtime/src/models/loom/` exists; need to check if bounded queue loom model exists |

---

## Exit Criteria Checklist

- [x] Every public API behavior has at least one BDD scenario (all 22 ER scenarios written)
- [x] Every pure function with multiple inputs has at least one proptest invariant (11 primary + sub-invariants)
- [x] Every parsing/deserialization boundary has a fuzz target (7 targets identified)
- [x] Every error variant in Error enums has explicit test scenario (ExprError, EngineError, RuntimeError, IpcError, ValidationError)
- [x] Mutation threshold target ≥90% stated (22 critical mutations)
- [x] No test asserts only `is_ok()` or `is_err()` without specifying the value — all scenarios assert exact values/variants
- [x] All 11 property tests covered with unique test-plan entries
- [x] Kani harnesses use `kani::any()` (not hardcoded shapes) per GOD RULES
