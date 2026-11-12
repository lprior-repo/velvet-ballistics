# Hazard Analysis — vb-xi2f.34: Finish Digest Semantics

**Bead**: vb-xi2f.34
**Date**: 2026-05-24
**Rust Contract Agent**: rust-contract

---

## Hazard Taxonomy

Each hazard is tagged with categories:
- **Temporal**: Hazards arising from ordering, interleaving, or state machine transitions
- **Invariant**: Hazards that could violate a core domain invariant
- **Bounded State**: Hazards from numeric overflow, bounds violations, or resource exhaustion
- **Refinement**: Hazards from incomplete or lossy abstraction
- **Concurrency**: Hazards from shared mutable state or parallel execution
- **Unsafe/Provenance**: Hazards from raw pointers, FFI, or undefined behavior
- **Hostile Input**: Hazards from malicious or malformed input
- **Performance**: Hazards from algorithmic complexity or resource consumption
- **Release/API**: Hazards from versioning, deprecation, or external API changes

---

## HAZ-1: Duplicate Code Divergence

**Tags**: Temporal, Invariant, Release/API
**Severity**: HIGH
**Invariant violated**: INV-7 (single source of truth)

**Description**: Two copies of `canonical_digest()` and `digest_step_primitive()` exist in the codebase. If one is modified and the other is not, the digest computation diverges. The legacy path (`compile/mod.rs:220-261`) and canonical path (`mod_compile_lowering/part_05.rs:116-162`) are currently near-identical but differ in:
- Legacy `TriggerAst` match lacks `_` arm (will fail to compile when new trigger variant added)
- Legacy `ScalarValue` inner match lacks `_` arm (will fail to compile when new variant added)

**Consequence**: Proptest helpers using the legacy path could produce different digests than production code, leading to false positives or false negatives in tests.

**Mitigation**: 
- Consolidate into a single function (shared in a common module)
- OR add an equivalence test that verifies both functions produce identical output for all valid inputs
- OR remove the legacy path entirely

**Proof strategy**: Equivalence test or removal. Not a property to prove — a code organization concern.

---

## HAZ-2: Silent Hash Collapse on Unknown ScalarValue

**Tags**: Refinement, Release/API
**Severity**: MEDIUM
**Invariant violated**: INV-4 (hash discrimination by variant)

**Description**: In `digest_step_primitive()` (canonical path, line 155), the inner match on `ScalarValue` has a `_ => hasher.update(b"unsupported")` arm. If a new `ScalarValue` variant is added (e.g., `Bool`, `Float`), two different values of that new variant would produce identical hash inputs (`b"finish"` + `b"unsupported"`).

**Consequence**: Two workflows that differ only in a new `ScalarValue` variant's value would produce identical digests. This breaks digest sensitivity.

**Current safety**: `ScalarValue` has exactly two variants (`String`, `Integer`), both explicitly handled. The `_` arm is unreachable with current variants.

**Future risk**: When a new variant is added, the compiler will NOT warn about the `_` arm in `digest_step_primitive` (it's a catch-all), but WILL require updates to `canonical_finish_slot()` and `canonical_primitive_name()` which also match on `ScalarValue`.

**Mitigation**:
- Add a test that verifies the `_` arm is unreachable (assert that all current `ScalarValue` variants are matched)
- Consider removing the `_` arm and making the match exhaustive, so adding a new variant forces a compile error
- Document that new `ScalarValue` variants MUST update `digest_step_primitive`

---

## HAZ-3: No Digest-to-Source Cross-Validation

**Tags**: Invariant, Temporal
**Severity**: MEDIUM
**Invariant violated**: None directly, but INV-6 (digest survival) is untested

**Description**: `WorkflowParts.digest` can be set to any `WorkflowDigest`. Nothing in the system re-computes the digest from the source and cross-validates at `try_from_parts()` time. If a bug causes the digest to be computed incorrectly, or set to a stale/zero value, the `CompiledWorkflow` would carry an incorrect digest.

**Consequence**: The digest field in `CompiledWorkflow` becomes untrustworthy as an identity token. Recovery/replay could match workflows incorrectly.

**Mitigation**: The digest is computed once and carried through. Since the digest is a pure function of the source, re-computation at validation time would add overhead but provide defense-in-depth. Not currently required but worth documenting as a gap.

---

## HAZ-4: Integer LE Encoding Truncation

**Tags**: Bounded State, Refinement
**Severity**: LOW
**Invariant violated**: None (current behavior is correct)

**Description**: `ScalarValue::Integer(i64)` is hashed as `i.to_le_bytes()` — a fixed 8-byte little-endian encoding. This is correct for all `i64` values. The encoding is:
- Deterministic
- Bijective (distinct `i64` values → distinct `[u8; 8]`)
- Platform-independent (LE is explicit)

**Consequence**: None. The encoding is correct. The hazard is theoretical: if `to_le_bytes()` were changed to a different encoding in a future Rust edition (vanishingly unlikely), digests would change.

**Mitigation**: None required. Document the encoding choice in the contract.

---

## HAZ-5: String vs Integer Discriminator Collision

**Tags**: Refinement
**Severity**: LOW
**Invariant violated**: None (current behavior is correct)

**Description**: `Finish { result: String("42") }` hashes `b"finish"` + `[52, 50]` (2 bytes), while `Finish { result: Integer(42) }` hashes `b"finish"` + `[42, 0, 0, 0, 0, 0, 0, 0]` (8 bytes). Could there be a collision?

**Analysis**:
- `String("")` → `[]` (0 bytes after "finish")
- `Integer(0)` → `[0; 8]` (8 bytes after "finish")
- These produce different hash inputs because the byte sequences differ.
- No known collision between any `String` and `Integer` encoding.
- blake3 is collision-resistant, so even if the prefixes were identical, the probability of an accidental collision is ~2^-128.

**Conclusion**: No hazard. The discriminator is inherent in the different byte sequences.

---

## HAZ-6: Digest Computed Before Validation

**Tags**: Temporal
**Severity**: LOW
**Invariant violated**: None (design intent)

**Description**: The digest is computed from the raw AST at the START of `compile_source()`, before any validation or lowering. This means:
- A workflow with `Finish` at a non-terminal position gets a digest that includes the Finish step at its actual (invalid) position.
- A workflow referencing a non-existent output name gets a digest that includes that name.
- The digest reflects what the author WROTE, not what VALIDATED successfully.

This is by design — the digest is a source fingerprint, not a "valid workflow" fingerprint.

**Consequence**: Two authors who make the same mistake produce the same digest. An author who fixes a mistake produces a different digest. This is semantically correct for a source fingerprint.

**Mitigation**: Document this design decision clearly. If the team later wants to digest only the validated IR, the digest must move to AFTER `try_from_parts()`.

---

## HAZ-7: Trigger `_` Arm Produces "unknown"

**Tags**: Refinement, Release/API
**Severity**: LOW
**Invariant violated**: INV-2 (digest must cover all fields)

**Description**: In `canonical_digest()` (canonical path, line 131), the `TriggerAst` match has `_ => hasher.update(b"unknown")`. If a new trigger variant is added, all workflows using that trigger would produce digests with the same `"unknown"` suffix, regardless of the trigger's parameters.

**Consequence**: Two workflows using different new trigger variants would produce identical digests for the trigger portion. This breaks digest sensitivity for the trigger field.

**Current safety**: All current trigger variants (`Manual`, `Schedule`, `Event`, `Webhook`) are explicitly matched.

**Mitigation**: Same as HAZ-2 — remove the `_` arm to enforce exhaustiveness at compile time.

---

## HAZ-8: Step ID Encoding (Empty String)

**Tags**: Hostile Input
**Severity**: LOW
**Invariant violated**: None

**Description**: If a step has an empty `id` (`""`), `hasher.update("".as_bytes())` produces an empty update. This is technically valid and does not break hashing, but it means a step with an empty ID contributes zero entropy to the digest for that step.

**Consequence**: Two workflows where one step's ID is `""` and the same metric step's ID is `""` (in different workflows) would both contribute zero entropy for that step. The remaining fields (primitive, other steps) still distinguish them.

**Mitigation**: The YAML schema SHOULD reject empty step IDs at the parser boundary. If it doesn't, this is a parser gap, not a digest gap. The digest correctly hashes whatever the parser provides.

---

## HAZ-9: `canonical_primitive_name` Semantic Errors

**Tags**: Refinement
**Severity**: LOW
**Invariant violated**: None for Finish (Finish has its own match arm)

**Description**: `canonical_primitive_name()` has known semantic errors:
- `Together` → `"parallel"` (should be `"together"`)
- `Aggregate` → `"aggregate"` (should be `"reduce"` per some documentation?)

**Consequence for Finish digest**: **None.** `Finish` has its own explicit match arm in `digest_step_primitive()` that writes `b"finish"`, bypassing `canonical_primitive_name()`. This hazard only affects other primitive types.

**Mitigation**: Fix `canonical_primitive_name()` for correctness, but note this WILL change digests for workflows using `Together` or `Aggregate` primitives (since the hash input bytes would change). This is a breaking change for existing compiled artifacts.

---

## Risk Matrix Summary

| Hazard | Severity | Likelihood | Impact | Urgency |
|---|---|---|---|---|
| HAZ-1: Duplicate code divergence | HIGH | LOW | Digest mismatch in tests vs production | Address in bead |
| HAZ-2: Silent hash collapse | MEDIUM | LOW (no new variants yet) | Loss of digest sensitivity for new variant | Address before adding ScalarValue variant |
| HAZ-3: No cross-validation | MEDIUM | LOW | Stale/incorrect digest in compiled workflow | Document; address later |
| HAZ-4: Integer encoding | LOW | VERY LOW | Pre-existing contract; no action needed | None |
| HAZ-5: String/Integer collision | LOW | VERY LOW | Analyzed; no action needed | None |
| HAZ-6: Digest before validation | LOW | N/A (design) | By design; document | None |
| HAZ-7: Trigger `_` arm | LOW | LOW | Same pattern as HAZ-2 | Address with HAZ-2 |
| HAZ-8: Empty step ID | LOW | LOW | Parser gap | Address at parser |
| HAZ-9: canonical_primitive_name bugs | LOW | N/A (doesn't affect Finish) | Breaks Together/Aggregate digests if fixed | Separate bead |
