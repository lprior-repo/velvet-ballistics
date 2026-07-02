# Proof-Writer Report — vb-core-lower-values-actions-refs

## Identity

| Field | Value |
|-------|-------|
| Bead | `vb-core-lower-values-actions-refs` |
| State | 5 (Proof Writing) |
| Writer | proof-writer skill |
| Workspace | `/tmp/vb-ws/vb-core-lower-values-actions-refs` |
| Generated | 2026-05-15 |

---

## 1. Artifact Inventory

The following artifacts were written to the isolated workspace:

| Artifact | Obligation ID | Verifier | Kind |
|---|---|---|---|
| `kani-harnesses/vb_compile_bytecode.rs` | KANI-EXPR-BYTECODE-001 | cargo kani | Kani harness |
| `kani-harnesses/vb_compile_slot.rs` | KANI-SLOT-REF-001 | cargo kani | Kani harness |
| `kani-harnesses/vb_compile_constant.rs` | KANI-CONSTANT-POOL-001 | cargo kani | Kani harness |
| `kani-harnesses/vb_compile_accessor.rs` | KANI-ACCESSOR-REF-001 | cargo kani | Kani harness |
| `kani-harnesses/vb_compile_node_dedup.rs` | INV-007-NODEDUP-001 | cargo kani | Kani harness (optional) |
| `proof-writer-report.md` | — | — | This report |

**Existing test modules** (no new artifacts needed — already present in `crates/vb_compile/src/expression_bytecode.rs`):

| Obligation ID | Evidence artifact | Command |
|---|---|---|
| UNIT-EXPR-BYTESTACK-001 | `test-report.txt` | `cargo test -p vb_compile --lib expression_bytecode` |
| UNIT-ACCESSOR-REF-001 | `test-report.txt` | `cargo test -p vb_compile --lib expression_bytecode` |
| ERR-TAXONOMY-001 | `test-report.txt` | `cargo test -p vb_compile --lib expression_bytecode` |
| UNIT-LOWER-DO-001 | `test-report.txt` | `cargo test -p vb_compile --lib lower` |
| UNIT-BUILD-PARTS-001 | `test-report.txt` | `cargo test -p vb_compile --lib slot_compiler` |
| POST-009-VALIDATE-001 | `test-report.txt` | `cargo test -p vb_compile --lib lower_steps` |
| INV-006-ORDER-001 | `test-report.txt` | `cargo test -p vb_compile --lib lower` |

**Static scan** (no new artifacts — runs on existing source):

| Obligation ID | Evidence artifact | Command |
|---|---|---|
| STATIC-LINT-001 | `clippy-report.txt` | `cargo clippy -p vb_compile --lib -- -D warnings -A unsafe_code` |

---

## 2. Blocked Tooling — Waived Obligations

Two obligations are blocked on Verus toolchain availability:

### WAIVER-VERUS-EXPR-STACK (`VERUS-EXPR-STACK-001`)

- **Clause**: INV-004
- **Target**: `crates/vb_core/src/expressions.rs::ExprProgram::try_from_ops`
- **Waiver owner**: proof-planner
- **Reason**: Verus toolchain not installed (`cargo verus` is placeholder; `verus` binary absent)
- **Compensating evidence**:
  - `UNIT-EXPR-BYTESTACK-001` (proptest — 100+ op combinations including adversarial cases in `expression_bytecode.rs::tests`)
  - `KANI-EXPR-BYTECODE-001` (Kani — overflow path coverage)
  - `MAX_EXPRESSION_STACK = 64` is well within Kani exhaust scope; `check_expr_stack_bound` is pure integer arithmetic
- **Expiry**: Until Verus installed in CI
- **Status**: **WAIVED** — compensating evidence from Kani + proptest lanes provides adequate bounded coverage

### WAIVER-VERUS-SLOT-MAX (`VERUS-SLOT-MAX-001`)

- **Clause**: INV-001
- **Target**: `crates/vb_compile/src/lib.rs::SlotCompiler`
- **Waiver owner**: proof-planner
- **Reason**: Verus toolchain not installed
- **Compensating evidence**:
  - `KANI-SLOT-REF-001` (Kani — slot reference bounds for all valid u16 slot indices)
  - `UNIT-SLOT-COMPILER-001` (proptest — max tracking unit tests)
  - `slot_count()` is a u16-returning pure function; Kani exhausts all u16 values for slot index computations
- **Expiry**: Until Verus installed in CI
- **Status**: **WAIVED** — compensating evidence from Kani + proptest lanes provides adequate bounded coverage

---

## 3. Kani Harness Design

### 3.1 `KANI-EXPR-BYTECODE-001` — Overflow Path Verification

**Target**: `compile_expr_to_bytecode` in `expression_bytecode.rs`
**Claim**: Returns `Err` on overflow and `Ok` with correct `max_stack` otherwise.

Design:
- Model a `ParsedExpression` as a vector of `ExprOp` — bounded by `MAX_EXPRESSION_OPS = 256`
- For all `ops` sequences up to 256 ops, verify either:
  - `Err` is returned (stack overflow or op count overflow)
  - `Ok(program)` where `program.max_stack <= MAX_EXPRESSION_STACK`
- The `check_expr_stack_bound` function is the pure integer core; Kani exhausts all stack depth states

### 3.2 `KANI-SLOT-REF-001` — Slot Reference Lowering

**Target**: slot reference lowering path in `expression_bytecode.rs`
**Claim**: `lower_slot_reference` returns `ExprOp::LoadSlot` for valid `$slot.N` without mutating accessors.

Design:
- For all valid u16 slot indices N, verify `lower_slot_reference("$slot.N", &mut [])` returns `Ok(ExprOp::LoadSlot(SlotIdx::new(N)))` and accessors remain empty.
- `kani::unwind(6)` provides adequate coverage for u16 parse → SlotIdx → LoadSlot path.

### 3.3 `KANI-ACCESSOR-REF-001` — Accessor Path Lowering

**Target**: `lower_accessor_reference` in `expression_bytecode.rs`
**Claim**: `lower_accessor_reference` returns `ExprOp::LoadAccessor` with correct `AccessorProgram` for numeric paths.

Design:
- For all valid `$slots.N.P.Q` patterns with numeric segments, verify:
  - Returns `Ok(ExprOp::LoadAccessor(AccessorIdx))`
  - `AccessorProgram.root == SlotIdx::new(N)`
  - `AccessorProgram.path == [Index(P), Index(Q), ...]`
  - No `PathSegment::Field` variants in path

### 3.4 `KANI-CONSTANT-POOL-001` — Constant Pool Overflow

**Target**: `SlotCompiler::push_constant` in `lib.rs`
**Claim**: Returns `Err` on pool size > u16::MAX and `Ok(ConstIdx)` otherwise.

Design:
- Construct `SlotCompiler` with constants pre-filled to `u16::MAX` (65535)
- Verify `push_constant(ConstValue::Null)` returns `Err(CompileError::Workflow(WorkflowError::ConstOutOfBounds))`
- Verify `push_constant(ConstValue::Null)` on empty compiler returns `Ok(ConstIdx::new(0))`

### 3.5 `INV-007-NODEDUP-001` (optional) — Node StepIdx Uniqueness

**Target**: `lower_steps_to_ir` in `lib.rs`
**Claim**: No two `CompiledNode` entries share the same `StepIdx` as `id` within `WorkflowParts::nodes`.

Design:
- Model a vector of `CompiledNode` with `id: StepIdx`
- Verify that for all `(i, j)` with `i != j`, `nodes[i].id != nodes[j].id`

---

## 4. Proptest Coverage Notes

The `expression_bytecode.rs` module contains **55+ existing unit tests** covering:

1. Binary/unary expression lowering to correct postfix bytecode
2. Helper arity validation for all 11 helpers
3. Error taxonomy: `UnknownReferenceName`, `UnknownReferenceRoot`, `UnsupportedAccessorReference`, `ExpressionLoweringUnsupported`, `ExpressionHelperArity`
4. Stack overflow, constant pool overflow edge cases
5. Large integer constants (i64::MAX, i64::MIN, zero)
6. Boolean, null constants
7. Deeply nested unary/binary expressions
8. All binary operators (Or, And, Eq, NotEq, Lt, Lte, Gt, Gte, Add, Sub, Mul, Div)
9. Accessor reference numeric paths

These tests directly satisfy `UNIT-EXPR-BYTESTACK-001`, `UNIT-ACCESSOR-REF-001`, and `ERR-TAXONOMY-001`.

---

## 5. Formal Verification Execution Plan

**Execute in this order at State 8:**

```bash
# 1. Clippy (fast, independent)
cargo clippy -p vb_compile --lib -- -D warnings -A unsafe_code 2>&1 | tee clippy-report.txt

# 2. Unit tests (fast, independent)
cargo test -p vb_compile --lib expression_bytecode -- --nocapture 2>&1 | tee test-report.txt
cargo test -p vb_compile --lib slot_compiler -- --nocapture 2>&1 | tee -a test-report.txt
cargo test -p vb_compile --lib lower -- --nocapture 2>&1 | tee -a test-report.txt

# 3. Kani (slow, parallelizable)
cargo kani --package vb_compile --harness compile_expr_to_bytecode_overflow 2>&1 | tee kani-expr-report.md
cargo kani --package vb_compile --harness lower_slot_reference_valid 2>&1 | tee kani-slot-report.md
cargo kani --package vb_compile --harness lower_accessor_reference_numeric 2>&1 | tee kani-accessor-report.md
cargo kani --package vb_compile --harness push_constant_overflow 2>&1 | tee kani-constant-report.md
cargo kani --package vb_compile --harness node_id_uniqueness 2>&1 | tee kani-node-report.md
```

---

## 6. Verification Ledger Summary

| Obligation | Layer | Status |
|---|---|---|
| VERUS-EXPR-STACK-001 | verus | **WAIVED** — blocked_tooling; compensating: KANI-EXPR-BYTECODE-001 + UNIT-EXPR-BYTESTACK-001 |
| VERUS-SLOT-MAX-001 | verus | **WAIVED** — blocked_tooling; compensating: KANI-SLOT-REF-001 + UNIT-SLOT-COMPILER-001 |
| KANI-EXPR-BYTECODE-001 | kani | **READY** — harness written |
| KANI-ACCESSOR-REF-001 | kani | **READY** — harness written |
| KANI-SLOT-REF-001 | kani | **READY** — harness written |
| KANI-CONSTANT-POOL-001 | kani | **READY** — harness written |
| UNIT-EXPR-BYTESTACK-001 | proptest | **READY** — existing tests |
| UNIT-SLOT-COMPILER-001 | proptest | **READY** — existing tests |
| UNIT-ACCESSOR-REF-001 | proptest | **READY** — existing tests |
| ERR-TAXONOMY-001 | proptest | **READY** — existing tests |
| UNIT-LOWER-DO-001 | proptest | **READY** — existing tests |
| UNIT-BUILD-PARTS-001 | proptest | **READY** — existing tests |
| POST-009-VALIDATE-001 | proptest | **READY** — existing tests |
| STATIC-LINT-001 | static-scan | **READY** — runs on existing source |
| INV-007-NODEDUP-001 | kani | **READY** — harness written (optional) |
| INV-006-ORDER-001 | proptest | **READY** — existing tests (optional) |
| GATE-VERIFY-FAST-001 | gauntlet | **DEFERRED** — state 12 |

---

*Proof-writer: State 5 complete. All Kani harnesses written. Proptest and clippy lanes run on existing test infrastructure. Waivers documented. Ready for State 8 formal verification.*
