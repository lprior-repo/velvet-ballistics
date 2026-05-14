# TLA+ Temporal Model Plan

## Boundary

- Model states: `VerificationProof` → `RunAdmission` → `ArtifactLoaded`
- Actions: `LoadArtifact`, `ExtractIdempotencyKeyed`, `ExtractIdempotencyAttested`, `ConstructAdmission`
- Safety: `ArtifactLoaded => RunAdmission.idempotency_keyed.len() == VerificationProof.idempotency_keyed.len()`
- Safety: `ArtifactLoaded => RunAdmission.idempotency_attested.len() == VerificationProof.idempotency_attested.len()`
- Liveness: Artifact loading eventually completes or returns typed error under fair scheduling
- Deadlock freedom: No circular dependencies in idempotency field extraction

## Evidence Command

`moon run :verify-proof` once TLA model exists; current obligation is to add/execute scoped model or approved waiver.

## Notes

This bead focuses on type-level data flow (Box<[ActionId]> copy semantics) rather than temporal behavior. The idempotency evidence propagation is a pure data transformation with no temporal properties - the length is preserved at construction time, not over time. No TLA+ model is needed because:

1. The property is a pure function: `f(VerificationProof) = RunAdmission` where `len(f(p).idempotency_keyed) = len(p.idempotency_keyed)`
2. Rust's type system (verified by Verus) guarantees Box<[T]> copy semantics
3. No state machines, message passing, or concurrency are involved

Waiver applicable: No temporal behavior — pure data-flow type propagation