# Benchmark Suite

Benchmarks are part of the product contract. The first benchmark targets are:

- `save_const` single-step transition.
- `save` chain with 10 steps.
- `save` chain with 1,000 steps.
- `choose` with true and false branches.
- full no-op run with observability off.
- memory ingress submit/receive throughput.
- Fjall journal append without strict persist.
- Fjall journal append with group commit.
- replay from ordered journal events.
- JSONL projection cost outside the hot loop.

Current Criterion IDs implemented in `benches/velvet_ballistics.rs` include the cheap real engine, memory ingress, append, and replay surfaces named in `test-plan.md`. Deferred IDs are recorded there when the current runtime model or harness cannot yet report the required measurement honestly.

Acceptance rule: runtime changes that claim latency or throughput improvements must include before/after numbers on the affected benchmark.

## Evidence: 2026-05-01 `vb-ws4m`

Environment: local Linux workspace `/home/lewis/src/Velvet-ballistics-r3/vb-ws4m`, Cargo bench profile, Criterion 0.8. Gnuplot was not installed, so Criterion used the plotters backend.

Build proof:

```text
$ cargo bench --bench velvet_ballistics --no-run
Finished `bench` profile [optimized + debuginfo] target(s) in 30.30s
Executable benches/velvet_ballistics.rs (target/release/deps/velvet_ballistics-25b770ac3386c08b)
```

Measured Criterion samples:

| Command | Reported time interval |
|---|---:|
| `cargo bench --bench velvet_ballistics -- runtime_core/bench_engine_run_save_chain_10_steps --sample-size 10 --measurement-time 1` | `[66.274 ns 67.600 ns 69.172 ns]` |
| `cargo bench --bench velvet_ballistics -- storage_ipc/bench_memory_ingress_submit_recv_single_thread --sample-size 10 --measurement-time 1` | `[27.949 ns 28.041 ns 28.089 ns]` |
| `cargo bench --bench velvet_ballistics -- storage_ipc/bench_fjall_append_run_accepted_no_persist --sample-size 10 --measurement-time 1` | `[626.68 ns 639.61 ns 652.05 ns]` |

Rejected non-evidence:

```text
$ cargo bench --bench velvet_ballistics -- runtime_core/bench_engine_run_save_chain_1000_steps --sample-size 10 --measurement-time 1
runtime_core/bench_engine_run_save_chain_1000_steps;profile=bench;tool=criterion-0.8;durability=mixe...
                        time:   [217.27 ps 220.59 ps 225.87 ps]
```

That sub-nanosecond result is not credible for a 1,001-transition workflow. Treat it as harness-invalid until the benchmark proves the full run cannot be optimized away. It must not support any performance claim.

## Evidence: 2026-11-04 (main `260ad7c4`)

Full per-bench tables, including p50/p95/p99, throughput where set, and declared fixture metadata, are in `benchmark-evidence-2026-11-04.md`. All 17 bench targets build (`cargo bench -p vb_core -p vb_validate -p velvet-ballistics-workspace-tests --no-run`, exit 0) and all 128 benchmarks ran with exit 0 under criterion defaults (3 s warmup, 100 samples, ~5 s measurement; durability mixed, mode ir). This closes the 2026-11-01 gaps on current main: the 7 formerly build-failed targets, the 3 missing timer_wheel_tick benches, and the capability_schema bench (which now registers 1 benchmark) all measure, following merged PR #29. Flagged: `action_dispatch_resolve_compile_time` at 1.77 ns mean is below a plausible measurement floor and must not support claims without a `black_box` audit; `timer_wheel_fire_empty` throughput `[0, 0, 0] elem/s` is expected (the bench sets zero elements). Still missing for a full §44 Pt 22 close: instruction counts, allocation counts, and bytes allocated, which criterion does not measure and which need a separate counter harness; 5 benches also do not declare a fixture size (listed in the evidence file).

## Evidence: 2026-11-04 fixture declarations and resolve-benchmark audit

Follow-up to the 2026-11-04 Pt 22 re-measurement, which left 5 benches without a declared fixture size and flagged `action_dispatch_resolve_compile_time` at 1.77 ns mean as harness-suspect. All 5 benches now declare fixture metadata in the same `fixture=...;fixture_bytes=...` form as the rest of the suite, verified against criterion's own `benchmark.json` records for this run. Bench code only; no production code changed. The suite now has 129 benchmarks (128 plus the new varying-id bench below).

| Bench | Declared fixture | fixture_bytes | What the number counts |
|---|---|---:|---|
| `aggregate_resource_budget_contract_surface` | `budget_rs_source` | 112436 | Byte length of the `budget.rs` source the bench scans, taken from `source.len()` at bench setup |
| `capability schema validation` | `workflow_2_nodes_1_contract` | 3 | Validated items: 2 workflow nodes plus 1 action contract |
| `vb_kkvb_parse_ai_context` | `argv_6_args` | 41 | Sum of the 6 argv strings' byte lengths |
| `vb_kkvb_route_ai_context_deferred` | `route_1_command` | 1 | One routed command |
| `vb_qi37_1_1_no_output_recovery` | `events_3` | 3 | Journal events in the recovery fixture |
| `action_dispatch_resolve_compile_time_varying` (new) | `10_actions` | 10 | Registered actions in the registry, same as the fixed-id bench |

The 5 reworked benches also black-box their results now; previously the capability, kkvb, and qi37 benches dropped their measured call's return value, which left the call open to elimination in principle. The fixed-id resolve bench is unchanged apart from its companions.

Measured on this run under criterion defaults (3 s warmup, 100 samples, ~5 s measurement):

| Bench | Reported time interval |
|---|---:|
| `aggregate_resource_budget_contract_surface` | `[1.6381 µs 1.6664 µs 1.7101 µs]` |
| `capability schema validation` | `[147.62 ns 148.57 ns 149.79 ns]` |
| `vb_kkvb_parse_ai_context` | `[250.60 ns 251.37 ns 252.15 ns]` |
| `vb_kkvb_route_ai_context_deferred` | `[140.28 ns 140.73 ns 141.31 ns]` |
| `vb_qi37_1_1_no_output_recovery` | `[321.09 ns 322.58 ns 324.12 ns]` |
| `action_dispatch_resolve_compile_time` (fixed id 5, re-measured) | `[1.7525 ns 1.7538 ns 1.7552 ns]` |
| `action_dispatch_resolve_compile_time_varying` (new, ids 0-9) | `[2.7303 ns 2.7964 ns 2.8800 ns]` |

Audit conclusion on the flagged bench: the fixed-id figure reproduces at 1.75 ns, and the varying-id companion measures 2.80 ns, so the lookup is not being eliminated — a varying id per iteration still costs low-single-digit nanoseconds. That is consistent with the implementation, which is a `Vec` slot index plus an enum match plus a `u16` id comparison over an L1-resident 10-slot table, with `black_box` on both the queried id and the returned contract reference. The 1.75 ns figure is genuine for a single hot slot lookup, but it is the specialized case; cite 2.80 ns as the robust figure for resolve cost across ids. Still missing for a full §44 Pt 22 close: instruction counts, allocation counts, and bytes allocated, which criterion does not measure and which need a separate counter harness.

Update 2026-11-09: allocation counts and bytes allocated are now measured (heaptrack totals for all 17 targets — see the 2026-11-09 section below); instruction counts remain the only unmeasured item, and the counter harness decision is still John's.

## Evidence: 2026-11-09 (main `ca9cb7e6`; policy evidence refreshed on `73f3a610`)

Full per-bench tables, including p50/p95/p99, throughput where set, and declared fixture metadata, are in `benchmark-evidence-2026-11-09.md`. Same 129 bench ids as 2026-11-06; all 17 bench targets build and run with exit 0 under criterion defaults (3 s warmup, 100 samples, ~5 s measurement; durability mixed, mode ir). All 129 benches declare `fixture_bytes` (0 without), so the 2026-11-04 fixture gap is closed. Criterion ran 02:05–02:50 CST, heaptrack 02:52–03:24 CST, on this VM.

Allocation counts and bytes allocated are now measured — first allocation evidence: heaptrack 1.5.0 totals for all 17 targets from reduced 10-sample runs (per-target-process totals, not scaled to 100 samples; heaptrack timings are not reported because heaptrack distorts timings). The histogram count sum matches the reported call total exactly for all 17 targets. Largest totals: `velvet_ballistics` 93,181,232 calls / 56.66 GiB, `action_queuing` 85.90 GiB, `array_queue` 73.96 GiB, `collect_page` 3.87 GiB; peak heap is 5.0–5.3M for most targets and 12.39M for `velvet_ballistics`, so the allocation is churn (allocate/free in a tight loop), not growth. Leaked bytes include a ~35.6K baseline present in nearly every target (criterion harness/startup), not bench code.

Instruction counts remain unmeasured: `perf_event_open` is blocked on this VM, so no perf counts exist here. The 3 budgeted rows in `evidence/benchmark-evidence.jsonl` carry callgrind whole-process Ir totals instead (step_once 26,774,267,670; ipc_frame_decode 26,719,833,865; run_until_blocked 26,597,956,987) — dominated by criterion harness overhead, not comparable to perf numbers. Nothing in the evidence estimates, copies forward, or invents instruction counts.

Deltas vs 2026-11-06, with the same noisy-box caveat as the evidence file: `aggregate_resource_budget_contract_surface` 1.57 µs → 20.5 ns tracks its fixture shrinking (`budget.rs` source 115330 → 8462 bytes after the budget splits), not a product speedup — do not quote it as one. `bench_fjall_append_run_accepted_no_persist` mean 1.79 µs, p99 3.35 µs, the same order as 2026-11-06. `action_dispatch_resolve_compile_time` fixed 1.76 ns, varying 2.73 ns — same conclusion as the 2026-11-04 audit, cite 2.73 ns as the robust figure. Flagged, not concluded: the `admit_run_*` benches measure 150–390 ns slower in absolute terms than 2026-11-06 (admission paths gained work since then — tenant journaling #229, gate-decision submission #217 — and sub-µs benches on this shared box are noisy); `parse_yaml_1mb` mean 80 → 130 µs with p99 89 → 329 µs and p50 87.95 µs, a heavy right tail on this run.

Gate status: the `benchmark-regression-policy` gate passes on this branch (base main `df1f5d7e`). The 3 budgeted rows in `evidence/benchmark-evidence.jsonl` were re-measured on `df1f5d7e` on this VM (snapshot baseline == result, no speedup claim): step_once 48.496 ns mean (interval [48.367 48.505 48.688] ns), ipc_frame_decode 246.498 ns mean (interval [238.38 246.52 258.49] ns), run_until_blocked 48.696 ns mean (interval [48.581 49.027 49.528] ns); callgrind whole-process Ir totals 37,995,666,627 / 38,559,801,317 / 38,138,356,471; heaptrack allocations 574,808 / 2,317,141 / 579,057, bytes 165,896,670 / 201,288,495 / 168,115,281. Two first runs on `df1f5d7e` were discarded as load-contaminated (step_once ~99.2 ns, ipc ~332.1 ns under concurrent lane-B load) per the #249 precedent; run-1 raw logs are preserved under `bench-logs/evidence-1109b/` and the discard is noted in the two raw logs. A VM reboot mid-run wiped the apt-installed heaptrack/valgrind; both were reinstalled (heaptrack 1.5.0, valgrind 3.22.0) before the published measurement. Caveat: main has since moved to `1ebf4b57` (#256–#258, vb_storage types/keys splits and a vb_runtime shard-helpers split, all `crates/` changes), so this evidence will be stale on the new main after merge and the next lane-D refresh re-measures on that tip. Instruction counts remain callgrind totals, not perf — `perf_event_open` is still blocked on this VM.

Still missing for a full §44 Pt 22 close: instruction counts only, which need a counter harness (for example perf counters) — a separate batch, and a tooling decision that is still John's, since `perf_event_open` is blocked on this VM.

