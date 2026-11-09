# Benchmark evidence — 2026-11-09 (§44 Pt 22 re-measurement on current main)

## Run conditions

- **Main tip measured:** `ca9cb7e6` ("docs(verus): re-point stale budget mirror citations to live production spans (#238)")
- **Measured:** 2026-11-09 02:05–02:50 CST (08:05–08:50 UTC) criterion, one session, targets run sequentially; heaptrack pass 02:52–03:24 CST (08:52–09:24 UTC). The hammer run itself started 2026-11-09 01:37 CST.
- **Branch measured:** `vb/pt22-remeasure-1109`, cut from the main tip above; no production code changed by this run
- **Harness:** criterion 0.8 (workspace pins `0.8`); heaptrack 1.5.0 (installed via apt during this run — it was absent after the VM replacement; `perf` 6.8.12 is present but `perf_event_open` is blocked, so no instruction counts)
- **Toolchain:** pinned nightly-2026-04-28
- **Env for every command:** `PATH="$HOME/.moon/bin:$HOME/.cargo/bin:$PATH"`, `TMPDIR=$HOME/tmp`, `RUSTC_WRAPPER=""`, stdin from `/dev/null`, `-j2`
- **Env override for the velvet_ballistics target only:** first attempt at the default 100 ms per-iteration budget (aborted — see "Fjall append stalls" below); retry with `VB_BENCH_LATENCY_BUDGET_US=1000000`
- **Box:** VM, 4 CPUs, 7.7 GiB RAM, shared load; the VM rebooted once ~09:00 UTC (~03:00 CST), killing the first velvet_ballistics heaptrack attempt — see below. Lane B was running a source-length split concurrently for part of the run.
- **Criterion defaults (no overrides in any measured bench):** 3 s warmup, 100 samples, ~5 s measurement per benchmark
- **Metadata for every measured bench:** `durability=mixed`, `mode=ir` (all 129 benches); all 129 benches declare `fixture_bytes`
- **Bench-binary note:** `cargo bench` produces different bench binaries for multi-package vs single-package invocations (feature unification changes the metadata hash). The canonical timings here come from single-target invocations (`cargo bench -p <pkg> --bench <target>`), and every heaptrack total below was verified to come from those same binaries — `capability_schema` was re-run after the first heaptrack pass used the multi-package variant (2,382,887 calls on the correct binary vs 4,124,028 on the wrong one, so the distinction is material).

## How to read the numbers

- **time** = criterion's headline estimate (regression slope) with its 95% CI, exactly as printed to stdout.
- **p50** = median point estimate from criterion's own `estimates.json`.
- **p95 / p99** = percentiles computed from the harness's own raw samples (`sample.json`: per-sample time = `times[i]/iters[i]`, linear interpolation over the 100 sorted per-sample values). The computed p50 matches `estimates.json`'s median exactly for every bench, which validates the method. **These percentiles are over 100 sample means (each averaging thousands of iterations), NOT per-iteration percentiles** — they understate true per-iteration tail spread.
- **thrpt** = criterion throughput estimate with 95% CI, as printed (units as set by each bench). Benches without a `Throughput` setting have no thrpt value.
- **fixture / fixture_bytes** = the bench's own declared fixture metadata, taken from its full criterion bench id.
- **Heaptrack totals** (separate section below) come from reduced 10-sample runs under heaptrack, not from the timing runs. They are per-target-process totals for that reduced run; they are NOT scaled to 100 samples. Heaptrack timings are not reported — heaptrack distorts timings.
- **Timings from this box are noisy and comparative, not absolute.** Do not quote them as guarantees; use them to compare benches against each other and against future runs on comparable hardware.

## Status summary (17 bench targets, all build, all run, exit 0)

| Target | Package | Benches measured |
|---|---|---|
| `aggregate_resource_budget` | vb_core | 1 / 1 |
| `capability_schema` | vb_validate | 1 / 1 |
| `velvet_ballistics` | velvet-ballistics-workspace-tests | 63 / 63 |
| `vb_qi37_1_1_recovery` | velvet-ballistics-workspace-tests | 1 / 1 |
| `vb_kkvb_xtask_routing_red` | velvet-ballistics-workspace-tests | 2 / 2 |
| `ir_traversal` | velvet-ballistics-workspace-tests | 6 / 6 |
| `collect_page` | velvet-ballistics-workspace-tests | 5 / 5 |
| `action_dispatch` | velvet-ballistics-workspace-tests | 6 / 6 |
| `memory_footprint` | velvet-ballistics-workspace-tests | 2 / 2 |
| `cold_start` | velvet-ballistics-workspace-tests | 3 / 3 |
| `pagination_cost` | velvet-ballistics-workspace-tests | 5 / 5 |
| `action_queuing` | velvet-ballistics-workspace-tests | 5 / 5 |
| `timer_wheel_tick` | velvet-ballistics-workspace-tests | 7 / 7 |
| `snapshot_save` | velvet-ballistics-workspace-tests | 5 / 5 |
| `snapshot_restore` | velvet-ballistics-workspace-tests | 5 / 5 |
| `array_queue` | velvet-ballistics-workspace-tests | 6 / 6 |
| `rtrb` | velvet-ballistics-workspace-tests | 6 / 6 |

**Measured total: 129 benchmarks, 0 failures in the final pass.** Bench build proof: `cargo bench -p vb_core -p vb_validate -p velvet-ballistics-workspace-tests --no-run` exits 0 on main `ca9cb7e6` (verified before measuring; 25 min build, 0 errors, one pre-existing warning — unused import `node` in the vb_core test file `crates/vb_core/src/engine/validate/tests/red_phase_cycle_detection_tests.rs:11`, test code only). `cargo bench -p vb_validate --bench capability_schema --no-run` exits 0 and the built binary run with `--list` reports 1 benchmark. All 17 target binaries report the same bench counts as 2026-11-06 (129 total, same bench ids).

## What this closes relative to the 2026-11-06 evidence

- Same 129 bench ids; the extraction reports 0 benches without `fixture_bytes`.
- `aggregate_resource_budget_contract_surface` fixture changed: `budget.rs` source length 115330 → 8462 after the budget splits (#198/#228/#232). The bench searches that source for `AggregateResourceBudget`, so its 1.57 µs → 20.5 ns change tracks the smaller fixture, not a product speedup. Do not quote it as one.
- The `action_dispatch_resolve_compile_time` 1.77 ns flag stays resolved the same way as 2026-11-06: fixed-id bench 1.76 ns [1.76, 1.77, 1.79]; the varying bench (cycles all 10 registered ids with a black-boxed input, asserts each resolved contract id) 2.73 ns [2.72, 2.73, 2.74]. Same order of magnitude, same conclusion.
- `bench_fjall_append_run_accepted_no_persist`: mean 1.79 µs, p50 1.72 µs, p99 3.35 µs — the same order as 2026-11-06 (mean 1.77 µs, p99 2.93 µs).
- **New: heaptrack allocation totals for all 17 targets** (first allocation evidence; see the section below). Allocation counts and bytes allocated are no longer unmeasured — at reduced sample size.
- Flagged, not concluded: the `admit_run_*` benches measure 150–390 ns slower in absolute terms than 2026-11-06 (e.g. `admit_run_relaxed` 98.7 → 482.5 ns mean; `admit_run_multiple_action_caps` 149.0 → 489.0 ns; `capability_check_action_match_first` 54.4 → 144.5 ns). Admission paths gained work since 2026-11-06 (tenant journaling #229, gate-decision submission #217), and sub-µs benches on this shared box are noisy — treat as a flag to watch, not a proven regression.
- `parse_yaml_1mb` mean 80 → 130 µs with p99 89 → 329 µs — a heavy right tail this run, same noisy-box caveat; its p50 (87.95 µs) moved far less than its mean.

## Flagged results (do not base claims on these without further work)

- `timer_wheel_fire_empty`: throughput `[0, 0, 0] elem/s`. Expected, not a failure: the bench sets `Throughput::Elements(0)` for the empty wheel.
- The 2026-05-01 rejected sub-nanosecond result recorded in `benchmark-suite.md` is unchanged by this run and remains rejected.
- `bench_fjall_append_run_accepted_no_persist`: mean 1.79 µs, p50 1.72 µs, p99 3.35 µs — the per-iteration tail on this box is much heavier than the sample statistics show. See "Fjall append stalls" below; the p99 here is a per-sample-mean percentile and does not capture single-iteration stalls.
- The `admit_run_*` and `parse_yaml_1mb` deltas above.

## Fjall append stalls (why the velvet_ballistics target needed two attempts)

- The velvet_ballistics target's bench harness asserts every timed iteration stays inside a per-iteration latency budget (default 100 ms, overridable with `VB_BENCH_LATENCY_BUDGET_US`, a knob built into `crates/workspace_tests/benches/velvet_ballistics.rs`). Attempt 1 aborted in the `bench_fjall_append_run_accepted_no_persist` warmup: one append took 404 ms (budget 100 ms). Attempt 2, with `VB_BENCH_LATENCY_BUDGET_US=1000000`, completed all 63 benches, exit 0; the slowest single fjall-append iteration in that run was 53.8 ms.
- The stalls are disk stalls on this shared VM, not a product regression: most append iterations are 10–400 µs, but spikes recur across runs, including in the 2026-11-06 run. No production code changed between the attempts — the branch is unmodified main `ca9cb7e6` — so the aborts are a property of this box's disk latency, not of the code under test.
- The 1 s budget does not change what is measured: criterion's mean, p50, p95 and p99 for this bench come from the same iterations, and a stalled iteration is one slow iteration among thousands per sample. It only stops one stall from aborting the whole target. Any future re-measurement on a quieter disk should re-try the default budget first and say so.

## VM reboot during the run

- The VM rebooted once at ~09:00 UTC (~03:00 CST; uptime showed 12 min at 09:11 UTC), killing the first velvet_ballistics heaptrack attempt at bench 45 of 63 with no error and no exit line. The criterion runs had finished at 08:50 UTC and were unaffected. The velvet_ballistics heaptrack run was re-run from scratch afterwards and completed rc=0; the truncated first-attempt trace was discarded, not used.

## Heaptrack allocation totals (new — first allocation evidence)

- **Method:** each bench binary run once under heaptrack 1.5.0 with REDUCED criterion settings (`--bench --sample-size 10 --warm-up-time 1 --measurement-time 1`), so the pass finishes in reasonable time. Reports went to a separate `CRITERION_HOME` so the canonical 100-sample reports are untouched. Totals are per-target-process totals (all benches in that target binary in one process run) for the reduced run; they are NOT scaled to 100 samples and are NOT comparable to 100-sample totals. Heaptrack timings are not reported — heaptrack distorts timings.
- **Total bytes allocated** = sum over heaptrack's allocation-size histogram (size × count). The histogram count sum matches the reported call total exactly for all 17 targets, which cross-checks the extraction.
- **Leaked bytes** include a ~35.6K baseline present in nearly every target (criterion harness/startup), not bench code. `cold_start` (243.79K) and `velvet_ballistics` (234.10K) leak more than the baseline.
- **Peak heap** is 5.0–5.3M for most targets, 8.83M for `aggregate_resource_budget` and `capability_schema`, 12.39M for `velvet_ballistics`. Peak heap stays flat while total bytes run into the GiB range for the queue/ring-buffer targets — the allocation is churn (allocate/free in a tight loop), not growth.
- **Binary check:** the totals below come from the exact single-target binaries the canonical timings came from (verified via heaptrack's recorded debuggee command for all 17 traces).

## Heaptrack allocation totals — reduced runs (10 samples, 1 s warmup, 1 s measurement)

| Target | Benches | Allocation calls | Bytes allocated | Peak heap | Leaked |
|---|---|---|---|---|---|
| aggregate_resource_budget | 1 | 521,026 | 124.81 MiB (124,806,438) | 8.83M | 36.62K |
| capability_schema | 1 | 2,382,887 | 169.21 MiB (169,209,329) | 8.83M | 37.65K |
| velvet_ballistics | 63 | 93,181,232 | 56.66 GiB (56,656,284,600) | 12.39M | 234.10K |
| vb_qi37_1_1_recovery | 1 | 3,767,381 | 171.95 MiB (171,950,642) | 5.00M | 35.60K |
| vb_kkvb_xtask_routing_red | 2 | 8,025,148 | 419.98 MiB (419,980,270) | 5.00M | 35.73K |
| ir_traversal | 6 | 11,252,979 | 544.81 MiB (544,809,396) | 5.07M | 35.60K |
| collect_page | 5 | 13,832,165 | 3.87 GiB (3,873,023,551) | 5.01M | 35.73K |
| action_dispatch | 6 | 15,770,347 | 802.36 MiB (802,363,315) | 5.06M | 69.78K |
| memory_footprint | 2 | 7,765,522 | 452.11 MiB (452,108,802) | 5.05M | 35.60K |
| cold_start | 3 | 11,788,016 | 1.83 GiB (1,831,109,167) | 5.27M | 243.79K |
| pagination_cost | 5 | 10,389,857 | 2.82 GiB (2,818,795,564) | 5.02M | 47.38K |
| action_queuing | 5 | 11,719,192 | 85.90 GiB (85,901,479,157) | 5.01M | 35.60K |
| timer_wheel_tick | 7 | 8,147,046 | 5.14 GiB (5,142,858,627) | 5.01M | 38.29K |
| snapshot_save | 5 | 10,230,733 | 599.58 MiB (599,575,458) | 5.01M | 35.60K |
| snapshot_restore | 5 | 19,004,037 | 696.04 MiB (696,042,021) | 5.01M | 39.95K |
| array_queue | 6 | 10,022,449 | 73.96 GiB (73,958,319,765) | 5.04M | 72.59K |
| rtrb | 6 | 18,659,113 | 14.71 GiB (14,706,353,796) | 5.02M | 44.05K |

## Still missing for a full §44 Pt 22 close

1. Instruction counts: criterion does not measure these; they need a counter harness (for example perf counters), which is a separate batch. `perf_event_open` is blocked on this VM (EACCES), so instruction counts cannot be measured here at all. Nothing in this doc estimates, copies forward, or invents instruction counts — they are unmeasured, stated plainly.
2. Allocation counts and bytes allocated are now measured, at reduced sample size (see above). A 100-sample heaptrack pass would be the fuller version; the reduced pass is the honest one for this box and this run window.

## Measured results

### vb_core — aggregate_resource_budget (1 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| aggregate_resource_budget_contract_surface | [20.224 ns 20.532 ns 20.867 ns] | 21.32 ns | 22.80 ns | 23.20 ns | — | budget_rs_source | 8462 |

### vb_validate — capability_schema (1 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| capability schema validation | [189.30 ns 198.75 ns 207.99 ns] | 153.10 ns | 293.99 ns | 436.56 ns | — | workflow_2_nodes_1_contract | 3 |

### velvet-ballistics-workspace-tests — velvet_ballistics (63 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| parse_yaml_small | [−35.508% −22.768% −10.015%] | 22.9685 µs | 28.4454 µs | 29.6374 µs | +11.129% +29.480% +55.057% | small_workflow | 161 |
| parse_yaml_1mb | [+45.427% +62.579% +79.184%] | 87.9548 µs | 228.4521 µs | 329.4897 µs | −44.191% −38.492% −31.237% | generated_1mb_yaml | 1048640 |
| validate_minimal | [+24.680% +50.838% +87.177%] | 26.4932 µs | 77.3045 µs | 158.5804 µs | −46.575% −33.704% −19.795% | small_workflow | 161 |
| compile_ir_minimal | [+2.9057% +6.0853% +9.1635%] | 27.7473 µs | 33.6103 µs | 37.8937 µs | −8.3943% −5.7363% −2.8237% | small_workflow | 161 |
| compile_ir_1000_steps | [−28.598% −24.014% −18.759%] | 6.6420 ms | 7.3252 ms | 11.0601 ms | +23.091% +31.603% +40.051% | generated_1000_steps | 43894 |
| validate_1000_steps | [−23.337% −17.202% −10.789%] | 6.3538 ms | 6.4431 ms | 6.5000 ms | +12.094% +20.775% +30.441% | generated_1000_steps | 43894 |
| expr_eq_symbol | [537.12 ns 594.59 ns 651.45 ns] | 431.79 ns | 959.38 ns | 1.1364 µs | — | expression | 17 |
| expr_number_compare | [887.62 ns 976.21 ns 1.0623 µs] | 708.59 ns | 2.2223 µs | 3.5552 µs | — | expression | 5 |
| expr_boolean_chain | [100.08 ns 101.77 ns 103.90 ns] | 99.36 ns | 116.26 ns | 133.07 ns | — | expression | 21 |
| expr_arithmetic | [878.05 ns 883.76 ns 890.37 ns] | 886.57 ns | 906.05 ns | 932.61 ns | — | expression | 9 |
| bench_engine_numeric_slots_read_write_i64 | [109.73 ns 110.63 ns 111.55 ns] | 108.53 ns | 121.67 ns | 158.55 ns | — | run_frame_slot | 161 |
| slot_read | [108.28 ns 109.26 ns 110.50 ns] | 107.25 ns | 128.95 ns | 184.66 ns | — | run_frame_slot | 161 |
| bench_engine_step_once_save_const_single_transition | [48.897 ns 49.003 ns 49.117 ns] | 48.84 ns | 49.66 ns | 49.95 ns | — | small_workflow | 161 |
| engine_run_until_blocked_budget_10_small_workflow | [48.875 ns 49.174 ns 49.557 ns] | 48.79 ns | 50.57 ns | 55.17 ns | — | small_workflow | 161 |
| bench_engine_run_save_chain_10_steps | [542.03 ns 546.41 ns 553.18 ns] | 540.81 ns | 555.71 ns | 585.12 ns | — | ir_save_chain_10 | 36 |
| bench_engine_run_save_chain_1000_steps | [38.125 µs 38.404 µs 38.862 µs] | 38.0559 µs | 41.0399 µs | 45.6668 µs | — | ir_save_chain_1000 | 38 |
| bench_engine_choose_true_branch | [255.28 ns 258.85 ns 262.94 ns] | 251.97 ns | 292.30 ns | 379.08 ns | — | ir_choose_slot_true | 31 |
| bench_engine_choose_false_branch | [254.73 ns 258.10 ns 263.30 ns] | 253.92 ns | 266.24 ns | 377.71 ns | — | ir_choose_slot_false | 32 |
| bench_engine_finish_no_observability | [159.55 ns 162.98 ns 168.92 ns] | 158.86 ns | 171.08 ns | 230.85 ns | — | ir_finish_only | 36 |
| bench_memory_ingress_try_submit_capacity_1024 | [33.982 µs 34.485 µs 35.281 µs] | 34.2280 µs | 35.6665 µs | 47.0967 µs | — | memory_ingress_1024 | 161 |
| bench_memory_ingress_submit_recv_single_thread | [128.26 ns 129.18 ns 130.27 ns] | 127.07 ns | 137.39 ns | 145.78 ns | — | memory_ingress_pair | 161 |
| bench_memory_ingress_backpressure_full_queue | [108.94 ns 124.11 ns 141.28 ns] | 75.25 ns | 502.99 ns | 765.30 ns | — | memory_ingress_full | 161 |
| postcard_encode_event | [263.14 ns 266.48 ns 270.15 ns] | 260.21 ns | 309.41 ns | 333.90 ns | — | run_accepted_event | 161 |
| postcard_decode_event | [548.96 ns 568.47 ns 601.04 ns] | 543.02 ns | 581.36 ns | 712.84 ns | — | run_accepted_event | 161 |
| ipc_frame_encode | [63.814 ns 64.618 ns 65.931 ns] | 63.62 ns | 67.16 ns | 84.09 ns | — | submit_run_payload | 161 |
| ipc_frame_decode | [235.04 ns 236.41 ns 238.30 ns] | 235.78 ns | 262.28 ns | 308.34 ns | — | submit_run_payload | 161 |
| bench_fjall_append_run_accepted_no_persist | [1.7430 µs 1.7940 µs 1.8624 µs] | 1.7176 µs | 2.3265 µs | 3.3505 µs | — | fjall_run_events | 161 |
| bench_replay_ordered_journal_1000_events | [825.23 µs 831.83 µs 839.17 µs] | 817.8451 µs | 861.4020 µs | 908.7458 µs | — | fjall_run_events_1000 | 161 |
| ir_execution_1_step | [162.44 ns 164.23 ns 166.45 ns] | 162.24 ns | 189.82 ns | 200.16 ns | — | finish_1 | 8 |
| ir_execution_1000_steps | [38.438 µs 38.885 µs 39.468 µs] | 38.1990 µs | 43.9818 µs | 52.4783 µs | — | save_chain_1000 | 15 |
| ir_execution_choose_100 | [48.933 ns 49.063 ns 49.202 ns] | 48.76 ns | 50.54 ns | 51.22 ns | — | choose_100 | 10 |
| ir_execution_expr | [48.783 ns 48.990 ns 49.268 ns] | 48.68 ns | 49.82 ns | 53.77 ns | — | expression_workflow | 10 |
| eval_expr_scalar_arithmetic_taint | [253.03 ns 258.20 ns 264.66 ns] | 251.47 ns | 266.84 ns | 327.88 ns | — | scalar_expr | 17 |
| eval_expr_slot_load_all_clean | [290.67 ns 292.61 ns 295.09 ns] | 289.22 ns | 301.62 ns | 746.84 ns | — | slot_load_clean | 16 |
| eval_expr_slot_load_mixed_taint | [294.60 ns 297.05 ns 300.12 ns] | 293.26 ns | 316.45 ns | 456.15 ns | — | slot_load_mixed | 16 |
| build_object_2_fields_taint | [49.370 ns 50.106 ns 51.181 ns] | 49.18 ns | 51.74 ns | 54.86 ns | — | build_object_2 | 2 |
| build_object_8_fields_taint | [48.939 ns 49.230 ns 49.703 ns] | 49.10 ns | 56.39 ns | 62.74 ns | — | build_object_8 | 2 |
| build_object_16_fields_taint | [49.098 ns 49.453 ns 49.911 ns] | 48.77 ns | 52.16 ns | 53.51 ns | — | build_object_16 | 2 |
| build_list_2_items_taint | [376.33 ns 379.45 ns 383.58 ns] | 375.23 ns | 402.82 ns | 456.57 ns | — | build_list_2 | 2 |
| build_list_8_items_taint | [804.37 ns 806.17 ns 807.88 ns] | 805.17 ns | 815.84 ns | 820.29 ns | — | build_list_8 | 2 |
| build_list_16_items_taint | [2.7803 µs 2.9983 µs 3.2187 µs] | 2.8010 µs | 5.9841 µs | 9.9775 µs | — | build_list_16 | 2 |
| full_workflow_all_clean | [76.119 ns 92.638 ns 110.57 ns] | 50.25 ns | 294.38 ns | 499.67 ns | — | full_workflow_clean | 16 |
| full_workflow_mixed_taint | [86.012 ns 102.16 ns 120.13 ns] | 51.85 ns | 341.16 ns | 846.12 ns | — | full_workflow_mixed | 16 |
| submit_artifact_relaxed | [49.473 ns 50.118 ns 51.027 ns] | 49.02 ns | 72.05 ns | 111.81 ns | — | small_workflow | 161 |
| submit_artifact_journaled | [48.798 ns 49.087 ns 49.435 ns] | 48.67 ns | 50.35 ns | 53.16 ns | — | small_workflow | 161 |
| submit_artifact_strict | [67.911 ns 75.121 ns 82.344 ns] | 49.81 ns | 171.47 ns | 316.12 ns | — | small_workflow | 161 |
| budget_compute_small_workflow | [60.753 ns 67.938 ns 75.180 ns] | 49.00 ns | 108.45 ns | 124.41 ns | — | small_workflow | 161 |
| budget_compute_save_chain_10 | [1.3781 µs 1.3853 µs 1.3933 µs] | 1.3667 µs | 1.4147 µs | 1.4345 µs | — | save_chain_10 | 13 |
| budget_compute_save_chain_1000 | [1.0578 ms 1.1458 ms 1.2431 ms] | 1.0456 ms | 1.7521 ms | 2.0868 ms | — | save_chain_1000 | 15 |
| budget_validate_default_policy | [94.824 ns 109.25 ns 123.62 ns] | 51.09 ns | 286.02 ns | 572.40 ns | — | small_workflow | 161 |
| evidence_chain_accumulate_100_events | [16.741 µs 18.253 µs 19.858 µs] | 16.4689 µs | 40.3441 µs | 73.7006 µs | — | volatile_journal_100 | 12 |
| evidence_chain_accumulate_1000_events | [156.04 µs 171.67 µs 187.37 µs] | 140.9421 µs | 376.4769 µs | 713.4425 µs | — | volatile_journal_1000 | 13 |
| evidence_chain_snapshot_100_events | [5.8616 µs 6.5425 µs 7.2154 µs] | 3.9493 µs | 14.1010 µs | 27.2079 µs | — | volatile_journal_snapshot_100 | 17 |
| admit_run_relaxed | [302.58 ns 357.07 ns 411.78 ns] | 163.51 ns | 990.93 ns | 2.0606 µs | — | always_present | 17 |
| admit_run_strict_artifact_present | [295.25 ns 364.37 ns 437.95 ns] | 182.50 ns | 1.3641 µs | 2.0917 µs | — | always_present | 16 |
| admit_run_multiple_action_caps | [409.32 ns 470.25 ns 533.00 ns] | 283.76 ns | 1.4075 µs | 1.8171 µs | — | always_present | 20 |
| admit_run_empty_caps | [229.75 ns 263.70 ns 297.41 ns] | 140.28 ns | 598.84 ns | 1.1434 µs | — | always_present | 20 |
| capability_check_any_workflow_grants | [110.02 ns 137.43 ns 165.52 ns] | 62.41 ns | 352.72 ns | 516.93 ns | — | any_workflow_set | 16 |
| capability_check_action_match_first | [117.88 ns 147.30 ns 178.48 ns] | 66.65 ns | 542.88 ns | 826.66 ns | — | action_set_10 | 16 |
| capability_check_action_miss | [89.774 ns 106.25 ns 124.18 ns] | 59.72 ns | 292.22 ns | 564.20 ns | — | action_set_10 | 15 |
| capability_check_empty_denies | [67.956 ns 78.493 ns 90.025 ns] | 50.30 ns | 156.25 ns | 223.00 ns | — | empty_set | 9 |
| capability_check_mixed_set | [95.184 ns 116.74 ns 140.00 ns] | 55.34 ns | 399.89 ns | 572.77 ns | — | mixed_set | 9 |
| capability_check_admission_gate | [261.50 ns 298.74 ns 336.50 ns] | 138.81 ns | 736.71 ns | 1.6905 µs | — | action_set_10 | 13 |

### velvet-ballistics-workspace-tests — vb_qi37_1_1_recovery (1 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| vb_qi37_1_1_no_output_recovery | [316.80 ns 322.96 ns 333.34 ns] | 318.74 ns | 331.09 ns | 351.48 ns | — | events_3 | 3 |

### velvet-ballistics-workspace-tests — vb_kkvb_xtask_routing_red (2 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| vb_kkvb_parse_ai_context | [513.00 ns 560.69 ns 610.76 ns] | 488.42 ns | 1.1856 µs | 1.3517 µs | — | argv_6_args | 41 |
| vb_kkvb_route_ai_context_deferred | [202.50 ns 221.63 ns 242.54 ns] | 148.44 ns | 441.55 ns | 865.19 ns | — | route_1_command | 1 |

### velvet-ballistics-workspace-tests — ir_traversal (6 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| ir_traverse_df_small | [29.080 ns 29.811 ns 30.674 ns] | 28.39 ns | 37.02 ns | 43.74 ns | 1.9432 GiB/s 1.9994 GiB/s 2.0497 GiB/s | chain_2 | 64 |
| ir_traverse_df_100_steps | [710.73 ns 713.56 ns 717.32 ns] | 708.36 ns | 758.11 ns | 867.98 ns | 1.3295 GiB/s 1.3365 GiB/s 1.3418 GiB/s | chain_100 | 1024 |
| ir_traverse_df_1000_steps | [7.2240 µs 7.2806 µs 7.3467 µs] | 7.1848 µs | 7.5491 µs | 8.0074 µs | 1.2981 GiB/s 1.3099 GiB/s 1.3202 GiB/s | chain_1000 | 10240 |
| ir_traverse_bfs_1000_steps | [7.1014 µs 7.1116 µs 7.1223 µs] | 7.0965 µs | 8.7313 µs | 9.1485 µs | 1.3390 GiB/s 1.3410 GiB/s 1.3429 GiB/s | chain_1000 | 10240 |
| ir_traverse_expr_programs | [14.522 ns 15.279 ns 16.132 ns] | 14.25 ns | 28.90 ns | 46.33 ns | 591.18 GiB/s 624.19 GiB/s 656.70 GiB/s | expr_10 | 512 |
| ir_traverse_mixed_kinds | [90.177 ns 102.87 ns 116.29 ns] | 62.03 ns | 215.24 ns | 290.41 ns | 82.006 GiB/s 92.705 GiB/s 105.76 GiB/s | mixed_wf | 256 |

### velvet-ballistics-workspace-tests — collect_page (5 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| collect_page_first_page_small | [263.30 ns 288.15 ns 312.91 ns] | 223.17 ns | 589.63 ns | 1.2196 µs | 304.78 MiB/s 330.96 MiB/s 362.21 MiB/s | list_100_page_50 | 100 |
| collect_page_second_page | [521.24 ns 567.08 ns 612.79 ns] | 384.40 ns | 1.0340 µs | 1.6858 µs | 155.63 MiB/s 168.17 MiB/s 182.96 MiB/s | list_100_page_50 | 100 |
| collect_page_exhausted | [320.21 ns 345.78 ns 369.97 ns] | 207.71 ns | 565.14 ns | 1.1183 µs | 257.77 MiB/s 275.81 MiB/s 297.83 MiB/s | list_100_page_50 | 100 |
| collect_page_large_10_pages | [1.4507 µs 1.5198 µs 1.6095 µs] | 1.3797 µs | 4.6438 µs | 6.6862 µs | 592.54 MiB/s 627.51 MiB/s 657.40 MiB/s | list_1000_page_100 | 1000 |
| collect_page_find_existing | [24.367 ns 25.202 ns 26.062 ns] | 22.96 ns | 29.16 ns | 33.11 ns | 3.8370 Gelem/s 3.9679 Gelem/s 4.1040 Gelem/s | 100_entry_table | 100 |

### velvet-ballistics-workspace-tests — action_dispatch (6 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| action_dispatch_single_registered | [134.53 ns 136.52 ns 139.13 ns] | 133.83 ns | 168.17 ns | 194.51 ns | 7.1877 Melem/s 7.3247 Melem/s 7.4331 Melem/s | 1_action | 1 |
| action_dispatch_10_registered | [131.24 ns 131.65 ns 132.25 ns] | 131.21 ns | 132.78 ns | 144.86 ns | 75.617 Melem/s 75.958 Melem/s 76.199 Melem/s | 10_actions | 10 |
| action_dispatch_100_registered | [138.01 ns 142.27 ns 150.44 ns] | 136.82 ns | 145.55 ns | 159.78 ns | 664.70 Melem/s 702.89 Melem/s 724.60 Melem/s | 100_actions | 100 |
| action_dispatch_unknown_action | [141.82 ns 148.46 ns 156.33 ns] | 136.76 ns | 179.48 ns | 224.45 ns | 639.67 Melem/s 673.60 Melem/s 705.12 Melem/s | 10_actions | 10 |
| action_dispatch_resolve_compile_time | [1.7600 ns 1.7744 ns 1.7931 ns] | 1.76 ns | 1.79 ns | 1.97 ns | 55.768 Gelem/s 56.359 Gelem/s 56.817 Gelem/s | 10_actions | 10 |
| action_dispatch_resolve_compile_time_varying | [2.7235 ns 2.7324 ns 2.7438 ns] | 2.72 ns | 2.82 ns | 3.13 ns | 36.446 Gelem/s 36.598 Gelem/s 36.718 Gelem/s | 10_actions | 10 |

### velvet-ballistics-workspace-tests — memory_footprint (2 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| memory_save_chain_1000_peak | [1.0755 ns 1.1481 ns 1.2421 ns] | 1.52 ns | 12.95 ns | 38.85 ns | 7678.0 GiB/s 8306.5 GiB/s 8867.0 GiB/s | save_chain_1000 | 10240 |
| memory_valuestore_growth | [1.9983 ns 2.1322 ns 2.3093 ns] | 2.83 ns | 24.05 ns | 72.14 ns | 433.02 Gelem/s 469.00 Gelem/s 500.42 Gelem/s | 1000_slot_writes | 1000 |

### velvet-ballistics-workspace-tests — cold_start (3 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| cold_start_100_steps | [66.202 ns 67.284 ns 68.555 ns] | 65.32 ns | 73.58 ns | 96.62 ns | 13.911 GiB/s 14.174 GiB/s 14.406 GiB/s | save_chain_100 | 1024 |
| cold_start_1000_steps | [74.216 ns 75.133 ns 76.171 ns] | 73.33 ns | 81.81 ns | 85.46 ns | 125.20 GiB/s 126.93 GiB/s 128.50 GiB/s | save_chain_1000 | 10240 |
| cold_start_full_pipeline | [683.11 µs 695.49 µs 709.03 µs] | 664.4963 µs | 806.4571 µs | 1.6321 ms | 5.7756 MiB/s 5.8881 MiB/s 5.9948 MiB/s | small_workflow_yaml | 4294 |

### velvet-ballistics-workspace-tests — pagination_cost (5 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| pagination_insert_first | [197.00 ns 202.35 ns 207.64 ns] | 185.61 ns | 259.60 ns | 310.75 ns | 4.8160 Melem/s 4.9420 Melem/s 5.0761 Melem/s | empty_table | 1 |
| pagination_upsert_second_page | [399.96 ns 412.29 ns 424.93 ns] | 388.43 ns | 578.72 ns | 683.08 ns | 2.3533 Melem/s 2.4255 Melem/s 2.5003 Melem/s | 1_entry_table | 1 |
| pagination_find_existing | [24.527 ns 25.112 ns 25.725 ns] | 23.38 ns | 28.90 ns | 33.06 ns | 3.8872 Gelem/s 3.9821 Gelem/s 4.0771 Gelem/s | 100_entry_table | 100 |
| pagination_find_missing | [20.240 ns 20.902 ns 21.684 ns] | 19.66 ns | 26.59 ns | 28.72 ns | 4.6118 Gelem/s 4.7842 Gelem/s 4.9407 Gelem/s | 100_entry_table | 100 |
| pagination_lineage_tracking | [1.8135 µs 1.9227 µs 2.0415 µs] | 1.6360 µs | 2.8469 µs | 3.3512 µs | 48.983 Melem/s 52.010 Melem/s 55.142 Melem/s | 10_page_lineage | 10 |

### velvet-ballistics-workspace-tests — action_queuing (5 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| action_queue_enqueue | [1.8231 µs 1.8941 µs 1.9864 µs] | 1.7492 µs | 4.5635 µs | 6.0511 µs | 503.43 Kelem/s 527.94 Kelem/s 548.52 Kelem/s | queue_empty_1024 | 1 |
| action_queue_dequeue | [91.568 µs 92.960 µs 95.149 µs] | 91.1582 µs | 116.0385 µs | 127.3491 µs | 1.0510 Melem/s 1.0757 Melem/s 1.0921 Melem/s | queue_100_items | 100 |
| action_queue_full_enqueue_err | [1.3248 µs 1.3447 µs 1.3783 µs] | 1.3277 µs | 1.3650 µs | 1.9137 µs | 72.551 Melem/s 74.364 Melem/s 75.481 Melem/s | queue_full | 1 |
| action_queue_batch_100 | [71.035 µs 71.957 µs 73.395 µs] | 70.6108 µs | 86.0897 µs | 95.2819 µs | 1.3625 Melem/s 1.3897 Melem/s 1.4078 Melem/s | queue_empty_1024 | 100 |
| action_queue_len_is_full | [689.05 µs 701.39 µs 717.88 µs] | 681.4918 µs | 759.2705 µs | 995.2207 µs | 1.4264 Melem/s 1.4600 Melem/s 1.4861 Melem/s | queue_1024_items | 1024 |

### velvet-ballistics-workspace-tests — timer_wheel_tick (7 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| timer_wheel_fire_empty | [34.844 ns 35.650 ns 36.363 ns] | 36.35 ns | 38.21 ns | 48.82 ns | 0.0000  elem/s 0.0000  elem/s 0.0000  elem/s | wheel_empty | 0 |
| timer_wheel_fire_1_expired | [263.24 ns 277.18 ns 292.71 ns] | 245.45 ns | 392.24 ns | 585.58 ns | 3.4163 Melem/s 3.6077 Melem/s 3.7988 Melem/s | wheel_1_expired | 1 |
| timer_wheel_fire_10_expired | [2.0585 µs 2.1638 µs 2.2626 µs] | 2.0289 µs | 3.7627 µs | 4.6698 µs | 4.4197 Melem/s 4.6215 Melem/s 4.8579 Melem/s | wheel_10_expired | 10 |
| timer_wheel_fire_90_of_100 | [11.539 µs 11.558 µs 11.578 µs] | 11.5312 µs | 11.6849 µs | 11.8043 µs | 8.6367 Melem/s 8.6521 Melem/s 8.6660 Melem/s | wheel_100_mixed | 100 |
| timer_wheel_cancel_1 | [11.991 µs 12.302 µs 12.650 µs] | 11.8107 µs | 12.8386 µs | 15.0526 µs | 7.9049 Melem/s 8.1288 Melem/s 8.3399 Melem/s | wheel_100_mixed | 100 |
| timer_wheel_next_deadline | [911.81 ns 918.00 ns 925.80 ns] | 906.38 ns | 1.0267 µs | 1.0624 µs | 108.01 Melem/s 108.93 Melem/s 109.67 Melem/s | wheel_10 | 10 |
| timer_wheel_insert_100 | [11.024 µs 11.701 µs 12.435 µs] | 10.2359 µs | 22.1497 µs | 35.6111 µs | 8.0415 Melem/s 8.5463 Melem/s 9.0709 Melem/s | wheel_empty | 100 |

### velvet-ballistics-workspace-tests — snapshot_save (5 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| snapshot_save_1_step | [248.08 ns 266.51 ns 287.28 ns] | 212.21 ns | 480.70 ns | 693.03 ns | 3.4809 Melem/s 3.7522 Melem/s 4.0309 Melem/s | frame_1_step | 1 |
| snapshot_save_50_steps | [345.65 ns 350.50 ns 356.85 ns] | 348.83 ns | 372.74 ns | 420.15 ns | 140.12 Melem/s 142.65 Melem/s 144.66 Melem/s | frame_50_steps | 50 |
| snapshot_save_large_slots | [231.17 ns 242.20 ns 253.26 ns] | 222.48 ns | 286.29 ns | 319.91 ns | 37.656 GiB/s 39.375 GiB/s 41.254 GiB/s | frame_10kb_slots | 10240 |
| snapshot_encode_postcard | [276.76 ns 291.20 ns 307.99 ns] | 247.85 ns | 447.70 ns | 484.91 ns | 3.0964 MiB/s 3.2750 MiB/s 3.4459 MiB/s | snapshot_small | 1 |
| snapshot_encode_max_correlation | [1.3514 µs 1.4616 µs 1.5780 µs] | 1.2913 µs | 2.7735 µs | 3.5756 µs | 618.86 KiB/s 668.16 KiB/s 722.65 KiB/s | snapshot_max_corr | 1 |

### velvet-ballistics-workspace-tests — snapshot_restore (5 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| snapshot_restore_1_step | [319.50 ns 356.53 ns 394.56 ns] | 209.47 ns | 862.76 ns | 1.3224 µs | 2.5345 Melem/s 2.8048 Melem/s 3.1299 Melem/s | snapshot_1_step | 1 |
| snapshot_restore_50_steps | [360.25 ns 402.11 ns 446.47 ns] | 293.91 ns | 853.04 ns | 1.4966 µs | 111.99 Melem/s 124.34 Melem/s 138.79 Melem/s | snapshot_50_steps | 50 |
| snapshot_restore_large_slots | [303.56 ns 332.44 ns 360.03 ns] | 212.39 ns | 805.70 ns | 1.4942 µs | 26.489 GiB/s 28.687 GiB/s 31.416 GiB/s | snapshot_10kb_slots | 10240 |
| snapshot_decode_postcard | [152.95 ns 164.58 ns 177.56 ns] | 155.77 ns | 324.58 ns | 457.92 ns | 53.710 GiB/s 57.946 GiB/s 62.350 GiB/s | snapshot_encoded_50 | 1 |
| snapshot_restore_correlation | [180.19 ns 195.99 ns 211.22 ns] | 133.90 ns | 453.21 ns | 839.36 ns | 45.150 GiB/s 48.660 GiB/s 52.927 GiB/s | snapshot_with_correlation | 1 |

### velvet-ballistics-workspace-tests — array_queue (6 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| array_queue_push_1 | [517.58 ns 544.98 ns 572.14 ns] | 464.83 ns | 696.98 ns | 756.17 ns | 1.7478 Melem/s 1.8349 Melem/s 1.9321 Melem/s | aq_empty_1024 | 1 |
| array_queue_pop_1 | [2.4618 µs 2.7057 µs 2.9336 µs] | 1.9438 µs | 3.9855 µs | 4.5723 µs | 34.088 Melem/s 36.959 Melem/s 40.621 Melem/s | aq_100_items | 100 |
| array_queue_push_full_err | [128.38 ns 138.98 ns 149.00 ns] | 90.63 ns | 312.78 ns | 536.72 ns | 671.12 Melem/s 719.55 Melem/s 778.95 Melem/s | aq_full_1 | 1 |
| array_queue_capacity_1024 | [18.876 µs 20.443 µs 22.032 µs] | 13.2134 µs | 45.0561 µs | 71.4343 µs | 46.479 Melem/s 50.090 Melem/s 54.248 Melem/s | aq_empty_1024 | 1024 |
| array_queue_is_full_len | [6.2036 µs 6.2427 µs 6.2933 µs] | 6.1682 µs | 6.3301 µs | 6.6877 µs | 162.71 Melem/s 164.03 Melem/s 165.06 Melem/s | aq_512_items | 512 |
| array_queue_fifo_1000 | [23.647 µs 23.732 µs 23.866 µs] | 23.5897 µs | 25.3393 µs | 27.3859 µs | 41.901 Melem/s 42.137 Melem/s 42.288 Melem/s | aq_1000_items | 1000 |

### velvet-ballistics-workspace-tests — rtrb (6 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| rtrb_push_1 | [125.68 ns 132.43 ns 140.52 ns] | 112.57 ns | 258.30 ns | 412.95 ns | 7.1164 Melem/s 7.5511 Melem/s 7.9567 Melem/s | rtrb_empty | 1 |
| rtrb_pop_1 | [487.51 ns 507.20 ns 530.78 ns] | 462.13 ns | 621.85 ns | 767.04 ns | 188.40 Melem/s 197.16 Melem/s 205.13 Melem/s | rtrb_100_items | 100 |
| rtrb_push_full_err | [589.30 ns 630.67 ns 681.34 ns] | 502.22 ns | 1.3182 µs | 2.0510 µs | 146.77 Melem/s 158.56 Melem/s 169.69 Melem/s | rtrb_full | 1 |
| rtrb_peek | [540.29 ns 545.35 ns 549.86 ns] | 528.30 ns | 569.07 ns | 571.21 ns | 181.87 Melem/s 183.37 Melem/s 185.09 Melem/s | rtrb_100_items | 100 |
| rtrb_is_full_is_empty | [337.01 ns 346.18 ns 356.65 ns] | 326.96 ns | 369.78 ns | 431.65 ns | 280.39 Melem/s 288.86 Melem/s 296.73 Melem/s | rtrb_50_items | 50 |
| rtrb_fifo_1000 | [4.2192 µs 4.3311 µs 4.4669 µs] | 4.1097 µs | 4.7834 µs | 6.0465 µs | 223.87 Melem/s 230.89 Melem/s 237.01 Melem/s | rtrb_1000_items | 1000 |
