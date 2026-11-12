# Contract: ResourceContract Digest Coverage

## Bead

`vb-xi2f.35` — P1: digest covers resource contract semantics

## Contract Version

`velvet-ballastics/v1`

## Contract Scope

This contract defines the correct binding between `ResourceContract` fields and the `WorkflowDigest` produced by compilation. It governs:
1. The canonical digest function's input contract
2. The single-canonical-type requirement
3. The compilation entry point contract
4. The validation contract for all 17 fields
5. The taint sensitivity contract

## Clause 1: Digest-Contract Binding (CRITICAL)

**Statement**: The canonical digest `canonical_digest(source, contract)` SHALL produce a deterministic `WorkflowDigest` that is a function of BOTH the source definition AND all 17 fields of the resource contract.

**Formalization**:
```
For all source: WorkflowSource, contract_a: ResourceContract, contract_b: ResourceContract:
  contract_a ≠ contract_b ⇒ canonical_digest(source, contract_a) ≠ canonical_digest(source, contract_b)
```

**Acceptance Criteria**:
- AC-1.1: Adding contract fields to the hash changes the digest
- AC-1.2: Changing any single contract field changes the digest
- AC-1.3: Same (source, contract) always produces same digest (determinism)
- AC-1.4: All 17 fields are included in the hash, in a stable order
- AC-1.5: The hash domain-tags each field to prevent cross-field collisions

**Violation (Current State)**: `canonical_digest()` accepts only `source`, not `contract`. Zero contract fields are hashed. AC-1.1 through AC-1.4 all fail.

---

## Clause 2: Single Canonical ResourceContract Type (REQUIRED)

**Statement**: There shall be exactly one `ResourceContract` type in the codebase. All compilation, validation, budget, and runtime code shall use the same type.

**Acceptance Criteria**:
- AC-2.1: `vb_core::compiled_workflow::ResourceContract` (15-field) is either deleted or extended to 17 fields
- AC-2.2: `vb_core::workflow::ResourceContract` (17-field) is the single source of truth
- AC-2.3: `validation/resource.rs` imports from the canonical type
- AC-2.4: `CompiledWorkflow` and `WorkflowParts` in both modules use the same `ResourceContract` type
- AC-2.5: No code can accidentally use the wrong `ResourceContract`

**Violation (Current State)**: Two types exist. AC-2.1 through AC-2.5 all fail.

---

## Clause 3: Compilation Entry Point Contract

**Statement**: Every compilation entry point shall accept a `ResourceContract` parameter (not hardcode DEFAULT).

**Acceptance Criteria**:
- AC-3.1: `compile_source(source, contract)` is the primary API signature
- AC-3.2: All 6 hardcoded DEFAULT locations are removed
- AC-3.3: A convenience function `compile_source_with_default(source)` delegates to `compile_source(source, ResourceContract::DEFAULT)` for backward compatibility
- AC-3.4: Both compilation paths (`mod_compile_lowering` and `compile/mod.rs`) are updated

**Violation (Current State)**: All 6 entry points hardcode DEFAULT. AC-3.1 through AC-3.4 all fail.

---

## Clause 4: Taint Flag Digest Sensitivity

**Statement**: The `allows_secret_results` field of `ResourceContract` SHALL be hashed into the canonical digest. Changing this flag SHALL produce a different digest.

**Rationale**: `allows_secret_results` gates runtime behavior in `handle_ask_answer()` (returns `Err(SecretResultNotAllowed)` when `false`). Two workflows that differ only in this flag have different runtime behavior and shall have different digests.

**Acceptance Criteria**:
- AC-4.1: `allows_secret_results: true` produces different digest from `allows_secret_results: false`
- AC-4.2: The runtime check (`chunk_002.rs:6`) references the same `allows_secret_results` that was hashed
- AC-4.3: There is a test verifying AC-4.1

**Violation (Current State)**: `allows_secret_results` is not hashed. AC-4.1 through AC-4.3 all fail.

---

## Clause 5: Validation Must Cover All 17 Fields

**Statement**: `validate_resource_contract()` shall validate all 17 fields of `ResourceContract` against the compiled workflow's actual resource usage.

**Acceptance Criteria**:
- AC-5.1: `max_transitions_per_tick` is validated against `HARD_MAX_TRANSITIONS_PER_TICK`
- AC-5.2: `allows_secret_results` is validated for consistency (valid bool)
- AC-5.3: Validation errors for the two new fields produce `WorkflowError::ResourceContractExceeded` or `ResourceContractTooLarge` with appropriate `resource` identifiers
- AC-5.4: All 17 fields have explicit validation checks

**Violation (Current State)**: `validation/resource.rs` uses the 15-field type. AC-5.1 through AC-5.4 fail for the two missing fields.

---

## Clause 6: Dual Compilation Path Consistency

**Statement**: Both compilation paths (`mod_compile_lowering` and `compile/mod.rs`) shall produce identical digests given identical inputs.

**Acceptance Criteria**:
- AC-6.1: `canonical_digest()` in both paths hashes the same fields in the same order
- AC-6.2: A test exists verifying cross-path digest equality
- AC-6.3: Ideally, both paths share a single `canonical_digest()` implementation (deduplication)

**Violation (Current State)**: Both paths have identical (buggy) implementations today. Risk of drift if one is fixed and not the other.

---

## Clause 7: YAML Contract Parsing (Future)

**Statement**: If resource contracts are sourced from YAML, the parser shall accept, validate, and propagate a `resource_contract` section.

**Acceptance Criteria**:
- AC-7.1: Parser whitelist includes `"resource_contract"` as a valid top-level key
- AC-7.2: All 17 contract fields are parseable from YAML
- AC-7.3: Unknown fields inside the contract section are rejected
- AC-7.4: Missing contract section → parser returns `None`, caller uses DEFAULT
- AC-7.5: Invalid contract field types produce `YamlError::InvalidResourceContract`

**Violation (Current State)**: Parser whitelist excludes `resource_contract`. AC-7.1 through AC-7.5 not applicable yet (P2 priority).

---

## Clause 8: Backward Compatibility

**Statement**: Existing compiled artifacts that were produced with `ResourceContract::DEFAULT` must continue to be valid after the digest fix.

**Acceptance Criteria**:
- AC-8.1: `compile_source_with_default(source)` produces the same digest as the old `canonical_digest(source)` **IF the contract stored in the workflow is DEFAULT**. Actually, this is impossible if we add contract to the hash — the new digest WILL differ.

**Reconsidered**: Adding the contract to the digest WILL change the digest for all workflows, even those using DEFAULT. This is a one-time migration. The acceptance criteria become:

- AC-8.1: A migration note documents that digest values change
- AC-8.2: Previously admitted artifacts are not invalidated (admission uses policy_digest, not canonical digest, for contract identity)
- AC-8.3: Fresh compilations produce new digests that include the contract

---

## Clause 9: Proof Obligation

**Statement**: The correctness of Clause 1 (digest-contract binding) shall be provable via formal or bounded verification.

**Acceptance Criteria**:
- AC-9.1: A Kani harness proves `canonical_digest` is deterministic
- AC-9.2: A Kani harness proves different contracts produce different digests (for at least one contract field)
- AC-9.3: A proptest verifies digest sensitivity across many random contract pairs
- AC-9.4: A Verus spec formally binds the hash function to contract equality

---

## Clause 10: Non-Requirements (Out of Scope)

The following are explicitly NOT required by this contract:
- Unifying `canonical_digest` with `compute_policy_digest` (they serve different purposes)
- Changing `compute_compiled_digest()` (byte-level hash already correct)
- Modifying runtime behavior (runtime already correctly enforces contract limits)
- Changing the YAML language specification (P2, out of scope for this bead)
- Adding new contract dimensions beyond the 17 existing fields

---

## Contract Status

| Clause | Status | Severity |
|--------|--------|----------|
| C1: Digest-Contract Binding | VIOLATED | CRITICAL |
| C2: Single Canonical Type | VIOLATED | HIGH |
| C3: Entry Point Contract | VIOLATED | HIGH |
| C4: Taint Digest Sensitivity | VIOLATED | HIGH |
| C5: Full Validation Coverage | VIOLATED | HIGH |
| C6: Dual Path Consistency | AT-RISK | MEDIUM |
| C7: YAML Contract Parsing | NOT-IMPLEMENTED | MEDIUM |
| C8: Backward Compatibility | NEEDS-PLANNING | MEDIUM |
| C9: Proof Obligation | NOT-STARTED | REQUIRED |
| C10: Non-Requirements | CONFIRMED | N/A |
