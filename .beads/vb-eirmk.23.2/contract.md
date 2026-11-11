# Draft contract: Phase 29 trace overhead benchmark

## Authoritative requirement

- `velvet-ballistics-MASTER.md` Phase 29 requires the trace ring, counters, binary drain, and overhead benchmarks; §76.13 calls for statistical local Criterion benchmarks, lists instruction count, allocations, allocated bytes, p50/p95/p99 and subsystem latency metrics, and forbids speed claims without stored benchmark evidence.
- The master names no trace-specific regression threshold; its §76.13 examples and `contracts/perf-budget.yaml` budgets cover other benchmarks, so this draft defines no numeric pass/fail threshold or speed claim.

## Existing surfaces

- `crates/workspace_tests/Cargo.toml` already has Criterion, `rtrb`, and `vb_runtime` dev-dependencies and registers `[[bench]] name = "rtrb"`; reuse that target and dependencies.
- `crates/workspace_tests/benches/rtrb.rs` currently measures raw `rtrb::RingBuffer` operations using a private `TraceEvent(u64)` and does not benchmark the runtime wrapper.
- `crates/vb_runtime/src/lib.rs` publicly exports `TraceRing` and `TraceEvent`; `crates/vb_runtime/src/trace.rs` exposes `new`, `push`, `drain_into`, `drain`, `drain_for_run`, `dropped`, and history queries.
- `TraceRing::push` clones accepted events into bounded `VecDeque` history and increments `dropped` on overflow; `drain_into` consumes pending events into caller storage, while raw `rtrb` only supplies producer/consumer operations.

## Proposed bounded benchmark

- Add paired Criterion cases inside the already registered `rtrb` bench, using the same public `vb_runtime::TraceEvent` payload, equal nonzero ring capacity, equal occupancy, and identical fixture for wrapper and raw-ring paths.
- Measure successful `TraceRing::push` against raw `Producer::push`, full-ring `TraceRing::push`/drop accounting against raw full-ring `Producer::push`, and bounded `TraceRing::drain_into` against raw `Consumer::pop` into an equivalently preallocated output vector.
- The four typed raw oracles compare `Ok(())` for empty push and both prefill paths, then compare the full-ring result to an independently prepared `Err(PushError::Full(event.clone()))`; setup prepares that expected error outside timing, and the routine retains the expected value, actual result, producer, and consumer through post-measurement destruction. `raw_push_full_and_verify_returned_event` times raw rejection plus returned-event validation, while `wrapper_push_full_and_observe_dropped` times wrapper rejection plus `dropped()` and dropped-count validation, so their ratio is not an isolated push or one-to-one wrapper-overhead comparison.
- Use public APIs only, keep ring and fixture construction outside measured iterations with Criterion batched setup, and black-box operation results; do not claim scheduler, whole-runtime, or binary-encoding performance from these isolated operations.
- Every case uses `BatchSize::NumIterations(32)`, bounding each setup batch to at most 32 owned ring fixtures independent of calibrated iteration count. Fixed-batch setup, `Vec::extend`, routine-output retention, `black_box`, and Criterion timing instrumentation contribute overhead; results are component-harness observations, not pure latency, a comparative push ratio, or a speed claim.
- Preserve the existing raw benchmark cases; add no production, behavior-test, dependency, registration, gate, or configuration changes beyond extending the existing bench source if this draft is approved.

## Evidence and unresolved threshold

- Record paired baseline and result values, metric and unit, raw Criterion output, exact command, source commit and worktree state, Rust/nightly and Criterion versions, CPU model/governor, kernel, build profile/RUSTFLAGS, sample count, and deterministic event/capacity/workload fixture identity.
- The referenced `contracts/perf-budget.yaml` also lists evidence fields for allocation counts/bytes and latency percentiles; capture measurements only where the proposed benchmark actually measures them and do not synthesize unrelated subsystem values.
- That evidence schema includes `threshold_percent`, but neither the master nor the current budget defines a trace threshold or a representation for an absent threshold; resolve that with the phase owner before claiming a regression gate.
- Proposed scoped measurement command: `cargo bench -p velvet-ballistics-workspace-tests --bench rtrb`; preserve raw output and source/tool/machine provenance for separate baseline and result runs.

## Recommendation

- Keep `vb-eirmk.23` qualification-only and request one bounded benchmark child to extend the registered `rtrb` bench with the paired public-wrapper/raw-ring cases; do not create a production rewrite or a new threshold from this discovery.
