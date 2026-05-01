# Runtime Architecture

Velvet Ballastics is optimized for raw in-process orchestration performance. YAML is source input only. The hot path never interprets YAML, never resolves string references, and never performs HTTP request handling.

## Pipeline

```text
workflow.yaml
  -> byte limits and UTF-8 validation
  -> strict YAML profile parser
  -> schema and semantic validation
  -> compiled workflow IR
  -> immutable workflow snapshot
  -> numeric-slot RunFrame
  -> synchronous engine shard loop
  -> Fjall append-only journal
```

## Hot Path Rules

1. Runtime state transitions use `StepIdx`, not YAML nodes.
2. Runtime data uses `SlotIdx`, not `HashMap<String, Value>`.
3. Future expressions compile to bytecode before a run is accepted; Phase 0 encodes public `save` plus internal `SetConst`/`Copy`/`Choose`/`Finish` IR nodes.
4. Actions resolve to numeric `ActionId` values before execution.
5. Deterministic steps execute synchronously until finish or an async boundary.
6. Engine shards do not spawn one task per step.
7. Durable events are compact binary records.
8. JSON/JSONL is a cold projection only.
9. Every queue and fanout path has a fixed bound.
10. Every performance claim requires a benchmark.

## Workspace Mapping

`vb-core` owns the hot loop and compiled IR. It has no YAML, no async runtime, no storage dependency, and no HTTP dependency.

`vb-compiler` owns the cold native-Rust YAML boundary. It uses `saphyr` for strict-profile parsing and emits `vb-core` compiled IR; no YAML value crosses into the hot runtime.

`vb-ipc` owns bounded in-memory ingress primitives. It is the place to add Unix-domain sockets, shared memory rings, or io_uring-backed IPC later. It must not grow an HTTP layer.

`vb-storage` owns the Fjall durability boundary. It writes append-only binary journal events and exposes explicit persistence barriers.

`velvet-ballastics` wires the binary surface. The binary may expose operator commands, but the runtime control plane remains memory/IPC-first.

## Nightly Rust Policy

The repo is pinned to `nightly` through `rust-toolchain.toml`. Nightly is used for max-performance build profiles, forward-looking compiler optimizations, Miri/model-checking hooks, and benchmark-only experiments. Unstable features must stay behind explicit feature gates until proven useful by benchmark data.
