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

Current Criterion IDs implemented in `crates/workspace_tests/benches/velvet_ballistics.rs` include the cheap real engine, memory ingress, append, and replay surfaces named in `test-plan.md`. Deferred IDs are recorded there when the current runtime model or harness cannot yet report the required measurement honestly.

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

Update 2026-11-12 (main `589c0a64`): the 3 budgeted rows in `evidence/benchmark-evidence.jsonl` were replaced by session `r178-6d29f951-session01`, measured on John's Omarchy box (AMD Ryzen 9 9950X3D, Linux omarchy 7.2.11-arch1-1, perf 7.2.11-1 `instructions:u`, heaptrack 1.5.0) at candidate `6d29f951` with A1/B1/B2/A2 recorded sessions; the r163 session is archived at `evidence/benchmark-archives/r163-e203e034-session03-a50f5f1b4544.jsonl` and the r178 raw logs are under `evidence/benchmark-runs/r178-6d29f951-session01/metric-logs/`. The callgrind Ir totals and the `df1f5d7e` VM re-measurement above are superseded: the rows now carry real perf `instructions:u` counts (candidate-only whole filtered-process totals from separate tools, not comparative or per-operation costs) — step_once 48,748,247,715, ipc_frame_decode 32,521,621,430, run_until_blocked 34,513,009,941 — with means step_once 38.081 ns (baseline 43.449 ns), ipc_frame_decode 61.316 ns (baseline 72.459 ns), run_until_blocked 59.122 ns (baseline 67.316 ns), thresholds 3%/5%/3%, and pooled-sample-mean quantiles step_once p50 37.990 / p95 38.679 / p99 38.887 ns, ipc_frame_decode p50 61.191 / p95 62.648 / p99 62.725 ns, run_until_blocked p50 58.939 / p95 60.296 / p99 60.439 ns; heaptrack allocations 13,180,595 / 8,549,550 / 8,663,760 and bytes 233,250,862 / 242,679,130 / 174,531,148 in the same row order. The `benchmark-regression-policy` gate passed on main `589c0a64` (exit 0, claims=3), verified first-hand at that tip; it no longer passes on main `d521ea69`, where #433's xtask split (`xtask/src/main.rs`, `xtask/src/proof_cmds.rs`) makes the r178 evidence stale per the gate (exit 1, same command, verified first-hand on the clean base tree) — the next refresh re-measures on the new tip. The evidence carries the session's own caveat — background activity and variance limit interpretation, no noise-free causal speedup or release claim, quantiles describe pooled sample means, not individual-operation tails — so no speedup is claimed here. No commit between the measured candidate `6d29f951` and `589c0a64` touched benchmarked production code (only `evidence/` and one Verus re-point), so the r178 evidence was current for that tip; #433 has since moved the tip past it. Instruction counts are now measured for the 3 budgeted rows only; the full-suite counter harness for all 129 benches is still John's tooling decision, and `perf_event_open` is still blocked on this VM, so a future refresh measured here still cannot supply perf counts.

Update 2026-11-12 (main `f0781bde`): John's direct commit `b24d9122` ripped out `xtask` entirely — zero tracked `xtask/` files on this main, verified first-hand — and the `benchmark-regression-policy` gate's runner lived in `xtask/src`, so the gate now has no runner at all: it cannot run, not merely fail stale. `.moon/tasks/` carries only `benchmark-proof` (Criterion baseline, `.moon/tasks/all.yml:801`); no `benchmark-regression-policy` task exists anywhere in the tree. The `d521ea69` exit-1 staleness recorded in the Update above is superseded by this — freshness can no longer even be evaluated by the gate. MASTER §40 (`velvet-ballistics-MASTER.md:1680`/:1685) and `docs/master/section-040-ci-gate.md:20`/:25 still list the gate in the `moon ci` pipeline, so the pipeline now names a gate with no runner; restoring a runner or dropping the gate from the pipeline is John's call and is not decided here. The next evidence refresh therefore waits on that call first, and a refresh measured on this VM still cannot supply perf counts (`perf_event_open` blocked). The r178 rows (measured candidate `6d29f951`, 3 rows in `evidence/benchmark-evidence.jsonl`) remain the latest published evidence.

Update 2026-11-12 (main `c5e922a7`): the gate runner is restored — `scripts/check-benchmark-regression-policy.py`, a stdlib-only Python port of the deleted `xtask/src/benchmark_policy.rs` + `benchmark_freshness.rs` + `benchmark_budget.rs` (at `b24d9122^1`), with the `benchmark-regression-policy` moon task re-added to the `moon ci` pipeline in its old position (after `bench-build`, before `fuzz-smoke`); the `f0781bde` "no runner at all" note above is superseded. The port keeps the original checks, messages and exit contract verbatim (budget validation, required-fields, threshold and regression arithmetic, raw-log existence, forbidden placeholder markers, and the conservative perf-relevant freshness comparison against the working tree, including the `xtask/` path, which no longer exists and matches nothing); the only deliberate differences are a strict stdlib YAML-subset parser for the budget instead of `serde_yaml` (no new dependency, so the gate runs wherever `python3` and `git` exist) and explicit `u64` bounds on the evidence integers. Verification, all first-hand: 26/26 fixture probes pass (clean-head fresh, evidence-only descendant fresh, dirty/staged/untracked relevant rejections, regression at/over threshold, duplicate/unbudgeted/deferred/missing-field/missing-log/missing-commit/non-ancestor cases, and the real budget plus the 3 real r178 rows rewritten to the fixture HEAD passing with `claims=3`); the restored gate run on clean main `c5e922a7` exits 1 STALE, correctly — the r178 evidence was measured at `6d29f951` and perf-relevant files differ since, including the `xtask/` removal itself and `crates/` changes — so the gate is red on current main until the evidence is re-measured, exactly as the old gate was on `d521ea69`. Caveat: in a shallow clone without the measured commit's ancestry the gate fails closed with "not an ancestor" (the original `merge-base` behaviour, not a new message); full-history checkouts are unaffected. The next refresh re-measures on the new tip and still needs John's box for perf `instructions:u` counts (`perf_event_open` blocked on this VM).
