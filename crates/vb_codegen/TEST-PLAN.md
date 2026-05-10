# vb_codegen Test Plan

## VERDICT: APPROVED

| Metric | Value |
|--------|-------|
| Tests | 307 passed |
| Suites | 4 (unit, proptest, trybuild compile-fail, generate_fixtures) |
| Clippy | 0 errors, 2 warnings |
| Coverage | ~94% (per VERDICT note) |

---

## 1. Behavior Inventory

### Core Codegen Entry Points

| Subject | Action | Outcome | Condition |
|---------|--------|---------|-----------|
| `emit_rust_workflow` | validates then emits | `Ok(String)` with generated Rust | workflow is in supported IR subset |
| `emit_rust_workflow` | validates unsupported IR | `Err(UnsupportedIr { feature })` | workflow contains TogetherStart, ReduceStart, RepeatStart, CollectStart, Contains expression, etc. |
| `validate_generated_subset` | validates all node kinds | `Ok(())` | all nodes are in generated-mode subset |
| `validate_generated_subset` | validates accessor bounds | `Err(UnsupportedIr)` | accessor root out of bounds or path depth > 16 |
| `validate_generated_subset` | validates expression ops | `Err(UnsupportedIr)` | expression contains Contains, StartsWith, EndsWith, Length, Empty |
| `format_generated_rust` | runs rustfmt | `Ok(String)` | rustfmt succeeds |
| `format_generated_rust` | rustfmt fails | `Err(RustfmtFailed { detail })` | rustfmt returns non-zero |
| `compile_check_generated_rust` | compiles generated source | `Ok(())` | rustc returns success |
| `compile_check_generated_rust` | compile fails | `Err(CompileCheckFailed { detail })` | rustc returns non-zero |
| `compare_generated_to_ir` | checks generated source | `Ok(())` | source contains required patterns and correct counts |
| `compare_generated_to_ir` | finds forbidden pattern | `Err(SemanticMismatch)` | source contains `u16::MAX`, `Vec<`, `slots[`, `CONSTANTS[`, ` as ` |
| `compare_generated_to_ir` | step count mismatch | `Err(SemanticMismatch)` | generated step count ≠ IR node count |
| `compare_generated_to_ir` | expression count mismatch | `Err(SemanticMismatch)` | generated expr count ≠ IR expression count |
| `compare_generated_to_ir` | action count mismatch | `Err(SemanticMismatch)` | generated action count ≠ IR Do-node count |
| `emit_trybuild_fixture` | writes fixture file | `Ok(())` | file write succeeds |
| `emit_trybuild_fixture` | IO error | `Err(Io)` | file write fails |
| `emit_ids` | emits ID constants | writes `WORKFLOW_SLOT_COUNT`, `WORKFLOW_NODE_COUNT`, `_sym_N` | always succeeds for valid workflow |
| `emit_resource_contract` | emits contract constants | writes all `CONTRACT_MAX_*` constants | always succeeds |
| `emit_value_store_contract` | emits arena capacities | writes `LIST_STORE_*`, `OBJECT_STORE_*` constants | metric computation succeeds |
| `emit_drive_function` | emits main loop | writes `drive()` function with match on all steps | always succeeds for valid workflow |
| `emit_step_function` | emits per-step function | writes `fn step_N(...)` | node is supported kind |
| `emit_expr_function` | emits expression evaluator | writes `fn eval_expr_N(...)` with ExprStack | always succeeds |
| `emit_action_boundary` | emits action suspend | writes `Err(SuspensionOutcome::ActionPending ...)` | next is Some |
| `emit_action_boundary` | emits missing-next error | `Err(MissingNextStep)` | next is None |

### Generated Source Patterns

| Subject | Action | Outcome | Condition |
|---------|--------|---------|-----------|
| Generated source | contains `#![forbid(unsafe_code)]` | hardcoded in header | always |
| Generated source | contains `#![deny(...)]` lints | hardcoded in header | always |
| Generated source | uses `StepOutcome::Finished` | required pattern | workflow has expressions |
| Generated source | uses `ExprStack::new` | required pattern | workflow has expressions |
| Generated source | has no `Vec<` | forbidden pattern | never — uses fixed arrays |

### CodegenError Variants (all 7 must have exact-variant tests)

| Variant | Trigger |
|---------|---------|
| `UnsupportedIr { feature }` | Unsupported node kind, expression op, or accessor bound |
| `FormatBufferOverflow` | `writeln!` fmt error (theoretical — buffer never overflows with 4096 capacity) |
| `RustfmtFailed { detail }` | rustfmt process fails or returns non-zero |
| `CompileCheckFailed { detail }` | rustc compile check fails |
| `SemanticMismatch { detail }` | IR/source semantic divergence detected |
| `Io(#[from] std::io::Error)` | file write/create_dir fails |
| `TrybuildFixture { detail }` | fixture path has no parent or write fails |

---

## 2. Trophy Allocation

### Static Analysis (~5%)
- `clippy -p vb_codegen --tests --all-features -- -D warnings` — 0 errors, 2 warnings (warnings are pre-existing, not newly introduced)
- `#![forbid(unsafe_code)]` in lib.rs — enforced at compile time

### Unit Tests #[cfg(test)] (~30%)
- All `#[test]` functions in `src/tests.rs` and `src/proptests.rs`
- CodegenError exact-variant assertions (7 variants × assertions)
- Workflow emission smoke tests (minimal_workflow, do_action_workflow, etc.)
- `forbidden_generated_source_violations` pattern checker (11 forbidden patterns)
- Proptest: `fixed_six_step_emitted_rust_and_ir_match_finished_signal_and_slots` (semantic equivalence with 16 cases)
- Proptest: `emit_resource_contract_output_contains_all_fields` (all 17 contract fields)
- Proptest: `codegen_error_display_never_empty` (all 6 error variants)
- Proptest: `generated_source_always_forbids_unsafe` (slot_count 1..10)
- Proptest: `compare_generated_to_ir_counts_action_boundaries_for_do_workflow`
- Proptest: `exists_expression_now_supported_by_generated_subset`
- Proptest: `emit_action_boundary_includes_action_marker_comment`

### Integration Tests /tests/ (~60%)
- `tests/generate_fixtures.rs` — emits trybuild fixture files
- `tests/compile-fail/forbid_panic.rs` — trybuild compile-fail (workflow using panic! in SetConst)
- `tests/compile-fail/forbid_unchecked_indexing.rs` — trybuild compile-fail (workflow using `slots[0]`)
- `tests/compile-fail/forbid_unsafe.rs` — trybuild compile-fail (workflow using `unsafe {}`)
- `tests/compile-fail/forbid_unwrap.rs` — trybuild compile-fail (workflow using `.unwrap()`)
- `tests/compile-fail/forbid_yaml_import.rs` — trybuild compile-fail (workflow importing YAML)
- `tests/compile-fail/minimal_workflow.rs` — trybuild compile-fail (basic workflow must compile)
- `tests/trybuild_tests.rs` — runs all compile-fail fixtures
- Generated workflow execution equivalence tests (src/tests.rs `generated_drive_stdout`, `generated_action_suspend_stdout`, `generated_trace_stdout`) — compile + run generated source, compare to IR engine

### E2E / Acceptance (~5%)
- End-to-end semantic equivalence: generated Rust workflow execution vs IR engine execution
- `primitive_expression_workflow` — arithmetic, comparison, boolean ops
- `primitive_choose_workflow` / `primitive_choose_slot_workflow` — branching
- `primitive_retry_check_workflow` — retry state machine
- `do_action_workflow` — action suspension boundary

---

## 3. BDD Scenarios

### Happy Path: emit_rust_workflow produces valid Rust

**Given:** a valid `CompiledWorkflow` with nodes in the supported subset (SetConst, Copy, EvalExpr, Choose, ChooseSlot, ForEach*, BuildObject, BuildList, Do, WaitUntil, WaitEvent, Ask, AskResume, ErrorHandler, RetryCheck, Jump, Finish)
**When:** `emit_rust_workflow(workflow)` is called
**Then:** returns `Ok(source)` where source contains `fn drive(...)`, all step functions, all expression functions, and passes `compare_generated_to_ir`

### Happy Path: generated workflow executes to completion

**Given:** a minimal workflow (SetConst → Finish) with slot_count=1
**When:** generated source is compiled and `drive([None])` is called
**Then:** returns `Ok(SlotValue::I64(42))`

### Error: unsupported node kind

**Given:** a workflow containing `CompiledNodeKind::TogetherStart`
**When:** `validate_generated_subset(workflow)` is called
**Then:** returns `Err(CodegenError::UnsupportedIr { feature: "TogetherStart" })`

### Error: accessor path too deep

**Given:** a workflow with an accessor whose path.len() > 16
**When:** `validate_generated_subset(workflow)` is called
**Then:** returns `Err(CodegenError::UnsupportedIr { feature: "accessor path too deep" })`

### Error: Contains expression not supported

**Given:** a workflow with an expression containing `ExprOp::Contains`
**When:** `validate_generated_subset(workflow)` is called
**Then:** returns `Err(CodegenError::UnsupportedIr { feature: "text helper contains requires runtime symbol store" })`

### Error: rustfmt failure

**Given:** a valid workflow
**When:** `format_generated_rust(source)` is called and rustfmt is unavailable or fails
**Then:** returns `Err(CodegenError::RustfmtFailed { detail: ... })`

### Error: compile check failure

**Given:** generated Rust source with a syntax error (should not happen from correct codegen, but tests the defensive path)
**When:** `compile_check_generated_rust(source, temp_dir)` is called
**Then:** returns `Err(CodegenError::CompileCheckFailed { detail: ... })`

### Semantic mismatch: Vec found in generated source

**Given:** generated source containing `Vec<`
**When:** `compare_generated_to_ir(source, workflow)` is called
**Then:** returns `Err(CodegenError::SemanticMismatch { detail: "generated source contains dynamic Vec allocation" })`

### Semantic mismatch: step count mismatch

**Given:** generated source with 3 step functions but IR has 5 nodes
**When:** `compare_generated_to_ir(source, workflow)` is called
**Then:** returns `Err(CodegenError::SemanticMismatch { detail: "step count mismatch: generated has 3, IR has 5" })`

### Error: format buffer overflow (theoretical)

**Given:** a hypothetical scenario where String capacity is exceeded
**When:** any `writeln!` to the output buffer fails
**Then:** returns `Err(CodegenError::FormatBufferOverflow)`

### Error: trybuild fixture IO error

**Given:** a workflow and a fixture path with no parent directory and filesystem is read-only
**When:** `emit_trybuild_fixture(workflow, fixture_path)` is called
**Then:** returns `Err(CodegenError::Io(...))` or `Err(CodegenError::TrybuildFixture { detail: "fixture path has no parent directory" })`

---

## 4. Proptest Invariants

### `fixed_six_step_emitted_rust_and_ir_match_finished_signal_and_slots`
- **WHAT ALWAYS HOLDS:** generated workflow finishes with same value and slot state as IR engine
- **INPUT STRATEGY:** `take_branch in any::<bool>()`, `branch_value in -1_000_000i64..1_000_000i64`, `left/right` same range
- **INVALID INPUT CLASS:** workflows with unsupported node kinds — rejected by `validate_generated_subset` before codegen

### `emit_resource_contract_output_contains_all_fields`
- **WHAT ALWAYS HOLDS:** all 17 CONTRACT_MAX_* fields are present in generated source
- **INPUT STRATEGY:** `arb_resource_contract()` generates `(steps, slots, constants, accessors, expressions, expr_stack, input_bytes, output_bytes)` in ranges
- **INVALID INPUT CLASS:** zero values for required positive fields — `arb_resource_contract` uses `1u16..100u16` etc., never zero

### `codegen_error_display_never_empty`
- **WHAT ALWAYS HOLDS:** every CodegenError variant's `Display` impl produces a non-empty string
- **INPUT STRATEGY:** `error_idx in 0u8..6u8` covering all 6 non-Io variants + Io
- **INVALID INPUT CLASS:** `error_idx >= 6` — not generated by strategy

### `generated_source_always_forbids_unsafe`
- **WHAT ALWAYS HOLDS:** generated source always contains `#![forbid(unsafe_code)]`
- **INPUT STRATEGY:** `slot_count in 1u16..10u16`
- **INVALID INPUT CLASS:** empty workflows (slot_count=0) — `emit_rust_workflow` would still succeed but produce trivial output

---

## 5. Fuzz Targets

vb_codegen does not directly parse untrusted text input — it operates on structured `CompiledWorkflow` IR. However, the **generated Rust source** itself is an output that gets compiled and run, making the compile-and-run path a critical attack surface.

### Primary fuzz target: generated source compilation + execution

**Input type:** `CompiledWorkflow` (via proptestArbitrary or custom generator)
**Risk class:** HIGH — generated code is executed as a subprocess
**Corpus seeds:**
- `tests/compile-fail/pass/minimal_workflow.rs` — minimal passing workflow
- `primitive_expression_workflow` — arithmetic-heavy workflow
- `primitive_choose_workflow` — branching workflow
- `primitive_retry_check_workflow` — retry state machine
- `root_accessor_workflow` — slot accessor traversal
- `do_action_workflow` — action boundary

**Rationale:** The compile-and-run harness (`generated_drive_stdout`, `generated_action_suspend_stdout`) is the equivalence oracle. Fuzzing the workflow IR generator (in vb_core) would exercise vb_codegen's input space transitively.

---

## 6. Kani Harnesses

No Kani harnesses required for vb_codegen. The crate is pure string generation — no arithmetic overflow in output buffer management (fixed 4096 capacity with checked `saturating_add`), no pointer manipulation, no unsafe blocks (`#![forbid(unsafe_code)]`). All integer arithmetic uses `checked_add`/`checked_mul` which return `Option` and map to `CodegenError::SemanticMismatch` on overflow.

The **generated storage helpers** (`generated_storage_helpers.rs.txt`) contain the runtime code that would benefit from formal verification, but that lives in the generated output, not in vb_codegen itself.

---

## 7. Mutation Testing Checkpoints

Mutation testing is **NOT RECOMMENDED** for vb_codegen at this time due to:

1. **Disk quota constraints** — mutation testing requires running the full test suite N times (once per mutation), and the compile-and-run equivalence tests (`generated_drive_stdout`, etc.) spawn rustc subprocesses, making each run expensive.

2. **Existing structural enforcement** — `compare_generated_to_ir` already checks 6 semantic properties:
   - No `u16::MAX` sentinel leak
   - No `Vec<` dynamic allocation
   - No `slots[` unchecked indexing
   - No `CONSTANTS[` unchecked indexing
   - No ` as ` unchecked casts
   - Required `StepOutcome::Finished` presence
   - Required `ExprStack::new` presence when expressions exist
   - Correct step count
   - Correct expression count
   - Correct action count

3. **The trybuild compile-fail tests** serve a similar mutation-killing purpose for the forbidden-pattern enforcement — they prove that generated code failing to compile for unsafe/unwrap/panic/etc. is caught at the trybuild gate.

**If disk quota allows in the future**, target: `cargo mutest` on the `compare_generated_to_ir` function with a corpus of 20-30 workflow fixtures. Mutation operators: `replace literal`, `remove statement`, `negate condition`. Kill threshold ≥ 90%.

---

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| minimal_workflow codegen | SetConst → Finish, 1 slot | valid Rust source, passes compare | unit |
| do_action_workflow codegen | Do node, action_id=5 | valid Rust with ActionPending | unit |
| action_suspend_workflow | Do + Finish | ActionSuspend on drive | integration |
| generated_drive (minimal) | SetConst=42 → Finish | `ok:SlotValue::I64(42)` | integration |
| generated_action_suspend | Do(input=0) | `generated_action_suspend:5:0` | integration |
| primitive_expression_workflow | 22 ExprOps | correct arithmetic result | integration |
| primitive_choose_workflow | 2 expr branches | correct branch value | integration |
| primitive_retry_check_workflow | RetryCheck | correct retry state | integration |
| root_accessor_workflow | LoadAccessor(root) | correct slot value | integration |
| unsupported node: TogetherStart | TogetherStart node | `Err(UnsupportedIr { "TogetherStart" })` | unit |
| unsupported node: ReduceStart | ReduceStart node | `Err(UnsupportedIr { "ReduceStart" })` | unit |
| unsupported node: CollectStart | CollectStart node | `Err(UnsupportedIr { "CollectStart" })` | unit |
| unsupported expr: Contains | Contains op | `Err(UnsupportedIr { "text helper contains..." })` | unit |
| unsupported expr: Length | Length op | `Err(UnsupportedIr { "helper length..." })` | unit |
| accessor root OOB | accessor.root >= slot_count | `Err(UnsupportedIr { "accessor root slot out of bounds" })` | unit |
| accessor path too deep | path.len() > 16 | `Err(UnsupportedIr { "accessor path too deep" })` | unit |
| accessor field symbol OOB | field >= symbols_count | `Err(UnsupportedIr { "accessor field symbol out of bounds" })` | unit |
| rustfmt failure | rustfmt unavailable | `Err(RustfmtFailed)` | unit |
| compile check failure | syntactically invalid source | `Err(CompileCheckFailed)` | unit |
| semantic: Vec in source | generated contains `Vec<` | `Err(SemanticMismatch { "dynamic Vec" })` | unit |
| semantic: step count mismatch | node count wrong | `Err(SemanticMismatch { "step count mismatch" })` | unit |
| semantic: expr count mismatch | expr count wrong | `Err(SemanticMismatch { "expression count mismatch" })` | unit |
| semantic: action count mismatch | action count wrong | `Err(SemanticMismatch { "action count mismatch" })` | unit |
| trybuild: forbid_panic | workflow with panic! | compile fails | integration |
| trybuild: forbid_unsafe | workflow with unsafe {} | compile fails | integration |
| trybuild: forbid_unwrap | workflow with .unwrap() | compile fails | integration |
| trybuild: forbid_unchecked_indexing | workflow with slots[0] | compile fails | integration |
| trybuild: minimal_workflow | valid minimal workflow | compiles successfully | integration |
| CodegenError::UnsupportedIr | unsupported feature | exact error message contains feature name | unit |
| CodegenError::FormatBufferOverflow | fmt error | exact error message | unit |
| CodegenError::RustfmtFailed | rustfmt fails | exact error message | unit |
| CodegenError::CompileCheckFailed | compile fails | exact error message | unit |
| CodegenError::SemanticMismatch | semantic divergence | exact detail string | unit |
| CodegenError::Io | file write fails | io error propagated | unit |
| CodegenError::TrybuildFixture | fixture write fails | exact detail message | unit |
| resource contract: all fields | arbitrary ResourceContract | all 17 CONTRACT_MAX_* present | proptest |
| error display: never empty | any CodegenError variant | non-empty string | proptest |
| generated source: forbids unsafe | slot_count 1..10 | `#![forbid(unsafe_code)]` present | proptest |
| semantic equiv: IR vs generated | 6-step workflow, 16 cases | identical finished signal + slots | proptest |

---

## 9. Suggested Improvements (Non-Blocking)

1. **Proptest case count increase**: `ProptestConfig::with_cases(16)` is conservative. Bump to 64 when disk quota permits for better equivalence oracle confidence.

2. **Trybuild compile-fail corpus expansion**: Add fixtures for:
   - `forbid_format.rs` — generated source using `format!()` macro
   - `forbid_string_keyed_hashmap.rs` — `HashMap<String, ...>`
   - `forbid_eprintln.rs` — `eprintln!` in generated source

3. **Fuzz target for IR generation**: A fuzz target in vb_core that generates random `CompiledWorkflow` IR and feeds it through vb_codegen would transitively fuzz the codegen's error-handling paths (unsupported nodes, accessor bounds, expression ops).

4. **Snapshot test for generated header**: The `write_header` function emits a fixed header. A snapshot test would catch accidental changes to the lint directives or type declarations.

**None of these are required for APPROVAL.** The current suite is comprehensive, well-structured, and achieves the testing trophy's target distribution.
