bead_id: vb-5h50
bead_title: storage: Trim journal events after durable snapshots
phase: state-3-contract
updated_at: 2026-05-09T00:00:00Z

# Verification Layers

## Layer 0: Unit Tests (Fast)
- Every public function has at least one direct unit test.
- Boundary conditions: empty journal, single event, max sequence numbers.
- Error paths: no snapshot, retention block, storage failure.

## Layer 1: Property Tests (Proptest)
- `test_trim_preserves_replay_equivalence`: For arbitrary event sequences + snapshot, replay after trim == replay before trim.
- `test_trim_idempotent`: Second trim with same policy is always NoOp.
- `test_retention_policy_never_trims_retained_runs`: If a terminal run is among the N most recent, it is never trimmed.

## Layer 2: Integration Tests
- Full round-trip: write events → write snapshot → trim → recover → assert state equality.
- Multi-run scenario: multiple runs, some terminal, some active; trim_all_eligible_runs respects retention.

## Layer 3: Formal Verification (Kani)
- `verify_trim_boundary`: Prove that for any event key, if seq >= cutoff, the key is NOT in the deletion set.
- `verify_idempotence`: Prove that two consecutive trims with identical inputs produce identical outputs.
- Scope: Pure key-comparison logic only; Fjall I/O is out of scope for Kani.

## Layer 4: Miri (Stack/Heap Safety)
- Run all unit tests under Miri to detect undefined behavior in key slicing/byte manipulation.

## Waivers
- Kani cannot verify Fjall storage semantics (external crate, async I/O).
- Loom not applicable: no concurrent data structures in trimming logic (Fjall handles concurrency internally).
