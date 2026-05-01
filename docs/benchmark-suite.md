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

Current Criterion IDs implemented in `benches/velvet_ballastics.rs` include the cheap real engine, memory ingress, append, and replay surfaces named in `test-plan.md`. Deferred IDs are recorded there when the current runtime model or harness cannot yet report the required measurement honestly.

Acceptance rule: runtime changes that claim latency or throughput improvements must include before/after numbers on the affected benchmark.
