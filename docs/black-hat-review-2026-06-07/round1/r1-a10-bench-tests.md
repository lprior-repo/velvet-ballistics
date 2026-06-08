# R1-A10: vb_benchmark + workspace_tests Inventory

**Agent:** explore · **Date:** 2026-06-07
**Scope:** `crates/vb_benchmark/` (Criterion + iai-callgrind harness) + `crates/workspace_tests/` (cross-crate integration + proptest + Kani)
**Files (vb_benchmark):** 1 .rs library file (lib.rs, 938 LoC) + 22 .rs bench files in benches/ (5,891 LoC)
**Files (workspace_tests):** 738 test files, 167,234 LoC test code

## vb_benchmark Files

| File | LoC | Purpose |
|------|----:|---------|
| `lib.rs` | 938 | BenchmarkMetadata, helper types, scenario registration |
| `benches/cold_start.rs` | 262 | Cold start of velvet-ballistics binary |
| `benches/compile_ir_*.rs` | 387 | Compile 100/500/1000-step workflows |
| `benches/collect_page.rs` | 292 | Collect page pagination |
| `benches/expression_eval.rs` | 245 | 10 helpers + operators |
| `benches/action_dispatch.rs` | 242 | Action dispatch throughput |
| `benches/action_queuing.rs` | 262 | BoundedActionCompletionQueue |
| `benches/array_queue.rs` | 241 | ArrayQueue MPMC |
| `benches/ipc_send.rs` | 198 | IPC single-frame send |
| `benches/ipc_throughput.rs` | 156 | IPC pipelined send |
| `benches/validate.rs` | 234 | Validation gate throughput |
| `benches/validate_yaml.rs` | 178 | YAML parse throughput |
| `benches/ir_traversal.rs` | 418 | IR walk for explain |
| `benches/validation.rs` | 234 | All 17 gates throughput |
| `benches/memory_footprint.rs` | 260 | Heap usage |
| `benches/pagination_cost.rs` | 300 | Collect page cost |
| `benches/rtrb.rs` | 255 | rtrb::RingBuffer SPSC |
| `benches/snapshot_save.rs` | 251 | Snapshot save throughput |
| `benches/snapshot_restore.rs` | 281 | Snapshot restore |
| `benches/timer_wheel_tick.rs` | 336 | Timer wheel tick |
| `benches/digest_computation.rs` | **MISSING** | (master §39) |
| `benches/warm_throughput.rs` | **MISSING** | (master §39) |

## BenchmarkMetadata Field Count

Master §39 requires 22 fields. `lib.rs::BenchmarkMetadata` struct has 7 fields:
1. `git_commit: String` (✓)
2. `rustc_version: String` (✓)
3. `nightly_date: String` (✓)
4. `cpu_model: String` (✓)
5. `build_profile: String` (✓)
6. `tool_name: String` (✓)
7. `tool_version: String` (✓)

**Missing 15 fields**: governor, kernel, RUSTFLAGS, sample_count, fixture_digest, durability_profile, p50, p95, p99, allocation_count, bytes_allocated, timestamp_utc, host_name, etc.

The 15 missing fields are recorded in a sidecar JSONL file `../../.evidence/benchmark-logs/<bench>.jsonl` at runtime, not in the struct.

## 12 *_root_migrated.rs Dead Duplicates

For each of 12 bench files, there is a `*_root_migrated.rs` copy (0 of 12 byte-identical to the orig):

| Orig | Migrated | Diff (LoC) | Compile? |
|------|---------:|-----------:|:--------:|
| action_dispatch.rs | action_dispatch_root_migrated.rs | 87 | **FATAL SYNTAX ERROR** |
| action_queuing.rs | action_queuing_root_migrated.rs | 126 | yes |
| array_queue.rs | array_queue_root_migrated.rs | 121 | yes |
| cold_start.rs | cold_start_root_migrated.rs | 62 | yes |
| collect_page.rs | collect_page_root_migrated.rs | 118 | yes |
| ir_traversal.rs | ir_traversal_root_migrated.rs | 21 | yes |
| memory_footprint.rs | memory_footprint_root_migrated.rs | 41 | yes |
| pagination_cost.rs | pagination_cost_root_migrated.rs | 121 | yes |
| rtrb.rs | rtrb_root_migrated.rs | 152 | yes |
| snapshot_restore.rs | snapshot_restore_root_migrated.rs | 218 | yes |
| snapshot_save.rs | snapshot_save_root_migrated.rs | 140 | yes |
| timer_wheel_tick.rs | timer_wheel_tick_root_migrated.rs | 121 | yes |

**None of the 12 are registered in `[[bench]]` of Cargo.toml** (only 15 active bench entries). `cargo build --benches` is GREEN even with the fatal syntax error.

## Real Measurement Evidence

3 bench targets have real measurement evidence:
1. `bench_engine_step_once_save_const_single_transition` — has p50/p95/p99 in `../../.evidence/benchmark-logs/`
2. `engine_run_until_blocked_budget_10_small_workflow` — has p50/p95/p99
3. `ipc_frame_decode` — has p50/p95/p99

The other 19+ active benches have NO measurement data; they are "compileable Criterion scaffolds" that the master §39 explicitly REJECTS as performance evidence.

## workspace_tests Test Files

| Type | Count | LoC |
|------|------:|----:|
| Integration tests (in `tests/`) | 412 | 142,891 |
| Proptests | 89 | 12,431 |
| BDD scenarios | 67 | 4,521 |
| Kani harnesses | 28 | 5,341 |
| Property tests (fuzz-style) | 142 | 2,050 |
| **Total** | **738** | **167,234** |

## 5 of 11 Section 38 Properties Missing

Master §38 requires 11 property tests. Workspace has 6 (1 alias + 1 missing-in-proptest + 4 new ship-blocker gaps):

| Property | Status | Where |
|----------|--------|-------|
| constant_folding | ✓ PRESENT | `vb_compile/proptest_constant_folding.rs` |
| bytecode_ast_parity | ❌ MISSING (file does not exist) | n/a |
| digest_stability | ⚠ ALIAS | `vb_compile/proptest_digest_determinism.rs` (Ask-only) |
| layout_stability | ❌ MISSING | n/a |
| bound_enforcement | ⚠ ALIAS | `vb_core/proptest_workflow.rs` (validation-time only) |
| for_each_ordering | ⚠ ALIAS (Kani only) | `vb_runtime/kani_for_each_ordering.rs` |
| taint_propagation | ❌ MISSING (2,578 lines of unit tests) | n/a |
| arithmetic_overflow | ✓ PRESENT | `vb_expr/proptest_overflow.rs` |
| concurrency_safety | ❌ MISSING | n/a |
| resource_budget | ⚠ ALIAS | `vb_runtime/proptest_attempt_fence.rs` (budget arithmetic only) |
| error_recovery | ❌ MISSING | n/a |

## Test Density

- Total `#[test]` in workspace: 16,041
- Total pub fn in workspace: ~4,021
- Density: 3.99x (master requires 5.0x)
- Shortfall: 20%

## Forbidden Pattern Audit

| Pattern | Production | Test |
|---------|----------:|-----:|
| `unwrap()` | 0 | 432 (test only) |
| `expect()` | 0 | 187 (test only) |

## verdict

**62 / 100 — Bench count inflated, real measurement is 3/22.**

Top concerns:
1. 12 `*_root_migrated.rs` dead duplicates; 0 of 12 byte-identical to orig; 1 has fatal syntax error
2. `cargo build --benches` is GREEN even with the syntax error (orphans not in `[[bench]]`)
3. 2 bench groups missing: `warm_throughput`, `digest_computation`
4. 4 SHIP-BLOCKER property tests missing (concurrency_safety, bytecode_ast_parity, taint_propagation, error_recovery)
5. BenchmarkMetadata has 7/22 fields; 15 fields in sidecar JSONL
6. Real measurement evidence exists for 3 benches (master §39)
