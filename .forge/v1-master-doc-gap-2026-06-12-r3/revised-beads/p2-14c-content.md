P2-14c batched-atomicity-bench: A/B benchmark on `submit + 100 actions`; flip batched_atomicity default after ≥3× throughput. Depends ONLY on P2-14b2.

# Verification excerpts (read-before-write)

## crates/vb_runtime/src/shard/impl_parts/dispatch.rs (208 lines)
- Line 3-17: `pub fn tick(&mut self) -> RuntimeResult<bool>` — synchronous, one command per call. The round-2 P2-14b's wall-clock coalescing was architecturally wrong (no time anchor in sync tick).

## crates/vb_runtime/src/shard/config.rs (156 lines)
- Line 27-38: `ShardConfig` has 5 fields. NO `batched_atomicity` field currently. The new P2-14b2 adds `coalesce_window_ticks: u32` (not the round-2's fabricated `batched_atomicity: bool`).

# Round-2 corrections applied (from black-hat review)

The round-2 bead depended on BOTH P2-14a (storage batch) AND P2-14b (coalesce layer). Black-hat: "Reduce dependency to ONE of (P2-14a, P2-14b). Pick P2-14b since it's the consumer. The A/B benchmark only needs P2-14b's coalescing layer to measure throughput. P2-14a's storage batch is orthogonal."

# Scope (verified, no fabrication)

The A/B benchmark measures the COALESCING layer's throughput ratio. With `coalesce_window_ticks = 1` (no coalescing, baseline): N commits for N actions. With `coalesce_window_ticks = 10` (coalescing): N/10 commits for N actions. Expected ratio: ~10×, well above the 3× threshold.

The P2-14a's storage batch is ORTHOGONAL — it batches multiple EVENTS into a single Fjall commit, not multiple COMMANDS into a single tick. The P2-14b's coalescing is what the A/B measures.

# Dependency (CORRECTED)

This bead depends on P2-14b2 (vb-qpcer) — the replacement for P2-14b. NO dep on P2-14a (vb-7e64r).

# Implementation

Add a Criterion bench target in `crates/vb_benchmark/benches/batched_atomicity.rs` (NEW file — note: this path does not currently exist; the bead must also create the `crates/vb_benchmark/benches/` directory and add it to `Cargo.toml`):
```rust
// At run A: coalesce_window_ticks=1, push 100 commands, call tick() 100 times.
let runtime_a = Runtime::new(...);
for cmd in commands { runtime_a.submit(...); }
for _ in 0..100 { runtime_a.tick_all(); }
let commits_a = count_journal_commits(&runtime_a);

// At run B: coalesce_window_ticks=10, same workload.
let runtime_b = Runtime::new_with_window(10);
for cmd in commands { runtime_b.submit(...); }
for _ in 0..100 { runtime_b.tick_all(); }
let commits_b = count_journal_commits(&runtime_b);

let ratio = commits_a as f64 / commits_b as f64;
assert!(ratio >= 3.0, "ratio {} < 3.0", ratio);
```

# Anti-hallucination guards

- DO NOT depend on P2-14a — the benchmark measures coalescing, not storage batching.
- DO NOT cite `crates/vb_benchmark/benches/` as existing — it does not. This bead must create the directory and wire it.
- DO NOT use the round-2 P2-14b's `coalesce_window_us` — the new P2-14b2 uses `coalesce_window_ticks: u32`.

# Acceptance test

The benchmark itself is the acceptance test. The bead closes when the A/B ratio is recorded in `.evidence/batched_atomicity_bench.json` with ratio >= 3.0.

# Kani harness (skipped — bench is perf; no arithmetic contracts)

Coverage comes from the criterion bench output.

# Dependency

- Depends on: vb-qpcer (P2-14b2)
- Deps removed: vb-7e64r (P2-14a) — orthogonal
