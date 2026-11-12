---
section: 71
title: "Competitive Performance Targets"
parent: velvet-ballistics-MASTER.md
---

## 71. Competitive Performance Targets


The following are internal engineering targets for `velvet-ballistics` as a single-server engine. They are not public performance claims, but no external claim is allowed until the measurement contract below is satisfied.

### Step-Level Latency Targets

| Metric | Velvet Ballastics (single-server) | Notes |
|--------|-----------------------------------|-------|
| Single step p50 (no replication) | <= 1ms | No network roundtrip for quorum |
| Single step p50 (journaled) | <= 5ms | Fjall group commit |
| Single step p50 (strict) | <= 10ms | fsync on every step |
| Full workflow p50 (9 steps, low load) | <= 15ms | Compiled IR, no SDK roundtrip |
| Full workflow p50 (9 steps, high load) | <= 60ms | Single-server removes coordination overhead |
| Full workflow p99 (9 steps, high load) | <= 100ms | Tight bound from no-unsafe, checked arithmetic |

### Throughput Targets

| Metric | Velvet Ballastics | Notes |
|--------|-------------------|-------|
| Full workflows per second (9 steps) | >= 10,000 | Single-server removes replication overhead |
| Concurrent active runs | >= 4,096 | Frame pool capacity |

### Why These Targets Are Achievable

`velvet-ballistics` eliminates replication overhead:
1. No replication — local Fjall write
2. No leader — single shard owns the run
3. No SDK — action dispatch is a function call within the same process
4. No async — synchronous deterministic loop
5. No competing flush — Fjall writes happen through bounded writer queue, not in the hot path

Generated Rust performance advantages are out of scope. Current speed claims must be scoped to the IR interpreter.

### Measurement Contract

Every performance claim must include:
- `criterion` or `iai-callgrind` output with p50/p95/p99
- Hardware: CPU model, cores, RAM, disk type (NVMe vs SSD)
- Build profile: debug, release, bench for current scope; maxperf/PGO removed
- Execution mode: IR interpreter only
- Durability profile: volatile, journaled, strict
- Number of concurrent runs
- Benchmark fixture digest (reproducible)

---
