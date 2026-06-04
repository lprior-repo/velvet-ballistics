# IPC And Memory Boundary

`velvet-ballistics` does not use HTTP as the runtime control plane. The primary ingress shape is a bounded memory queue carrying binary frames.

## Initial Boundary

`vb-ipc` provides `MemoryIngress`, a bounded multi-producer/single-consumer queue. Producers use `try_submit`; a full queue returns backpressure immediately instead of blocking or allocating without limit.

## Future IPC Options

The next IPC transports should preserve the same bounded frame contract:

- Unix domain sockets for local process integration.
- Shared memory rings for lowest-copy same-host producers.
- Memory-mapped files for replayable local handoff.
- io_uring-backed local pipes only if benchmarked faster for the target workload.

## Forbidden In Hot Path

- HTTP ingress.
- JSON request routing.
- Dynamic plugin lookup by string.
- Unbounded channels.
- Per-step task spawning.
- Blocking filesystem calls inside deterministic step chains.
