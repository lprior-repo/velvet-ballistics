# Proof Strategy - vb-0253.2

## Bead
**ID**: vb-0253.2  
**Title**: Finish ingress modularization and dedupe

## Risk Classification
| Risk | Assessment |
|------|------------|
| Concurrency | High - MPSC queue with multiple producers |
| Persistence | Low - in-memory queue |
| Error Handling | Medium - Full/Disconnected error variants |
| Public API | High - IngressFrame, MemoryIngress public |

## Verifier Selection
| Risk | Verifier | Rationale |
|------|----------|-----------|
| Queue capacity overflow | Kani + Verus | Bounded model checking + invariant proofs |
| FIFO ordering | Verus | Functional correctness of ordering |
| Error propagation | Unit tests | Error variant coverage |

## Proof Obligations

### PO-001: Queue Capacity Never Exceeded
- **Requirement**: INV-001
- **Verifier**: Kani + Verus
- **Artifact**: vb_ipc/src/ingress.rs
- **Command**: `cargo kani --harness ingress_capacity` + `verus vb_ipc/src/ingress.rs`
- **Expected Evidence**: Kani no witness + Verus 0 errors
- **Required**: Yes
- **Mode**: verify-deep

### PO-002: FIFO Ordering Preserved
- **Requirement**: INV-002
- **Verifier**: Verus
- **Artifact**: vb_ipc/src/ingress.rs
- **Command**: `verus vb_ipc/src/ingress.rs`
- **Expected Evidence**: Verus verified ordering
- **Required**: Yes
- **Mode**: verify-proof

## Waiver Requests
None.

## Strategy Summary
- Primary: Kani for bounded capacity, Verus for invariants
- Secondary: Unit tests for error variant coverage
- TLA+ for protocol model (if temporal behavior needed)
