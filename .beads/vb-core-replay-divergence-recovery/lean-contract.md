# Theorem Kernel Projection — vb-core-replay-divergence-recovery

## Boundary

- TLA+-owned temporal model: None (see tla-spec.md)
- Verus-owned Rust core: RecoveryError invariants, RecoveryFrameSeed round-trip, JournalEvent seq ordering, ActionReplayTracker blocking
- Theorem-owned kernel: None
- Rust/runtime shell: Fjall journal I/O, Postcard encode/decode, frame hydration orchestration
- External systems excluded from theorem proof: Fjall storage backend, CompiledWorkflow artifact store

## Theorem-Owned Clauses

None.

Rationale: All critical invariants for this bead are either:
1. Expressible in Verus (typed error exhaustive mapping, seq ordering, Postcard round-trip, ActionReplayTracker blocking logic)
2. Covered by miri on integration/property tests (JournalEvent ordering, corrupt snapshot handling, fail-closed boundaries)

No tiny algebraic theorem kernel exists in this bead that would benefit from Lean/Aeneas/Hax extraction. The algebraic content is the RecoveryError enum exhaustiveness (trivial in Verus), the Postcard round-trip (miri-covered), and the seq ordering (miri-covered).

## Lean/Verus Waiver

| Clause | Reason | Compensating Evidence |
|---|---|---|
| No Lean theorem for RecoveryError | RecoveryError is a simple enum with 10 variants; exhaustive mapping to error semantics is provably correct by construction via Rust's match exhaustiveness and covered by miri on integration tests | Verus match exhaustiveness check + miri on recovery_integration.rs |
| No Lean theorem for Postcard round-trip | Round-trip invariant (input == postcard.decode(postcard.encode(input))) is a property of the postcard codec; miri covers this for all recoverable types in the integration test suite | miri on full_round_trip_recovery_* tests + proptest |
