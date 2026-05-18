# FINAL BLACK-HAT REVIEW: velvet-ballistics Test Suite

**Reviewer:** Black Hat Reviewer Agent
**Date:** 2026-05-17
**Scope:** vb_validate::type_taint, vb_expr::eval, vb_runtime::runtime (tick_shard), vb_runtime::action_queue, vb_expr::property_tests

---

## VERDICT: **CONDITIONAL APPROVAL — REWRITE MANDATORY** (3 critical findings)

---

## PHASE 1: CONTRACT PARITY

### ✅ type_taint.rs — PASS
- 3-level lattice correctly implemented: `Clean < DerivedFromSecret < Secret` (lines 52-75)
- `Taint::merge()` follows lattice join-semilattice rules
- `validate_resource_limits()` enforces all 16 resource bounds against protocol hard limits
- `validate_taint()` explicitly notes non-rejection of Secret in Finish (line 527-529) — matches spec

### ⚠️ eval.rs — MINOR CONCERN
- AND/OR are **non-short-circuit** by design (constant folding requirement). Lines 161-170 evaluate both operands unconditionally.
- **CONCERN:** The comment header at line 1 says "Bounded stack-based expression bytecode evaluator" but nowhere documents the non-short-circuit semantics. A future developer could "optimize" to short-circuit and break constant folding.

### ✅ runtime.rs tick_shard — PASS (with caveats)
- ShardDirective::Continue/Suspend/Migrate/Shutdown all handled
- Self-migration check at line 240
- Source/target validation before migration

### ✅ action_queue.rs — PASS
- VecDeque with bounded capacity enforcement
- Backpressure at 80% threshold (line 109-118)
- Result-based error types (ActionQueueError::QueueFull, InvalidCapacity)

### ✅ property_tests — PASS
- Arithmetic overflow: AO-1 through AO-8 covered with proptest
- Constant folding: CF-1 through CF-25 covered
- Eval bounds: BE-1 through BE-12 covered

---

## PHASE 2: FARLEY ENGINEERING RIGOR

### 🚨 CRITICAL: runtime.rs tick_shard — REDUNDANT UNWRAPS (Lines 251, 261, 268)

```rust
// Line 251 — SHARD VALIDATION PERFORMED AT LINE 221, THEN REPEATED
let shard = self.shards.get_mut(shard_index_usize).unwrap(); // UNWRAP #1

// Line 261 — TARGET VALIDATED AT LINE 245
let target_shard = self.shards.get_mut(target_usize).unwrap(); // UNWRAP #2

// Line 268
let shard = self.shards.get_mut(shard_index_usize).unwrap(); // UNWRAP #3
```

**VIOLATION:** The code validates shard existence at lines 221-223 and 245-247 with explicit error returns. Then immediately uses `.unwrap()` which will **panic** if the validation was somehow wrong. This is a panic vector disguised as defensive coding.

**FIX:** Replace `.unwrap()` with the already-validated references, or remove the redundant checks.

### ⚠️ CONCERN: eval.rs — `eval_expr_program` creates ValueStore on every call (lines 27-34)

```rust
pub fn eval_expr_program(...) -> ExprResult<SlotValue> {
    let mut store = ValueStore::new();  // ALLOCATION on hot path
    eval_expr_program_with_store(program, slots, constants, &mut store)
}
```

This function creates a **new arena** on every invocation. The comment at lines 19-26 admits this: "For performance-critical code that already has a ValueStore, prefer `eval_expr_program_with_store`."

**FARLEY RULE VIOLATION:** I/O (heap allocation) hidden inside what appears to be pure calculation. The hot path `eval_expr_program` should have a zero-allocation variant that doesn't create a throwaway arena.

### ✅ action_queue.rs — CLEAN
- `BoundedActionCompletionQueue::new()` validates capacity > 0
- No functions exceed 25 lines
- No functions exceed 5 parameters

### ✅ type_taint.rs — CLEAN
- All functions under 25 lines
- Clean separation: build_facts() → validate_step_types/taint()

---

## PHASE 3: HOLZMAN RUST (The Big 6)

### ✅ ALL FILES — PASS

1. **Make illegal states unrepresentable:**
   - `Taint` enum (Clean/DerivedFromSecret/Secret) — exhaustive
   - `ActionQueueError` enum (QueueFull/InvalidCapacity) — exhaustive
   - `ValueType` enum — exhaustive

2. **Parse, Don't Validate:**
   - `Taint::merge()` returns validated lattice join, not raw markers
   - `ResourceLimits::default()` provides safe defaults
   - `BoundedActionCompletionQueue::new()` rejects zero capacity at construction

3. **Types as Documentation:**
   - No boolean parameters found
   - `ShardDirective` enum documents intent: Continue/Suspend/Migrate/Shutdown

4. **Workflows as explicit state transitions:**
   - `StepKind` enum: Save/Choose/Finish — workflow steps as types
   - `validate_step_taint()` tracks taint through Save→slot, Choose→condition, Finish→result

5. **Newtypes:**
   - `ValueFact` wraps `(ValueType, Taint)` as an atomic unit
   - `ResourceLimits` newtype over primitive limits
   - `BackpressureWarning` newtype over depth/capacity pair

### ⚠️ eval.rs — Taint propagation not tracked for expression results

The expression evaluator operates on `SlotValue` but has no taint tracking. If expressions can produce secret-derived values, those would be untainted in the slot store. This is **outside the scope of eval.rs** (it's a type-level evaluator, not a taint checker), but the contract should be documented.

---

## PHASE 4: RUTHLESS SIMPLICITY & DDD

### 🚨 CRITICAL: eval.rs — DUPLICATED HELPER ERROR PATTERNS (Lines 540-677)

Every `eval_helper_*` function that requires a ValueStore has a **twin** error function that returns "value-store context required" without the store:

```rust
// WITH store (correct implementation):
fn eval_helper_length_with_store(...) // lines 688-720

// WITHOUT store (always errors):
fn eval_helper_length(...) // lines 545-561 — returns TypeMismatch
```

**CUPID VIOLATION:** This is a "abstract trait with one implementer" antipattern split into two functions. The non-store version is **dead code** that always errors. A developer calling `eval_helper_length` directly (bypassing the store-aware path) would get a misleading error.

**YAGNI VIOLATION:** The non-store helpers are untestable dead code. Delete them or make them panic with a clear message.

### ⚠️ type_taint.rs — `reference_name()` IS DEAD CODE (Lines 478-483)

```rust
fn reference_name(tail: &str) -> &str {
    match tail.split_once('.') {
        Some((name, _)) => name,
        None => tail,
    }
}
```

This function is defined but **never called**. It appears to be scaffolding for a feature that was planned but never implemented.

### ✅ action_queue.rs — CLEAN
- No Option-based state machines
- FIFO queue is trivially understandable
- Backpressure warning is a single clear concern

### ✅ runtime.rs tick_shard — CLEAN LOGIC, MURKY STRUCTURE
The `tick_shard` function handles 5 directive types with clear separation. Logic is sound.

---

## PHASE 5: BITTER TRUTH (Velocity & Legibility)

### 🚨 CRITICAL: runtime.rs — `tick_shard` EXCEEDS 25 LINE LIMIT

**Lines 216-286 = 70 lines** for a single function.

```rust
pub fn tick_shard(&mut self, shard_index: u32, directive: ShardDirective) -> RuntimeResult<bool> {
    let shard_index_usize = usize::try_from(shard_index)
        .map_err(|_| RuntimeError::ShardNotFound { shard: shard_index })?;

    // Validate source shard exists first
    if self.shards.get(shard_index_usize).is_none() {       // <-- LINE 221
        return Err(RuntimeError::ShardNotFound { shard: shard_index });
    }

    match directive {
        ShardDirective::Continue => { ... },                 // <-- 5 lines
        ShardDirective::Suspend => { ... },                  // <-- 3 lines
        ShardDirective::Migrate { target } => { ... },      // <-- 35 lines
        ShardDirective::Shutdown => { ... },                 // <-- 4 lines
        ShardDirective::Cancel | ShardDirective::Barrier => { ... }, // <-- 5 lines
    }
}
```

**BITTER TRUTH:** The Migrate arm (lines 235-270) is itself ~35 lines. Extract it to `migrate_shard()`.

### 🚨 CRITICAL: eval.rs — HANDLE-BASED STORES VIOLATE "PAINFULLY OBVIOUS"

Lines 693-714, 728-745, 756-768: Every store-aware helper follows this pattern:

```rust
SlotValue::Symbol(id) => {
    let s = store.symbol(id).map_err(|_| ExprError::InvalidReference { ... })?;
    s.len()
}
```

**PROBLEM:** `store.symbol(id)?` is opaque. The error message `InvalidReference { reference: format!("symbol:{id:?}") }` is **meaningless** to a developer debugging a failing workflow. What is `id`? What symbol? There is no context.

**BITTER TRUTH:** The phrase "PAINFULLY OBVIOUS" is violated. A developer seeing `InvalidReference { reference: "symbol:SymbolId(...)" }` at 3am during an outage has learned nothing.

### ⚠️ action_queue.rs — HARD-CODED 80% THRESHOLD (Line 109-118)

```rust
let threshold = (self.capacity * 8) / 10;  // 80%
```

This magic number (8/10 = 80%) is documented but not configurable. If a user wants 90% backpressure, they cannot configure it. However, this is a **reasonable default** and the 80% value is industry-standard. Not a rejection.

### ✅ property_tests — WELL STRUCTURED
- Clear section headers with test plan references (AO-1, CF-1, BE-1)
- Each test has a single clear assertion
- Property tests use `proptest::prelude::*` correctly

### ⚠️ eval.rs constant_folding — SOME TESTS USE `Just(())` NOT `any::<()>`

Lines 18-21 in `arithmetic_overflow.rs`:
```rust
#[test]
fn ao_add_i64_max_plus_one_returns_overflow(_unit in Just(())) {
```

This **does not test** any arbitrary input — it only tests the single edge case `i64::MAX + 1`. While this is correct for this specific test (you want to test exactly `i64::MAX`), the naming is misleading. The test is labeled as a property test but only has one sample.

**However:** For edge case tests, `Just(())` is appropriate and deterministic. Not a rejection — just noting the pattern.

---

## SUMMARY OF FINDINGS

| Severity | File | Lines | Issue | Fix Required |
|----------|------|-------|-------|--------------|
| **CRITICAL** | runtime.rs | 251, 261, 268 | Redundant `.unwrap()` after validated access | Replace with safe references |
| **CRITICAL** | runtime.rs | 216-286 | 70-line function (exceeds 25-line limit) | Extract `migrate_shard()` |
| **CRITICAL** | eval.rs | 540-677 | Dead twin error functions for helpers | Delete or make panic-on-use |
| **CONCERN** | eval.rs | 27-34 | I/O hidden in pure calculation path | Document allocation cost |
| **CONCERN** | eval.rs | 693-714 | Opaque error messages lack context | Enrich error with symbol name |
| **MINOR** | type_taint.rs | 478-483 | `reference_name()` dead code | Delete function |
| **MINOR** | eval.rs | lines 1-1045 | Non-short-circuit AND/OR undocumented | Add doc comment |

---

## MANDATED FIXES (Required before approval)

1. **runtime.rs tick_shard**: Replace lines 251, 261, 268 `.unwrap()` calls with already-validated references extracted from the earlier `get_mut()` calls.

2. **runtime.rs tick_shard**: Extract `ShardDirective::Migrate` arm into `fn migrate_shard(&mut self, source: usize, target: u32) -> RuntimeResult<bool>`.

3. **eval.rs**: Delete the dead twin helper functions (eval_helper_exists, eval_helper_length, eval_helper_empty, eval_helper_count, eval_helper_unique, eval_helper_contains, eval_helper_starts_with, eval_helper_ends_with, eval_helper_has, eval_helper_append, eval_helper_append_if, eval_helper_merge, eval_helper_sum) or make them `unimplemented!()` with a comment that they require a ValueStore.

4. **eval.rs**: Add doc comment explaining that AND/OR are non-short-circuit by design to support constant folding.

---

## PARTIAL CREDIT

- **type_taint.rs**: APPROVED — Correct 3-level lattice, correct taint merge, proper resource limit validation.
- **action_queue.rs**: APPROVED — Clean bounded queue with proper Result error types.
- **property_tests/**: APPROVED — Comprehensive coverage of arithmetic overflow, constant folding, and eval bounds.

---

**FINAL VERDICT: REJECT — REWRITE MANDATED**

The critical issues in runtime.rs and eval.rs violate Phase 2 (Farley — panic vectors) and Phase 4 (Ruthless Simplicity — dead code, overlong function). Fix the mandated items and resubmit.

---

*Black Hat Reviewer — Veloxide Assurance Division*
*Document ID: final-black-hat-review.md*
