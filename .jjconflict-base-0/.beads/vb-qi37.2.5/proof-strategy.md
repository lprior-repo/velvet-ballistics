# Proof Strategy — vb-qi37.2.5

## Bead Identity
- **Bead**: vb-qi37.2.5
- **Title**: quality: Boundedness adversarial tests
- **State**: 4 (Proof Planning)
- **Focus**: Boundedness — budget enforcement, value arena caps, adversarial test generation

---

## Scope

**In scope**: `vb_core` crate — budget computation, limits, value store arena, step budget, run loop.
**Out of scope**: `vb_runtime` (DEFERRED_GLOBAL — chunk_001.rs build failure), `velvet_ballastics` integration (owned by test loop), fuzz infrastructure (pre-existing).

---

## Boundedness Risk Summary

The core boundedness risks are:
1. `StepBudget` counter under-/overflow through `try_take` and `new`
2. `ValueStore` arena cap enforcement — inserts beyond `max_arena_entries` must return `CoreError::BudgetExceeded`
3. `run_until_blocked` loop must terminate within available budget — no infinite loop with budget remaining
4. `count_total_steps` u64 accumulator must not overflow; must return `WorkflowError` instead
5. `WholeWorkflowBudget::compute` must reject overflow in any budget dimension

These are Rust-local invariants and API contracts — no temporal/concurrent behavior.

---

## Verifier Lane Strategy

### Lane 1 — Verus (Primary Proof)

**Risk trigger**: Rust-local invariant, pure function correctness, loop termination
**Obligations**: VERUS-INV-001, VERUS-INV-002, VERUS-INV-003, VERUS-INV-004, VERUS-INV-005, VERUS-INV-006
**Strategy**: Verify the kernel invariants in the trusted core without I/O, async, or storage. Use spec functions and proof lemmas for loop termination, monotonic decrease, and cap enforcement.

| Obligation | Target | Spec/Proof |
|-----------|--------|------------|
| VERUS-INV-001 | `signals.rs::StepBudget` | `spec_step_budget_invariant`: remaining ≤ MAX_STEP_BUDGET always |
| VERUS-INV-002 | `value_store.rs::ValueStore` | `spec_value_store_cap`: total_arena_count ≤ max_arena_entries always |
| VERUS-INV-003 | `budget.rs::count_total_steps` | `spec_count_total_steps_bounded`: result ≤ MAX_STEPS_PER_WORKFLOW or error |
| VERUS-INV-004 | `run_loop.rs::run_until_blocked` | `spec_run_until_blocked_terminates`: loop iterations ≤ initial_budget |
| VERUS-INV-005 | `budget.rs::WholeWorkflowBudget` | `spec_budget_non_decreasing`: fields monotonic across compute calls |
| VERUS-INV-006 | `signals.rs::StepBudget::try_take` | `spec_try_take_decreases`: remaining decreases by 1 on Ok(true), unchanged on Ok(false) |

**Waiver target**: None — Verus is the primary lane for these invariants; no cheaper equivalent.

---

### Lane 2 — Kani (Bounded Model Checking)

**Risk trigger**: Bounded state machine, arithmetic/index bounds, finite transition system
**Obligations**: KANI-INV-001, KANI-INV-004, KANI-POST-004
**Strategy**: Concrete harnesses with bounded unrolling. Kani checks that the API surface cannot produce a counterexample to boundedness under all inputs within honest bounds.

| Obligation | Harness | Key Property |
|-----------|---------|--------------|
| KANI-INV-001 | `step_budget_kani` | `StepBudget::try_take` never panics/overflows through normal API |
| KANI-INV-004 | `run_until_blocked_kani` | `run_until_blocked` terminates for all inputs within budget bounds |
| KANI-POST-004 | `value_store_cap_kani` | ValueStore inserts return `BudgetExceeded` before cap is exceeded |

**Commands**:
- `cargo kani --package vb_core --harness step_budget_kani`
- `cargo kani --package vb_core --harness run_until_blocked_kani`
- `cargo kani --package vb_core --harness value_store_cap_kani`

**Assumptions**:
- `StepBudget::new` input bounded to `u64::MAX` (honest API use)
- `ValueStore::with_max_slots` cap bounded to `u16::MAX` (honest construction)
- `run_until_blocked` workflow and budget inputs within reasonable bounds

---

### Lane 3 — Miri (Undefined Behavior)

**Risk trigger**: Unsafe Rust, raw pointer handling in arena, interior mutability
**Obligations**: MIRI-INV-002
**Strategy**: Run ValueStore insert operations under Miri to detect UB, use-after-free, leaks, or double-free. StepBudget `saturating_sub` also covered.

**Command**: `cargo miri test --package vb_core -- value_store`
**Scope focus**: `ValueStore::insert_list`, `ValueStore::insert_object`, `ValueStore::insert_symbol`, `ValueStore::insert_blob`, and `check_arena_cap`

**Assumptions**:
- Miri can execute the full `vb_core` test suite on this platform
- `cfg(miri)` test variants are enabled

**Blocked**: If Miri is not available in the current toolchain, record `blocked_tooling`.

---

### Lane 4 — Proptest (Adversarial Property Tests)

**Risk trigger**: Broad input space, domain invariants over generated values
**Obligations**: PROPTEST-PRE-001, PROPTEST-POST-001, PROPTEST-PRE-002, PROPTEST-POST-006
**Strategy**: Generate adversarial inputs to stress budget clamping, try_take count behavior, and ValueStore cap enforcement across random sequences.

| Obligation | Property |
|-----------|---------|
| PROPTEST-PRE-001 | `StepBudget::new(v).remaining == min(v, MAX_STEP_BUDGET)` for all u64 v |
| PROPTEST-POST-001 | `try_take` returns Ok(true) exactly `min(n, initial)` times when called n times |
| PROPTEST-PRE-002 | ValueStore inserts return `BudgetExceeded` when total_arena_count >= max_arena_entries |
| PROPTEST-POST-006 | `BoundednessPolicy::validate` returns Ok for budgets within policy limits |

**Commands**:
- `cargo test --package vb_core -- property_step_budget_new_clamp`
- `cargo test --package vb_core -- property_try_take_count`
- `cargo test --package vb_core -- property_value_store_cap`
- `cargo test --package vb_core -- property_boundedness_policy`

**Assumptions**: Proptest is available and the existing proptest framework in `crates/vb_core` is compatible.

---

### Lane 5 — Fuzz (Adversarial Input Boundary)

**Risk trigger**: Untrusted/crash input boundary, security-critical robustness
**Obligations**: FUZZ-001
**Strategy**: Run existing fuzz target `fuzz_resource_budget` with increased runs to catch any panic on clamping boundary.

**Command**: `cargo fuzz run step_budget_new -- -runs=10000` (or the equivalent existing fuzz target)
**Evidence**: Fuzz target completes 10_000 runs without panic or sanitizer failure

**Assumptions**: `cargo-fuzz` is set up in the workspace; the `step_budget_new` or equivalent fuzz target exists and is compilable.

---

### Lane 6 — Unit Tests (Compile-time Exhaustion)

**Risk trigger**: Deterministic behavior coverage for key postconditions
**Obligations**: UNIT-POST-003, UNIT-POST-005
**Strategy**: Verify that `run_until_blocked` returns `StepBudgetExhausted` signal and that `WholeWorkflowBudget::compute` propagates overflow errors.

**Commands**:
- `cargo test --package vb_core -- run_until_blocked`
- `cargo test --package vb_core -- test_step_count_overflow`

---

## TLA+ Applicability

**NOT APPLICABLE** — `run_until_blocked` is a single-threaded deterministic loop with a hard iteration bound (`budget.remaining`). No concurrent actors, message passing, eventual liveness, or deadlock concerns. Termination is proven by the Verus loop invariant.

Waiver rationale recorded in `verification-layers.md`.

---

## Obligation-to-Verifier Mapping Summary

| ID | Verifier | Command | Key Assumption | owner_state | rerun_from |
|----|----------|---------|---------------|-------------|------------|
| VERUS-INV-001 | verus | `verus crates/vb_core/src/engine/signals.rs` | new clamps, try_take decreases by 1 | 5 | 5 |
| VERUS-INV-002 | verus | `verus crates/vb_core/src/value_store.rs` | cap set once at construction | 5 | 5 |
| VERUS-INV-003 | verus | `verus crates/vb_core/src/budget.rs` | count_total_steps uses u64 accumulator | 5 | 5 |
| VERUS-INV-004 | verus | `verus crates/vb_core/src/engine/run_loop.rs` | budget.try_take decreases by 1 on Ok(true) | 5 | 5 |
| VERUS-INV-005 | verus | `verus crates/vb_core/src/budget.rs` | WholeWorkflowBudget::compute sole constructor | 5 | 5 |
| VERUS-INV-006 | verus | `verus crates/vb_core/src/engine/signals.rs` | remaining private, only try_take mutates | 5 | 5 |
| KANI-INV-001 | cargo kani | `cargo kani --package vb_core --harness step_budget_kani` | honest u64 input bounds | 6 | 5 |
| KANI-INV-004 | cargo kani | `cargo kani --package vb_core --harness run_until_blocked_kani` | workflow and budget within reasonable bounds | 6 | 5 |
| KANI-POST-004 | cargo kani | `cargo kani --package vb_core --harness value_store_cap_kani` | ValueStore constructed with honest cap | 6 | 5 |
| MIRI-INV-002 | cargo miri test | `cargo miri test --package vb_core -- value_store` | Miri available and tests enabled | 6 | 5 |
| PROPTEST-PRE-001 | cargo test | `cargo test --package vb_core -- property_step_budget_new_clamp` | proptest framework available | 8 | 7 |
| PROPTEST-POST-001 | cargo test | `cargo test --package vb_core -- property_try_take_count` | proptest framework available | 8 | 7 |
| PROPTEST-PRE-002 | cargo test | `cargo test --package vb_core -- property_value_store_cap` | proptest framework available | 8 | 7 |
| PROPTEST-POST-006 | cargo test | `cargo test --package vb_core -- property_boundedness_policy` | proptest framework available | 8 | 7 |
| FUZZ-001 | cargo fuzz run | `cargo fuzz run step_budget_new -- -runs=10000` | cargo-fuzz set up, target exists | 8 | 7 |
| UNIT-POST-003 | cargo test | `cargo test --package vb_core -- run_until_blocked` | tests compile and pass | 8 | 7 |
| UNIT-POST-005 | cargo test | `cargo test --package vb_core -- test_step_count_overflow` | tests compile and pass | 8 | 7 |

---

## Deferred / Waived Lanes

| Lane | Decision | Reason |
|------|----------|--------|
| TLA+ | waived | Single-threaded deterministic loop; Verus INV-004 loop invariant proves termination; no temporal/concurrent/actor behavior in scope |
| Flux | not_applicable | No refinement-type predicates requiring Flux; Verus covers all type-state concerns |
| Loom | not_applicable | No concurrent actors, threads, atomics, or channels in scope |

---

## vb_runtime DEFERRED_GLOBAL

`vb_runtime` chunk_001.rs build failure is **outside this bead's scope**. It does not block any obligation in this bead. Documented in `codebase-map.md` and `STATE.md`.
