# Proof Strategy - vb-0253.1

## Bead
**ID**: vb-0253.1  
**Title**: Wrap shard command queue boundary

## Risk Classification
| Risk | Assessment |
|------|------------|
| Concurrency | Medium - command queue shared within shard |
| Persistence | Low - in-memory queue only |
| Public API | Medium - shard config exposed |
| Unsafe/UB | Low - no unsafe code expected |

## Verifier Selection
| Risk | Verifier | Rationale |
|------|----------|-----------|
| Queue capacity overflow | Kani | Bounded model checking for the `#[cfg(kani)]` queue model plus shared capacity predicate |
| Queue invariants | Verus | Rust-local pure invariants |
| Length correctness | Verus | Functional correctness of accessor |

## Proof Obligations

### PO-001: Queue Capacity Never Exceeded
- **Requirement**: INV-001
- **Verifier**: Kani
- **Artifact**: vb_runtime/src/shard/types.rs
- **Command**: `cargo kani --harness command_queue_bounds`
- **Expected Evidence**: Kani reports no witness for the queue-model/shared-predicate lane
- **Assumptions**: Capacity bounded at construction
- **Required**: Yes
- **Mode**: verify-deep

### PO-002: Queue Invariants Proven
- **Requirement**: INV-001, INV-002
- **Verifier**: Verus
- **Artifact**: vb_runtime/src/shard/types.rs
- **Command**: `verus vb_runtime/src/shard/types.rs`
- **Expected Evidence**: Verus verified with 0 errors
- **Assumptions**: Validated ShardConfig
- **Required**: Yes
- **Mode**: verify-proof

## Waiver Requests
None - all obligations have appropriate verifiers.

## Strategy Summary
- Primary: Kani for bounded model checking of the queue-model/shared capacity predicate lane
- Secondary: Verus for invariant proofs
- No TLA+ needed - local data structure, no temporal behavior
