# Benchmark evidence — 2026-11-04 (§44 Pt 22 re-measurement on current main)

## Run conditions

- **Main tip measured:** `260ad7c4084a8d218e04f54e6d3ec0d708cf9324` ("[vb-82snf] Fuzz & tests: recovery corruption assertions, mutation strength")
- **Measured:** 2026-11-04 03:33–04:05 CST (09:33–10:05 UTC), one session, targets run sequentially
- **Branch measured:** `vb/pt22-remeasure`, cut from the main tip above; no production code changed by this run
- **Harness:** criterion 0.8 (workspace pins `0.8`)
- **Toolchain:** pinned nightly-2026-04-28
- **Env for every command:** `PATH="$HOME/.moon/bin:$HOME/.cargo/bin:$PATH"`, `TMPDIR=$HOME/tmp`, `RUSTC_WRAPPER=""`, stdin from `/dev/null`, `-j2`
- **Box:** VM, 4 CPUs, 7 GiB RAM, shared load
- **Criterion defaults (no overrides in any measured bench):** 3 s warmup, 100 samples, ~5 s measurement per benchmark
- **Metadata for every measured bench:** `durability=mixed`, `mode=ir` (all 128 benches); 123 benches declare `fixture_bytes`, 5 do not (listed below)

## How to read the numbers

- **time** = criterion's headline estimate (regression slope) with its 95% CI, exactly as printed to stdout.
- **p50** = median point estimate from criterion's own `estimates.json`.
- **p95 / p99** = percentiles computed from the harness's own raw samples (`sample.json`: per-sample time = `times[i]/iters[i]`, linear interpolation over the 100 sorted per-sample values). The computed p50 matches `estimates.json`'s median exactly for every bench, which validates the method. **These percentiles are over 100 sample means (each averaging thousands of iterations), NOT per-iteration percentiles** — they understate true per-iteration tail spread.
- **thrpt** = criterion throughput estimate with 95% CI, as printed (units as set by each bench). Benches without a `Throughput` setting have no thrpt value.
- **fixture / fixture_bytes** = the bench's own declared fixture metadata, taken from its full criterion bench id. `not declared` means the bench source does not pass a fixture size into its metadata string; no value was invented for it.
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
| `action_dispatch` | velvet-ballistics-workspace-tests | 5 / 5 |
| `memory_footprint` | velvet-ballistics-workspace-tests | 2 / 2 |
| `cold_start` | velvet-ballistics-workspace-tests | 3 / 3 |
| `pagination_cost` | velvet-ballistics-workspace-tests | 5 / 5 |
| `action_queuing` | velvet-ballistics-workspace-tests | 5 / 5 |
| `timer_wheel_tick` | velvet-ballistics-workspace-tests | 7 / 7 |
| `snapshot_save` | velvet-ballistics-workspace-tests | 5 / 5 |
| `snapshot_restore` | velvet-ballistics-workspace-tests | 5 / 5 |
| `array_queue` | velvet-ballistics-workspace-tests | 6 / 6 |
| `rtrb` | velvet-ballistics-workspace-tests | 6 / 6 |

**Measured total: 128 benchmarks, 0 failures.** Bench build proof: `cargo bench -p vb_core -p vb_validate -p velvet-ballistics-workspace-tests --no-run` exits 0.

## What this closes relative to the 2026-11-01 evidence

- The 7 targets that failed to build on 2026-11-01 (velvet_ballistics, ir_traversal, memory_footprint, cold_start, action_queuing, snapshot_save, snapshot_restore) all build and measure on current main. No import fix was needed in this run: main's node() migration had already rewritten those files, and merged PR #29 fixed the remaining timer/snapshot/capability gaps.
- The 3 timer_wheel_tick benches missing on 2026-11-01 (`timer_wheel_cancel_1`, `timer_wheel_next_deadline`, `timer_wheel_insert_100`) all measure now, following merged PR #29's bench-fixture fix.
- `capability_schema` now registers and measures 1 benchmark, following merged PR #29's `[[bench]] harness = false` fix.
- A per-bench fixture-bytes table is included below (the `fixture` and `fixture_bytes` columns).

## Flagged results (do not base claims on these without further work)

- `action_dispatch_resolve_compile_time`: mean 1.77 ns. Below a plausible measurement floor for a dispatch; the value is consistent with the resolve being folded to a constant at compile time. Treat as harness-suspect until a `black_box` audit proves the work is actually performed.
- `timer_wheel_fire_empty`: throughput `[0, 0, 0] elem/s`. Expected, not a failure: the bench sets `Throughput::Elements(0)` for the empty wheel.
- The 2026-05-01 rejected sub-nanosecond result recorded in `benchmark-suite.md` is unchanged by this run and remains rejected.

## Still missing for a full §44 Pt 22 close

1. Instruction counts: criterion does not measure these; they need a counter harness (for example perf counters), which is a separate batch.
2. Allocation counts and bytes allocated: same counter-harness batch; criterion does not measure these.
3. Fixture sizes for 5 benches that do not declare them: `aggregate_resource_budget_contract_surface`, `capability schema validation`, `vb_kkvb_parse_ai_context`, `vb_kkvb_route_ai_context_deferred`, `vb_qi37_1_1_no_output_recovery`. Adding a declared fixture size to those five bench sources is small follow-up work, not done here because it would change bench code without changing any measurement.

## Measured results

### vb_core — aggregate_resource_budget (1 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| aggregate_resource_budget_contract_surface | [1.6040 µs 1.6347 µs 1.6865 µs] | 1.5985 µs | 1.7904 µs | 1.9855 µs | — | — | not declared |

### vb_validate — capability_schema (1 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| capability schema validation | [145.83 ns 149.91 ns 155.69 ns] | 145.07 ns | 151.76 ns | 211.57 ns | — | — | not declared |

### velvet-ballistics-workspace-tests — velvet_ballistics (63 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| parse_yaml_small | [22.592 µs 22.629 µs 22.670 µs] | 22.6446 µs | 22.9898 µs | 23.1363 µs | 6.7729 MiB/s 6.7851 MiB/s 6.7963 MiB/s | small_workflow | 161 |
| parse_yaml_1mb | [79.011 µs 79.105 µs 79.210 µs] | 79.0021 µs | 79.7207 µs | 80.4874 µs | 12.329 GiB/s 12.346 GiB/s 12.361 GiB/s | generated_1mb_yaml | 1048640 |
| validate_minimal | [26.711 µs 27.212 µs 27.840 µs] | 26.5825 µs | 28.0218 µs | 36.5476 µs | 5.5151 MiB/s 5.6423 MiB/s 5.7483 MiB/s | small_workflow | 161 |
| compile_ir_minimal | [26.511 µs 26.579 µs 26.661 µs] | 26.5071 µs | 27.4547 µs | 30.0883 µs | 5.7591 MiB/s 5.7767 MiB/s 5.7915 MiB/s | small_workflow | 161 |
| compile_ir_1000_steps | [6.2648 ms 6.3041 ms 6.3492 ms] | 6.2557 ms | 6.6916 ms | 7.0318 ms | 6.5931 MiB/s 6.6402 MiB/s 6.6819 MiB/s | generated_1000_steps | 43894 |
| validate_1000_steps | [6.1820 ms 6.1935 ms 6.2052 ms] | 6.2031 ms | 6.2725 ms | 6.3284 ms | 6.7461 MiB/s 6.7588 MiB/s 6.7714 MiB/s | generated_1000_steps | 43894 |
| expr_eq_symbol | [363.80 ns 366.02 ns 368.37 ns] | 360.92 ns | 379.88 ns | 453.79 ns | — | expression | 17 |
| expr_number_compare | [432.88 ns 441.84 ns 454.20 ns] | 426.64 ns | 533.25 ns | 557.49 ns | — | expression | 5 |
| expr_boolean_chain | [98.710 ns 99.224 ns 99.770 ns] | 99.29 ns | 102.35 ns | 104.31 ns | — | expression | 21 |
| expr_arithmetic | [978.68 ns 992.12 ns 1.0086 µs] | 971.31 ns | 1.0510 µs | 1.1451 µs | — | expression | 9 |
| bench_engine_numeric_slots_read_write_i64 | [112.65 ns 114.39 ns 116.33 ns] | 112.06 ns | 123.89 ns | 216.32 ns | — | run_frame_slot | 161 |
| slot_read | [110.42 ns 111.27 ns 112.20 ns] | 110.19 ns | 119.28 ns | 120.09 ns | — | run_frame_slot | 161 |
| bench_engine_step_once_save_const_single_transition | [49.617 ns 50.377 ns 51.450 ns] | 49.47 ns | 59.67 ns | 64.16 ns | — | small_workflow | 161 |
| engine_run_until_blocked_budget_10_small_workflow | [48.879 ns 50.206 ns 51.821 ns] | 48.66 ns | 49.80 ns | 63.37 ns | — | small_workflow | 161 |
| bench_engine_run_save_chain_10_steps | [605.82 ns 619.20 ns 634.96 ns] | 596.26 ns | 692.70 ns | 755.66 ns | — | ir_save_chain_10 | 36 |
| bench_engine_run_save_chain_1000_steps | [39.258 µs 39.713 µs 40.223 µs] | 38.9394 µs | 43.4785 µs | 45.1635 µs | — | ir_save_chain_1000 | 38 |
| bench_engine_choose_true_branch | [248.89 ns 251.84 ns 254.80 ns] | 244.43 ns | 269.38 ns | 277.90 ns | — | ir_choose_slot_true | 31 |
| bench_engine_choose_false_branch | [249.37 ns 252.23 ns 256.33 ns] | 248.17 ns | 258.08 ns | 339.22 ns | — | ir_choose_slot_false | 32 |
| bench_engine_finish_no_observability | [157.83 ns 158.74 ns 159.85 ns] | 157.71 ns | 166.40 ns | 178.88 ns | — | ir_finish_only | 36 |
| bench_memory_ingress_try_submit_capacity_1024 | [33.154 µs 33.475 µs 33.883 µs] | 33.1914 µs | 35.2399 µs | 39.8206 µs | — | memory_ingress_1024 | 161 |
| bench_memory_ingress_submit_recv_single_thread | [126.55 ns 128.13 ns 130.97 ns] | 127.79 ns | 167.16 ns | 218.59 ns | — | memory_ingress_pair | 161 |
| bench_memory_ingress_backpressure_full_queue | [70.853 ns 71.297 ns 72.025 ns] | 70.80 ns | 74.62 ns | 99.97 ns | — | memory_ingress_full | 161 |
| postcard_encode_event | [254.79 ns 258.68 ns 264.19 ns] | 254.45 ns | 263.54 ns | 289.67 ns | — | run_accepted_event | 161 |
| postcard_decode_event | [531.14 ns 533.82 ns 536.55 ns] | 526.42 ns | 548.92 ns | 551.83 ns | — | run_accepted_event | 161 |
| ipc_frame_encode | [62.578 ns 62.747 ns 62.927 ns] | 62.37 ns | 64.08 ns | 65.99 ns | — | submit_run_payload | 161 |
| ipc_frame_decode | [231.47 ns 232.05 ns 232.66 ns] | 231.92 ns | 236.45 ns | 237.19 ns | — | submit_run_payload | 161 |
| bench_fjall_append_run_accepted_no_persist | [1.7464 µs 1.7736 µs 1.8052 µs] | 1.7277 µs | 2.0023 µs | 2.1453 µs | — | fjall_run_events | 161 |
| bench_replay_ordered_journal_1000_events | [800.75 µs 803.96 µs 807.43 µs] | 807.1695 µs | 825.3511 µs | 831.6816 µs | — | fjall_run_events_1000 | 161 |
| ir_execution_1_step | [156.59 ns 157.84 ns 159.70 ns] | 156.86 ns | 163.55 ns | 173.64 ns | — | finish_1 | 8 |
| ir_execution_1000_steps | [38.727 µs 39.135 µs 39.805 µs] | 38.5536 µs | 40.8348 µs | 44.6423 µs | — | save_chain_1000 | 15 |
| ir_execution_choose_100 | [48.365 ns 48.533 ns 48.791 ns] | 48.47 ns | 49.43 ns | 50.31 ns | — | choose_100 | 10 |
| ir_execution_expr | [48.666 ns 49.293 ns 50.152 ns] | 48.51 ns | 50.23 ns | 56.04 ns | — | expression_workflow | 10 |
| eval_expr_scalar_arithmetic_taint | [268.65 ns 271.49 ns 274.85 ns] | 276.00 ns | 305.32 ns | 350.14 ns | — | scalar_expr | 17 |
| eval_expr_slot_load_all_clean | [307.86 ns 308.96 ns 310.04 ns] | 307.61 ns | 324.06 ns | 326.10 ns | — | slot_load_clean | 16 |
| eval_expr_slot_load_mixed_taint | [308.06 ns 309.39 ns 310.88 ns] | 310.78 ns | 325.07 ns | 326.10 ns | — | slot_load_mixed | 16 |
| build_object_2_fields_taint | [48.604 ns 48.812 ns 49.138 ns] | 48.45 ns | 50.04 ns | 55.47 ns | — | build_object_2 | 2 |
| build_object_8_fields_taint | [48.692 ns 48.803 ns 48.929 ns] | 48.70 ns | 50.01 ns | 50.85 ns | — | build_object_8 | 2 |
| build_object_16_fields_taint | [48.828 ns 49.187 ns 49.601 ns] | 48.53 ns | 50.85 ns | 54.45 ns | — | build_object_16 | 2 |
| build_list_2_items_taint | [362.24 ns 364.46 ns 368.00 ns] | 361.09 ns | 366.14 ns | 377.24 ns | — | build_list_2 | 2 |
| build_list_8_items_taint | [827.82 ns 830.72 ns 835.12 ns] | 831.84 ns | 858.59 ns | 877.69 ns | — | build_list_8 | 2 |
| build_list_16_items_taint | [1.3688 µs 1.3776 µs 1.3870 µs] | 1.3491 µs | 1.4214 µs | 1.4530 µs | — | build_list_16 | 2 |
| full_workflow_all_clean | [48.558 ns 48.676 ns 48.817 ns] | 48.41 ns | 50.32 ns | 50.80 ns | — | full_workflow_clean | 16 |
| full_workflow_mixed_taint | [48.383 ns 48.735 ns 49.276 ns] | 48.32 ns | 50.25 ns | 56.79 ns | — | full_workflow_mixed | 16 |
| submit_artifact_relaxed | [48.533 ns 48.612 ns 48.698 ns] | 48.41 ns | 49.26 ns | 49.77 ns | — | small_workflow | 161 |
| submit_artifact_journaled | [48.598 ns 48.873 ns 49.206 ns] | 48.37 ns | 49.46 ns | 53.41 ns | — | small_workflow | 161 |
| submit_artifact_strict | [48.789 ns 48.931 ns 49.107 ns] | 48.78 ns | 50.43 ns | 53.26 ns | — | small_workflow | 161 |
| budget_compute_small_workflow | [48.514 ns 48.677 ns 48.899 ns] | 48.50 ns | 49.77 ns | 52.86 ns | — | small_workflow | 161 |
| budget_compute_save_chain_10 | [1.3449 µs 1.3517 µs 1.3608 µs] | 1.3478 µs | 1.3766 µs | 1.4045 µs | — | save_chain_10 | 13 |
| budget_compute_save_chain_1000 | [717.34 µs 721.62 µs 726.68 µs] | 714.3110 µs | 739.5925 µs | 779.6689 µs | — | save_chain_1000 | 15 |
| budget_validate_default_policy | [48.472 ns 48.583 ns 48.725 ns] | 48.43 ns | 49.34 ns | 51.68 ns | — | small_workflow | 161 |
| evidence_chain_accumulate_100_events | [7.6733 µs 7.6894 µs 7.7089 µs] | 7.6789 µs | 7.8909 µs | 8.0891 µs | — | volatile_journal_100 | 12 |
| evidence_chain_accumulate_1000_events | [71.394 µs 71.566 µs 71.747 µs] | 71.6719 µs | 72.8108 µs | 74.0690 µs | — | volatile_journal_1000 | 13 |
| evidence_chain_snapshot_100_events | [2.6597 µs 2.6909 µs 2.7345 µs] | 2.6499 µs | 2.8177 µs | 3.2453 µs | — | volatile_journal_snapshot_100 | 17 |
| admit_run_relaxed | [94.104 ns 94.677 ns 95.286 ns] | 94.75 ns | 98.62 ns | 100.23 ns | — | always_present | 17 |
| admit_run_strict_artifact_present | [134.59 ns 135.52 ns 136.78 ns] | 135.34 ns | 145.88 ns | 175.70 ns | — | always_present | 16 |
| admit_run_multiple_action_caps | [139.20 ns 140.66 ns 143.09 ns] | 139.74 ns | 142.44 ns | 162.56 ns | — | always_present | 20 |
| admit_run_empty_caps | [80.279 ns 80.820 ns 81.411 ns] | 79.57 ns | 84.89 ns | 85.72 ns | — | always_present | 20 |
| capability_check_any_workflow_grants | [50.078 ns 50.209 ns 50.400 ns] | 50.04 ns | 50.83 ns | 53.06 ns | — | any_workflow_set | 16 |
| capability_check_action_match_first | [54.800 ns 54.927 ns 55.066 ns] | 54.68 ns | 55.55 ns | 56.59 ns | — | action_set_10 | 16 |
| capability_check_action_miss | [54.911 ns 55.014 ns 55.134 ns] | 54.90 ns | 56.38 ns | 59.33 ns | — | action_set_10 | 15 |
| capability_check_empty_denies | [49.163 ns 49.618 ns 50.268 ns] | 48.99 ns | 52.46 ns | 63.68 ns | — | empty_set | 9 |
| capability_check_mixed_set | [50.708 ns 50.875 ns 51.103 ns] | 50.66 ns | 52.65 ns | 70.60 ns | — | mixed_set | 9 |
| capability_check_admission_gate | [117.95 ns 118.11 ns 118.30 ns] | 117.89 ns | 119.03 ns | 121.63 ns | — | action_set_10 | 13 |

### velvet-ballistics-workspace-tests — vb_qi37_1_1_recovery (1 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| vb_qi37_1_1_no_output_recovery | [308.12 ns 309.77 ns 312.04 ns] | 307.94 ns | 338.37 ns | 378.63 ns | — | — | not declared |

### velvet-ballistics-workspace-tests — vb_kkvb_xtask_routing_red (2 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| vb_kkvb_parse_ai_context | [263.91 ns 266.38 ns 268.71 ns] | 270.81 ns | 277.60 ns | 278.90 ns | — | — | not declared |
| vb_kkvb_route_ai_context_deferred | [141.47 ns 142.02 ns 142.65 ns] | 142.40 ns | 146.72 ns | 147.81 ns | — | — | not declared |

### velvet-ballistics-workspace-tests — ir_traversal (6 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| ir_traverse_df_small | [29.205 ns 29.620 ns 30.063 ns] | 28.75 ns | 33.67 ns | 35.17 ns | 1.9827 GiB/s 2.0123 GiB/s 2.0409 GiB/s | chain_2 | 64 |
| ir_traverse_df_100_steps | [702.88 ns 704.43 ns 706.24 ns] | 701.78 ns | 716.22 ns | 730.44 ns | 1.3504 GiB/s 1.3538 GiB/s 1.3568 GiB/s | chain_100 | 1024 |
| ir_traverse_df_1000_steps | [7.1997 µs 7.2226 µs 7.2540 µs] | 7.1817 µs | 7.2712 µs | 7.3796 µs | 1.3147 GiB/s 1.3204 GiB/s 1.3246 GiB/s | chain_1000 | 10240 |
| ir_traverse_bfs_1000_steps | [7.0728 µs 7.0809 µs 7.0908 µs] | 7.0620 µs | 7.1316 µs | 7.2593 µs | 1.3449 GiB/s 1.3468 GiB/s 1.3484 GiB/s | chain_1000 | 10240 |
| ir_traverse_expr_programs | [8.1232 ns 8.1513 ns 8.1883 ns] | 8.13 ns | 8.55 ns | 10.83 ns | 1164.7 GiB/s 1170.0 GiB/s 1174.0 GiB/s | expr_10 | 512 |
| ir_traverse_mixed_kinds | [40.847 ns 41.042 ns 41.219 ns] | 40.88 ns | 41.72 ns | 41.85 ns | 231.37 GiB/s 232.37 GiB/s 233.47 GiB/s | mixed_wf | 256 |

### velvet-ballistics-workspace-tests — collect_page (5 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| collect_page_first_page_small | [160.75 ns 162.28 ns 164.40 ns] | 160.10 ns | 166.32 ns | 178.32 ns | 580.10 MiB/s 587.69 MiB/s 593.28 MiB/s | list_100_page_50 | 100 |
| collect_page_second_page | [268.29 ns 271.46 ns 276.55 ns] | 267.14 ns | 282.04 ns | 283.29 ns | 344.85 MiB/s 351.31 MiB/s 355.46 MiB/s | list_100_page_50 | 100 |
| collect_page_exhausted | [189.26 ns 189.89 ns 190.53 ns] | 189.12 ns | 193.35 ns | 194.51 ns | 500.54 MiB/s 502.22 MiB/s 503.90 MiB/s | list_100_page_50 | 100 |
| collect_page_large_10_pages | [1.1442 µs 1.1664 µs 1.1951 µs] | 1.1355 µs | 1.2256 µs | 1.6602 µs | 797.98 MiB/s 817.63 MiB/s 833.48 MiB/s | list_1000_page_100 | 1000 |
| collect_page_find_existing | [23.389 ns 23.587 ns 23.832 ns] | 23.33 ns | 24.85 ns | 26.67 ns | 4.1961 Gelem/s 4.2396 Gelem/s 4.2755 Gelem/s | 100_entry_table | 100 |

### velvet-ballistics-workspace-tests — action_dispatch (5 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| action_dispatch_single_registered | [129.76 ns 129.92 ns 130.09 ns] | 129.62 ns | 131.37 ns | 131.96 ns | 7.6867 Melem/s 7.6969 Melem/s 7.7066 Melem/s | 1_action | 1 |
| action_dispatch_10_registered | [130.42 ns 130.51 ns 130.61 ns] | 130.43 ns | 131.48 ns | 132.54 ns | 76.563 Melem/s 76.620 Melem/s 76.673 Melem/s | 10_actions | 10 |
| action_dispatch_100_registered | [137.43 ns 139.81 ns 142.57 ns] | 135.77 ns | 156.28 ns | 168.34 ns | 701.43 Melem/s 715.25 Melem/s 727.67 Melem/s | 100_actions | 100 |
| action_dispatch_unknown_action | [139.25 ns 141.57 ns 144.55 ns] | 137.24 ns | 163.52 ns | 189.49 ns | 691.79 Melem/s 706.39 Melem/s 718.14 Melem/s | 10_actions | 10 |
| action_dispatch_resolve_compile_time | [1.7661 ns 1.7736 ns 1.7852 ns] | 1.77 ns | 1.82 ns | 1.90 ns | 56.015 Gelem/s 56.383 Gelem/s 56.622 Gelem/s | 10_actions | 10 |

### velvet-ballistics-workspace-tests — memory_footprint (2 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| memory_save_chain_1000_peak | [1.0755 ns 1.1481 ns 1.2427 ns] | 1.52 ns | 12.95 ns | 38.85 ns | 7674.1 GiB/s 8306.5 GiB/s 8867.5 GiB/s | save_chain_1000 | 10240 |
| memory_valuestore_growth | [1.7482 ns 1.8657 ns 2.0195 ns] | 2.48 ns | 21.04 ns | 63.13 ns | 495.18 Gelem/s 536.00 Gelem/s 572.01 Gelem/s | 1000_slot_writes | 1000 |

### velvet-ballistics-workspace-tests — cold_start (3 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| cold_start_100_steps | [64.887 ns 65.120 ns 65.376 ns] | 65.12 ns | 68.00 ns | 73.19 ns | 14.588 GiB/s 14.645 GiB/s 14.697 GiB/s | save_chain_100 | 1024 |
| cold_start_1000_steps | [75.270 ns 75.504 ns 75.741 ns] | 75.49 ns | 78.05 ns | 79.46 ns | 125.91 GiB/s 126.31 GiB/s 126.70 GiB/s | save_chain_1000 | 10240 |
| cold_start_full_pipeline | [637.45 µs 640.77 µs 645.06 µs] | 636.3806 µs | 688.8744 µs | 725.3251 µs | 6.3483 MiB/s 6.3909 MiB/s 6.4242 MiB/s | small_workflow_yaml | 4294 |

### velvet-ballistics-workspace-tests — pagination_cost (5 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| pagination_insert_first | [155.28 ns 155.96 ns 156.97 ns] | 155.62 ns | 158.46 ns | 160.12 ns | 6.3708 Melem/s 6.4121 Melem/s 6.4399 Melem/s | empty_table | 1 |
| pagination_upsert_second_page | [292.27 ns 294.25 ns 297.44 ns] | 292.52 ns | 316.69 ns | 364.06 ns | 3.3621 Melem/s 3.3985 Melem/s 3.4215 Melem/s | 1_entry_table | 1 |
| pagination_find_existing | [23.825 ns 24.104 ns 24.523 ns] | 23.65 ns | 25.62 ns | 29.43 ns | 4.0779 Gelem/s 4.1487 Gelem/s 4.1973 Gelem/s | 100_entry_table | 100 |
| pagination_find_missing | [20.073 ns 20.303 ns 20.595 ns] | 19.89 ns | 21.71 ns | 23.66 ns | 4.8556 Gelem/s 4.9253 Gelem/s 4.9818 Gelem/s | 100_entry_table | 100 |
| pagination_lineage_tracking | [1.1690 µs 1.1786 µs 1.1899 µs] | 1.1595 µs | 1.2134 µs | 1.2570 µs | 84.042 Melem/s 84.849 Melem/s 85.541 Melem/s | 10_page_lineage | 10 |

### velvet-ballistics-workspace-tests — action_queuing (5 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| action_queue_enqueue | [1.3747 µs 1.3834 µs 1.3932 µs] | 1.3687 µs | 1.4089 µs | 1.4417 µs | 717.75 Kelem/s 722.86 Kelem/s 727.41 Kelem/s | queue_empty_1024 | 1 |
| action_queue_dequeue | [50.640 µs 50.834 µs 51.049 µs] | 50.3908 µs | 54.3760 µs | 57.4765 µs | 1.9589 Melem/s 1.9672 Melem/s 1.9747 Melem/s | queue_100_items | 100 |
| action_queue_full_enqueue_err | [798.33 ns 801.58 ns 805.29 ns] | 794.95 ns | 817.94 ns | 856.91 ns | 124.18 Melem/s 124.75 Melem/s 125.26 Melem/s | queue_full | 1 |
| action_queue_batch_100 | [43.838 µs 43.854 µs 43.871 µs] | 43.8353 µs | 44.1774 µs | 45.2041 µs | 2.2794 Melem/s 2.2803 Melem/s 2.2811 Melem/s | queue_empty_1024 | 100 |
| action_queue_len_is_full | [429.72 µs 430.90 µs 432.17 µs] | 429.1601 µs | 440.5216 µs | 446.2342 µs | 2.3695 Melem/s 2.3764 Melem/s 2.3829 Melem/s | queue_1024_items | 1024 |

### velvet-ballistics-workspace-tests — timer_wheel_tick (7 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| timer_wheel_fire_empty | [35.283 ns 36.153 ns 36.904 ns] | 37.25 ns | 38.79 ns | 39.51 ns | 0.0000  elem/s 0.0000  elem/s 0.0000  elem/s | wheel_empty | 0 |
| timer_wheel_fire_1_expired | [191.82 ns 193.13 ns 194.56 ns] | 190.41 ns | 201.19 ns | 208.73 ns | 5.1397 Melem/s 5.1779 Melem/s 5.2131 Melem/s | wheel_1_expired | 1 |
| timer_wheel_fire_10_expired | [1.3184 µs 1.3230 µs 1.3291 µs] | 1.3109 µs | 1.3470 µs | 1.3519 µs | 7.5241 Melem/s 7.5584 Melem/s 7.5850 Melem/s | wheel_10_expired | 10 |
| timer_wheel_fire_90_of_100 | [11.555 µs 11.569 µs 11.587 µs] | 11.5657 µs | 11.6580 µs | 11.7363 µs | 8.6304 Melem/s 8.6441 Melem/s 8.6540 Melem/s | wheel_100_mixed | 100 |
| timer_wheel_cancel_1 | [11.621 µs 11.864 µs 12.368 µs] | 11.6135 µs | 11.8908 µs | 12.3805 µs | 8.0854 Melem/s 8.4291 Melem/s 8.6049 Melem/s | wheel_100_mixed | 100 |
| timer_wheel_next_deadline | [878.42 ns 881.52 ns 886.36 ns] | 876.38 ns | 889.63 ns | 897.42 ns | 112.82 Melem/s 113.44 Melem/s 113.84 Melem/s | wheel_10 | 10 |
| timer_wheel_insert_100 | [7.4836 µs 7.5417 µs 7.6138 µs] | 7.5026 µs | 7.7748 µs | 8.5045 µs | 13.134 Melem/s 13.260 Melem/s 13.362 Melem/s | wheel_empty | 100 |

### velvet-ballistics-workspace-tests — snapshot_save (5 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| snapshot_save_1_step | [192.74 ns 193.07 ns 193.41 ns] | 193.12 ns | 195.79 ns | 196.74 ns | 5.1703 Melem/s 5.1794 Melem/s 5.1883 Melem/s | frame_1_step | 1 |
| snapshot_save_50_steps | [303.97 ns 305.43 ns 307.36 ns] | 303.72 ns | 326.75 ns | 366.95 ns | 162.67 Melem/s 163.70 Melem/s 164.49 Melem/s | frame_50_steps | 50 |
| snapshot_save_large_slots | [200.95 ns 201.37 ns 201.83 ns] | 201.07 ns | 204.61 ns | 208.19 ns | 47.251 GiB/s 47.359 GiB/s 47.458 GiB/s | frame_10kb_slots | 10240 |
| snapshot_encode_postcard | [234.80 ns 235.25 ns 235.72 ns] | 235.65 ns | 239.79 ns | 242.00 ns | 4.0459 MiB/s 4.0539 MiB/s 4.0616 MiB/s | snapshot_small | 1 |
| snapshot_encode_max_correlation | [545.24 ns 551.96 ns 560.42 ns] | 543.22 ns | 557.70 ns | 662.07 ns | 1.7017 MiB/s 1.7278 MiB/s 1.7491 MiB/s | snapshot_max_corr | 1 |

### velvet-ballistics-workspace-tests — snapshot_restore (5 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| snapshot_restore_1_step | [146.03 ns 146.69 ns 147.37 ns] | 145.09 ns | 151.10 ns | 158.46 ns | 6.7857 Melem/s 6.8171 Melem/s 6.8481 Melem/s | snapshot_1_step | 1 |
| snapshot_restore_50_steps | [147.80 ns 148.81 ns 149.88 ns] | 146.56 ns | 153.84 ns | 172.67 ns | 333.60 Melem/s 336.00 Melem/s 338.30 Melem/s | snapshot_50_steps | 50 |
| snapshot_restore_large_slots | [139.88 ns 140.25 ns 140.61 ns] | 140.10 ns | 142.65 ns | 145.14 ns | 67.822 GiB/s 67.996 GiB/s 68.180 GiB/s | snapshot_10kb_slots | 10240 |
| snapshot_decode_postcard | [78.302 ns 79.142 ns 80.413 ns] | 78.72 ns | 83.07 ns | 91.08 ns | 118.60 GiB/s 120.50 GiB/s 121.79 GiB/s | snapshot_encoded_50 | 1 |
| snapshot_restore_correlation | [86.898 ns 87.217 ns 87.558 ns] | 87.16 ns | 88.69 ns | 89.79 ns | 108.92 GiB/s 109.34 GiB/s 109.75 GiB/s | snapshot_with_correlation | 1 |

### velvet-ballistics-workspace-tests — array_queue (6 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| array_queue_push_1 | [447.72 ns 452.67 ns 459.45 ns] | 446.16 ns | 471.26 ns | 638.45 ns | 2.1765 Melem/s 2.2091 Melem/s 2.2336 Melem/s | aq_empty_1024 | 1 |
| array_queue_pop_1 | [1.6000 µs 1.6344 µs 1.6878 µs] | 1.5869 µs | 1.6943 µs | 1.9398 µs | 59.248 Melem/s 61.184 Melem/s 62.500 Melem/s | aq_100_items | 100 |
| array_queue_push_full_err | [64.186 ns 64.770 ns 65.552 ns] | 63.69 ns | 66.89 ns | 75.09 ns | 1.5255 Gelem/s 1.5439 Gelem/s 1.5580 Gelem/s | aq_full_1 | 1 |
| array_queue_capacity_1024 | [12.432 µs 12.445 µs 12.461 µs] | 12.4704 µs | 12.6288 µs | 12.7949 µs | 82.174 Melem/s 82.282 Melem/s 82.370 Melem/s | aq_empty_1024 | 1024 |
| array_queue_is_full_len | [6.2035 µs 6.2161 µs 6.2313 µs] | 6.2233 µs | 6.3876 µs | 6.9562 µs | 164.33 Melem/s 164.73 Melem/s 165.07 Melem/s | aq_512_items | 512 |
| array_queue_fifo_1000 | [23.397 µs 23.488 µs 23.584 µs] | 23.2783 µs | 24.3942 µs | 27.3411 µs | 42.402 Melem/s 42.574 Melem/s 42.740 Melem/s | aq_1000_items | 1000 |

### velvet-ballistics-workspace-tests — rtrb (6 benches)

| Bench | time (95% CI) | p50 | p95 | p99 | thrpt (95% CI) | fixture | fixture_bytes |
|---|---|---|---|---|---|---|---|
| rtrb_push_1 | [108.38 ns 109.28 ns 110.36 ns] | 107.12 ns | 112.27 ns | 114.53 ns | 9.0609 Melem/s 9.1512 Melem/s 9.2265 Melem/s | rtrb_empty | 1 |
| rtrb_pop_1 | [459.41 ns 462.00 ns 464.60 ns] | 456.26 ns | 474.58 ns | 498.69 ns | 215.24 Melem/s 216.45 Melem/s 217.67 Melem/s | rtrb_100_items | 100 |
| rtrb_push_full_err | [644.62 ns 653.95 ns 663.67 ns] | 631.23 ns | 756.00 ns | 857.19 ns | 150.68 Melem/s 152.92 Melem/s 155.13 Melem/s | rtrb_full | 1 |
| rtrb_peek | [464.13 ns 465.19 ns 466.34 ns] | 464.35 ns | 477.01 ns | 494.04 ns | 214.44 Melem/s 214.96 Melem/s 215.45 Melem/s | rtrb_100_items | 100 |
| rtrb_is_full_is_empty | [333.89 ns 336.88 ns 340.52 ns] | 330.82 ns | 352.92 ns | 376.01 ns | 293.67 Melem/s 296.84 Melem/s 299.50 Melem/s | rtrb_50_items | 50 |
| rtrb_fifo_1000 | [5.1438 µs 5.1914 µs 5.2569 µs] | 5.1187 µs | 5.2990 µs | 7.1474 µs | 190.23 Melem/s 192.63 Melem/s 194.41 Melem/s | rtrb_1000_items | 1000 |
