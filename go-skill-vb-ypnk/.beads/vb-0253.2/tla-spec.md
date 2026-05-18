# TLA+ Temporal Model Plan - vb-0253.2

## Boundary
- **Temporal/workflow behavior**: MPSC queue protocol - submit and receive actions
- **Rust/core behavior excluded from TLA+**: Capacity bounds handled by Kani/Verus
- **External systems abstracted**: None

## TLA+-Owned Clauses
- INV-001: Queue never exceeds capacity during submit/recv protocol

## Model Shape
- Module: MemoryIngress
- Variables: queue (finite sequence), capacity (nat), sender_alive (boolean)
- Init: queue = <<>>, capacity = N, sender_alive = true
- Actions: Submit (enqueue), Recv (dequeue), Disconnect

## Properties
- Safety: queue.len <= capacity
- Liveness: Every submitted frame is eventually received (if sender alive)
- Deadlock freedom: System can always make progress

## Evidence Command
- tlc -config specs/MemoryIngress.cfg specs/MemoryIngress.tla

## Waivers
- None
