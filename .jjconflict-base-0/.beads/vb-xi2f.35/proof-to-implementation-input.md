# Proof-to-Implementation Bridge Input: ResourceContract Digest Coverage

## Bead

`vb-xi2f.35` — P1: digest covers resource contract semantics

## Purpose

This document maps every planned proof obligation to concrete Rust implementation requirements. The `proof-to-implementation` agent will use this to produce `proof-to-implementation.md` with exact source refs, harness refs, and evidence commands.

## Bridge Phasing

This bead has a strong **proof-first** dependency: the implementation cannot be verified until the contract is fixed. However, proofs require the implementation to be testable. The recommended phasing is:

### Phase A: Implementation Prerequisites (proof-writer needs these)
1. Resolve duplicate `ResourceContract` types (C2)
2. Add `contract: ResourceContract` parameter to `canonical_digest()` (C1)
3. Implement tagged field encoding (C1, C3)
4. Add `contract: ResourceContract` to all compilation entry points (C3)
5. Add validation for `max_transitions_per_tick` and `allows_secret_results` (C5)
6. Implement `compile_source_with_default()` convenience function (C3)

### Phase B: Proof Writing (after prerequisites)
7. Write Kani harnesses (14 obligations)
8. Write Verus proofs (4 obligations)
9. Write proptest tests (7 obligations)
10. Write cargo-fuzz target (1, waived)

### Phase C: Proof Execution and Feedback
11. Run proofs, fix implementation if proofs fail
12. Iterate until all proofs pass

## Obligation → Implementation Mapping

### PO-K01: Digest Determinism (Kani)

| Aspect | Detail |
|--------|--------|
| **Proof claim** | `canonical_digest(source, contract)` is deterministic for bounded inputs |
| **Required Rust API** | `pub fn canonical_digest(source: &WorkflowSource, contract: ResourceContract) -> WorkflowDigest` |
| **Implementation location** | `crates/vb_compile/src/mod_compile_lowering/part_05.rs` (primary) and `crates/vb_compile/src/compile/mod.rs` (duplicate — must be synced or deduplicated) |
| **Implementation requirement** | Function must hash all 17 contract fields alongside source properties. Must NOT use any non-deterministic state (no RNG, no time, no I/O). |
| **Harness target** | `verification/kani/vb_compile/digest_determinism.rs` |
| **Test file for unit tests** | `crates/vb_compile/src/tests/` (digest determinism test) |
| **Pre-existing tests** | `compiled_digest_is_deterministic` (source-only, must be extended) |

### PO-K02: Single-Field Sensitivity (Kani)

| Aspect | Detail |
|--------|--------|
| **Proof claim** | For each of 17 fields, changing only that field changes the digest |
| **Required Rust API** | Same `canonical_digest(source, contract)` as PO-K01 |
| **Implementation requirement** | Each field must contribute independently to the hash. The encoding must not ignore or merge fields. |
| **Harness target** | `verification/kani/vb_compile/digest_field_sensitivity.rs` |
| **Test file** | `crates/vb_compile/tests/proptest_contract_field_sensitivity.rs` |

### PO-K03: Cross-Field Collision Prevention (Kani)

| Aspect | Detail |
|--------|--------|
| **Proof claim** | Domain-tagged encoding prevents cross-field hash collisions |
| **Required Rust API** | `fn hash_contract_fields(hasher: &mut blake3::Hasher, contract: &ResourceContract)` |
| **Implementation requirement** | Each field update must be preceded by `hasher.update(b"field_name")`. Field names must be unique, stable strings. |
| **Implementation location** | New function in shared location (both compilation paths call it). Suggested: `crates/vb_compile/src/digest.rs` or within `canonical_digest()` itself. |
| **Harness target** | `verification/kani/vb_compile/digest_cross_field_collision.rs` |

### PO-K04: Migration Digest (Kani)

| Aspect | Detail |
|--------|--------|
| **Proof claim** | New digest = blake3(old_digest_bytes \|\| contract_hash_bytes) for DEFAULT |
| **Required Rust API** | `fn migration_digest(old_digest: &WorkflowDigest, contract: &ResourceContract) -> WorkflowDigest` (or inlined) |
| **Implementation requirement** | The migration path must be deterministic. Old digest is the pre-fix v1 digest (source-only). New digest includes contract. |
| **Implementation location** | `crates/vb_compile/src/mod_compile_core.rs` or new migration module |
| **Harness target** | `verification/kani/vb_compile/migration_digest.rs` |

### PO-K05: Single Canonical Type (Kani)

| Aspect | Detail |
|--------|--------|
| **Proof claim** | Canonical ResourceContract has exactly 17 accessible fields |
| **Required implementation** | Delete `compiled_workflow::ResourceContract` and route all consumers to `workflow::ResourceContract` |
| **Implementation locations** | `crates/vb_core/src/compiled_workflow.rs:130-163` (delete type definition), `crates/vb_core/src/validation/resource.rs:12` (change import), all other references |
| **Harness target** | `verification/kani/vb_core/type_canonical_fields.rs` |
| **Type identity test** | `crates/vb_core/tests/` — test asserting field count == 17 |

### PO-K06: Type Identity Across Code Paths (Kani)

| Aspect | Detail |
|--------|--------|
| **Proof claim** | All modules import the identical ResourceContract type |
| **Required implementation** | No duplicate type definitions. All modules import from canonical source. |
| **Implementation locations** | `crates/vb_core/src/validation/resource.rs:12` (change to `use crate::workflow::ResourceContract`), `crates/vb_core/src/compiled_workflow.rs` (remove duplicate, use canonical), `crates/vb_compile/` (verify via `vb_core::ResourceContract`) |
| **Test assertion** | `std::any::TypeId::of::<validation::ResourceContract>() == std::any::TypeId::of::<workflow::ResourceContract>()` |

### PO-K07: Entry Point Contract Parameter (Kani)

| Aspect | Detail |
|--------|--------|
| **Proof claim** | Non-DEFAULT contract survives compilation and matches `CompiledWorkflow.resource_contract()` |
| **Required implementation** | All 6 entry points accept `contract: ResourceContract` parameter and pass it through to `WorkflowParts` |
| **Implementation locations** | `part_01.rs:54` (compile_source), `part_05.rs:189` (lower_steps_to_ir), `part_08.rs:103` (build_parts), `compile/mod.rs:105` (compile_source), `compile/mod.rs:308` (lower_steps_to_ir), `compile/mod.rs:854-872` (build_parts) |
| **Harness target** | `verification/kani/vb_compile/entry_point_contract.rs` |
| **Proptest target** | `crates/vb_compile/tests/proptest_entry_point_contract.rs` |

### PO-K08: allows_secret_results Digest Sensitivity (Kani)

| Aspect | Detail |
|--------|--------|
| **Proof claim** | `allows_secret_results: true` produces different digest from `allows_secret_results: false` |
| **Required implementation** | `allows_secret_results` must be hashed as part of tagged field encoding |
| **Implementation location** | `hash_contract_fields()` — the `allows_secret_results` field |
| **Harness target** | `verification/kani/vb_compile/digest_field_sensitivity.rs` (sub-harness for allows_secret_results) |

### PO-K09: Runtime Enforcement (Kani)

| Aspect | Detail |
|--------|--------|
| **Proof claim** | Runtime `SecretResultNotAllowed` enforcement matches hashed contract |
| **Required implementation** | Runtime reads `allows_secret_results` from the same `ResourceContract` that was hashed |
| **Implementation location** | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:5-8` — verify this reads from `CompiledWorkflow.resource_contract().allows_secret_results` |
| **Harness target** | `verification/kani/vb_runtime/secret_result_enforcement.rs` |
| **Test file** | `crates/vb_runtime/tests/` — test both true and false for SecretResultNotAllowed |

### PO-K10: Dual Path Equivalence (Kani)

| Aspect | Detail |
|--------|--------|
| **Proof claim** | Both compilation paths produce identical digests |
| **Required implementation** | Either deduplicate `canonical_digest()` into a shared function, or ensure both implementations are identical |
| **Implementation locations** | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:116-138` and `crates/vb_compile/src/compile/mod.rs:220-241` |
| **Recommended** | Move `canonical_digest()` to `crates/vb_compile/src/digest.rs` (new shared module) |
| **Harness target** | `verification/kani/vb_compile/dual_path_equivalence.rs` |

### PO-K11: Validation 17 Fields (Kani)

| Aspect | Detail |
|--------|--------|
| **Proof claim** | `validate_resource_contract()` validates all 17 fields |
| **Required implementation** | `validation/resource.rs` must use canonical 17-field type and validate `max_transitions_per_tick` and `allows_secret_results` |
| **Implementation locations** | `crates/vb_core/src/validation/resource.rs:12` (change import), `crates/vb_core/src/validation/resource.rs:17-21` (extend validate_resource_contract) |
| **New validation checks** | `max_transitions_per_tick == 0 → error`, `max_transitions_per_tick > HARD_MAX → ResourceContractTooLarge`, `allows_secret_results` valid bool → OK |
| **Harness target** | `verification/kani/vb_core/validation_17_fields.rs` |

### PO-K12: Encoding Injectivity (Kani)

| Aspect | Detail |
|--------|--------|
| **Proof claim** | Concrete bounded contract pairs produce different encodings |
| **Required implementation** | The encoding function must be injective for the tested pairs |
| **Implementation location** | `hash_contract_fields()` — the encoding implementation |
| **Harness target** | `verification/kani/vb_core/encoding_injectivity.rs` |

### PO-K13: with_default Equivalence (Kani)

| Aspect | Detail |
|--------|--------|
| **Proof claim** | `compile_source_with_default(source) ≡ compile_source(source, DEFAULT)` |
| **Required implementation** | Add `pub fn compile_source_with_default(source: &WorkflowSource) -> Result<CompiledWorkflow, CompileErrors>` that delegates to `compile_source(source, ResourceContract::DEFAULT)` |
| **Implementation location** | `crates/vb_compile/src/mod_compile_core.rs` |
| **Harness target** | `verification/kani/vb_compile/with_default_equivalence.rs` |

### PO-K14: Canonical vs Policy Digest Agreement (Kani)

| Aspect | Detail |
|--------|--------|
| **Proof claim** | Both digest systems agree on contract identity direction |
| **Required implementation** | No code change needed — this verifies the relationship between existing `canonical_digest()` (post-fix) and existing `compute_policy_digest()` |
| **Implementation locations** | `crates/vb_compile/src/mod_compile_lowering/part_05.rs` (canonical) and `crates/vb_storage/src/admission.rs:204-218` (policy) |
| **Harness target** | `verification/kani/vb_compile/digest_determinism.rs` (sub-harness) |

### PO-V01 through PO-V04: Verus Proofs

| Aspect | Detail |
|--------|--------|
| **Proof claims** | For-all injectivity, encoding injectivity, secret_results injectivity, contract identity preservation |
| **Required Verus setup** | Verus toolchain installed. Verus-compatible builds for `vb_compile` and `vb_runtime`. |
| **Implementation requirement** | The `exec fn` implementations must have `requires`/`ensures` clauses. `spec fn` models must bind to `exec fn` bodies. |
| **Artifact location** | `verification/verus/vb_compile/` and `verification/verus/vb_runtime/` |

### PO-P01 through PO-P07: Proptest Tests

| Aspect | Detail |
|--------|--------|
| **Required setup** | `proptest` crate in dev-dependencies. `Arbitrary` impls for `ResourceContract` and `WorkflowSource`. |
| **Implementation requirement** | `ResourceContract` fields need `proptest::strategy::Strategy` generators. `WorkflowSource` needs an `Arbitrary` impl or manual strategy. |
| **Test locations** | `crates/vb_compile/tests/proptest_*.rs` |

### PO-F01: Cargo-Fuzz (Waived)

| Aspect | Detail |
|--------|--------|
| **Status** | WAIVED for P1 (WC-001). Will be implemented in P2 bead for YAML contract parsing. |

## Implementation Priority Order

1. **C2**: Resolve duplicate types (unblocks everything)
2. **C1**: Add contract parameter to `canonical_digest()` + tagged encoding (core fix)
3. **C3**: Add contract parameter to all entry points + `with_default` convenience
4. **C5**: Extend validation to 17 fields
5. **C8**: Migration digest computation
6. **C6**: Verify/ensure dual path consistency

## Source File Impact Summary

| File | Change | Risk |
|------|--------|------|
| `crates/vb_core/src/compiled_workflow.rs` | Delete duplicate ResourceContract type; update CompiledWorkflow/WorkflowParts to use canonical type | HIGH |
| `crates/vb_core/src/workflow/mod.rs` | No changes to ResourceContract definition; may need additional constructors | LOW |
| `crates/vb_core/src/lib.rs` | May need to adjust re-exports after type resolution | LOW |
| `crates/vb_core/src/validation/resource.rs` | Change import; add validation for max_transitions_per_tick and allows_secret_results | MEDIUM |
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | Add contract param to canonical_digest; add tagged field hashing; pass contract through lower_steps_to_ir | CRITICAL |
| `crates/vb_compile/src/mod_compile_lowering/part_01.rs` | Add contract param to compile_source | HIGH |
| `crates/vb_compile/src/mod_compile_lowering/part_08.rs` | Add contract param to build_parts | MEDIUM |
| `crates/vb_compile/src/compile/mod.rs` | Mirror all part_05.rs changes; add contract param to compile_source, lower_steps_to_ir, build_parts | CRITICAL |
| `crates/vb_compile/src/mod_compile_core.rs` | Add compile_source_with_default; update compile_workflow to accept contract | MEDIUM |
| `crates/vb_compile/src/lib.rs` | Re-export new API; may need new module declarations | LOW |
| `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` | May need update if CompiledWorkflow.resource_contract() changes (depends on type resolution) | MEDIUM |
| `crates/vb_storage/src/admission.rs` | No changes (policy digest is separate) | NONE |

## Dependency Map for Implementation

```
Phase A: Fix types
    vb_core::compiled_workflow (delete duplicate)
    → vb_core::validation::resource (switch import)
    → vb_core::budget (verify import)
    → vb_compile (verify all imports use canonical)

Phase B: Add contract to digest
    vb_compile::part_05::canonical_digest (add parameter + encoding)
    vb_compile::compile::mod::canonical_digest (mirror)
    → New shared function hash_contract_fields()

Phase C: Add contract to entry points
    vb_compile::part_01::compile_source (add parameter)
    vb_compile::part_05::lower_steps_to_ir (add parameter)
    vb_compile::part_08 (add parameter)
    vb_compile::compile::mod::compile_source (add parameter)
    vb_compile::compile::mod::lower_steps_to_ir (add parameter)
    vb_compile::compile::mod::build_parts (add parameter)
    vb_compile::mod_compile_core (add with_default, update compile_workflow)

Phase D: Extend validation
    vb_core::validation::resource (add max_transitions_per_tick + allows_secret_results checks)

Phase E: Proof harnesses + tests
    verification/kani/** (14 harnesses)
    verification/verus/** (4 proofs)
    crates/vb_compile/tests/** (7 proptest suites)
```

## Implementation Constraints from Proof Obligations

1. **No unsafe code**: All affected paths must remain `#![forbid(unsafe_code)]`.
2. **No unwrap/expect/panic**: All error paths must use `Result<T, E>`.
3. **Deterministic hashing**: `canonical_digest()` must be pure — no I/O, no RNG, no system time.
4. **Stable field order**: The 17 fields must be hashed in a stable, canonical order. The order in type-contracts.md Contract 3 is the canonical order.
5. **Tagged field encoding**: Each field must be preceded by a domain tag string. Tag strings must be unique.
6. **No silent DEFAULT**: Compilation must not silently fall back to DEFAULT when a non-DEFAULT contract is provided.
7. **Dual-path consistency**: Both compilation paths must produce identical digests. Prefer deduplication.

## Outstanding Design Decisions

These questions must be resolved before implementation begins:

1. **Shared or duplicated digest logic?** Should `canonical_digest()` be moved to a shared module (`crates/vb_compile/src/digest.rs`) or kept duplicated in both paths? Recommendation: shared module, single source of truth.

2. **Contract in WorkflowSource AST?** Should `WorkflowSource` gain an `Option<ResourceContract>` field (YAML-sourced contracts, P2)? Or should the contract remain a separate parameter? Recommendation: keep as separate parameter for P1; revisit for P2 YAML integration.

3. **Migration strategy**: Should the new digest replace the old one immediately (breaking change for existing artifacts) or should there be a compatibility window? Recommendation: immediate replacement with migration note (per contract.md C8 reconsideration). Old artifacts used policy_digest for admission, not canonical_digest.

4. **Postcard vs tagged encoding for policy digest**: The policy digest uses postcard serialization; canonical digest uses tagged field encoding. Should they be unified? Recommendation: keep separate per contract.md C10 — they serve different purposes.

## Ready for

- `proof-plan-reviewer`: Review lane decisions and obligation sufficiency
- `proof-writer`: After implementation prerequisites, write Kani/Verus/Proptest artifacts
- `proof-to-implementation`: After proof-reviewer approval, produce final bridge document
