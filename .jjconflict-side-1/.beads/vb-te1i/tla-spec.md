# TLA+ Temporal Model Plan: vb-te1i Binary IPC

## Boundary

- **Temporal/workflow behavior**: None. The Binary IPC acceptance bead is a data-validation and serialization layer. There are no state machines, no concurrent client sessions interacting, no schedulers, no queues with ordering invariants beyond the Rust-level SPSC queue, no retry/lease logic, no liveness properties, and no deadlock possibilities.
- **Rust/core behavior excluded from TLA+**: Header decode/encode, payload bounds, command ID mapping, error variant mapping, bounded queue push/pop, async mio server loop. These are handled by Verus, unit tests, proptest, Kani, and Loom.
- **External systems abstracted**: Unix kernel socket buffer, OS scheduler for async I/O.
- **Non-applicability rationale**: TLA+ is designed for state-over-time behavior in concurrent/distributed systems. A 24-byte binary frame codec is a pure function `bytes → Result<Frame, Error>`. The only "state" is the fixed header layout, which is a data structure invariant, not a temporal property.

## TLA+-Owned Clauses

**None.**

Rationale: This bead does not involve:
- State machines with temporal transitions
- Concurrent client sessions with ordering constraints
- Schedulers or work queues
- Retry, claim, or lease logic
- Liveness, eventual consistency, or fairness properties
- Deadlock-prone resource cycles

The concurrent surface (mio event loop, SPSC queue) is covered by:
- **Loom**: Permutation testing of concurrent `serve_ipc` / `poll_once` / `MemoryIngress` operations
- **Unit tests**: Deterministic single-threaded decode/encode/roundtrip tests
- **Kani**: Bounded model checking of header decode (already exists at `crates/vb_ipc/src/kani_ipc_header.rs`)

## Waiver Record

| Clause | Owner | Reason | Expiry | Compensating Evidence |
|---|---|---|---|---|
| TLA+ model for IPC server concurrency | vb-te1i | mio event loop + SPSC queue is not a TLA+-suitable model; Loom covers interleavings | N/A | Loom test `crates/vb_ipc/src/queue/tests/array_queue_tests.rs` + `kani_ipc_header.rs` |
| TLA+ model for frame decode | vb-te1i | Pure function, not a state machine | N/A | Kani harness `kani_ipc_header.rs` + `kani_ipc_header_rejects_oversize.rs` |

## Evidence

No TLA+ commands are required for this bead. Downstream agents should verify using the existing Kani and Loom evidence paths.
