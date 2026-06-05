# ADR 015 (v1): Binary IPC and Backpressure

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

Direct Rust API is the fastest local ingress. External local process ingress uses binary IPC with bounded frames, bounded queues, `mio` eventing, and Postcard payloads.

## Invariants

- No HTTP ingress in runtime core.
- No JSON request routing in runtime core.
- Payload size is checked before decode.
- Full queues return typed backpressure errors.
- v1 IPC does not support pipelining.
- Shutdown acknowledges but does not forcibly cancel pending runs.

## Master Anchors

- Section 21: Binary IPC Protocol
- Section 50: IPC Transport, Backpressure, and Error Codes
