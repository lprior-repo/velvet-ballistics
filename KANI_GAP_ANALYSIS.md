# Kani Proof Coverage Gap Analysis — Velvet-Ballistics

> Comprehensive audit of all 18 crates. Quality assessed against GOD RULE #1
> (no hardcoded structural inputs; must use `kani::any()` or `kani::Arbitrary`).

## Executive Summary

| Grade | Crates |
|-------|--------|
| **STRONG** | `vb_core`, `vb_runtime`, `vb_validate`, `vb_ipc`, `vb_compile`, `vb_benchmark`, `vb_verification` |
| **MIXED** | `vb_storage`, `vb_expr`, `vb_yaml`, `vb_proof_kernels`, `vb_cli` |
| **WEAK** | `vb_boundary_inventory` |
| **CRITICAL GAP (no Kani on critical functions)** | `vb_queue_semantics`, `vb_proof_kernels` (Budget/taint/step_state) |
| **LOW PRIORITY** | `vb_doc`, `vb_test_util` |

**Total Kani harness files found: ~260 across 14 crates.**
**Crates with zero Kani coverage: `vb_doc`, `vb_test_util` (low priority), `vb_queue_semantics` (critical gap). `vb_proof_kernels` has 7 harnesses but critical gaps in Budget/taint kernels (0/3 modules covered).**

---

## Per-Crate Detailed Assessment

### vb_core (18 crates — heaviest Kani coverage)
- **Kani files:** 70+ harnesses
- **Quality:** STRONG
- **Highlights:**
  - `kani_workflow_arbitrary.rs`: Gold-standard `kani::Arbitrary` impls for `WorkflowParts`, `RunFrame`, `CompiledNodeKind`, `ResourceContract`, `ActionContract`, `SlotValue`, `FiniteF64`, `ExprOp`
  - `kani_idempotency_gates.rs`: Symbolic contracts with `kani::assume()`, `kani::cover!`, proper bounds
  - `kani_taint.rs` / `kani_taint_propagation.rs` / `kani_taint_5var_laws_vbjpq733.rs`: Taint lattice law verification
  - `kani_budget_arithmetic_refinement.rs` / `kani_step_budget*.rs`: Budget arithmetic proofs
  - `kani_step_state_transition.rs`: Step state machine proofs
  - `kani_vbjpq733_proofs.rs`: Core proof harnesses
  - `verification/kani/`: `kani_parallel_in_flight`, `kani_idempotency_tracker`, `kani_retry_math`, `kani_resume_state_machine`, `kani_for_each_ordering`, `kani_together_ordering`, etc.
- **Gaps:** Minimal — this is the reference crate for Kani patterns.

### vb_runtime
- **Kani files:** ~20 harnesses
- **Quality:** STRONG
- **Highlights:**
  - `kani_shard_command_queue.rs`: Proper `kani::any()`, in-harness sequential model
  - `kani_shard_lifecycle.rs`: Full lifecycle state machine
  - `kani_cancel_kill_lattice.rs`: Lattice law verification
  - `kani_action_queue.rs`: Queue semantics
  - `kani_resource_contract_secret_enforcement.rs`: Security property
  - `verification/kani/`: `kadi_for_each_ordering`, `kani_resume_state_machine`, `kani_retry_math`, `kani_idempotency_tracker`, `kani_submit_frame_release`, `kani_attempt_fence_harnesses`, etc.
- **Gaps:** None significant.

### vb_storage
- **Kani files:** 60+ harnesses
- **Quality:** MIXED — volume is excellent, some quality issues
- **Issues:**
  - `kani_admission.rs`: **WEAK** — `minimal_valid_workflow()` hardcodes fixed `WorkflowParts`; `bounded_journal()` opens real filesystem DB (not pure model checking)
  - `kani_recovery_hydrate.rs`: Mixed — some hardcoded data, some symbolic
  - `kani_digest_checks_vb_2bzz.rs`: Moderate quality
- **Strengths:** Heavy MRWE-7 series proofs, VU8GI/WVCUF series proofs, strong digest determinism coverage.
- **Gaps:** Replace hardcoded `minimal_valid_workflow()` with `kani::any()` + `kani::Arbitrary`.

### vb_compile
- **Kani files:** 50+ harnesses
- **Quality:** STRONG (with one noted exception)
- **Issues:**
  - `kani_idempotency_parity.rs`: **ACCEPTABLE** — hardcoded `ActionContract` fields but intentional exhaustive enum-variant decision-table (5×3×3=45 combos). This is a design choice, not a violation.
- **Strengths:** Exhaustive digest determinism, foreach semantics, lowering proofs, resource contract proofs, byte code compilation proofs.
- **Gaps:** None significant.

### vb_validate
- **Kani files:** 3 harnesses
- **Quality:** STRONG
- **Highlights:**
  - `kani_gate_08_accessor.rs`: Full symbolic `WorkflowParts` inputs, proper `kani::assume()` bounds
  - `kani_gate_08_structural.rs`: Structural validation proofs
  - `kani_idempotency_contract.rs`: Idempotency gate proofs
- **Gaps:** Limited coverage breadth (only 3 harnesses) but quality is high.

### vb_expr
- **Kani files:** 7 harnesses (in `src/kani/` and `src/proofs/`)
- **Quality:** MIXED
- **Issues:**
  - `kani_expr_stack.rs`: **WEAK** — all hardcoded `ExprOp` arrays, no `kani::any()`, only one program path proven. Three fixed-size tests: `[LoadConst; 1]`, `[Add]` (underflow), `[LoadConst; 65]` (overflow). No symbolic inputs.
  - `proofs/f64_ops.rs`: GOOD — `kani::any()` inputs with proper bounds
  - `kani/vb_jpq7_35_arithmetic.rs`: GOOD — `kani::any()` for i64 range, proper bounds
  - `kani/vb_jpq7_35_bytecode_bound.rs`: GOOD
  - `kani/vb_jpq7_35_parser_depth.rs`: GOOD
  - `kani/vb_jpq7_35_stack.rs`: GOOD
  - `kani/vb_jpq7_35_token_bound.rs`: GOOD
- **Gaps:** `kani_expr_stack.rs` needs complete rewrite with `kani::any()` inputs. No Kani proofs for `eval_expr_program`, `parse_expr`, `compile_expr`.

### vb_ipc
- **Kani files:** 8 harnesses
- **Quality:** STRONG
- **Highlights:**
  - `kani_ipc_dispatch_exhaustive.rs`: `kani::any()` for command values, exhaustive dispatch arm verification
  - `kani_ipc_header.rs`: `kani::any()` with proper bounds
  - `kani_ipc_command_exhaustive.rs`: Command enum exhaustion
  - `kani_ipc_decode_order.rs`: Decode order verification
  - `kani_ipc_header_rejects_oversize.rs`: Overflow rejection
- **Gaps:** None significant.

### vb_yaml
- **Kani files:** 4 harnesses
- **Quality:** MIXED
- **Issues:**
  - `kani_is_primitive_legacy.rs`: **MIXED** — some hardcoded string tests, one arbitrary-string harness with tautology assertion
- **Strengths:** `kani_panic_freedom.rs` — panic-freedom proofs, `kani_all_variants_registered.rs` — enum exhaustiveness
- **Gaps:** Hardcoded test cases should be replaced with `kani::any()` symbolic inputs.

### vb_benchmark
- **Kani files:** 3+ harnesses
- **Quality:** STRONG
- **Highlights:**
  - `kani_gate.rs`: Proper symbolic inputs with `kani::any()` for `BenchmarkMetadata`, exhaustive zero-latency field testing
  - `kani_enum.rs`: Enum exhaustiveness proofs
  - `kani_capture.rs`: Data capture proofs
- **Gaps:** None significant.

### vb_proof_kernels
- **Kani files:** 7 harnesses total (6 in `profile_contract/kani/` + 1 in `step_state.rs`)
- **Quality:** STRONG (profile_contract), WEAK (step_state), **CRITICAL GAP** (Budget/taint)
- **Existing Kani:**
  - `profile_contract/kani/` (6 harnesses): GOOD quality — profile contract verification
  - `step_state.rs` (1 harness, lines 562-617): WEAK quality — manual `kani::any()` with modulo mapping (`s_raw2 % 8`), not using `kani::Arbitrary`. Proves terminal-state absorbing property only.
- **Issues — CRITICAL GAPS:**
  - **`resource_budget.rs`**: ZERO Kani proofs. `Budget::sequential_add()`, `Budget::branch_max()`, `Budget::loop_mul()`, `sequential_compose()`, `branch_compose()`, `loop_compose()`, `Policy::within()` — ALL unproven. These are pure, sequential, saturating arithmetic kernels — PERFECT candidates for Kani. 12 fields × 3 operations = 36 distinct computational paths.
  - **`taint.rs`**: ZERO Kani proofs. Taint lattice operations unverified. 3-element lattice with `join_taint`, `join_many`, `all_lattice_laws` — trivially bounded, ideal for exhaustive Kani proof.
  - `vb_kyyf_normalization.rs`: NO Kani proofs.
  - `envelope_header.rs`: NO Kani proofs.
- **Strengths:** `profile_contract/kani/` has 6 harnesses for profile contract verification.
- **Gaps:** CRITICAL — Budget arithmetic and taint lattice are both pure, tiny, bounded, sequential kernels. These are the IDEAL targets for Kani — zero I/O, zero async, zero unsafe.

### vb_queue_semantics
- **Kani files:** ZERO
- **Quality:** N/A — no Kani at all
- **Issues — CRITICAL GAP:**
  - `QueueState::new()`, `QueueState::from_vec_deque()` — capacity validation proofs
  - `action_enqueue_transition()`, `action_dequeue_transition()`, `command_enqueue_transition()`, `command_pop_transition()` — state transition proofs
  - `shard_tick_transition()` — tick semantics proofs
  - `enqueue_decision()`, `command_pop_transition_decision()`, `shard_tick_transition_decision()` — zero-allocation decision functions
  - `warning_payload()`, `warning_threshold()` — warning threshold proofs
  - `validate_capacity()`, `helper_valid_capacity()`, `helper_queue_is_full()`, `helper_enqueue_accepts()` — helper function proofs
  - `QueueStateRejection` — rejection invariants
  - `SHARED_QUEUE_CAPACITY_MAX` constant (65536) — boundary proofs
- **Gaps:** CRITICAL — this is a pure, sequential, dependency-free queue state machine. It's the PERFECT candidate for Kani proofs. 432 lines, zero unsafety, all pure functions.

### vb_cli
- **Kani files:** 2 modules (3 harnesses in lifecycle + 18 in agent_context)
- **Quality:** MIXED
- **Highlights:**
  - `agent_context/tests/kani_harnesses.rs`: **GOOD** — calls `build()` with `kani::any()` inputs, checks structural invariants (field presence, type correctness, determinism, size bounds, serialization roundtrip). 18 obligations (OBL-001 through OBL-018).
  - `kani_lifecycle.rs`: **WEAK** — does NOT call production `cancel()`/`complete()` functions. Instead, it SIMULATES the state machine internally using `kani::any()` + match expressions. This proves internal simulation logic, not production code behavior.
- **Gaps:** 162 files — most are I/O-heavy (CLI commands, naming scan, workflow helpers). Only computational functions need Kani. Assessment needed for:
  - `naming_scan/classify.rs` — classification logic
  - `naming_scan/ordering.rs` — ordering logic
  - `commands_workflow/simulate.rs` — simulation logic

### vb_doc
- **Kani files:** ZERO
- **Quality:** N/A — no Kani, not critical
- **Functions:** Evidence reconciliation, vocabulary management, workspace reconciliation. These are doc/reporting tools — low verification priority.
- **Gaps:** LOW PRIORITY — not computation-critical.

### vb_test_util
- **Kani files:** ZERO
- **Quality:** N/A — not critical
- **Gaps:** LOW PRIORITY — test utility crate.

### vb_verification
- **Kani files:** 3 harnesses (in single-file crate)
- **Quality:** STRONG (for its scope)
- **Highlights:**
  - `hydrate_run_frame_precond_run_id_mismatch`: Proper symbolic inputs, `kani::assume()` for precondition
  - `hydrate_run_frame_from_events_precond_empty`: Empty events proof
  - `hydrate_run_frame_postcond_ok`: Valid input non-panic proof
  - `ArbitraryRunSnapshot`: Custom `kani::Arbitrary` implementation for `VbRunSnapshot`
- **Gaps:** None significant for its scope.

### vb_boundary_inventory
- **Kani files:** 1 harness
- **Quality:** WEAK
- **Issues:**
  - `kani_harnesses.rs`: Trivial `FieldState`/`FreshnessMarker` tests, hardcoded data, trivial "never panics" assertions
- **Gaps:** Needs complete rewrite with symbolic inputs and meaningful property assertions.

### workspace_tests
- **Kani files:** 1 harness
- **Quality:** NEEDS ASSESSMENT
- **Harness:** `kani_error_types_code.rs` — error type coverage

---

## Hardcoded Input Violations (GOD RULE #1)

### CONFIRMED VIOLATIONS

| File | Issue |
|------|-------|
| `vb_expr/src/kani_expr_stack.rs` | All 3 harnesses use hardcoded `ExprOp` arrays: `[ExprOp::LoadConst(...)]`, `[ExprOp::Add]`, `[ExprOp::LoadConst(...); 65]`. No `kani::any()` on structural inputs. |
| `vb_storage/src/kani_admission.rs` | `minimal_valid_workflow()` constructs fixed `WorkflowParts` with hardcoded values. |
| `vb_yaml/src/kani_is_primitive_legacy.rs` | Multiple hardcoded string literals in tests (e.g., `"int"`, `"float"`, `"string"`, `"bool"`). One harness uses `kani::any()` but assertion is tautological. |
| `vb_boundary_inventory/src/kani_harnesses.rs` | Hardcoded `FieldState` and `FreshnessMarker` values. Trivial "never panics" assertions. |

### ACCEPTABLE (intentional design)

| File | Justification |
|------|---------------|
| `vb_compile/src/kani_idempotency_parity.rs` | Hardcoded `ActionContract` fields but exhaustive enum-variant decision table (5×3×3=45 combinations). Intentional, not a violation. |
| `vb_benchmark/src/kani_gate.rs` | Latency fields use literal `0` in three separate cases — this is intentional exhaustive case testing for "zero latency" detection, not structural hardcoding. |

---

## Functions Requiring New Kani Proofs (CRITICAL)

### vb_proof_kernels

| Module | Function | Priority | Notes |
|--------|----------|----------|-------|
| `resource_budget.rs` | `Budget::sequential_add()` | P0 | 12 fields, saturating_add vs max |
| `resource_budget.rs` | `Budget::branch_max()` | P0 | 12 fields, max() on all |
| `resource_budget.rs` | `Budget::loop_mul()` | P0 | 12 fields, saturating_mul |
| `resource_budget.rs` | `sequential_compose()` | P0 | Compose + sequential_add invariant |
| `resource_budget.rs` | `branch_compose()` | P0 | Compose + branch_max invariant |
| `resource_budget.rs` | `loop_compose()` | P0 | Compose + loop_mul invariant |
| `resource_budget.rs` | `Policy::within()` | P0 | Violation detection, boundary conditions |
| `resource_budget.rs` | `Budget::new()` / `Default` | P1 | All-zero initialization |
| `taint.rs` | `join_taint()` | P0 | 3×3=9 exhaustive input pairs |
| `taint.rs` | `join_many()` | P0 | Empty, single, multiple, order-independence |
| `taint.rs` | `all_lattice_laws()` | P0 | Commutative, associative, idempotent, identity, no-downgrade |
| `taint.rs` | `secret_never_downgrades()` | P0 | Rank monotonicity |
| `step_state.rs` | `is_valid_transition()` | P0 | 8×8=64 exhaustive transition pairs |
| `step_state.rs` | `validate_transition()` | P0 | Result type correctness |
| `step_state.rs` | `next_states()` | P0 | Next-state shape for all 8 states |
| `step_state.rs` | `terminal_cannot_transition_to_non_terminal()` | P0 | Already has 1 weak harness — needs rewrite |
| `envelope_header.rs` | Header construction | P2 | Basic construction proofs |
| `vb_kyyf_normalization.rs` | Normalization | P2 | Determinism proofs |

**Note:** `step_state.rs` has 1 existing Kani harness (weak — modulo-based `kani::any()` mapping). It only proves terminal-state absorbing property. Needs complete rewrite with proper `kani::Arbitrary` impl for `StepState`.

### vb_queue_semantics

| Function | Priority | Notes |
|----------|----------|-------|
| `validate_capacity()` | P0 | Zero rejection, above-maximum rejection |
| `QueueState::new()` | P0 | Capacity validation, empty queue creation |
| `QueueState::from_vec_deque()` | P0 | Import with capacity validation |
| `action_enqueue_transition()` | P0 | Full queue rejection, successful enqueue |
| `action_dequeue_transition()` | P0 | Empty vs non-empty transitions |
| `command_pop_transition()` | P0 | Alias of dequeue |
| `shard_tick_transition()` | P0 | Zero or one consumption |
| `enqueue_decision()` | P0 | Zero-allocation decision function |
| `command_pop_transition_decision()` | P0 | Zero-allocation pop decision |
| `shard_tick_transition_decision()` | P0 | Zero-allocation tick decision |
| `helper_enqueue_accepts()` | P0 | Pure boolean predicate |
| `helper_queue_is_full()` | P0 | Pure boolean predicate |
| `warning_payload()` | P0 | Threshold computation, boundary cases |
| `warning_threshold()` | P0 | Overflow on checked_mul, edge cases |
| `runtime_queue_full_error_transition()` | P1 | Error transition construction |

### vb_expr

| Function | Priority | Notes |
|----------|----------|-------|
| `eval_expr_program()` | P0 | Full program evaluation, no panic |
| `parse_expr()` | P0 | Parser correctness, error types |
| `compile_expr()` | P0 | Compilation to bytecode |
| `check_expr_stack_bound()` | P0 | Stack overflow/underflow detection |
| `eval_binary_op()` | P0 | Binary operator semantics |
| `eval_unary_op()` | P0 | Unary operator semantics |

---

## Quick Reference: Kani File Counts by Crate

| Crate | Kani Files | Quality |
|-------|-----------|---------|
| vb_core | 70+ | STRONG |
| vb_storage | 60+ | MIXED |
| vb_compile | 50+ | STRONG |
| vb_runtime | 20+ | STRONG |
| vb_expr | 7 | MIXED |
| vb_ipc | 8 | STRONG |
| vb_yaml | 4 | MIXED |
| vb_proof_kernels | 7 (6+1) | MIXED (gap) |
| vb_benchmark | 3+ | STRONG |
| vb_validate | 3 | STRONG |
| vb_verification | 3 | STRONG |
| vb_cli | 21 (3+18) | MIXED |
| vb_boundary_inventory | 1 | WEAK |
| workspace_tests | 1 | NEEDS ASSESSMENT |
| vb_doc | 0 | LOW PRIORITY |
| vb_test_util | 0 | LOW PRIORITY |
| vb_queue_semantics | 0 | CRITICAL GAP |

---

## Prioritized Action Items

### P0 — Critical (must fix before ship)
1. **`vb_queue_semantics`**: Create 10+ Kani harnesses for queue state transitions (432 lines, zero unsafety, all pure functions — perfect Kani target)
2. **`vb_proof_kernels` — `resource_budget.rs`**: Create 6+ Kani harnesses for Budget arithmetic (12 fields × 3 operations, saturating arithmetic — perfect Kani target)
3. **`vb_expr/src/kani_expr_stack.rs`**: Rewrite with `kani::any()` symbolic inputs (currently all hardcoded `ExprOp` arrays)
4. **`vb_storage/src/kani_admission.rs`**: Replace hardcoded `minimal_valid_workflow()` with symbolic inputs

### P1 — Important (fix in next iteration)
5. **`vb_proof_kernels` — `taint.rs`**: Create taint lattice law harnesses (3-element lattice, 9 input pairs, ideal for exhaustive Kani)
6. **`vb_proof_kernels` — `step_state.rs`**: Rewrite existing weak harness + add `is_valid_transition()`, `validate_transition()`, `next_states()` harnesses (8×8=64 exhaustive transitions)
7. **`vb_yaml/src/kani_is_primitive_legacy.rs`**: Replace hardcoded strings with symbolic inputs
8. **`vb_expr/src/`**: Add Kani proofs for `eval_expr_program`, `parse_expr`, `compile_expr`
9. **`vb_cli/kani_lifecycle.rs`**: Rewrite to call production `cancel()`/`complete()` functions instead of internal simulation

### P2 — Nice-to-have
10. **`vb_boundary_inventory/src/kani_harnesses.rs`**: Rewrite with meaningful assertions
11. **`vb_cli`**: Assess remaining 160 files for computational functions needing Kani
12. **`vb_proof_kernels` — `envelope_header.rs`, `vb_kyyf_normalization.rs`**: Basic construction proofs
