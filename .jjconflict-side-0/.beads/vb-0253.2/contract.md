# Contract Specification - vb-0253.2

## Context
- **Feature**: Finish ingress modularization and dedupe
- **Domain terms**: Ingress, MemoryIngress, IngressFrame, Bounded Queue, MPSC
- **Assumptions**: MemoryIngress is multi-producer single-consumer
- **Open questions**: What modularization is incomplete? What code is duplicated?

## Preconditions
- PRE-001: MemoryIngress::bounded() requires capacity > 0
- PRE-002: IngressFrame payload size must not exceed max configured limit
- PRE-003: try_submit requires sender not disconnected

## Postconditions
- POST-001: After successful try_submit, frame is available via try_recv
- POST-002: try_recv returns frames in FIFO order
- POST-003: When queue is full, try_submit returns Err(IpcError::Full)
- POST-004: When sender is disconnected and queue empty, try_recv returns Err(IpcError::Disconnected)

## Invariants
- INV-001: Queue length never exceeds bounded capacity
- INV-002: Frames are processed in FIFO order
- INV-003: Disconnected sender propagates to receiver as error

## Error Taxonomy
- IpcError::Full - queue at capacity
- IpcError::Disconnected - sender dropped
- IpcError::PayloadTooLarge - frame exceeds max size

## Contract Signatures
- `fn MemoryIngress::bounded(capacity: usize) -> Self`
- `fn try_submit(&self, frame: IngressFrame) -> Result<(), IpcError>`
- `fn try_recv(&mut self) -> Result<Option<IngressFrame>, IpcError>`
- `fn disconnect_sender(&mut self)`

## TLA+-Owned Clauses
- INV-001 -> TLA+ model for bounded queue capacity

## Verus-Owned Clauses
- INV-001: Queue capacity invariant
- INV-002: FIFO ordering preserved

## Non-goals
- Network-based IPC
- Async ingress with timeouts
