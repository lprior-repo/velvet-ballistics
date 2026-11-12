# Shard Scheduler

The shard scheduler is the ownership model for Phase 2. It does not exist as a production crate yet.

## Target Ownership

Each shard owns:

```text
ready queue
run frame pool
blob arena
binary trace ring
local metrics
```

No global `Arc<Mutex<RunState>>` is allowed in the hot path. A run belongs to one shard at a time.

## Loop Shape

```text
drain inbound commands
drive ready runs
poll action completions
poll timers
flush optional trace ring
wait or spin according to latency profile
```

Deterministic steps run synchronously until finish, error, budget exhaustion, or a future suspension boundary.

## Bounds

Shard queues, active run counts, frame pools, retry loops, fanout, and timer batches must be bounded by configuration.

## Current Integration Points

The current scaffold provides `RunFrame`, `run_until_blocked`, bounded `StepBudget`, and bounded `MemoryIngress`. Phase 2 will connect these into shard-owned execution.
