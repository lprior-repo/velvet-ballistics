# BIG-ASS TESTING GAP AUDIT — VELVET-BALLISTICS

## PURPOSE

4 rounds × 12 agents = 48 subagent reviews of all velvet-ballistics crates against master plan test criteria.
Findings drive remediation beads in beads.

---

## MASTER PLAN REFERENCE

`velvet-ballistics-MASTER.md` (authoritative source):
- Sections 2, 12, 15, 16, 17, 18, 20, 21, 22, 28, 30, 32, 33, 36, 37, 38, 39, 40, 45, 46, 47, 50, 53, 65, 66, 67

Key test mandates:
- **Section 36**: Statement + branch + path coverage; all helper functions exercised; error paths exercised
- **Section 37**: Fuzz for IR deserialization, YAML/JSON parse, expression evaluation, IPC encoding, collect_page pagination
- **Section 38**: 11 property tests — constant_folding, bytecode_ast_parity, digest_stability, layout_stability, bound_enforcement, for_each_ordering, taint_propagation, arithmetic_overflow, concurrency_safety, resource_budget, error_recovery
- **Section 39**: 22 benchmark groups — expression_eval (per-op), IR_traversal, YAML_parse, collect_page, action_dispatch, SlotWritten_write, codegen, validation, IPC_send, IPC_throughput, compile_throughput, evaluate_throughput, memory_footprint, cold_start, warm_throughput, pagination_cost, action_queuing, timer_wheel_tick, snapshot_save/restore, digest_computation, taint_check, budget_enforcement
- **Section 46**: 10 helpers (empty, unique, contains, starts_with, ends_with, has, append, append_if, merge, sum); no short-circuit; no F64 evaluation
- **Section 47**: Taint passed through; no rejection of Secret/DerivedFromSecret finish results
- **Section 50**: ArrayQueue for IPC (lock-free SPSC); crossbeam_channel FORBIDDEN
- **Section 66**: Definition of Done — all property tests pass, all fuzz pass, all benchmarks pass, coverage >= threshold, no forbidden patterns, DRIFT resolved

---

## CRATE INVENTORY

Production crates (crates/):
vb_core, vb_yaml, vb_validate, vb_expr, vb_compile, vb_storage, vb_runtime, vb_ipc, vb_codegen, vb_cli, vb_benchmark, vb_ui, vb_ui_model, vb_ui_snapshot, vb_ui_makepad, vb_doc, vb_boundary_inventory, vb_proof_kernels

Test/benchmark crates:
workspace_tests/, fuzz/, benches/

---

## MASTER PLAN STALENESS LOG

| Section | Issue | Found By | Date |
|---------|-------|----------|------|
| 46 | Helper function bugs (empty, unique, etc) — ALL FIXED | Research agent | 2026-05-17 |
| 67 | DRIFT-4 says ACTIVE — RESOLVED | Research agent | 2026-05-17 |
| 67 | DRIFT-5 says ACTIVE — RESOLVED | Research agent | 2026-05-17 |

---

## ROUND 1 FINDINGS (2026-05-17)

### APPROVED (5)
- **vb_yaml**: Well-tested, full property coverage for parse/dump
- **vb_storage**: Journal recovery, SlotWritten, replay infrastructure solid
- **vb_runtime**: Lifecycle, finish output, cancel/resume covered
- **vb_cli**: validate, compile, replay commands well-tested
- **fuzz**: yaml_events and expr_eval well-implemented with corpus

### REJECTED (5)
- **vb_core**: Rejected — FiniteF64 deserialization gap; SetConst taint propagation missing
- **vb_validate**: LETHAL — validate_taint rejects SecretResultLeak for Finish (violates Section 47)
- **vb_expr**: LETHAL — AND/OR short-circuit (violates Section 46); F64 arithmetic contradiction
- **vb_runtime**: LETHAL — tick_shard API missing (Section 30); bounded action completion queue missing (Section 4)
- **vb_ipc**: LETHAL — ArrayQueue specified, crossbeam_channel used (Section 50); pipelining contradiction

### PARTIAL (2)
- **vb_compile**: Partial — compile pipeline solid but $attempt.number restriction not tested
- **vb_codegen**: Partial — trybuild silent pass when no fixtures

### LETHAL CROSS-CUTTING (5)
1. **validate_taint** rejects SecretResultLeak for Finish — violates Section 47; tests assert the WRONG behavior
2. **AND/OR short-circuit** — eval.rs:161-162 uses `?` causing early return; violates Section 46
3. **F64 contradiction** — Section 46 plan says no F64 eval but codegen uses F64 arithmetic
4. **tick_shard missing** — Runtime::tick_shard not implemented; violates Section 30
5. **bounded action completion queue missing** — violates Section 4

---

## ROUND 2 FINDINGS (2026-05-17)

### APPROVED (4)
- **vb_yaml**: Parse/dump roundtrip + error handling well-covered
- **vb_cli**: replay, validate, compile commands functional
- **fuzz**: yaml_events and expr_eval with corpus approved
- **vb_ui_snapshot**: Layout and redaction checks pass

### REJECTED (7)
- **vb_codegen**: LETHAL — trybuild silently passes (Section 36); pattern rejection scanner untested; CodegenError::UnsupportedIr missing test
- **vb_cli**: LETHAL — ui command not implemented (Section 33); cli_postcard.rs unused
- **vb_benchmark**: LETHAL — BenchmarkMetadata only 7/22 required fields
- **vb_ui_model**: LETHAL — test density 1.9x below 5x threshold
- **fuzz**: LETHAL — generated_compare is a STUB (deserializes, discards results, no comparison)
- **workspace_tests**: CRITICAL GAP — 11/11 property tests missing; ~24/40 benchmarks missing
- **vb_ipc**: LETHAL — crossbeam_channel used instead of ArrayQueue (Section 50); pipelining contradiction

### PARTIAL (1)
- **vb_runtime**: tick_shard exists in one location but API is not consistent

### LETHAL CROSS-CUTTING (9)
1. **trybuild silently passes** when no compile-fail fixtures exist — Section 36 mandate invisible in CI
2. **Pattern rejection scanner** (slots[, Vec<, as) untested for injection attacks
3. **CodegenError::UnsupportedIr** not tested for missing compilation step
4. **ui command not implemented** — Section 33 requires it
5. **cli_postcard.rs** envelope unused by any command
6. **BenchmarkMetadata** only 7/22 fields (missing: git_hash, hostname, cpu_brand, os_version, rust_version, timestamp, user, etc.)
7. **test density** 1.9x below 5x threshold in vb_ui_model
8. **generated_compare is a STUB** — deserializes but discards all results, no comparison
9. **IPC ArrayQueue vs channel** — crossbeam_channel used, violates Section 50

---

## ROUND 3 FINDINGS

### APPROVED (4)
- **property_tests**: Plan-accurate property-based test suite structure
- **idempotency**: Basic idempotency coverage present
- **DRIFT-2 (storage)**: Read path verified, SlotWritten + journal_event infrastructure present
- **DRIFT-3 (validate)**: validate_budget infrastructure present

### REJECTED (7)
- **DRIFT-2 (runtime collect)**: CRITICAL — no test verifies SlotWritten BEFORE PC advance; journal_event fuzz target MISSING from fuzz/
- **DRIFT-2 (IPC fuzz)**: journal_event not fuzzed
- **DRIFT-3 (runtime)**: ValueStore uncapped; validate_budget NOT called at admission; collect_page pagination state NOT validated
- **cancellation/shutdown**: cancel while Running not tested; shutdown remaining runs not tested
- **async_primitives (CLI)**: ask_answer_resume and wait_timer_resume gaps
- **workspace_tests**: 11/11 Section 38 property tests MISSING (confirmed); ~24/40 Section 39 benchmarks MISSING
- **DRIFT-2 (IPC)**: crossbeam_channel used instead of ArrayQueue per Section 50

### PARTIAL (1)
- **DRIFT-2 (storage)**: Read path verified but SlotWritten-before-PC-advance not proven in actual execution path

### LETHAL CROSS-CUTTING (11)
1. **vb_expr:** AND/OR short-circuit still active (eval.rs:161-162 uses `?`)
2. **vb_expr:** F64 arithmetic contradiction still active (Section 46 plan says no F64 eval)
3. **vb_compile:** Section 65 SideEffect/RetrySafety enums DO NOT MATCH master plan taxonomy
4. **vb_compile:** `$attempt.number` restriction NOT implemented
5. **vb_compile:** `$random`/`$time` restrictions scaffolded but NOT enforced
6. **vb_storage:** journal_event fuzz target MISSING from fuzz/
7. **vb_storage:** DRIFT-2 CRITICAL: No test verifies SlotWritten BEFORE PC advance in actual execution
8. **vb_runtime:** collect_page does NOT validate pagination state (_states param unused)
9. **vb_runtime:** cancel while Running not tested; shutdown remaining runs not tested
10. **vb_cli:** async_primitives ask_answer_resume and wait_timer_resume GAPS
11. **workspace_tests:** 11/11 Section 38 property tests MISSING; ~24/40 benchmarks MISSING

### NEW BLOCKERS FROM ROUND 3
- `fuzz/journal_event` target does not exist — DRIFT-2 can't be closed
- `vb_runtime::collect_page` pagination state untested
- `vb_compile` enums mismatched vs master plan Section 65
- Cancellation + shutdown test gaps in runtime + CLI

---

## ROUND 4 FINDINGS

### APPROVED (2)
- **Section 38 property tests:** vb_expr has constant_folding, arithmetic_overflow, bound_enforcement; vb_runtime has for_each_ordering, taint_propagation, resource_budget, error_recovery
- **Section 37 fuzz:** yaml_events and expr_eval targets APPROVED with corpus

### REJECTED (10)
- **property_tests (Section 38):** vb_ipc, vb_ui_model, vb_ui_snapshot — 11/11 MISSING; vb_compile, vb_validate, vb_yaml — 10/11 MISSING; vb_storage — 8/11 MISSING; vb_expr, vb_runtime, vb_cli — partial; property_tests.rs in vb_runtime/engine/ is EMPTY
- **Section 37 fuzz:** generated_compare STUB (discards all results); compiled_ir STUB (discards results); ipc_frame discards all decode results; expression discards eval results; decode_record uses .ok() suppressing all failures; collect_page pagination fuzz target MISSING ENTIRELY; zero corpus entries for compiled_ir/generated_compare/ipc_frame/expression
- **Section 36 coverage:** vb_boundary_inventory ZERO tests; vb_doc ZERO tests; vb_core FAILS compilation under coverage; no llvm-cov full workspace coverage; moon coverage task only runs ONE test
- **Section 39 benchmarks:** 12/22 groups MISSING (IR_traversal, collect_page, action_dispatch, memory_footprint, cold_start, pagination_cost, action_queuing, timer_wheel_tick, snapshot_save/restore, digest_computation, ArrayQueue, rtrb); expression_evaluation REJECTED (sin/cos/tan/div missing)
- **taint_propagation (Section 47):** vb_validate rejects SecretResultLeak for Finish (WRONG per Section 47); vb_compile same violation; no end-to-end pipeline test; AND/OR short-circuit untested
- **helper_coverage (Section 46):** starts_with missing edge+error; ends_with missing edge+error; has missing error; append missing edge; append_if missing edge+error; merge missing edge; sum missing edge
- **CI gate adequacy:** Only 3/25 LETHALs caught by CI; moon miri runs only 3 tests; verify-* tasks all runInCI:false; fuzz-smoke only builds not runs; bench-build uses --no-run; trybuild silently passes
- **Section 30 tick_shard:** MISSING entirely — tick_shard API absent, ShardDirective enum absent, bounded action completion queue absent, no tick_shard tests
- **Section 33 CLI:** ui command MISSING (required); evaluate command MISSING (simulate doesn't satisfy --env/--budget); benchmark command lacks --iterations/--warmup
- **Section 50 IPC:** crossbeam_channel used instead of ArrayQueue (FORBIDDEN by Section 50)

### PARTIAL (multiple)
- **forbidden patterns:** 4 crates missing #![forbid(unsafe_code)]; 7 crates with expect() in production (418 total); 7 crates with unwrap() in production (518 total); 5 crates CLEAN

### LETHAL CROSS-CUTTING (Round 4)
1. **property_tests.rs is EMPTY** — vb_runtime/src/engine/property_tests.rs is a 1-line placeholder
2. **no centralized property_tests/ directory** — tests scattered across crates
3. **no minimization config** — fuzz/Cargo.toml has no cargo-fuzz minimization metadata
4. **12 benchmark groups MISSING** — IR_traversal, collect_page, action_dispatch, memory_footprint, cold_start, pagination_cost, action_queuing, timer_wheel_tick, snapshot_save/restore, digest_computation
5. **moon coverage task is a stub** — only runs 1 test from vb_core, not actual coverage
6. **vb_core fails compilation under coverage** — phantom match arm or unused Result must-use
7. **no llvm-cov full workspace coverage report exists**
8. **Section 47 violation CONFIRMED** — validate_taint in vb_validate and vb_compile rejects SecretResultLeak for Finish (should PASS)
9. **no end-to-end taint propagation test** spanning vb_validate → vb_expr → vb_compile → vb_runtime
10. **3/10 helpers fully covered** (empty, unique, contains); 7/10 have edge/error gaps
11. **moon ci miri runs only 3 tests** — 22 LETHALs unmri'd
12. **all verify-* tasks have runInCI:false** — Kani/Verus not in CI
13. **trybuild_tests.rs silently passes** when compile-fail/ directory is empty
14. **density audit Tier 0 not automated** in CI
15. **nightly-feature-gate only checks Rust features** — not $attempt.number/$random/$time restrictions
16. **tick_shard API MISSING** — Section 30 master plan violation
17. **ShardDirective enum MISSING** — Continue/Suspend/Migrate/Shutdown not implemented
18. **ui command MISSING** — Section 33 requires it
19. **evaluate command not implemented** — simulate doesn't satisfy --env/--budget contract
20. **benchmark command lacks --iterations/--warmup** flags
21. **crossbeam_channel used** instead of ArrayQueue — Section 50 violation
22. **4 crates missing #![forbid(unsafe_code)]:** vb_benchmark, vb_boundary_inventory, vb_proof_kernels, workspace_tests
23. **7 crates have 418+ expect() in production** — vb_core(82), vb_ipc(101), vb_ui(84), vb_runtime(57), vb_storage(74), vb_compile(19), vb_ui_model(1)
24. **7 crates have 518+ unwrap() in production** — vb_runtime(144), vb_ipc(122), vb_ui(100), vb_core(78), vb_storage(25), vb_compile(17), vb_expr(13)
25. **5 crates CLEAN** (zero forbidden patterns): vb_cli, vb_doc, vb_ui_makepad, vb_ui_snapshot, vb_yaml

### DEFINITION OF DONE (Round 4 DoD agent)
**8 MUST_FIX before shipping:**
1. validate_taint SecretResultLeak rejection (Section 47)
2. AND/OR short-circuit (Section 46)
3. F64 arithmetic contradiction (Section 46)
4. tick_shard missing (Section 30)
5. bounded action completion queue missing (Section 4)
6. ui command missing (Section 33)
7. journal_event fuzz target missing (DRIFT-2)
8. SlotWritten-before-PC-advance untested (DRIFT-2)

**6 SHOULD_FIX (quality degradations):**
1. ArrayQueue vs channel (Section 50)
2. trybuild silent pass (Section 36)
3. 11/11 property tests missing (Section 38)
4. ~24/40 benchmarks missing (Section 39)
5. $attempt.number restriction not implemented
6. SideEffect/RetrySafety enum mismatch

**Shipping is BLOCKED by 8 LETHALs.**

---

## STATUS

**PLAN:** Documented
**ROUND 1:** COMPLETE — 12 agents, 12 results
**ROUND 2:** COMPLETE — 12 agents, 12 results
**ROUND 3:** COMPLETE — 12 agents, 12 results
**ROUND 4:** COMPLETE — 12 agents, 12 results

**Round 1 Summary:** 5 APPROVED, 5 REJECTED, 2 PARTIAL, 5 LETHAL cross-cutting
**Round 2 Summary:** 4 APPROVED, 7 REJECTED, 1 PARTIAL, 9 LETHAL cross-cutting
**Round 3 Summary:** 4 APPROVED, 7 REJECTED, 1 PARTIAL, 11 LETHAL cross-cutting
**Round 4 Summary:** 2 APPROVED, 10 REJECTED, 0 PARTIAL, 25 LETHAL cross-cutting

**CUMULATIVE TOTALS (All 4 Rounds):**
- LETHAL findings: 50 (25 Round 1-2 + 11 Round 3 + 25 Round 4)
- APPROVED: 15 crates
- REJECTED: 29 crate-reviews
- CRITICAL GAPS: 80+
- **SHIPPING BLOCKED** by 8 MUST_FIX LETHALs
