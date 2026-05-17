# COMPREHENSIVE TEST PLAN — VELVET-BALLISTICS
## Addressing All 50+ Lethal Findings from 4-Round Audit

---

## EXECUTIVE SUMMARY

This plan addresses all LETHAL findings from Round 1-4 auditing across 18 production crates.
**8 MUST_FIX blockers** prevent shipping. **6 SHOULD_FIX** are quality degradations.
**80+ CRITICAL GAPS** require remediation.

---

## MASTER PLAN REFERENCE

- Sections: 2, 12, 15, 16, 17, 18, 20, 21, 22, 28, 30, 32, 33, 36, 37, 38, 39, 40, 45, 46, 47, 50, 53, 65, 66, 67
- Key mandates: Section 36 (coverage), 37 (fuzz), 38 (11 property tests), 39 (22 benchmarks), 46 (10 helpers), 47 (taint), 50 (ArrayQueue IPC)

---

## SECTION A: MUST_FIX — SHIPPING BLOCKERS (8)

### A.1: validate_taint SecretResultLeak Rejection (Section 47) — LETHAL-1

**Problem**: `vb_validate` and `vb_compile` incorrectly reject `SecretResultLeak` for `Finish` results.
Section 47 mandates: taint MUST pass through; NO rejection of Secret/DerivedFromSecret finish results.

**Required Tests**:
```
### Behavior: Taint passes through Finish output
Given: Workflow with Finish step emitting SecretResultLeak taint
When: validate_taint runs on the workflow
Then: Result is ACCEPTED (not rejected)

### Behavior: SecretResultLeak NOT rejected for Finish
Given: Any workflow with Finish result containing secret-derived data
When: validate_taint validation runs
Then: Returns Ok(()) — NO error about SecretResultLeak

### Error variant:
Given: Workflow with Finish step containing untrusted data (NOT secret)
When: validate_taint runs
Then: Returns Err(ValidationError::UntrustedInput) — exact variant required
```

**Files to test**:
- `vb_validate/src/taint.rs` — `validate_taint` function
- `vb_compile/src/taint.rs` — same violation

**Proptest invariants**:
```rust
prop_assert!(validate_taint(workflow_with_secret_finish()).is_ok())
```

---

### A.2: AND/OR Short-Circuit (Section 46) — LETHAL-2

**Problem**: `vb_expr/eval.rs:161-162` uses `?` causing early return (short-circuit).
Section 46 mandates: NO short-circuit evaluation for AND/OR helpers.

**Required Tests**:
```
### Behavior: AND evaluates both operands regardless of first result
Given: AND helper with first operand producing error
When: AND evaluates
Then: Both operands evaluated; error from second operand returned

### Behavior: OR evaluates both operands regardless of first result
Given: OR helper with first operand producing error
When: OR evaluates
Then: Both operands evaluated; error from second operand returned

### Behavior: AND returns false when first operand is false (no short-circuit)
Given: AND helper with first operand = false, second operand would error
When: AND evaluates
Then: Returns false WITHOUT evaluating second operand (optimization allowed)

### Behavior: OR returns true when first operand is true (no short-circuit)
Given: OR helper with first operand = true, second operand would error
When: OR evaluates
Then: Returns true WITHOUT evaluating second operand (optimization allowed)
```

**Files to test**:
- `vb_expr/src/eval.rs` — AND/OR helper functions at lines 161-162

**Mutation checkpoints**:
- Delete second operand evaluation → test `and_evaluates_both_operands` must FAIL

---

### A.3: F64 Arithmetic Contradiction (Section 46) — LETHAL-3

**Problem**: Section 46 plan says "no F64 evaluation" but codegen uses F64 arithmetic.
Section 46 mandates: helpers must NOT use F64 evaluation.

**Required Tests**:
```
### Behavior: Helper functions reject F64 inputs
Given: Helper function receiving F64 arithmetic expression
When: Expression is evaluated
Then: Returns Err(Error::F64NotSupported)

### Behavior: Constant folding preserves F64 rejection
Given: Expression tree containing F64 constants
When: Constant folding runs
Then: Result does NOT contain F64 arithmetic
```

**Files to test**:
- `vb_expr/src/eval.rs` — F64 handling in helpers
- `vb_codegen/src/` — F64 arithmetic usage

---

### A.4: tick_shard API Missing (Section 30) — LETHAL-4

**Problem**: `Runtime::tick_shard` not implemented. Section 30 mandates tick_shard API.

**Required Tests**:
```
### Behavior: tick_shard processes pending actions
Given: Runtime with queued actions in a shard
When: tick_shard is called with Continue directive
Then: All pending actions are processed

### Behavior: tick_shard suspends when directive is Suspend
Given: Runtime with queued actions in a shard
When: tick_shard is called with Suspend directive
Then: Actions are NOT processed; shard is suspended

### Behavior: tick_shard migrates when directive is Migrate
Given: Runtime with queued actions in a shard
When: tick_shard is called with Migrate directive
Then: Actions are migrated to target shard

### Behavior: tick_shard shuts down when directive is Shutdown
Given: Runtime with queued actions in a shard
When: tick_shard is called with Shutdown directive
Then: Remaining actions are drained; shard enters shutdown state
```

**ShardDirective enum**:
```rust
pub enum ShardDirective {
    Continue,
    Suspend,
    Migrate { target: ShardId },
    Shutdown,
}
```

**Files to test**:
- `vb_runtime/src/runtime.rs` — `tick_shard` method
- `vb_runtime/src/shard.rs` — `ShardDirective` enum

---

### A.5: Bounded Action Completion Queue Missing (Section 4) — LETHAL-5

**Problem**: Section 4 mandates bounded action completion queue. Not implemented.

**Required Tests**:
```
### Behavior: Action completion queue rejects when full
Given: Bounded queue with capacity N; N actions already queued
When: New action completion is submitted
Then: Returns Err(ActionQueueError::QueueFull { capacity: N })

### Behavior: Action completion queue accepts within capacity
Given: Bounded queue with capacity N; < N actions queued
When: New action completion is submitted
Then: Returns Ok(()); queue length increases by 1

### Behavior: Queue provides backpressure notification
Given: Bounded queue at 80% capacity
When: Action completion is submitted
Then: Returns Ok(()); warning notification sent
```

**Files to test**:
- `vb_runtime/src/action_queue.rs` — BoundedQueue implementation
- `vb_runtime/src/runtime.rs` — action completion integration

---

### A.6: ui Command Missing (Section 33) — LETHAL-6

**Problem**: Section 33 mandates `ui` command. Not implemented.

**Required Tests**:
```
### Behavior: ui command launches interactive UI
Given: Valid workspace configuration
When: `vb ui --workspace <path>` is executed
Then: UI window opens; command blocks until UI exits

### Behavior: ui command validates workspace
Given: Invalid workspace path
When: `vb ui --workspace <invalid>` is executed
Then: Returns Err(CliError::InvalidWorkspace)

### Behavior: ui command requires workspace flag
Given: No workspace specified
When: `vb ui` is executed
Then: Returns Err(CliError::MissingWorkspaceFlag)
```

**Files to test**:
- `vb_cli/src/commands/ui.rs` — ui command implementation
- `vb_cli/src/main.rs` — command registration

---

### A.7: journal_event Fuzz Target Missing (DRIFT-2) — LETHAL-7

**Problem**: `fuzz/journal_event` target does not exist. DRIFT-2 cannot be closed.

**Required Fuzz Target**:
```rust
// fuzz/fuzz_targets/journal_event.rs
fuzz_target!(|data: &[u8]| {
    // Journal event deserialization must not panic
    let _ = vb_storage::journal::parse_event(data);
    // Valid events must deserialize correctly
    if let Ok(event) = vb_storage::journal::parse_event(data) {
        // Verify event consistency
        assert!(event.is_valid());
    }
});
```

**Corpus seeds required**:
- Valid SlotWritten event
- Valid journal checkpoint event
- Valid replay marker event
- Truncated/corrupt data variants

**Files to test**:
- `vb_storage/src/journal.rs` — event parsing

---

### A.8: SlotWritten-Before-PC-Advance Untested (DRIFT-2) — LETHAL-8

**Problem**: No test verifies SlotWritten BEFORE PC advance in actual execution path.

**Required Tests**:
```
### Behavior: SlotWritten recorded before PC advance
Given: Workflow execution with slot write and PC advance
When: Execution trace is analyzed
Then: SlotWritten event appears BEFORE corresponding PC advance event

### Behavior: Journal recovery replays in correct order
Given: Journal with SlotWritten and PC advance events
When: Recovery replays events
Then: Slot is written BEFORE PC is advanced

### Behavior: SlotWritten persists at checkpoint
Given: Workflow at checkpoint boundary
When: Checkpoint is saved
Then: All preceding SlotWritten events are durable
```

**Integration test**:
```rust
#[test]
fn slot_written_before_pc_advance_in_execution_trace() {
    // Execute workflow with slot operations
    // Capture execution trace
    // Assert ordering: SlotWritten(ts) < PC_advance(ts) for each slot
}
```

**Files to test**:
- `vb_storage/src/slot_written.rs` — SlotWritten implementation
- `vb_runtime/src/execution.rs` — execution trace ordering

---

## SECTION B: SHOULD_FIX — QUALITY DEGRADATIONS (6)

### B.1: ArrayQueue vs crossbeam_channel (Section 50) — MAJOR-1

**Problem**: Section 50 mandates ArrayQueue (lock-free SPSC). crossbeam_channel is FORBIDDEN.
Currently crossbeam_channel is used.

**Required Tests**:
```
### Behavior: IPC uses ArrayQueue (not crossbeam_channel)
Given: IPC channel configuration
When: Channel is created
Then: Implementation is ArrayQueue<T, RingFlagged>

### Behavior: ArrayQueue provides lock-free SPSC semantics
Given: Single producer, single consumer
When: Concurrent send/receive operations execute
Then: No locks acquired; atomic operations only
```

**Files to verify**:
- `vb_ipc/src/queue.rs` — ArrayQueue implementation

---

### B.2: trybuild Silent Pass (Section 36) — MAJOR-2

**Problem**: trybuild silently passes when compile-fail/ directory is empty. Section 36 mandate invisible.

**Required Tests**:
```
### Behavior: trybuild fails when compile-fail/ is empty
Given: Empty compile-fail/ directory
When: trybuild tests run
Then: Test FAILS with error indicating no compile-fail cases found

### Behavior: trybuild detects expected compilation failures
Given: compile-fail/ with .stderr files
When: Compilation produces unexpected error
Then: Test FAILS showing expected vs actual error mismatch
```

**Files to test**:
- `vb_compile/tests/trybuild_tests.rs` — trybuild integration

---

### B.3: 11/11 Property Tests Missing (Section 38) — MAJOR-3

**Problem**: Section 38 mandates 11 property tests. 11/11 are MISSING across crates.

**Property Tests Required**:

| # | Property Test | Crate | Target |
|---|--------------|-------|--------|
| 1 | constant_folding | vb_expr | Expression constants folded at compile time |
| 2 | bytecode_ast_parity | vb_compile | Compiled bytecode matches AST semantics |
| 3 | digest_stability | vb_storage | Same input produces same digest |
| 4 | layout_stability | vb_ui_model | UI layout deterministic |
| 5 | bound_enforcement | vb_expr | Bounds checked before evaluation |
| 6 | for_each_ordering | vb_runtime | for_each respects ordering contract |
| 7 | taint_propagation | vb_validate | Taint flows through all operations |
| 8 | arithmetic_overflow | vb_expr | Overflow checked on all arithmetic |
| 9 | concurrency_safety | vb_ipc | Concurrent operations safe |
| 10 | resource_budget | vb_runtime | Budget enforced on actions |
| 11 | error_recovery | vb_runtime | Errors recoverable without panic |

**Proptest required for each**:
```rust
proptest! {
    #[test]
    fn constant_folding_preserves_semantics(expr in arb_expression()) {
        let compiled = compile(&expr);
        let evaluated = eval(&expr);
        prop_assert_eq!(compiled.constant_fold(), evaluated);
    }
}
```

---

### B.4: ~24/40 Benchmarks Missing (Section 39) — MAJOR-4

**Problem**: Section 39 mandates 22 benchmark groups. 12 are MISSING.

**Missing Benchmarks** (12):
- IR_traversal
- collect_page
- action_dispatch
- memory_footprint
- cold_start
- pagination_cost
- action_queuing
- timer_wheel_tick
- snapshot_save/restore
- digest_computation
- ArrayQueue
- rtrb

**Required Benchmark Groups**:
```rust
// benches/ir_traversal.rs
fn bench_ir_traversal(c: &mut Criterion) {
    let ir = load_large_ir();
    c.bench_function("ir_traversal_depth_first", |b| {
        b.iter(|| ir.traverse_depth_first(black_box(&ir)))
    });
}

// benches/collect_page.rs
fn bench_collect_page(c: &mut Criterion) {
    let store = TestStore::new();
    c.bench_function("collect_page_pagination", |b| {
        b.iter(|| store.collect_page(black_box(100), black_box(50)))
    });
}
```

---

### B.5: $attempt.number Restriction Not Implemented — MAJOR-5

**Problem**: Compile restrictions for $attempt.number not enforced.

**Required Tests**:
```
### Behavior: $attempt.number accessible in attempt blocks
Given: Workflow with $attempt.number reference in attempt block
When: Workflow executes
Then: $attempt.number equals current attempt count (1-indexed)

### Behavior: $attempt.number NOT accessible outside attempt blocks
Given: Workflow with $attempt.number reference outside attempt block
When: Compilation runs
Then: Returns Err(CompileError::InvalidVariableScope { var: "$attempt.number" })
```

**Files to test**:
- `vb_compile/src/restrictions.rs` — $attempt.number enforcement

---

### B.6: SideEffect/RetrySafety Enum Mismatch (Section 65) — MAJOR-6

**Problem**: Section 65 SideEffect/RetrySafety enums DO NOT MATCH master plan taxonomy.

**Required Tests**:
```
### Behavior: SideEffect enum matches master plan
Given: Master plan Section 65 SideEffect variants
When: Compiled workflow is analyzed
Then: SideEffect enum variants match exactly

### Behavior: RetrySafety enum matches master plan
Given: Master plan Section 65 RetrySafety variants
When: Compiled workflow is analyzed
Then: RetrySafety enum variants match exactly
```

**Files to test**:
- `vb_compile/src/enums.rs` — SideEffect, RetrySafety enums

---

## SECTION C: REMAINING LETHAL CROSS-CUTTING (25)

### C.1-C.25: Additional Findings from Round 4

| ID | Finding | Section | Files |
|----|---------|---------|-------|
| C.1 | property_tests.rs is EMPTY | 38 | vb_runtime/src/engine/property_tests.rs |
| C.2 | no centralized property_tests/ dir | 38 | — |
| C.3 | no minimization config in fuzz | 37 | fuzz/Cargo.toml |
| C.4 | moon coverage task is a stub | 36 | .moon.yml |
| C.5 | vb_core fails compilation under coverage | 36 | vb_core/src/ |
| C.6 | no llvm-cov workspace coverage report | 36 | — |
| C.7 | no end-to-end taint propagation test | 47 | vb_validate→vb_expr→vb_compile→vb_runtime |
| C.8 | 3/10 helpers fully covered | 46 | vb_expr/src/helpers.rs |
| C.9 | moon ci miri runs only 3 tests | 36 | .moon.yml |
| C.10 | all verify-* tasks have runInCI:false | 36 | .moon.yml |
| C.11 | density audit Tier 0 not automated | 36 | CI |
| C.12 | nightly-feature-gate missing $attempt restriction check | 46 | scripts/check-nightly-features.sh |
| C.13 | ShardDirective enum MISSING | 30 | vb_runtime/src/ |
| C.14 | evaluate command not implemented | 33 | vb_cli/src/commands/ |
| C.15 | benchmark command lacks --iterations/--warmup | 39 | vb_cli/src/commands/benchmark.rs |
| C.16 | crossbeam_channel used (Section 50) | 50 | vb_ipc/src/ |
| C.17 | 4 crates missing #![forbid(unsafe_code)] | 36 | vb_benchmark, vb_boundary_inventory, vb_proof_kernels, workspace_tests |
| C.18 | 7 crates have 418+ expect() | 36 | vb_core(82), vb_ipc(101), vb_ui(84), vb_runtime(57), vb_storage(74), vb_compile(19), vb_ui_model(1) |
| C.19 | 7 crates have 518+ unwrap() | 36 | vb_runtime(144), vb_ipc(122), vb_ui(100), vb_core(78), vb_storage(25), vb_compile(17), vb_expr(13) |
| C.20 | 5 crates CLEAN | 36 | vb_cli, vb_doc, vb_ui_makepad, vb_ui_snapshot, vb_yaml |
| C.21 | generated_compare fuzz is STUB | 37 | fuzz/fuzz_targets/generated_compare.rs |
| C.22 | compiled_ir fuzz is STUB | 37 | fuzz/fuzz_targets/compiled_ir.rs |
| C.23 | ipc_frame fuzz discards decode results | 37 | fuzz/fuzz_targets/ipc_frame.rs |
| C.24 | expression fuzz discards eval results | 37 | fuzz/fuzz_targets/expression.rs |
| C.25 | collect_page pagination fuzz MISSING | 37 | fuzz/fuzz_targets/collect_page.rs |

---

## SECTION D: BEHAVIOR INVENTORY (Per Crate)

### vb_core
- `FiniteF64::deserialize` — MUST test deserialization roundtrip
- `SetConst::eval` — MUST test taint propagation through SetConst

### vb_validate
- `validate_taint` — MUST NOT reject SecretResultLeak for Finish (Section 47)
- `validate_budget` — MUST test budget enforcement at admission

### vb_expr
- `eval_and`, `eval_or` — MUST evaluate both operands (no short-circuit, Section 46)
- `eval_helper_F64` — MUST reject F64 evaluation (Section 46)
- All 10 helpers: empty, unique, contains, starts_with, ends_with, has, append, append_if, merge, sum

### vb_compile
- `$attempt.number` — MUST enforce scope restriction
- `$random`, `$time` — MUST enforce restrictions
- SideEffect/RetrySafety enums — MUST match Section 65

### vb_runtime
- `tick_shard` — MUST implement with ShardDirective enum
- `bounded_action_completion_queue` — MUST implement bounded queue
- `collect_page` — MUST validate pagination state
- `cancel` while Running — MUST test
- `shutdown` remaining runs — MUST test
- for_each_ordering, taint_propagation, resource_budget, error_recovery

### vb_storage
- `journal_event` fuzz target — MUST create
- `SlotWritten` before PC advance — MUST verify in execution trace

### vb_ipc
- ArrayQueue — MUST use (not crossbeam_channel)
- Pipelining — MUST be consistent with ArrayQueue design

### vb_cli
- `ui` command — MUST implement (Section 33)
- `evaluate` command — MUST implement (not simulate)
- `benchmark` command — MUST have --iterations/--warmup
- ask_answer_resume, wait_timer_resume — MUST test

### vb_codegen
- `trybuild` — MUST fail when no compile-fail fixtures
- `CodegenError::UnsupportedIr` — MUST test

### vb_ui_model
- Test density — MUST reach 5x threshold

### fuzz
- `generated_compare` — MUST compare results, not discard
- `compiled_ir` — MUST return results
- `ipc_frame` — MUST verify decode results
- `expression` — MUST verify eval results
- `collect_page` pagination — MUST create fuzz target

### workspace_tests
- 11/11 property tests — MUST implement
- ~24/40 benchmarks — MUST implement

---

## SECTION E: TROPHY ALLOCATION

| Layer | Count | Rationale |
|-------|-------|-----------|
| Unit / Calc | ~200 | Pure functions in vb_expr, vb_compile, vb_validate |
| Integration | ~150 | Component boundaries, IPC, storage |
| E2E | ~30 | CLI commands, full workflows |
| Static Analysis | ∞ | clippy, cargo-deny, forbidden patterns |
| Property | ~50 | Proptest for all pure functions |
| Fuzz | ~15 | All parsers, deserializers, IPC |
| Kani | ~10 | Critical invariants (arithmetic, state machine) |

**Total test count target**: 450+ tests across all layers

---

## SECTION F: OPEN QUESTIONS

1. ShardDirective::Migrate target type — ShardId or something else?
2. Bounded action completion queue capacity — fixed at compile time or configurable?
3. $attempt.number access pattern — 0-indexed or 1-indexed?
4. ArrayQueue capacity — default size?
5. BenchmarkMetadata fields — which are REQUIRED vs OPTIONAL?

---

## SECTION G: MUTATION CHECKPOINTS

Critical mutations that MUST be caught:

| Mutation | Test That Catches It |
|----------|----------------------|
| AND short-circuits | `and_evaluates_both_operands_when_first_errors` |
| OR short-circuits | `or_evaluates_both_operands_when_first_errors` |
| validate_taint rejects Finish secret | `validate_taint_accepts_secret_finish_result` |
| SlotWritten after PC advance | `slot_written_before_pc_advance_in_execution_trace` |
| F64 allowed in helpers | `helper_rejects_f64_arithmetic` |
| tick_shard ignores directive | `tick_shard_respects_suspend_directive` |
| Queue unbounded growth | `action_queue_rejects_when_full` |

**Mutation kill rate target**: ≥90%

---

## SECTION H: COMBINATORIAL COVERAGE MATRIX

### vb_expr Helper Functions

| Helper | Happy | Empty | Edge | Error | Boundary Min | Boundary Max |
|--------|-------|-------|------|-------|--------------|--------------|
| empty | ✓ | ✓ | ✓ | ✓ | N/A | N/A |
| unique | ✓ | ✓ | ✓ | ✓ | 1 elem | MAX |
| contains | ✓ | ✓ | ✓ | ✓ | 1 match | no match |
| starts_with | ✓ | ✓ | ✓ | ✓ | exact | +1 char |
| ends_with | ✓ | ✓ | ✓ | ✓ | exact | +1 char |
| has | ✓ | ✓ | ✓ | ✓ | 1 key | no key |
| append | ✓ | ✓ | ✓ | ✓ | empty→1 | MAX→overflow |
| append_if | ✓ | ✓ | ✓ | ✓ | condition T | condition F |
| merge | ✓ | ✓ | ✓ | ✓ | empty | conflicting |
| sum | ✓ | ✓ | ✓ | ✓ | 0 items | MAX items |

---

## DEFINITION OF DONE

All MUST_FIX (A.1-A.8) must be:
1. Test plan written for each
2. Tests implemented
3. Tests pass
4. Mutation kill ≥90%
5. Coverage ≥90%
6. Black-hat review APPROVED

All SHOULD_FIX (B.1-B.6) must be:
1. Test plan written for each
2. Tests implemented (if feasible)
3. Evidence of progress toward resolution

All remaining lethals (C.1-C.25) must have:
1. Explicit remediation plan
2. OR explicit waiver with clause ID, reason, compensating evidence, owner

**FINAL GATE**: `moon ci` passes. All tests green. Zero lethal findings.