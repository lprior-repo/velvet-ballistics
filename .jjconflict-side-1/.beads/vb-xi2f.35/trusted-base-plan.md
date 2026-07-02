# Trusted Base Plan: ResourceContract Digest Coverage

## Bead

`vb-xi2f.35` — P1: digest covers resource contract semantics

## Trusted Base Philosophy

Every proof rests on assumptions. This document enumerates every assumption, stub, bound, trusted surface, and model reduction planned for the vb-xi2f.35 proof obligations. The proof-reviewer shall challenge each trust anchor.

## Trust Level Classification

| Level | Meaning |
|-------|---------|
| **T0** | Axiomatic trust: the Rust compiler, type system, and CPU arithmetic. Not questioned. |
| **T1** | Well-tested external crate: blake3, postcard. Assumed correct but documented. |
| **T2** | Codebase-internal trust: existing validated components. Boundary documented. |
| **T3** | Planned assumptions: beliefs that must hold for proofs to be sound. Reviewer must validate. |
| **T4** | Bounded reductions: deliberate scope limitations. Documented per-obligation. |
| **T5** | Stubs/mocks: replacements for external deps in harnesses. Must be equivalence-proven. |

## Trust Anchors

### T0: Axiomatic Trust

| Anchor | Scope | Justification |
|--------|-------|---------------|
| Rust type system soundness | All obligations | Compiler-enforced. `#![forbid(unsafe_code)]` on all affected crates. Type safety is axiomatic. |
| Rust `PartialEq`/`Eq` correctness | PO-K01..K14, PO-P01..P07 | WorkflowDigest, ResourceContract derive `PartialEq, Eq`. Comparison correctness is a compiler guarantee. |
| CPU integer arithmetic | All obligations | `u16`, `u32`, `u64`, `u8` arithmetic is correct on all target architectures. No custom integer types. |
| `std::any::TypeId` uniqueness | PO-K06 | Rust guarantees distinct types have distinct TypeIds. Compile-time invariant. |
| Rust `Copy` semantics | All obligations | ResourceContract is `Copy`. No aliasing or shared-mutation concerns. |

### T1: External Crate Trust

| Crate | Version | Trust Rationale | Affected Obligations |
|-------|---------|----------------|---------------------|
| **blake3** | latest (Cargo.lock) | Well-audited cryptographic hash. Collision resistance: p < 2^-128. Deterministic. Used as-is, no stubbing for determinism proofs. | All Kani, Verus, Proptest obligations |
| **postcard** | latest (Cargo.lock) | Deterministic COBS-based serializer. Roundtrip injectivity is a postcard guarantee. Trusted for policy digest serialization. | PO-K14 (policy digest) |
| **saphyr** (YAML parser) | latest (Cargo.lock) | Trusted YAML parser. Not in scope for P1 digest proofs; relevant only for P2 (PO-F01, waived). | PO-F01 (waived P2) |

### T2: Codebase Internal Trust

| Component | Trust Boundary | Affected Obligations |
|-----------|---------------|---------------------|
| `vb_core::workflow::ResourceContract` (17 field) | Canonical type. All 17 fields are `Copy`, `PartialEq`, `Eq`. Used as ground truth for digest computation. | All obligations |
| `vb_core::ids::WorkflowDigest` | 32-byte blake3 wrapper. `from_bytes`, `as_bytes`. Correct wrapping assumed. | All obligations |
| `vb_core::validation::resource::validate_resource_contract` | Existing validation for 15 fields is correct. Extending to 17 fields must not break existing checks. | PO-K11 |
| `vb_core::budget::HARD_MAX_TRANSITIONS_PER_TICK` | System hard limit constant. Value is correct and stable. | PO-K11 |
| `vb_yaml::ast::WorkflowSource` | Parsed AST. Correct structure assumed. No contract fields currently. | All source-using obligations |
| `vb_compile::mod_compile_core::compile_workflow` | Public compilation API. Wires together digest + lowering + validation. | PO-K07, PO-P02 |
| `vb_runtime::shard::lifecycle::chunk_002::handle_ask_answer` | Runtime enforcement of allows_secret_results. Current behavior is correct; we only add proof. | PO-K09, PO-V04 |

### T3: Planned Assumptions (Reviewer Gate)

These are beliefs about the post-fix implementation that proofs require. They must be validated by the implementation and reviewed.

| Assumption | Required By | Validation Method |
|-----------|------------|-------------------|
| `canonical_digest(source, contract)` hashes ALL 17 contract fields | PO-K01, PO-K02, PO-P01, PO-V01 | Code review: verify tagged-field hashing of every field. |
| Field encoding uses deterministic byte representations (`to_le_bytes` for multi-byte types) | PO-K03, PO-K12, PO-V02 | Code review: verify encoding function uses stable byte representations. |
| Field tag strings are unique and stable (e.g., `b"max_steps"`, `b"max_slots"`) | PO-K03, PO-V02 | Code review: verify no duplicate field names. |
| `compile_source_with_default(source)` delegates to `compile_source(source, ResourceContract::DEFAULT)` | PO-K13, PO-P06 | Code review: verify delegation, no extra processing. |
| Both compilation paths (part_05.rs and compile/mod.rs) use identical digest logic | PO-K10, PO-P04 | Code review: verify deduplication or manual sync. |
| `CompiledWorkflow` stores the contract passed during construction without modification | PO-K07, PO-V04 | Code review: verify struct field assignment is direct. |
| `CompiledWorkflow::resource_contract()` returns the stored contract faithfully | PO-K09, PO-V04 | Code review: verify accessor is a simple field read. |
| `postcard` serialization of `ResourceContract` is roundtrip-injective | PO-V04 | Unit test: serialize → deserialize → assert equality for many random contracts. |
| BLAKE3 is injective over byte sequences (collision resistance) | PO-V01, PO-V02, PO-V03 | Trusted crate assumption. No direct proof is possible; rely on cryptographic community audit. |
| Hard limits (HARD_MAX_TRANSITIONS_PER_TICK, etc.) are > 0 and < type::MAX | PO-K11 | Code review: verify constant values. |
| `WorkflowParts::resource_contract` is assigned from the input contract parameter (not DEFAULT) | PO-K07 | Code review: verify all 6 entry points use the parameter. |

### T4: Bounded Reductions

Proofs operate on bounded inputs. These bounds are honest representations of real-world usage. If runtime exceeds these bounds, proofs do not guarantee correctness.

| Reduction | Obligations | Bound | Honesty Justification |
|-----------|------------|-------|----------------------|
| Bounded field values for Kani determinism | PO-K01 | max_steps: 1..100, max_slots: 1..32, u64 fields: 1..16, bool: full | Real-world contracts use values in these ranges. Hard limits prevent extreme values. If contracts grow beyond these bounds, determinism still holds (hash is deterministic regardless of value magnitude). |
| Single-field sensitivity at two test values | PO-K02 | Each field tested at {small, small+1} | Property being tested (change ⇒ digest change) does not depend on value magnitude. Any single change must trigger. |
| Cross-field collision at 15 test pairs | PO-K03 | 15 representative cross-field pairs | Structural property. If tagged encoding works for one pair, it works for all (tag prefix uniqueness). |
| Single representative source for bounded Kani | PO-K01, PO-K04, PO-K07, PO-K10, PO-K13 | Single WorkflowSource with 1-3 steps | Digest algorithm is independent of source complexity. More steps would only increase hash time, not change correctness. |
| Validation at boundary values | PO-K11 | max_transitions_per_tick: {0, 1, HARD_MAX, HARD_MAX+1} | These are the only values that exercise the validation logic: zero (error), valid (ok), at-limit (ok), exceeds (error). |
| Dual-path equivalence at single input | PO-K10 | Single (source, contract) pair | If implementations are identical (deduplicated), equivalence is trivial. If not, single-pair proof is weak — but proptest (PO-P04) provides 1,000+ cases. |
| Migration path at DEFAULT contract only | PO-K04 | ResourceContract::DEFAULT only | Migration is a one-time path for existing DEFAULT-compiled workflows. Non-DEFAULT contracts did not exist pre-fix. |
| Encoding injectivity at 10 representative pairs | PO-K12 | 10 pairs: all-zeros vs all-ones, permutation pairs, single-bit diffs | These pairs exercise all collision mechanisms: value boundary, cross-field permutation, bit-level difference. If encoding handles these, it handles all (tag uniqueness). Note: Verus PO-V02 proves injectivity for ALL contracts, so Kani bounded pairs are defense-in-depth. |

### T5: Stubs and Mocks

| Stub | Replaces | Obligations | Equivalence Proof Required? |
|------|----------|------------|---------------------------|
| None | — | — | No stubs are planned. All Kani harnesses use real `blake3` crate. Verus proofs model `blake3` as an injective function — this is a trusted assumption (T1), not a stub. |

If Kani harnesses run too slowly with real `blake3`, a deterministic mock implementing the same `Hasher` trait may be introduced. If so, the mock must be proven equivalent to `blake3` for all bounded inputs used in harnesses. This would become a T5 trust anchor.

## Trusted Boundary: Implementation Pre-Work

The following implementation changes are prerequisites for proofs to be meaningful. If these are not done correctly, proofs become vacuous:

### Prerequisite 1: Single Canonical Type (C2)
- **Required**: Delete 15-field `compiled_workflow::ResourceContract` and route all consumers to `workflow::ResourceContract` (17-field canonical).
- **Vacuous without**: All proofs reference the 17-field type. If the 15-field duplicate still exists, proofs of "17-field contract" are disconnected from runtime.

### Prerequisite 2: Digest Contract Parameter (C1)
- **Required**: Add `contract: ResourceContract` parameter to `canonical_digest()` signature.
- **Vacuous without**: All digest sensitivity proofs require the contract as input. Current signature `canonical_digest(source)` makes proofs inapplicable.

### Prerequisite 3: Tagged Field Encoding (C1, C3)
- **Required**: Implement `hash_contract_fields(hasher, contract)` with domain tags per type-contracts.md Contract 3.
- **Vacuous without**: Cross-field collision proofs (PO-K03, PO-V02) require tagged encoding. Without tags, field-permutation collisions are possible.

### Prerequisite 4: Entry Point Contract Param (C3)
- **Required**: All 6 compilation entry points accept `contract: ResourceContract` parameter.
- **Vacuous without**: Contract-parameter-preservation proofs (PO-K07, PO-P02) require entry points that accept the parameter.

## Trust Validation Matrix

| Trust Anchor | Level | Reviewer Challenge | Mitigation |
|-------------|-------|-------------------|------------|
| blake3 collision resistance | T1 | What if blake3 has a collision bug? | 32-byte output space makes collision probability < 2^-128. Even if blake3 has weaknesses, the probability of accidentally finding a collision in test/proof inputs is astronomically low. |
| postcard injectivity | T1 | What if postcard has a roundtrip bug? | postcard is deterministic by design. A unit test verifying roundtrip for many random contracts mitigates this. |
| Field tag uniqueness | T3 | What if two fields have the same tag string? | Compile-time check via `static_assertions` or build script verifying field name uniqueness. |
| Bounded inputs are representative | T4 | What if real contracts have values outside bounds? | Bounds are conservative. Properties being proved (determinism, sensitivity) are independent of value magnitude. Kani bounds reduce verification time, not proof strength. |
| Dual-path equivalence | T3 | What if paths diverge after deduplication? | Proptest (PO-P04) catches drift. Deduplication is preferred. |
| No stubs needed | T5 | What if blake3 is too slow for Kani? | If needed, introduce a deterministic mock with equivalence proof. Not planned initially. |

## Verification of Trust Anchors

Before proofs can be accepted, these assertions must be independently verified:

1. **Field tag uniqueness audit**: Run a script or test asserting all 17 field tag strings are unique.
2. **Type identity audit**: Run a test asserting `TypeId::of::<compiled_workflow::ResourceContract>()` does not exist (duplicate deleted) or equals `TypeId::of::<workflow::ResourceContract>()` (alias).
3. **Postcard roundtrip test**: Serialize and deserialize 10,000 random ResourceContract values; assert equality.
4. **Entry point parameter audit**: Grep for `ResourceContract::DEFAULT` after fix; count must be 1 (only in the DEFAULT const itself and the `with_default` convenience function).
5. **Dual-path code comparison**: Diff `part_05::canonical_digest` with `compile::mod::canonical_digest`; they must be identical or call the same shared implementation.

## Open Trust Questions

1. **Should blake3 be stubbed for Kani?** Real blake3 may make Kani harnesses impractically slow. If so, a deterministic stub with equivalence proof is needed. This is a T5 decision for the `proof-writer` agent.
2. **Is postcard the right encoding for policy digest?** The policy digest uses postcard serialization; canonical digest uses field-tagged hashing. These are intentionally different. The trust relationship is: both must agree on contract identity direction (different contracts → different digests), not absolute values (PO-K14).
3. **Are Verus proofs feasible for blake3 injectivity?** Modeling blake3 as a black-box injective function is a strong assumption. If Verus cannot handle this, fall back to Verus proving encoding injectivity only (the contract encoding, not the full hash), leaving blake3 as T1 trust.
