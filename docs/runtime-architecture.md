# Runtime Architecture

`velvet-ballistics` is optimized for raw in-process orchestration performance. YAML is source input only. The hot path never interprets YAML, never resolves string references, parses JSON, or performs HTTP request handling.

## Pipeline

```text
workflow.yaml
  -> byte limits and UTF-8 validation
  -> strict YAML profile parser
  -> schema and semantic validation
  -> compiled numeric workflow IR
  -> accepted artifact bound by digest
  -> numeric-slot RunFrame
  -> synchronous engine shard loop
  -> Fjall journal, replay, and recovery
```

## Hot Path Rules

1. Runtime state transitions use `StepIdx`, not YAML nodes.
2. Runtime data uses `SlotIdx`, not `HashMap<String, Value>`.
3. Expressions compile to bounded bytecode before a run is accepted.
4. Actions resolve to numeric `ActionId` values before execution.
5. Deterministic steps execute synchronously until finish or a suspension boundary.
6. Engine shards do not spawn one task per step.
7. Durable events are compact binary records.
8. JSON/JSONL is a cold projection only.
9. Every queue and fanout path has a fixed bound.
10. Every performance claim requires a benchmark.

## Workspace Mapping

`vb_core` owns the hot loop and compiled IR. It has no YAML, no async runtime, no storage dependency, no JSON routing, and no HTTP dependency.

`vb_yaml`, `vb_validate`, `vb_expr`, and `vb_compile` own the cold YAML boundary. They parse the strict profile, validate shape and semantics, compile expressions, and emit `vb_core` compiled IR; no YAML value crosses into the hot runtime.

`vb_ipc` owns bounded binary ingress primitives. It may use `mio` for Unix-domain socket IPC, but it must not grow an HTTP or JSON routing layer.

`vb_storage` owns the Fjall durability boundary. It writes compact binary journal events and exposes explicit persistence barriers.

`velvet-ballistics` wires the binary surface. The binary may expose operator commands, but the runtime control plane remains memory/IPC-first.

## Nightly Rust Policy

The repo is pinned to `nightly` through `rust-toolchain.toml`. Nightly is used for bounded language features, verification tooling, Miri/model-checking hooks, and benchmark-only experiments. Unstable features must stay behind explicit feature gates. `maxperf`, PGO, generated Rust execution, and native CPU release gates are deferred from the current Backend / IR Interpreter Complete milestone.
