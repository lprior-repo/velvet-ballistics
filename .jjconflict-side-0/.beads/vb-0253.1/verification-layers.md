# Verification Layers - vb-0253.1

## Boundary
- **Verus-owned kernel**: Queue capacity invariants, length correctness, precondition checks
- **TLA+ temporal model**: None (queue is local, not temporal)
- **Theorem projection**: None needed
- **Runtime shell**: Enqueue/dequeue with Result error handling
- **External systems excluded**: None

## Layer Assignment
- INV-001 -> verus + kani (Rust-local queue capacity invariants)
- INV-002 -> verus (length accessor correctness)
- PRE-001 -> proptest + unit test
- POST-003 -> kani + unit test (queue full error case)
- ERR-FULL -> unit test (error variant coverage)

## Verus Scope
- **Rust target**: vb_runtime::shard::Shard
- **Spec/proof function**: Queue invariants as Verus specs
- **Invariants**: queue.len() <= queue.capacity(), length accessor matches
- **Trusted boundary**: Validated ShardConfig at construction
- **Shell exclusions**: None

## Kani Scope
- **Rust target**: vb_runtime::shard command queue operations
- **Claim**: Bounded model checking for capacity overflow
- **Command**: cargo kani --harness command_queue_bounds
- **Evidence**: Kani witness report

## Waivers
- TLA+ waived: no temporal/protocol behavior
- Lean waived: Verus sufficient for invariants
