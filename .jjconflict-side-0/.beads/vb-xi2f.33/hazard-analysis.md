# Hazard Analysis — vb-xi2f.33: Digest Covers Ask Semantics

## HAZ-001: Semantic Digest Does Not Cover Ask Fields (PRESENT BUG)

- **Category**: Semantic integrity / invariant violation
- **Severity**: HIGH
- **Status**: CONFIRMED — present in both `part_05.rs` and `compile/mod.rs`
- **Description**: `digest_step_primitive` uses a catch-all arm that hashes only `canonical_primitive_name(Ask)` = `"ask"`. Neither `prompt` nor `timeout` is hashed. The canonical digest is not a function of ask semantics.
- **Affected invariants**: INV-ASK-001 (semantic sensitivity)
- **Exploitation scenario**: An attacker modifies the ask prompt in a compiled workflow YAML source. The canonical digest remains unchanged. Admission and idempotency checks pass despite the semantic change. The user sees a different prompt than what was authorized.
- **Detection difficulty**: EASY (by test/proof). KNOWN bug. Requires static analysis or careful inspection of `digest_step_primitive`.
- **Fix**: Add explicit `Ask { prompt, timeout }` arm to `digest_step_primitive`.

## HAZ-002: Duplicate Implementation Drift

- **Category**: Maintenance / code duplication
- **Severity**: MEDIUM
- **Status**: CONFIRMED — `canonical_digest` + `digest_step_primitive` duplicated
- **Description**: Two implementations of the same digest logic exist. If only one is fixed, the other produces divergent digests. Future maintainers may fix one and not discover the other.
- **Affected invariants**: INV-ASK-005 (duplicate parity)
- **Fix**: Either unify to a single shared implementation, or apply the fix identically to both and add parity tests.

## HAZ-003: Empty Prompt Hash Ambiguity

- **Category**: Edge case / hash collision
- **Severity**: LOW (with current bug; MEDIUM once fixed)
- **Status**: POTENTIAL — depends on fix implementation
- **Description**: If the fix does not handle `prompt = ""` correctly, `hasher.update(b"")` could produce an ambiguous or degenerate hash contribution. Need to verify that feeding zero bytes to blake3 produces a well-defined and distinct result.
- **Affected invariants**: INV-ASK-003 (empty prompt edge case)
- **Fix**: Ensure the fix explicitly hashes `b""` for empty prompt. blake3 handles zero-length input correctly and produces distinct output for distinct total input.

## HAZ-004: None vs Some("") Timeout Collision

- **Category**: Sentinal ambiguity / hash collision
- **Severity**: MEDIUM (once HAZ-001 is fixed)
- **Status**: POTENTIAL — depends on fix implementation
- **Description**: `timeout: None` (no timeout) and `timeout: Some("")` (empty expression) must produce different digest contributions. If both hash to the same bytes (e.g., both produce zero-length input), digests collide.
- **Affected invariants**: INV-ASK-004 (None vs Some("") distinction)
- **Fix**: Use a sentinel: `b"no_timeout"` for `None`, `b"timeout"` + expression bytes for `Some`. This guarantees distinct hash inputs.
- **Alternative concern**: Should `Some("")` (empty timeout string) even be semantically valid? Does `parse_ask` accept an empty string for timeout? If empty string is rejected at parse time, this hazard is partially mitigated.

## HAZ-005: Blake3 Non-Determinism

- **Category**: External dependency / cryptographic assumption
- **Severity**: LOW (blake3 is well-audited)
- **Status**: TRUSTED ASSUMPTION
- **Description**: If the blake3 implementation were non-deterministic, `canonical_digest` would produce different digests for the same input across compilations.
- **Affected invariants**: INV-ASK-002 (determinism)
- **Mitigation**: blake3 is designed to be deterministic and verifiable. This is a foundational assumption of all hash-based identity systems.
- **Detection**: Integration test: compile same source twice, compare digests. Proptest: compile random valid sources, verify digest determinism.

## HAZ-006: Legacy Path Does Not Support Ask

- **Category**: Feature gap + silent failure risk
- **Severity**: MEDIUM
- **Status**: CONFIRMED — `compile/mod.rs::compile_source()` returns `UnsupportedStepPrimitive` for `Ask`
- **Description**: The legacy compilation path (`compile_source`) rejects `Ask` entirely with an error. If someone fixes the digest in the legacy path but not the compilation support, the digest fix is unreachable for `Ask` in that path. Conversely, if `Ask` is added to the legacy path's `compile_source`, the digest function must already be correct.
- **Affected invariants**: INV-ASK-005 (duplicate parity)
- **Fix**: Either add `Ask` support to the legacy path AND fix the digest, OR remove the duplicate `canonical_digest` from the legacy path and call the shared implementation.

## HAZ-007: Step Order Sensitivity

- **Category**: Design correctness
- **Severity**: INFO (working as designed, but worth documenting)
- **Status**: DESIGNED BEHAVIOR — not a bug
- **Description**: `canonical_digest` iterates `source.steps()` in order and hashes each step's ID and primitive. Changing step order changes the digest, which is correct for semantic equivalence. Two workflows with the same steps in different orders are NOT semantically equivalent.
- **Affected invariants**: WF-INV-002
- **Note**: This is intentional. Documented here so it is not mistaken for a bug.

## HAZ-008: Performance — Large Prompts

- **Category**: Performance / bounded resource
- **Severity**: LOW
- **Status**: DESIGN CONSIDERATION
- **Description**: `canonical_digest` hashes the entire `prompt` string. An extremely large prompt (megabytes) would consume corresponding hash time. However, `blake3` is designed for high throughput and large inputs.
- **Mitigation**: If prompt size is bounded by YAML parsing or workflow validation, the digest cost is bounded. No additional mitigation needed.

## HAZ-009: Compiler Panic on Malformed Source

- **Category**: Rust invariant / panic safety
- **Severity**: INFO (not a concern for the digest fix)
- **Status**: PRE-EXISTING, not introduced by this bead
- **Description**: `canonical_digest` receives a `&WorkflowSource` which has already been validated by the YAML parser. If the source were somehow malformed (e.g., a `StepPrimitive` variant with missing fields), a `match` arm could theoretically hit an unexpected state. However, the Rust type system and YAML parser prevent this.
- **Fix scope**: Not in scope for this bead. Covered by existing YAML parsing validation.

## Hazard Mitigation Matrix

| Hazard | Severity | Fix Required | Proof Lane Suggestion |
|--------|----------|-------------|----------------------|
| HAZ-001 | HIGH | Add Ask arm | Kani (bounded), proptest (property) |
| HAZ-002 | MEDIUM | Unify or test parity | Unit tests |
| HAZ-003 | LOW | Handle empty prompt | Kani (edge case) |
| HAZ-004 | MEDIUM | Distinct sentinels | Kani (None vs Some("")) |
| HAZ-005 | LOW | Trusted assumption | Integration test |
| HAZ-006 | MEDIUM | Fix or deprecate legacy | Unit tests |
| HAZ-007 | INFO | Document only | None |
| HAZ-008 | LOW | None (bounded by YAML) | None |
| HAZ-009 | INFO | Not in scope | Pre-existing coverage |
