# TLA+ Temporal Model Plan — vb-0253.3

## Boundary
- **Temporal/workflow behavior**: None — the IPC bridge's polling loop, send/poll semantics, recv_timeout behavior, and thread lifecycle are unchanged. This is a Rust-local API change (unbounded → bounded channel) with no state-machine or protocol change.
- **Rust/core behavior excluded from TLA+**: Bounded channel capacity enforcement, `try_send` error taxonomy, `send()` return type change, `poll()` drain behavior
- **External systems abstracted**: Unix socket IPC server, `IpcClient` socket I/O, Makepad UI render loop
- **Non-applicability rationale**: The IPC bridge is a single-threaded UI-to-background-thread conduit. The only state-over-time behavior is the request queue length, which is bounded by construction. There is no concurrent workflow, no retry logic, no claim/lease, no distributed coordination, and no liveness condition that depends on timeouts beyond the existing 100ms recv_timeout. The bounded-channel safety property (channel never exceeds capacity) is a Rust-level invariant provable by unit test and compilation. TLA+ would not add value over a Rust unit test for this change.

## TLA+-Owned Clauses
None.

## Model Shape (Not Applicable)
No TLA+ model required for this change. The bounded channel is a Rust-local API constraint, not a temporal protocol.

## Properties
- Not applicable.

## Evidence Command
Not applicable.

## Waivers
- **WAIVER-TLA-001**: No TLA+ model for bounded channel backpressure
  - **Owner**: vb-0253.3 contract
  - **Reason**: Single-threaded IPC conduit; no temporal protocol, workflow, liveness, or deadlock conditions to verify. Bounded channel capacity is enforced at construction and verified by unit tests.
  - **Expiry**: Never — this is a Rust-local API change
  - **Compensating evidence**: Unit tests (`bridge_send_on_full_returns_error`, etc.) + compile verification + optional proptest for capacity boundary exploration
