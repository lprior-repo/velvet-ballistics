# Contract — vb-xi2f.34: Finish Digest Semantics

**Bead**: vb-xi2f.34
**Date**: 2026-05-24
**Rust Contract Agent**: rust-contract

---

## Contract Purpose

This contract defines the behavioral requirements for the `Finish` primitive's contribution to the `WorkflowDigest`. It specifies what MUST be true, what MUST NOT be true, and what is explicitly out of scope. Proof seeds are derived from this contract.

---

## C1: Finish Result Value Sensitivity

**Requirement**: The `result` field of the `Finish` primitive MUST be included in the structural digest computation.

**Specification**: `digest_step_primitive(hasher, StepPrimitive::Finish { result })` MUST:
1. Write the discriminator `b"finish"` to the hasher.
2. Encode `result: ScalarValue` into the hasher using variant-specific encoding:
   - `ScalarValue::String(s)` → `hasher.update(s.as_bytes())`
   - `ScalarValue::Integer(i)` → `hasher.update(&i.to_le_bytes())`
   - Any other variant → `hasher.update(b"unsupported")`

**Acceptance**: Changing the `result` value of a `Finish` step in the source AST MUST change the `WorkflowDigest`. This includes:
- Changing `result: "output_a"` to `result: "output_b"`
- Changing `result: 1` to `result: 2`
- Changing `result: "42"` (String) to `result: 42` (Integer)

**Behavior-affecting**: YES. Digest changes affect workflow identity, admission matching, and recovery.

---

## C2: Finish Step ID Sensitivity

**Requirement**: The step `id` of the `Finish` step MUST be included in the structural digest.

**Specification**: `canonical_digest()` iterates `source.steps()` and writes `hasher.update(step.id.as_bytes())` for each step, including the Finish step.

**Acceptance**: Renaming the Finish step's `id` field MUST change the `WorkflowDigest`.

**Behavior-affecting**: YES. Digest changes affect workflow identity.

---

## C3: Finish Step Position Sensitivity

**Requirement**: The position of the `Finish` step in the source step list MUST affect the digest through the ordering of step ID hashing.

**Specification**: `canonical_digest()` iterates steps in source order. The Finish step's position determines its order in the hash sequence relative to other steps.

**Acceptance**: Moving the `Finish` step from position N to position M (and adjusting other steps accordingly) MUST change the `WorkflowDigest` unless all step IDs and primitives are also reordered to match (which is impossible since Finish is terminal).

**Behavior-affecting**: YES. Digest changes reflect reordering.

---

## C4: Canonical Digest Determinism

**Requirement**: `canonical_digest(source)` MUST be a pure, deterministic function.

**Specification**: Given the same `WorkflowSource` AST, `canonical_digest()` MUST return the same `WorkflowDigest` every time it is called.

**Acceptance**: Any two calls to `canonical_digest()` with structurally identical `WorkflowSource` MUST return equal `WorkflowDigest` values.

**Behavior-affecting**: YES. Determinism is essential for admission consistency and recovery matching.

---

## C5: Hash Discrimination by ScalarValue Variant

**Requirement**: Different `ScalarValue` variants used in `Finish { result }` MUST produce distinct hash contributions that cannot collide by accident.

**Specification**: 
- `String` encoding: UTF-8 bytes (variable length)
- `Integer` encoding: 8-byte little-endian (fixed length)
- These produce different byte sequences for all possible values.

**Acceptance**: `Finish { result: String("42") }` and `Finish { result: Integer(42) }` MUST produce different digests. More generally, for any `s: String` and `i: i64`, unless `s.as_bytes() == &i.to_le_bytes()` (extremely unlikely), the digests MUST differ.

**Behavior-affecting**: YES. Type-level discrimination is essential.

---

## C6: Digest Survives Compilation to CompiledWorkflow

**Requirement**: The digest computed by `canonical_digest()` at the start of `compile_source()` MUST be preserved through to `CompiledWorkflow.digest()`.

**Specification**: 
1. `compile_source()` calls `canonical_digest(source)` → digest
2. `compile_source()` constructs `WorkflowParts { digest, ... }`
3. `vb_validate::shared::validate(&parts)` — digest is read-only, not modified
4. `CompiledWorkflow::try_from_parts(parts)` — digest is moved from `WorkflowParts.digest` to `CompiledWorkflow.digest`

**Acceptance**: After successful compilation, `compiled_workflow.digest()` MUST equal the value originally returned by `canonical_digest(source)`.

**Behavior-affecting**: YES. The digest on the compiled artifact must match the source.

---

## C7: Single Canonical Implementation (Consolidation)

**Requirement**: There MUST be exactly one canonical implementation of the structural digest algorithm in the `vb_compile` crate.

**Specification**: `canonical_digest()` and `digest_step_primitive()` MUST be defined in exactly one location. All callers (including proptest helpers) MUST use the same function.

**Acceptance**: 
- The duplicate definitions in `compile/mod.rs:220-261` MUST be removed OR
- Those definitions MUST be replaced with re-exports/delegations to the canonical implementation in `mod_compile_lowering/part_05.rs` OR
- An equivalence test MUST prove that both implementations produce identical output for all valid inputs, AND the equivalence test MUST be re-run on every CI build.

**Behavior-affecting**: YES. Code duplication creates a divergence risk.

---

## C8: Forward Compatibility of ScalarValue Handling

**Requirement**: When a new `ScalarValue` variant is added, the digest computation MUST either:
1. Explicitly handle the new variant with variant-specific encoding, OR
2. Fail to compile (exhaustive match, no `_` arm), forcing the developer to make an explicit decision.

**Current state**: The `_` arm in the inner match of `digest_step_primitive` (canonical path, line 155) silently produces `"unsupported"` for unknown variants. This is acceptable for the current two-value enum but becomes a hazard if more variants are added without updating the match.

**Acceptance**: A test MUST verify that all current `ScalarValue` variants are explicitly matched in `digest_step_primitive`, i.e., the `_` arm is unreachable for the current enum definition.

**Behavior-affecting**: YES. Forward compatibility of hash semantics.

---

## C9: Digest Is Pre-Validation, Not Post-Validation

**Requirement**: The digest MUST be computed from the source AST, not from the validated compiled IR.

**Specification**: `canonical_digest()` takes `&WorkflowSource`, not `&CompiledWorkflow` or `&WorkflowParts`. It operates on AST types (`StepPrimitive`, `ScalarValue`), not IR types (`CompiledNodeKind::Finish`, `SlotIdx`).

**Rationale**: The digest fingerprints what the author wrote, not what the IR layout decided. The finish result is hashed as a `ScalarValue` (source intent), not as a `SlotIdx` (IR implementation detail).

**Acceptance**: Changing only the slot layout (e.g., adding unused slots) without changing any AST field MUST NOT change the digest.

**Behavior-affecting**: YES. Defines the scope of digest coverage.

---

## C10: Digest Exclusion of Runtime Concerns

**Requirement**: The digest MUST NOT include runtime state, execution results, or any non-source-derived data.

**Specification**: The following are explicitly excluded from the digest:
- `SlotIdx` values (IR implementation detail)
- `ConstIdx` / `ConstValue` pool contents (derived from AST, but layout-dependent)
- Expression bytecode layout
- Accessor program layout
- `ResourceContract`
- `slot_count`, `symbols_count`
- Run-time results or run IDs
- Compilation timestamp or environment

**Acceptance**: Re-compiling the same source with different constant pool layouts or expression optimizations MUST produce the same digest, provided the AST is unchanged.

**Behavior-affecting**: YES. Defines digest stability across compiler versions.

---

## Contract Scope Exclusions (what this contract does NOT cover)

1. **Raw byte digest** (`compute_compiled_digest()`): This is a separate function that hashes raw source bytes. Not covered here.
2. **Digest collision resistance against adversarial input**: blake3's cryptographic properties are assumed, not proven here.
3. **Cross-implementation digest compatibility**: Only the Rust implementation's digest is in scope.
4. **Digest of non-Finish primitives**: Only Finish semantics are in scope. Other primitives are covered by existing (undocumented) contracts.
5. **Budget validation**: Resource contracts and budget enforcement are out of scope.
6. **Taint propagation**: Secret/policy propagation through Finish nodes is out of scope (covered by `secret_finish_tests.rs`).
7. **Codegen correctness**: Generated Rust code from compiled workflows is out of scope.

---

## Contract Compliance Matrix

| Clause | Invariant | Risk | Requires Test | Requires Proof |
|---|---|---|---|---|
| C1: Result value sensitivity | INV-3 | HAZ-5 | YES | Kani / proptest |
| C2: Step ID sensitivity | INV-2 | — | YES | Proptest |
| C3: Position sensitivity | INV-2 | — | YES | Proptest |
| C4: Determinism | INV-1 | — | YES | Proptest |
| C5: Variant discrimination | INV-4 | HAZ-2 | YES | Kani (encoding analysis) |
| C6: Digest survives compilation | INV-6 | HAZ-3 | YES | Integration test |
| C7: Single implementation | INV-7 | HAZ-1 | YES | Code review / consolidation |
| C8: Forward compatibility | — | HAZ-2 | YES | Exhaustiveness test |
| C9: Pre-validation | INV-5 | HAZ-6 | Documentation | Design review |
| C10: Exclusion of runtime | — | — | YES | Proptest |
