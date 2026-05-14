# TLA+ Specification Notes for vb-7m54

## Temporal Verification Approach

This bead uses **Loom** (not TLA+) for temporal/concurrency verification because:
1. Loom is the appropriate tool for Rust-level concurrency seams (per master doc line 4964)
2. The concurrency seams involve shared mutable state and atomic ordering, not state-machine protocol properties
3. TLA+ is used for macro-level temporal properties (recovery, scheduling, frame transitions) as documented in the root formal-verification-report.md

## Loom Models

The 5 loom models cover:

| Model | Concurrency Property |
|-------|---------------------|
| journal_writer_queue | Ordered write before flush |
| action_completion_cancel | Completion vs cancel ordering |
| timer_fired_cancel | Timer vs cancel ordering |
| shutdown_drain | Graceful shutdown ordering |
| bounded_queue | Enqueue/dequeue invariants |

## Relationship to TLA+ Specs

The loom models complement (not replace) the TLA+ specs in `specs/tla/`:
- ShardScheduler.tla: Verifies shard-level scheduling decisions (already PASS)
- RecoveryReplay.tla: Verifies journal replay ordering (already PASS)

The loom models verify the **implementation-level** concurrency of the runtime seams that the TLA+ specs model abstractly.

## No Additional TLA+ Required

No new TLA+ spec is needed for VB-CONC-001..005 because:
- The master doc defines the ordering invariants in Section 49
- The proof_obligations.yaml defines the exact properties to verify
- Loom provides the appropriate Rust-level model checking
