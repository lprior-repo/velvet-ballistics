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

Environment: local Linux workspace `/home/lewis/src/velvet-ballistics-r3/vb-ws4m`, Cargo bench profile, Criterion 0.8. Gnuplot was not installed, so Criterion used the plotters backend.

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
