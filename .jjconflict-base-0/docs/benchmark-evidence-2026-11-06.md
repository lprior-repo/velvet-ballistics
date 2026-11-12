# Benchmark evidence — 2026-11-06 (§44 Pt 22 re-measurement on current main)

## Run conditions

- **Main tip measured:** `5b08302cedb5d2891e674224c000f5a30a099bb1` ("fix(engine): split files that outgrew the source-length limit")
- **Measured:** 2026-11-07 00:03–00:57 CST (06:03–06:57 UTC), one session, targets run sequentially; the hammer run itself started 2026-11-06 23:34 CST
- **Branch measured:** `vb/pt22-remeasure-1106`, cut from the main tip above; no production code changed by this run
- **Harness:** criterion 0.8 (workspace pins `0.8`)
- **Toolchain:** pinned nightly-2026-04-28
- **Env for every command:** `PATH="$HOME/.moon/bin:$HOME/.cargo/bin:$PATH"`, `TMPDIR=$HOME/tmp`, `RUSTC_WRAPPER=""`, stdin from `/dev/null`, `-j2`
- **Env override for the velvet_ballistics target only:** `VB_BENCH_LATENCY_BUDGET_US=1000000` (see "Fjall append stalls" below); every other target ran at the default 100 ms per-iteration budget
- **Box:** VM, 4 CPUs, 7.7 GiB RAM, shared load; the VM rebooted once mid-run (~00:40 CST), killing the second velvet_ballistics attempt — see below
- **Criterion defaults (no overrides in any measured bench):** 3 s warmup, 100 samples, ~5 s measurement per benchmark
- **Metadata for every measured bench:** `durability=mixed`, `mode=ir` (all 129 benches); all 129 benches declare `fixture_bytes`

## How to read the numbers

- **time** = criterion's headline estimate (regression slope) with its 95% CI, exactly as printed to stdout.
- **p50** = median point estimate from criterion's own `estimates.json`.
- **p95 / p99** = percentiles computed from the harness's own raw samples (`sample.json`: per-sample time = `times[i]/iters[i]`, linear interpolation over the 100 sorted per-sample values). The computed p50 matches `estimates.json`'s median exactly for every bench, which validates the method. **These percentiles are over 100 sample means (each averaging thousands of iterations), NOT per-iteration percentiles** — they understate true per-iteration tail spread.
- **thrpt** = criterion throughput estimate with 95% CI, as printed (units as set by each bench). Benches without a `Throughput` setting have no thrpt value.
- **fixture / fixture_bytes** = the bench's own declared fixture metadata, taken from its full criterion bench id.
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

**Measured total: 129 benchmarks, 0 failures in the final pass.** Bench build proof: `cargo bench -p vb_core -p vb_validate -p velvet-ballistics-workspace-tests --no-run` exits 0 on main `5b08302c` (verified before measuring; 18 min build, 0 errors). `cargo bench -p vb_validate --bench capability_schema --no-run` exits 0 and the built binary run with `--list` reports 1 benchmark.

## What this closes relative to the 2026-11-04 evidence

- Fixture sizes for all five benches the 2026-11-04 doc listed as "not declared" are now declared in their bench sources and present in the criterion report ids: `aggregate_resource_budget_contract_surface` 115330 (`budget.rs` source length), `capability schema validation` 3 (2 nodes + 1 contract), `vb_qi37_1_1_no_output_recovery` 3 (3 events), `vb_kkvb_parse_ai_context` 41 (sum of the 6 argv lengths), `vb_kkvb_route_ai_context_deferred` 1 (1 command). The 2026-11-06 extraction reports 0 benches without `fixture_bytes`.
- The `action_dispatch_resolve_compile_time` 1.77 ns flag is resolved, not just re-measured. The new `action_dispatch_resolve_compile_time_varying` bench (added after 2026-11-04) cycles through all 10 registered ids with a black-boxed input and asserts each resolved contract id, so no single-id specialization can stand in for the lookup; it measures 2.73 ns [2.73, 2.73, 2.74]. The fixed-id bench measures 1.78 ns [1.77, 1.78, 1.79] on this run, the same order of magnitude. Both are consistent with `resolve_compile_time` being what it is — a vector index plus a match plus a filter — and not with the work being folded away. The sub-2 ns anomaly scan flags only the fixed-id bench, and the varying bench is the answer to that flag.
- One new bench since 2026-11-04: `action_dispatch_resolve_compile_time_varying` (action_dispatch is 6 benches now, 129 total vs 128).

## Flagged results (do not base claims on these without further work)

- `timer_wheel_fire_empty`: throughput `[0, 0, 0] elem/s`. Expected, not a failure: the bench sets `Throughput::Elements(0)` for the empty wheel.
- The 2026-05-01 rejected sub-nanosecond result recorded in `benchmark-suite.md` is unchanged by this run and remains rejected.
- `bench_fjall_append_run_accepted_no_persist`: mean 1.77 µs, p50 1.75 µs, p99 2.93 µs — the same order as 2026-11-04 (mean 1.77 µs, p99 2.15 µs), but the per-iteration tail on this box is much heavier than the sample statistics show. See "Fjall append stalls" below; the p99 here is a per-sample-mean percentile and does not capture single-iteration stalls.

## Fjall append stalls (why the velvet_ballistics target needed three attempts)

- The velvet_ballistics target's bench harness asserts every timed iteration stays inside a per-iteration latency budget (default 100 ms, overridable with `VB_BENCH_LATENCY_BUDGET_US`, a knob built into `crates/workspace_tests/benches/velvet_ballistics.rs`). Attempt 1 aborted in the `bench_fjall_append_run_accepted_no_persist` warmup: one append took 174 ms (budget 100 ms). Attempt 2 aborted at the same bench the same way. Attempt 3, with `VB_BENCH_LATENCY_BUDGET_US=1000000`, completed all 63 benches, exit 0; the slowest single fjall-append iteration in that run was 223 ms.
- The stalls are disk stalls on this shared VM, not a product regression: most append iterations are 10–400 µs, but spikes of 41 ms, 46 ms, 53 ms, 109 ms, 146 ms, 174 ms and 223 ms recur across runs, including in the run started minutes after the VM rebooted. The same bench measured 1.77 µs mean on 2026-11-04 on this box, when the disk was quieter. No production code changed between the attempts — the branch is unmodified main `5b08302c` — so the aborts are a property of this box's disk latency, not of the code under test.
- The 1 s budget does not change what is measured: criterion's mean, p50, p95 and p99 for this bench come from the same iterations, and a stalled iteration is one slow iteration among thousands per sample. It only stops one stall from aborting the whole target. Any future re-measurement on a quieter disk should re-try the default budget first and say so.

## VM reboot during the run

- The VM rebooted once at ~00:40 CST (uptime showed 1 min at 00:40:57 CST), killing the second velvet_ballistics attempt (retry2) at bench 25 of 63 with no error and no exit line. State survived: the branch, the built binaries and the criterion reports for the 92 benches measured up to that point were intact on disk afterwards. Attempt 3 ran after the reboot and completed the target.

## Still missing for a full §44 Pt 22 close

1. Instruction counts: criterion does not measure these; they need a counter harness (for example perf counters), which is a separate batch.
2. Allocation counts and bytes allocated: same counter-harness batch; criterion does not measure these.

## Measured results

### vb_core — aggregate_resource_budget (1 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| aggregate_resource_budget_contract_surface | [1.5666 µs 1.5709 µs 1.5757 µs] | 1.5732 µs | 1.6134 µs | 1.6370 µs | — | budget_rs_source | 115330 |

### vb_validate — capability_schema (1 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| capability schema validation | [144.67 ns 145.31 ns 146.17 ns] | 144.56 ns | 149.11 ns | 155.90 ns | — | workflow_2_nodes_1_contract | 3 |

### velvet-ballistics-workspace-tests — velvet_ballistics (63 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| parse_yaml_small | [+2.8761% +4.6528% +6.3265%] | 23.8162 µs | 25.9063 µs | 30.6999 µs | −5.9501% −4.4460% −2.7957% | small_workflow | 161 |
| parse_yaml_1mb | [−2.9536% −0.8614% +0.9242%] | 78.9570 µs | 85.7141 µs | 88.9842 µs | −0.9158% +0.8689% +3.0435% | generated_1mb_yaml | 1048640 |
| validate_minimal | [+1.8011% +2.2201% +2.6980%] | 26.7400 µs | 27.1758 µs | 27.4121 µs | −2.6272% −2.1719% −1.7692% | small_workflow | 161 |
| compile_ir_minimal | [+0.5923% +2.7532% +4.6766%] | 26.9345 µs | 29.0320 µs | 34.6480 µs | −4.4677% −2.6794% −0.5889% | small_workflow | 161 |
| compile_ir_1000_steps | [+3.2358% +3.5756% +3.9123%] | 6.4128 ms | 6.4949 ms | 6.5637 ms | −3.7650% −3.4521% −3.1344% | generated_1000_steps | 43894 |
| validate_1000_steps | [+6.6537% +9.5018% +12.259%] | 6.4929 ms | 8.2163 ms | 10.7504 ms | −10.920% −8.6773% −6.2386% | generated_1000_steps | 43894 |
| expr_eq_symbol | [372.14 ns 379.53 ns 388.50 ns] | 370.96 ns | 431.01 ns | 543.86 ns | — | expression | 17 |
| expr_number_compare | [447.41 ns 453.59 ns 460.50 ns] | 444.24 ns | 513.26 ns | 570.76 ns | — | expression | 5 |
| expr_boolean_chain | [102.30 ns 103.93 ns 105.71 ns] | 101.70 ns | 119.88 ns | 123.95 ns | — | expression | 21 |
| expr_arithmetic | [984.49 ns 998.48 ns 1.0232 µs] | 988.72 ns | 1.0927 µs | 1.1510 µs | — | expression | 9 |
| bench_engine_numeric_slots_read_write_i64 | [106.49 ns 106.72 ns 106.98 ns] | 106.19 ns | 109.00 ns | 110.70 ns | — | run_frame_slot | 161 |
| slot_read | [108.06 ns 109.46 ns 111.21 ns] | 106.56 ns | 137.52 ns | 139.12 ns | — | run_frame_slot | 161 |
| bench_engine_step_once_save_const_single_transition | [48.713 ns 49.342 ns 50.363 ns] | 48.54 ns | 50.00 ns | 52.96 ns | — | small_workflow | 161 |
| engine_run_until_blocked_budget_10_small_workflow | [48.711 ns 49.153 ns 49.767 ns] | 48.57 ns | 49.47 ns | 60.42 ns | — | small_workflow | 161 |
| bench_engine_run_save_chain_10_steps | [539.27 ns 540.88 ns 542.56 ns] | 539.53 ns | 551.10 ns | 558.06 ns | — | ir_save_chain_10 | 36 |
| bench_engine_run_save_chain_1000_steps | [38.267 µs 38.985 µs 40.297 µs] | 37.9799 µs | 48.3630 µs | 49.6371 µs | — | ir_save_chain_1000 | 38 |
| bench_engine_choose_true_branch | [251.44 ns 252.54 ns 253.74 ns] | 250.73 ns | 261.37 ns | 265.45 ns | — | ir_choose_slot_true | 31 |
| bench_engine_choose_false_branch | [259.00 ns 260.07 ns 261.20 ns] | 258.02 ns | 272.16 ns | 287.64 ns | — | ir_choose_slot_false | 32 |
| bench_engine_finish_no_observability | [165.42 ns 168.22 ns 171.41 ns] | 161.86 ns | 184.73 ns | 240.58 ns | — | ir_finish_only | 36 |
| bench_memory_ingress_try_submit_capacity_1024 | [34.026 µs 34.466 µs 35.023 µs] | 34.0657 µs | 39.7577 µs | 49.6125 µs | — | memory_ingress_1024 | 161 |
| bench_memory_ingress_submit_recv_single_thread | [126.07 ns 126.42 ns 126.77 ns] | 126.20 ns | 129.25 ns | 130.31 ns | — | memory_ingress_pair | 161 |
| bench_memory_ingress_backpressure_full_queue | [71.300 ns 71.797 ns 72.295 ns] | 70.90 ns | 74.81 ns | 76.48 ns | — | memory_ingress_full | 161 |
| postcard_encode_event | [256.92 ns 260.67 ns 265.10 ns] | 253.22 ns | 278.06 ns | 319.63 ns | — | run_accepted_event | 161 |
| postcard_decode_event | [538.52 ns 539.43 ns 540.36 ns] | 538.10 ns | 544.10 ns | 550.34 ns | — | run_accepted_event | 161 |
| ipc_frame_encode | [63.265 ns 63.627 ns 64.201 ns] | 63.22 ns | 65.93 ns | 68.90 ns | — | submit_run_payload | 161 |
| ipc_frame_decode | [238.24 ns 239.20 ns 240.40 ns] | 237.83 ns | 252.97 ns | 258.74 ns | — | submit_run_payload | 161 |
| bench_fjall_append_run_accepted_no_persist | [1.7607 µs 1.7667 µs 1.7737 µs] | 1.7543 µs | 1.8728 µs | 2.9301 µs | — | fjall_run_events | 161 |
| bench_replay_ordered_journal_1000_events | [810.48 µs 821.64 µs 836.98 µs] | 805.1710 µs | 852.2221 µs | 1.1410 ms | — | fjall_run_events_1000 | 161 |
| ir_execution_1_step | [159.53 ns 159.91 ns 160.41 ns] | 159.53 ns | 161.89 ns | 163.58 ns | — | finish_1 | 8 |
| ir_execution_1000_steps | [38.170 µs 38.244 µs 38.324 µs] | 38.1106 µs | 39.5058 µs | 40.7592 µs | — | save_chain_1000 | 15 |
| ir_execution_choose_100 | [48.772 ns 49.098 ns 49.575 ns] | 48.70 ns | 50.87 ns | 57.29 ns | — | choose_100 | 10 |
| ir_execution_expr | [49.066 ns 49.431 ns 49.860 ns] | 48.72 ns | 53.14 ns | 57.29 ns | — | expression_workflow | 10 |
| eval_expr_scalar_arithmetic_taint | [251.85 ns 252.85 ns 253.90 ns] | 254.03 ns | 278.77 ns | 313.90 ns | — | scalar_expr | 17 |
| eval_expr_slot_load_all_clean | [291.04 ns 292.26 ns 293.68 ns] | 291.09 ns | 310.74 ns | 347.55 ns | — | slot_load_clean | 16 |
| eval_expr_slot_load_mixed_taint | [293.81 ns 295.34 ns 296.83 ns] | 293.35 ns | 304.66 ns | 310.21 ns | — | slot_load_mixed | 16 |
| build_object_2_fields_taint | [48.789 ns 48.941 ns 49.128 ns] | 48.73 ns | 49.88 ns | 50.98 ns | — | build_object_2 | 2 |
| build_object_8_fields_taint | [48.909 ns 49.052 ns 49.216 ns] | 48.93 ns | 50.68 ns | 51.44 ns | — | build_object_8 | 2 |
| build_object_16_fields_taint | [48.365 ns 48.401 ns 48.445 ns] | 48.40 ns | 50.09 ns | 52.51 ns | — | build_object_16 | 2 |
| build_list_2_items_taint | [380.85 ns 385.34 ns 390.75 ns] | 377.21 ns | 412.49 ns | 493.27 ns | — | build_list_2 | 2 |
| build_list_8_items_taint | [800.96 ns 803.96 ns 807.23 ns] | 800.44 ns | 832.13 ns | 840.60 ns | — | build_list_8 | 2 |
| build_list_16_items_taint | [1.3073 µs 1.3202 µs 1.3355 µs] | 1.2961 µs | 1.4529 µs | 1.8098 µs | — | build_list_16 | 2 |
| full_workflow_all_clean | [47.931 ns 47.967 ns 48.003 ns] | 47.92 ns | 48.29 ns | 49.13 ns | — | full_workflow_clean | 16 |
| full_workflow_mixed_taint | [49.119 ns 49.388 ns 49.695 ns] | 48.76 ns | 50.82 ns | 52.96 ns | — | full_workflow_mixed | 16 |
| submit_artifact_relaxed | [49.207 ns 49.625 ns 50.208 ns] | 48.90 ns | 56.51 ns | 73.91 ns | — | small_workflow | 161 |
| submit_artifact_journaled | [48.469 ns 48.529 ns 48.600 ns] | 48.44 ns | 49.52 ns | 49.80 ns | — | small_workflow | 161 |
| submit_artifact_strict | [48.054 ns 48.209 ns 48.493 ns] | 48.02 ns | 48.45 ns | 49.34 ns | — | small_workflow | 161 |
| budget_compute_small_workflow | [48.502 ns 48.570 ns 48.649 ns] | 48.51 ns | 50.28 ns | 52.20 ns | — | small_workflow | 161 |
| budget_compute_save_chain_10 | [1.3503 µs 1.3672 µs 1.3929 µs] | 1.3365 µs | 1.3904 µs | 1.5139 µs | — | save_chain_10 | 13 |
| budget_compute_save_chain_1000 | [725.19 µs 728.34 µs 732.51 µs] | 725.8654 µs | 751.9141 µs | 757.7084 µs | — | save_chain_1000 | 15 |
| budget_validate_default_policy | [48.767 ns 49.067 ns 49.453 ns] | 48.70 ns | 50.66 ns | 52.14 ns | — | small_workflow | 161 |
| evidence_chain_accumulate_100_events | [7.5223 µs 7.5625 µs 7.6008 µs] | 7.4595 µs | 7.7549 µs | 7.7909 µs | — | volatile_journal_100 | 12 |
| evidence_chain_accumulate_1000_events | [70.512 µs 70.963 µs 71.394 µs] | 69.7931 µs | 73.1792 µs | 85.6287 µs | — | volatile_journal_1000 | 13 |
| evidence_chain_snapshot_100_events | [2.6006 µs 2.6239 µs 2.6613 µs] | 2.6065 µs | 2.6749 µs | 2.6898 µs | — | volatile_journal_snapshot_100 | 17 |
| admit_run_relaxed | [97.161 ns 99.584 ns 103.59 ns] | 96.64 ns | 104.19 ns | 115.78 ns | — | always_present | 17 |
| admit_run_strict_artifact_present | [146.10 ns 147.14 ns 148.12 ns] | 146.11 ns | 154.21 ns | 168.51 ns | — | always_present | 16 |
| admit_run_multiple_action_caps | [150.08 ns 151.65 ns 153.91 ns] | 149.17 ns | 154.65 ns | 177.97 ns | — | always_present | 20 |
| admit_run_empty_caps | [84.138 ns 85.065 ns 86.106 ns] | 84.29 ns | 91.16 ns | 95.01 ns | — | always_present | 20 |
| capability_check_any_workflow_grants | [50.255 ns 50.664 ns 51.200 ns] | 49.99 ns | 53.44 ns | 59.56 ns | — | any_workflow_set | 16 |
| capability_check_action_match_first | [54.328 ns 54.393 ns 54.469 ns] | 54.30 ns | 55.22 ns | 56.42 ns | — | action_set_10 | 16 |
| capability_check_action_miss | [54.946 ns 55.119 ns 55.331 ns] | 54.71 ns | 61.21 ns | 66.64 ns | — | action_set_10 | 15 |
| capability_check_empty_denies | [49.503 ns 49.811 ns 50.217 ns] | 49.49 ns | 52.31 ns | 54.89 ns | — | empty_set | 9 |
| capability_check_mixed_set | [50.881 ns 51.053 ns 51.252 ns] | 50.75 ns | 52.85 ns | 53.35 ns | — | mixed_set | 9 |
| capability_check_admission_gate | [119.64 ns 120.38 ns 121.48 ns] | 119.61 ns | 131.38 ns | 157.10 ns | — | action_set_10 | 13 |

### velvet-ballistics-workspace-tests — vb_qi37_1_1_recovery (1 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| vb_qi37_1_1_no_output_recovery | [321.66 ns 324.83 ns 328.76 ns] | 321.37 ns | 374.93 ns | 408.00 ns | — | events_3 | 3 |

### velvet-ballistics-workspace-tests — vb_kkvb_xtask_routing_red (2 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| vb_kkvb_parse_ai_context | [258.94 ns 259.54 ns 260.13 ns] | 259.84 ns | 265.51 ns | 267.57 ns | — | argv_6_args | 41 |
| vb_kkvb_route_ai_context_deferred | [141.50 ns 143.27 ns 145.56 ns] | 141.00 ns | 146.79 ns | 183.03 ns | — | route_1_command | 1 |

### velvet-ballistics-workspace-tests — ir_traversal (6 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| ir_traverse_df_small | [27.992 ns 28.041 ns 28.103 ns] | 27.98 ns | 28.56 ns | 29.71 ns | 2.1209 GiB/s 2.1257 GiB/s 2.1294 GiB/s | chain_2 | 64 |
| ir_traverse_df_100_steps | [703.61 ns 709.82 ns 721.96 ns] | 703.51 ns | 711.84 ns | 719.24 ns | 1.3210 GiB/s 1.3435 GiB/s 1.3554 GiB/s | chain_100 | 1024 |
| ir_traverse_df_1000_steps | [7.0864 µs 7.0931 µs 7.1014 µs] | 7.0870 µs | 7.1361 µs | 7.2241 µs | 1.3429 GiB/s 1.3445 GiB/s 1.3458 GiB/s | chain_1000 | 10240 |
| ir_traverse_bfs_1000_steps | [7.1199 µs 7.1316 µs 7.1441 µs] | 7.1371 µs | 7.2277 µs | 7.2864 µs | 1.3349 GiB/s 1.3373 GiB/s 1.3394 GiB/s | chain_1000 | 10240 |
| ir_traverse_expr_programs | [8.1308 ns 8.1553 ns 8.1837 ns] | 8.10 ns | 8.23 ns | 8.43 ns | 1165.3 GiB/s 1169.4 GiB/s 1172.9 GiB/s | expr_10 | 512 |
| ir_traverse_mixed_kinds | [40.173 ns 40.223 ns 40.282 ns] | 40.14 ns | 40.52 ns | 40.69 ns | 236.75 GiB/s 237.09 GiB/s 237.39 GiB/s | mixed_wf | 256 |

### velvet-ballistics-workspace-tests — collect_page (5 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| collect_page_first_page_small | [162.43 ns 162.70 ns 163.02 ns] | 162.26 ns | 164.75 ns | 170.19 ns | 585.02 MiB/s 586.15 MiB/s 587.13 MiB/s | list_100_page_50 | 100 |
| collect_page_second_page | [292.63 ns 293.06 ns 293.56 ns] | 293.44 ns | 297.99 ns | 307.47 ns | 324.86 MiB/s 325.42 MiB/s 325.90 MiB/s | list_100_page_50 | 100 |
| collect_page_exhausted | [194.01 ns 194.52 ns 195.16 ns] | 194.09 ns | 200.02 ns | 212.68 ns | 488.66 MiB/s 490.26 MiB/s 491.55 MiB/s | list_100_page_50 | 100 |
| collect_page_large_10_pages | [1.3541 µs 1.3564 µs 1.3592 µs] | 1.3520 µs | 1.3671 µs | 1.4243 µs | 701.66 MiB/s 703.12 MiB/s 704.27 MiB/s | list_1000_page_100 | 1000 |
| collect_page_find_existing | [24.029 ns 24.342 ns 24.727 ns] | 23.93 ns | 25.32 ns | 29.06 ns | 4.0442 Gelem/s 4.1082 Gelem/s 4.1616 Gelem/s | 100_entry_table | 100 |

### velvet-ballistics-workspace-tests — action_dispatch (6 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| action_dispatch_single_registered | [131.84 ns 132.87 ns 134.29 ns] | 131.41 ns | 152.74 ns | 157.91 ns | 7.4465 Melem/s 7.5263 Melem/s 7.5849 Melem/s | 1_action | 1 |
| action_dispatch_10_registered | [131.97 ns 133.22 ns 134.62 ns] | 131.07 ns | 137.15 ns | 145.73 ns | 74.283 Melem/s 75.063 Melem/s 75.776 Melem/s | 10_actions | 10 |
| action_dispatch_100_registered | [136.97 ns 137.23 ns 137.56 ns] | 136.76 ns | 140.07 ns | 143.73 ns | 726.98 Melem/s 728.70 Melem/s 730.08 Melem/s | 100_actions | 100 |
| action_dispatch_unknown_action | [136.79 ns 137.80 ns 139.05 ns] | 136.52 ns | 139.32 ns | 147.52 ns | 719.15 Melem/s 725.69 Melem/s 731.07 Melem/s | 10_actions | 10 |
| action_dispatch_resolve_compile_time | [1.7730 ns 1.7792 ns 1.7866 ns] | 1.77 ns | 1.87 ns | 1.91 ns | 55.971 Gelem/s 56.204 Gelem/s 56.402 Gelem/s | 10_actions | 10 |
| action_dispatch_resolve_compile_time_varying | [2.7263 ns 2.7327 ns 2.7403 ns] | 2.73 ns | 2.84 ns | 2.94 ns | 36.493 Gelem/s 36.594 Gelem/s 36.680 Gelem/s | 10_actions | 10 |

### velvet-ballistics-workspace-tests — memory_footprint (2 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| memory_save_chain_1000_peak | [1.0761 ns 1.1481 ns 1.2427 ns] | 1.52 ns | 12.95 ns | 38.85 ns | 7674.0 GiB/s 8306.5 GiB/s 8862.2 GiB/s | save_chain_1000 | 10240 |
| memory_valuestore_growth | [1.7482 ns 1.8657 ns 2.0196 ns] | 2.48 ns | 21.04 ns | 63.13 ns | 495.14 Gelem/s 536.00 Gelem/s 572.01 Gelem/s | 1000_slot_writes | 1000 |

### velvet-ballistics-workspace-tests — cold_start (3 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| cold_start_100_steps | [65.308 ns 65.742 ns 66.237 ns] | 65.42 ns | 66.84 ns | 72.84 ns | 14.398 GiB/s 14.506 GiB/s 14.603 GiB/s | save_chain_100 | 1024 |
| cold_start_1000_steps | [98.489 ns 99.472 ns 100.67 ns] | 97.77 ns | 106.43 ns | 114.71 ns | 94.729 GiB/s 95.873 GiB/s 96.830 GiB/s | save_chain_1000 | 10240 |
| cold_start_full_pipeline | [637.56 µs 644.07 µs 652.08 µs] | 634.7107 µs | 711.4490 µs | 744.6028 µs | 6.2801 MiB/s 6.3581 MiB/s 6.4230 MiB/s | small_workflow_yaml | 4294 |

### velvet-ballistics-workspace-tests — pagination_cost (5 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| pagination_insert_first | [161.90 ns 163.23 ns 164.99 ns] | 160.90 ns | 173.66 ns | 195.70 ns | 6.0611 Melem/s 6.1265 Melem/s 6.1765 Melem/s | empty_table | 1 |
| pagination_upsert_second_page | [333.96 ns 336.64 ns 339.61 ns] | 332.12 ns | 366.83 ns | 441.65 ns | 2.9446 Melem/s 2.9705 Melem/s 2.9944 Melem/s | 1_entry_table | 1 |
| pagination_find_existing | [23.297 ns 23.378 ns 23.475 ns] | 23.29 ns | 24.34 ns | 27.49 ns | 4.2599 Gelem/s 4.2775 Gelem/s 4.2924 Gelem/s | 100_entry_table | 100 |
| pagination_find_missing | [19.985 ns 20.319 ns 20.729 ns] | 19.67 ns | 24.66 ns | 34.14 ns | 4.8241 Gelem/s 4.9214 Gelem/s 5.0037 Gelem/s | 100_entry_table | 100 |
| pagination_lineage_tracking | [1.3616 µs 1.3678 µs 1.3752 µs] | 1.3590 µs | 1.3974 µs | 1.4560 µs | 72.717 Melem/s 73.113 Melem/s 73.445 Melem/s | 10_page_lineage | 10 |

### velvet-ballistics-workspace-tests — action_queuing (5 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| action_queue_enqueue | [1.7164 µs 1.7183 µs 1.7206 µs] | 1.7151 µs | 1.7453 µs | 1.7790 µs | 581.20 Kelem/s 581.96 Kelem/s 582.62 Kelem/s | queue_empty_1024 | 1 |
| action_queue_dequeue | [89.838 µs 89.969 µs 90.124 µs] | 90.1448 µs | 94.0819 µs | 96.8219 µs | 1.1096 Melem/s 1.1115 Melem/s 1.1131 Melem/s | queue_100_items | 100 |
| action_queue_full_enqueue_err | [1.3772 µs 1.3805 µs 1.3849 µs] | 1.3816 µs | 1.3981 µs | 1.5362 µs | 72.210 Melem/s 72.436 Melem/s 72.610 Melem/s | queue_full | 1 |
| action_queue_batch_100 | [70.309 µs 70.600 µs 70.945 µs] | 70.0236 µs | 71.9747 µs | 75.1338 µs | 1.4095 Melem/s 1.4164 Melem/s 1.4223 Melem/s | queue_empty_1024 | 100 |
| action_queue_len_is_full | [682.80 µs 684.12 µs 686.30 µs] | 683.5168 µs | 690.6253 µs | 710.8034 µs | 1.4921 Melem/s 1.4968 Melem/s 1.4997 Melem/s | queue_1024_items | 1024 |

### velvet-ballistics-workspace-tests — timer_wheel_tick (7 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| timer_wheel_fire_empty | [32.257 ns 33.215 ns 34.244 ns] | 35.87 ns | 37.58 ns | 38.01 ns | 0.0000  elem/s 0.0000  elem/s 0.0000  elem/s | wheel_empty | 0 |
| timer_wheel_fire_1_expired | [189.58 ns 190.77 ns 192.03 ns] | 188.66 ns | 198.91 ns | 203.39 ns | 5.2075 Melem/s 5.2418 Melem/s 5.2747 Melem/s | wheel_1_expired | 1 |
| timer_wheel_fire_10_expired | [1.4046 µs 1.4157 µs 1.4286 µs] | 1.3930 µs | 1.4854 µs | 1.6176 µs | 6.9997 Melem/s 7.0638 Melem/s 7.1197 Melem/s | wheel_10_expired | 10 |
| timer_wheel_fire_90_of_100 | [11.986 µs 12.010 µs 12.036 µs] | 11.9455 µs | 14.5587 µs | 16.9878 µs | 8.3087 Melem/s 8.3263 Melem/s 8.3431 Melem/s | wheel_100_mixed | 100 |
| timer_wheel_cancel_1 | [11.894 µs 11.912 µs 11.932 µs] | 11.8872 µs | 12.0750 µs | 12.1554 µs | 8.3807 Melem/s 8.3952 Melem/s 8.4079 Melem/s | wheel_100_mixed | 100 |
| timer_wheel_next_deadline | [900.86 ns 916.31 ns 938.37 ns] | 897.66 ns | 937.10 ns | 1.0185 µs | 106.57 Melem/s 109.13 Melem/s 111.00 Melem/s | wheel_10 | 10 |
| timer_wheel_insert_100 | [7.4210 µs 7.4509 µs 7.4816 µs] | 7.4126 µs | 7.6522 µs | 7.7186 µs | 13.366 Melem/s 13.421 Melem/s 13.475 Melem/s | wheel_empty | 100 |

### velvet-ballistics-workspace-tests — snapshot_save (5 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| snapshot_save_1_step | [192.41 ns 193.31 ns 194.56 ns] | 192.20 ns | 197.88 ns | 218.33 ns | 5.1398 Melem/s 5.1730 Melem/s 5.1972 Melem/s | frame_1_step | 1 |
| snapshot_save_50_steps | [331.01 ns 333.22 ns 335.65 ns] | 329.24 ns | 347.30 ns | 364.20 ns | 148.96 Melem/s 150.05 Melem/s 151.05 Melem/s | frame_50_steps | 50 |
| snapshot_save_large_slots | [201.68 ns 202.28 ns 203.01 ns] | 201.90 ns | 204.46 ns | 207.09 ns | 46.978 GiB/s 47.147 GiB/s 47.286 GiB/s | frame_10kb_slots | 10240 |
| snapshot_encode_postcard | [237.28 ns 238.51 ns 240.07 ns] | 236.37 ns | 244.92 ns | 285.80 ns | 3.9724 MiB/s 3.9985 MiB/s 4.0192 MiB/s | snapshot_small | 1 |
| snapshot_encode_max_correlation | [602.13 ns 607.93 ns 613.75 ns] | 592.57 ns | 644.35 ns | 723.43 ns | 1.5539 MiB/s 1.5687 MiB/s 1.5838 MiB/s | snapshot_max_corr | 1 |

### velvet-ballistics-workspace-tests — snapshot_restore (5 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| snapshot_restore_1_step | [147.81 ns 148.30 ns 148.88 ns] | 147.94 ns | 151.48 ns | 157.97 ns | 6.7168 Melem/s 6.7433 Melem/s 6.7653 Melem/s | snapshot_1_step | 1 |
| snapshot_restore_50_steps | [148.35 ns 149.24 ns 150.24 ns] | 147.96 ns | 156.25 ns | 177.63 ns | 332.79 Melem/s 335.03 Melem/s 337.05 Melem/s | snapshot_50_steps | 50 |
| snapshot_restore_large_slots | [144.48 ns 145.37 ns 146.19 ns] | 146.07 ns | 148.26 ns | 152.04 ns | 65.233 GiB/s 65.605 GiB/s 66.009 GiB/s | snapshot_10kb_slots | 10240 |
| snapshot_decode_postcard | [85.589 ns 85.899 ns 86.220 ns] | 85.00 ns | 89.93 ns | 104.32 ns | 110.61 GiB/s 111.02 GiB/s 111.43 GiB/s | snapshot_encoded_50 | 1 |
| snapshot_restore_correlation | [87.789 ns 88.340 ns 89.048 ns] | 87.91 ns | 89.76 ns | 99.23 ns | 107.10 GiB/s 107.95 GiB/s 108.63 GiB/s | snapshot_with_correlation | 1 |

### velvet-ballistics-workspace-tests — array_queue (6 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| array_queue_push_1 | [454.96 ns 459.52 ns 466.24 ns] | 457.32 ns | 478.72 ns | 507.00 ns | 2.1448 Melem/s 2.1762 Melem/s 2.1980 Melem/s | aq_empty_1024 | 1 |
| array_queue_pop_1 | [1.5972 µs 1.6205 µs 1.6624 µs] | 1.6042 µs | 1.7239 µs | 1.8410 µs | 60.154 Melem/s 61.710 Melem/s 62.608 Melem/s | aq_100_items | 100 |
| array_queue_push_full_err | [63.708 ns 63.766 ns 63.827 ns] | 63.71 ns | 64.27 ns | 65.80 ns | 1.5667 Gelem/s 1.5682 Gelem/s 1.5697 Gelem/s | aq_full_1 | 1 |
| array_queue_capacity_1024 | [12.428 µs 12.441 µs 12.458 µs] | 12.4433 µs | 12.6188 µs | 12.9482 µs | 82.198 Melem/s 82.306 Melem/s 82.396 Melem/s | aq_empty_1024 | 1024 |
| array_queue_is_full_len | [6.1924 µs 6.2074 µs 6.2251 µs] | 6.1813 µs | 6.3798 µs | 6.5439 µs | 164.50 Melem/s 164.96 Melem/s 165.36 Melem/s | aq_512_items | 512 |
| array_queue_fifo_1000 | [23.342 µs 23.396 µs 23.457 µs] | 23.3029 µs | 23.9509 µs | 24.9628 µs | 42.632 Melem/s 42.743 Melem/s 42.842 Melem/s | aq_1000_items | 1000 |

### velvet-ballistics-workspace-tests — rtrb (6 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| rtrb_push_1 | [106.98 ns 107.13 ns 107.30 ns] | 106.98 ns | 108.52 ns | 110.23 ns | 9.3199 Melem/s 9.3345 Melem/s 9.3471 Melem/s | rtrb_empty | 1 |
| rtrb_pop_1 | [334.46 ns 336.51 ns 339.43 ns] | 334.27 ns | 392.81 ns | 511.31 ns | 294.61 Melem/s 297.17 Melem/s 298.99 Melem/s | rtrb_100_items | 100 |
| rtrb_push_full_err | [409.79 ns 411.75 ns 413.91 ns] | 408.04 ns | 424.35 ns | 448.33 ns | 241.60 Melem/s 242.86 Melem/s 244.03 Melem/s | rtrb_full | 1 |
| rtrb_peek | [424.41 ns 427.67 ns 431.32 ns] | 432.27 ns | 448.17 ns | 460.33 ns | 231.85 Melem/s 233.83 Melem/s 235.62 Melem/s | rtrb_100_items | 100 |
| rtrb_is_full_is_empty | [202.58 ns 205.16 ns 209.47 ns] | 202.00 ns | 209.58 ns | 216.70 ns | 477.39 Melem/s 487.41 Melem/s 493.62 Melem/s | rtrb_50_items | 50 |
| rtrb_fifo_1000 | [4.0178 µs 4.0742 µs 4.1412 µs] | 3.9537 µs | 4.2290 µs | 4.8390 µs | 241.48 Melem/s 245.45 Melem/s 248.89 Melem/s | rtrb_1000_items | 1000 |
