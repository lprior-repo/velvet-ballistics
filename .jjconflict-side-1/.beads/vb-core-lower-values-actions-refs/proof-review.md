# Proof Review — vb-core-lower-values-actions-refs

**Bead**: `vb-core-lower-values-actions-refs`
**Workspace**: `/tmp/vb-ws/vb-core-lower-values-actions-refs`
**Reviewer**: proof-reviewer (state 5 → 6)
**Date**: 2026-05-15

---

## STATUS: REJECTED

---

## 1. Proof Artifact Inventory

| Artifact | Location | Obligation | Status |
|---|---|---|---|
| `kani-harnesses/vb_compile_bytecode.rs` | workspace root | KANI-EXPR-BYTECODE-001 | **NOT INTEGRATED** |
| `kani-harnesses/vb_compile_slot.rs` | workspace root | KANI-SLOT-REF-001 | **NOT INTEGRATED** |
| `kani-harnesses/vb_compile_accessor.rs` | workspace root | KANI-ACCESSOR-REF-001 | **NOT INTEGRATED** |
| `kani-harnesses/vb_compile_constant.rs` | workspace root | KANI-CONSTANT-POOL-001 | **NOT INTEGRATED** |
| `kani-harnesses/vb_compile_node_dedup.rs` | workspace root | INV-007-NODEDUP-001 | **NOT INTEGRATED** |
| `crates/vb_compile/src/expression_bytecode.rs` (tests) | crate | UNIT-EXPR-BYTESTACK-001, UNIT-ACCESSOR-REF-001, ERR-TAXONOMY-001 | EXISTS |
| `crates/vb_compile/src/lib.rs` (tests) | crate | UNIT-SLOT-COMPILER-001, UNIT-BUILD-PARTS-001, POST-009-VALIDATE-001 | EXISTS |
| `crates/vb_compile/src/lib.rs` (lower) | crate | UNIT-LOWER-DO-001, INV-006-ORDER-001 | EXISTS |

---

## 2. Critical Blockers (LETHAL)

### BLOCKER-1: `lower_slot_reference_for_testing` does not exist
**Severity**: LETHAL
**File**: `kani-harnesses/vb_compile_slot.rs` line 15
**Problem**: Harness imports `use vb_compile::lower_slot_reference_for_testing;` but no such function is exported from `vb_compile`. The function `lower_slot_reference` in `expression_bytecode.rs` is `fn` (private, not `pub` and not `pub(crate)`). This import will cause a **compile error** when the harness is integrated.

**Required fix**: Either:
(a) Export a test helper: `pub(crate) fn lower_slot_reference_for_testing(reference: &str, accessors: &mut Vec<AccessorProgram>) -> Result<ExprOp, CompileError>` that delegates to the private `lower_slot_reference`, OR
(b) Rewrite the harness to use `compile_expr_to_bytecode_with_accessors` on a `ParsedExpression::Reference` parsed from the slot string, which is public.

### BLOCKER-2: `kani-harnesses/` directory is not integrated into `vb_compile` crate
**Severity**: LETHAL
**Problem**: The `kani-harnesses/*.rs` files at the workspace root are standalone files. They are not declared in any `#[cfg(kani)]` module within `crates/vb_compile/src/`. As a result, `cargo kani --package vb_compile --harness <name>` will fail with "could not find harness."

**Required fix**: Create `crates/vb_compile/src/kani/` directory and move the harness files there. Add `#[cfg(kani)] pub mod kani;` module declarations in `lib.rs` (following the pattern of `kani_idempotency_parity`), and declare each harness file within the module.

### BLOCKER-3: `scripts/rust-verification-gauntlet.sh` does not exist
**Severity**: LETHAL
**Problem**: Moon tasks `verify-fast`, `verify-standard`, `verify-deep`, `verify-proof` all reference `bash scripts/rust-verification-gauntlet.sh` which does not exist in the workspace. Any attempt to run `moon run :verify-fast` will fail.

**Required fix**: Either create the verification gauntlet script, or update the moon tasks to use a different verification command.

---

## 3. Major Issues

### MAJOR-1: `vb_compile_slot.rs` — second harness missing `#[kani::proof]`
**Severity**: MAJOR
**File**: `kani-harnesses/vb_compile_slot.rs` lines 116-147
**Problem**: `lower_slot_reference_with_path_creates_accessor` is missing `#[kani::proof]` attribute. Kani will not recognize it as a proof harness.

**Required fix**: Add `#[kani::proof]` before line 116.

### MAJOR-2: `vb_compile_slot.rs` — `while` loops need explicit unwind bounds
**Severity**: MAJOR
**File**: `kani-harnesses/vb_compile_slot.rs` lines 95-113
**Problem**: `lower_slot_reference_valid` contains `while i < edge_cases.len()` with `#[kani::unwind(6)]`. The `edge_cases.len() == 6`, which is at the unwind boundary. Kani may not fully explore all iterations.

**Required fix**: Use `kani::unwind(8)` or restructure to avoid loop over fixed array.

### MAJOR-3: `vb_compile_bytecode.rs` — Test 5 has logically unreachable assertion
**Severity**: MAJOR
**File**: `kani-harnesses/vb_compile_bytecode.rs` lines 118-131
**Problem**: The assertion `if count > MAX_EXPRESSION_OPS { kani::assert(bound_result.is_err()) }` is unreachable because the preceding `kani::assume(count <= MAX_EXPRESSION_OPS)` eliminates all states where `count > MAX_EXPRESSION_OPS`. The test provides **zero coverage** of the overflow path for op count.

**Required fix**: Restructure the test — do NOT use `kani::assume` to constrain the dimension you're testing. Instead:
```rust
let count = kani::any::<usize>();
// No assume here — test what happens when count > MAX_EXPRESSION_OPS
let ops_vec: Vec<ExprOp> = std::iter::repeat_with(|| load_ops[0]).take(count).collect();
let bound_result = check_expr_stack_bound(&ops_vec, MAX_EXPRESSION_STACK);
if count > MAX_EXPRESSION_OPS {
    kani::assert(bound_result.is_err(), "ops exceeding MAX_EXPRESSION_OPS should return Err");
}
```

### MAJOR-4: `vb_compile_bytecode.rs` — Test 6 parses expressions in loop, but the parse results are not verified for the `Err` path
**Severity**: MAJOR
**File**: `kani-harnesses/vb_compile_bytecode.rs` lines 155-183
**Problem**: The `while` loop in Test 7 parses expressions and checks they compile, but does not verify that all valid expressions within bounds actually succeed and that expressions that would overflow are rejected. The test is incomplete.

**Required fix**: Expand to cover `Err` paths from `compile_expr_to_bytecode` (e.g., helper arity errors, stack overflow errors from deeply nested expressions).

### MAJOR-5: `vb_compile_constant.rs` — prefill loop of 65535 iterations with `#[kani::unwind(10)]`
**Severity**: MAJOR
**File**: `kani-harnesses/vb_compile_constant.rs` lines 32-50
**Problem**: The loop `while i < 65535` will not fully execute with `#[kani::unwind(10)]`. Kani's unwind bound limits how many iterations of a loop it explores. 65535 >> 10, so Kani will not exhaustively verify the overflow path. The overflow test is **not exhaustive**.

**Required fix**: Use a symbolic approach instead of filling concretely. Either:
(a) Pre-fill the compiler using `std::iter::repeat_n` with a concrete count, then verify the NEXT call fails, or
(b) Use `kani::any::<u16>()` for the index and verify `push_constant` returns `Err` when the pool is already at capacity.

### MAJOR-6: `vb_compile_node_dedup.rs` — `lower_steps_to_ir` takes 9 arguments but harness may not match signature
**Severity**: MAJOR
**File**: `kani-harnesses/vb_compile_node_dedup.rs` lines 74-83, 124-133, 158-167
**Problem**: Need to verify the harness arguments to `lower_steps_to_ir` match the actual function signature: `(nodes: Vec<CompiledNode>, expressions: Vec<ExprProgram>, accessors: Vec<AccessorProgram>, constants: Vec<ConstValue>, slot_count: u16, symbols_count: u32, name: &str, digest: WorkflowDigest)`. The harness passes `WorkflowDigest::from_bytes([0u8; 32])` which exists, but need to confirm all types are correct.

**Verification**: Function signature confirmed correct at `crates/vb_compile/src/lib.rs:261`. The harness calls are structurally correct.

---

## 4. Contract-Verification Review Cross-Check

### Coverage Analysis

All 17 proof obligations have corresponding entries in the traceability matrix.

| Obligation ID | Risk | Required | Layer | Status |
|---|---|---|---|---|
| VERUS-EXPR-STACK-001 | proof | true | verus | blocked_tooling (WAIVED) |
| VERUS-SLOT-MAX-001 | proof | true | verus | blocked_tooling (WAIVED) |
| KANI-EXPR-BYTECODE-001 | high | true | kani | planned (BLOCKED by INTEGRATION) |
| KANI-ACCESSOR-REF-001 | high | true | kani | planned (BLOCKED by INTEGRATION) |
| KANI-SLOT-REF-001 | high | true | kani | planned (BLOCKED by INTEGRATION) |
| KANI-CONSTANT-POOL-001 | high | true | kani | planned (BLOCKED by INTEGRATION) |
| INV-007-NODEDUP-001 | high | false | kani | planned (BLOCKED by INTEGRATION) |
| UNIT-EXPR-BYTESTACK-001 | medium | true | proptest | READY |
| UNIT-SLOT-COMPILER-001 | medium | true | proptest | READY |
| UNIT-ACCESSOR-REF-001 | medium | true | proptest | READY |
| ERR-TAXONOMY-001 | medium | true | proptest | READY |
| UNIT-LOWER-DO-001 | medium | true | proptest | READY |
| UNIT-BUILD-PARTS-001 | medium | true | proptest | READY |
| POST-009-VALIDATE-001 | medium | true | proptest | READY |
| INV-006-ORDER-001 | low | false | proptest | READY |
| STATIC-LINT-001 | medium | true | static-scan | READY |
| GATE-VERIFY-FAST-001 | high | true | gauntlet | DEFERRED (state 12) |

### Waiver Review

| Waiver | Owner | Reason | Evidence | Expiry | Valid? |
|---|---|---|---|---|---|
| WAIVER-VERUS-EXPR-STACK | proof-planner | Verus not installed | KANI-EXPR-BYTECODE-001 + UNIT-EXPR-BYTESTACK-001 | Until Verus in CI | **YES** |
| WAIVER-VERUS-SLOT-MAX | proof-planner | Verus not installed | KANI-SLOT-REF-001 + UNIT-SLOT-COMPILER-001 | Until Verus in CI | **YES** |

Both waivers are valid per the skill rules.

### Layer Fit Review

- **VERUS obligations**: WAIVED — Verus not installed; compensating Kani + proptest evidence is adequate.
- **Kani obligations**: Correct layer for bounded model checking of slot indices (u16 exhaust), constant pool (u16::MAX exhaust), bytecode stack bounds. Appropriate.
- **Unit test obligations**: Correct layer for deterministic data structure properties. Appropriate.
- **STATIC-LINT-001**: Correct layer for source-level linting. Command targets production code. Appropriate.

---

## 5. Findings Summary

| ID | Severity | Category | Description |
|---|---|---|---|
| F-001 | LETHAL | integration | `lower_slot_reference_for_testing` does not exist |
| F-002 | LETHAL | integration | `kani-harnesses/` not integrated into `vb_compile` crate |
| F-003 | LETHAL | infrastructure | `scripts/rust-verification-gauntlet.sh` does not exist |
| F-004 | MAJOR | harness-bug | `vb_compile_slot.rs`: missing `#[kani::proof]` on second harness |
| F-005 | MAJOR | harness-bug | `vb_compile_slot.rs`: while loop at unwind boundary |
| F-006 | MAJOR | harness-bug | `vb_compile_bytecode.rs`: Test 5 has unreachable assertion |
| F-007 | MAJOR | harness-bug | `vb_compile_bytecode.rs`: Test 7 incomplete overflow coverage |
| F-008 | MAJOR | harness-bug | `vb_compile_constant.rs`: 65535-iteration prefill with unwind(10) |

---

## 6. Required Repairs (before re-review)

1. **F-001 + F-002**: Integrate `kani-harnesses/` into `crates/vb_compile/src/kani/`. Add `#[cfg(kani)] pub mod kani;` in `lib.rs`. Export `lower_slot_reference_for_testing` as `pub(crate)` from `expression_bytecode.rs`.

2. **F-003**: Create `scripts/rust-verification-gauntlet.sh` or update moon tasks to use an existing verification command.

3. **F-004**: Add `#[kani::proof]` to `lower_slot_reference_with_path_creates_accessor`.

4. **F-005**: Increase unwind to `#[kani::unwind(8)]` or restructure to avoid loop.

5. **F-006**: Remove `kani::assume` before the dimension being tested. Test `count > MAX_EXPRESSION_OPS` path directly without assume.

6. **F-007**: Add explicit overflow/underflow test cases for `compile_expr_to_bytecode` that verify `Err` paths.

7. **F-008**: Use symbolic approach for overflow test — do not fill 65535 entries concretely.

---

## 7. Positive Findings

- Contract is comprehensive: all clauses (PRE-001–PRE-005, POST-001–POST-009, INV-001–INV-007, ERR-* taxonomy) are covered.
- All 17 proof obligations have valid JSONL entries with complete fields.
- Traceability matrix fully traces every clause to obligations and back.
- Verus waivers are valid and properly documented.
- TLA+ non-applicability rationale is correct and well-argued.
- Lean theorem non-applicability rationale is correct.
- Unit test infrastructure exists and is properly located in the crate.
- `lower_steps_to_ir` is public and harness calls are correctly structured.
- Harness `#![forbid(unsafe_code)]` is present.
- `WorkflowDigest::from_bytes` exists and is used correctly.
- `MAX_EXPRESSION_STACK = 64` and `MAX_EXPRESSION_OPS = 256` are well within Kani exhaust scope for the intended tests.
