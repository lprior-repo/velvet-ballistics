# Performance Contract

Velvet Ballistics optimizes latency and throughput by refusing to put authoring concerns on the execution path.

## Non-Negotiable Contract

1. YAML is parsed and validated before runs are accepted.
2. A run binds to an immutable compiled workflow digest.
3. Runtime references are numeric slot accesses.
4. Runtime steps are numeric state transitions.
5. Expressions are compiled programs, not strings.
6. The engine does not spawn one task per step.
7. Deterministic runs execute in a tight synchronous loop.
8. Side-effect boundaries are explicit and durable according to policy.
9. Durable journal events are binary records.
10. Observability records are projected outside the hot loop.
11. All queues, fanout, retries, waits, payloads, and logs are bounded.
12. No runtime PR can claim speed without benchmark evidence.

## Latency Truth

The in-memory transition loop can be extremely fast. These operations are not nanosecond-class and must not be hidden inside the hot loop:

- YAML parsing.
- Schema validation.
- JSON parsing.
- Disk write barriers and fsync.
- External action calls.
- Shell process spawning.
- Pretty JSONL logging.
- Full trace rendering.

## Release Profiles

The `maxperf` profile uses fat LTO and one codegen unit. Benchmark native CPU builds separately with `RUSTFLAGS="-C target-cpu=native"`; do not ship native builds unless deployment CPUs are controlled.
