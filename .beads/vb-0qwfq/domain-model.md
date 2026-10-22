# Domain Model: Artifact Acceptance Sequence

## Ubiquitous Language

- **Artifact**: A compiled workflow ready for runtime execution, identified by its content digest.
- **Acceptance**: The journal's act of admitting an artifact as executable, recording its provenance and binding it to a durable sequence.
- **Acceptance Sequence**: A journal-wide, monotonic counter value assigned at artifact acceptance time. Not a run event sequence; it identifies when the artifact was accepted, not when it is executed.
- **Certificate Freshness**: A caller's minimum acceptable acceptance sequence, used to reject artifacts accepted before a known point.
- **Journal Sequence Authority**: The FjallJournal instance's exclusive power to allocate acceptance sequence values.

## Entities

### ArtifactEnvelope (value object)
- `digest`: BLAKE3 hash of the envelope (includes accepted_at_seq)
- `source_digest`: digest of the original CompiledWorkflow parts
- `policy_digest`: digest of the resource contract at admission time
- `ir`: postcard-encoded WorkflowParts
- `verification`: proof structure with gate count and flags
- `accepted_at_seq`: **new** — the journal-assigned acceptance sequence (u64)
- `required_capabilities`: capabilities needed by artifact actions
- `action_contracts`: validated action contracts bound at admission

### FjallJournal (aggregate root)
- Owns the acceptance sequence counter
- Provides `accept_seq_allocate()`: atomic counter increment
- Provides artifact submission APIs that bind the sequence

## Relationships

- One FjallJournal → many artifacts accepted with distinct sequences
- Each artifact has exactly one acceptance sequence, assigned at submission
- Acceptance sequence is independent of run event sequences
- Run admission may require artifact's acceptance sequence >= floor

## Invariants

1. **Monotonicity**: Each allocation yields a strictly greater value than any prior allocation.
2. **Uniqueness per acceptance**: No two accepted artifacts share the same acceptance sequence.
3. **Nonzero after allocation**: After the first allocation, all values are >= 1.
4. **Digest inclusion**: The acceptance sequence is part of the artifact envelope digest.
5. **Journal authority**: Only the FjallJournal instance can allocate sequences; callers cannot fabricate them.
