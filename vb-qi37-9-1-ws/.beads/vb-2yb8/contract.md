# Contract Specification — vb-2yb8

## Context
- **Feature:** Per-primitive durability proof matrix
- **Bead:** vb-2yb8 (P0)
- **Domain terms:**
  - Primitive: A YAML step kind (`set`, `do`, `choose`, `for_each`, `together`, `collect`, `reduce`, `repeat`, `wait`, `ask`, `finish`)
  - JournalEvent: Durable storage record emitted before external acknowledgment
  - ShardCommand: External command accepted by the shard
  - Ack point: The moment a command handler returns Ok(()) to the caller
  - Replay assertion: A testable claim that replay from journal produces identical state
- **Assumptions:**
  - The matrix is a compile-time/static data structure, not a runtime hot path
  - CI enforces the matrix via a test that fails when a primitive lacks proof
  - The existing journal and shard code is correct; this bead only verifies and wires the proof
- **Open questions:**
  - Should the matrix include `CompiledNodeKind` variants or YAML primitives? → Use both: YAML primitive → IR node kind → journal events

## Preconditions
- [ ] All journal event types are stable and versioned (RecordKind IDs fixed)
- [ ] All shard command handlers emit events before returning Ok(())
- [ ] Existing tests cover at least one happy path per handler

## Postconditions
- [ ] Every primitive has a matrix row with: event type, storage partition, ack point, replay assertion, test evidence
- [ ] Missing evidence is converted to follow-up beads or failing tests
- [ ] The matrix is wired into a release durability gate test

## Invariants
- [ ] A primitive cannot be checked off without at least one real test or executable verifier assertion
- [ ] Ack-after-persist ordering is explicit for every mutation path
- [ ] Replay from journal produces the same externally visible state as the original execution

## Error Taxonomy
- `DurabilityError::MissingPrimitiveRow { primitive }` — primitive has no matrix entry
- `DurabilityError::MissingReplayProof { primitive, event }` — row exists but no test links the event to replay
- `DurabilityError::AckBeforePersist { primitive, handler }` — handler acknowledges before journal append
- `DurabilityError::OrphanEvent { event }` — journal event has no associated primitive
- `DurabilityError::MismatchedPartition { event, expected, actual }` — event stored in wrong partition

## Contract Signatures
```rust
/// A row in the durability matrix.
pub struct DurabilityRow {
    pub primitive: &'static str,
    pub compiled_node_kind: &'static str,
    pub journal_events: &'static [RecordKind],
    pub storage_partition: StoragePartition,
    pub ack_point: AckPoint,
    pub replay_assertion: &'static str,
    pub test_evidence: &'static [&'static str],
}

/// All matrix rows. Missing rows are compile errors or test failures.
pub const DURABILITY_MATRIX: &[DurabilityRow] = &[
    // ... rows for each primitive
];

/// Verifier that every primitive has a row and every row has replay proof.
pub fn verify_matrix_against_tests(test_registry: &TestRegistry) -> Result<(), DurabilityError>;

/// Verifier that no handler acknowledges before persisting.
pub fn verify_ack_after_persist(handlers: &HandlerAudit) -> Result<(), DurabilityError>;
```

## Non-goals
- [ ] Do not rewrite shard handlers (this bead verifies existing code)
- [ ] Do not change RecordKind IDs
- [ ] Do not add runtime overhead to hot paths
