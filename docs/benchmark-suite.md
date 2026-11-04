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
