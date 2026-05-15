# Theorem Kernel Projection

## Boundary
- TLA+-owned temporal model: Strict/Journaled admission and Do dispatch lifecycle in `verification/tla/CapabilityLifecycle.tla`.
- Verus-owned Rust core: exact capability match, profile cardinality, schema-valid abstractions, and accepted-certificate preservation in `verification/verus/capability_artifact_model.rs`.
- Theorem-owned kernel: none for State 3.
- Rust/runtime shell: Fjall, postcard, shard scheduling, public runtime APIs, external action dispatch, and UI projection.
- External systems excluded from theorem proof: filesystem/storage, wall-clock time, Makepad/UI, generated Rust, codegen, CLI shell.

## Theorem-Owned Clauses
- None. The algebraic kernel is small enough for Verus and does not require Lean/Aeneas/Hax.

## Verus Projection Instead Of Lean
- INV-001: exact name/action matching -> `proof_exact_match_requires_name_and_action`, `proof_prefix_or_action_mismatch_denies`.
- INV-004: cardinality-exact profile -> `proof_exact_profile_requires_cardinality`, `proof_missing_or_excess_grants_deny`.
- POST-002: certificate preservation -> `proof_certificate_preserves_required_capabilities`, `proof_non_empty_contract_not_erased`.
- PRE-002/PRE-003: schema abstraction -> `proof_gate12_rejects_invalid_schema`.

## Waivers
- Lean waiver: owner `vb-qi37.6`, reason `no theorem beyond Verus-owned equality/cardinality/schema abstractions is needed`, expiry `if capability semantics change to hierarchy, wildcard, lattice, or non-trivial algebra`, compensating evidence `Verus + Kani + TLA + fuzz/proptest obligations`.
