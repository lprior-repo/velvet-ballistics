# Domain Contract: Wait Digest Coverage

**Bead:** vb-xi2f.32
**Date:** 2026-05-24
**State:** rust-contract (State 3)
**Schema:** contract/v1

## Contract Clauses

### C1: Wait Field Hashing (REQUIRED)
**Clause:** `digest_step_primitive` SHALL hash the semantic content of `StepPrimitive::Wait` including both `event` and `timeout` fields.

**Rationale:** Currently only the string `"wait"` is hashed. This causes digest collisions for workflows with different wait conditions, violating the integrity property that the digest identifies the compiled artifact.

**Precondition:** `primitive` is a `StepPrimitive::Wait` with validated fields (not both None).
**Postcondition:** The hasher state reflects the values of `event` and `timeout`.
**Invariant:** Different wait configurations produce different hasher states.

**Source refs:** `vb_yaml::ast::StepPrimitive::Wait`, `vb_compile::digest_step_primitive`

### C2: WaitUntil vs WaitEvent Discrimination (REQUIRED)
**Clause:** The digest SHALL distinguish `WaitUntil` (event=None, timeout=Some) from `WaitEvent` (event=Some) using the positional sentinel `b"none"` in the event field position (DD-4 refinement — explicit discriminator strings replaced by sentinel discriminators).

**Rationale:** These produce different `CompiledNodeKind` variants (`WaitUntil` vs `WaitEvent`) with different runtime behavior. The digest must reflect this semantic difference. The positional `b"none"` sentinel serves as the discriminator because valid event slot strings are always integer-like (validated by `slot_from_text`) and can never equal `b"none"`.

**Precondition:** Same as C1.
**Postcondition:** For WaitUntil, the hasher receives `b"none"` in the event position, followed by the timeout value. For WaitEvent, the hasher receives the actual event text in the event position. The positional difference acts as the discriminator.
**Invariant:** WaitUntil and WaitEvent produce different hasher states because `b"none"` is a fixed constant that real slot expression text (e.g., `"0"`, `"5"`, `"slot_0"`) cannot match, and the field order guarantees distinct byte sequences.

**Source refs:** `WaitKind::Until`, `WaitKind::Event`, `CompiledNodeKind::WaitUntil`, `CompiledNodeKind::WaitEvent`, `crates/vb_compile/src/mod_compile_lowering/part_05.rs:158-168`

### C3: Absent Field Sentinels (REQUIRED)
**Clause:** Absent optional fields SHALL be represented by a fixed sentinel value in the digest.

**Rationale:** A WaitEvent with timeout=None must produce a different digest than a WaitEvent with timeout=Some("none") — otherwise an attacker or accident could create a collision.

**Precondition:** A field is `None`.
**Postcondition:** The hasher includes `b"none"` as the sentinel.
**Invariant:** The sentinel `"none"` is a fixed constant that real slot expression text (e.g., `"0"`, `"5"`, `"slot_0"`) cannot match.

**Source refs:** `Wait { timeout: Option<String> }`

### C4: Digest Determinism (PRESERVED)
**Clause:** The `canonical_digest` function SHALL remain deterministic after the fix.

**Rationale:** The fix must not introduce non-determinism (time, randomness, external state) into digest computation.

**Precondition:** Same `WorkflowSource` instance.
**Postcondition:** Same `WorkflowDigest` every time.
**Invariant:** All existing tests for digest determinism continue to pass.

**Source refs:** `canonical_digest`, existing `compiled_digest_is_deterministic` test

### C5: Dual Implementation Consistency (REQUIRED)
**Clause:** The fix SHALL be applied identically to both copies of `digest_step_primitive`:
- `crates/vb_compile/src/mod_compile_lowering/part_05.rs` (cold-path compiler)
- `crates/vb_compile/src/compile/mod.rs` (warm-path compiler)

**Rationale:** The two compiler paths must produce identical digests for the same workflow source. Fixing only one copy would create a divergence.

**Precondition:** Both copies exist with identical (broken) behavior.
**Postcondition:** Both copies produce identical digests for any `WorkflowSource`, including those with Wait steps.
**Invariant:** Future changes to either copy must be mirrored. (Follow-up bead: deduplication.)

**Source refs:** `part_05.rs:140`, `compile/mod.rs:243`

### C6: Backward Compatibility of Stability Tests (REQUIRED)
**Clause:** All existing tests that verify digest stability (same input → same output) SHALL continue to pass after the fix.

**Rationale:** The fix adds sensitivity to field changes; it must not break the existing stability property.

**Precondition:** Two identical `WorkflowSource` instances.
**Postcondition:** Same `WorkflowDigest`.
**Invariant:** The proptest `proptest_equal_primitive_sources_compile_to_equal_digest_and_ir` still passes.

**Source refs:** `v1_primitive_lowering.rs:828`, `error_variant_tests.rs:765`

### C7: No Digest Unification (OUT OF SCOPE)
**Clause:** This contract does NOT require `canonical_digest` to produce the same value as `compute_compiled_digest` for the same workflow.

**Rationale:** These are two different hashing algorithms (semantic AST hashing vs raw byte hashing). Unifying them is a separate concern with broader system impact.

**Source refs:** `compute_compiled_digest` in `mod_compile_core.rs`

### C8: Broader Digest Gap (OUT OF SCOPE)
**Clause:** This contract addresses only the Wait primitive. Other primitives (Ask, Do, Save, Choose, ForEach, Together, Parallel, Collect, Aggregate, Repeat) that also fall through to the name-only catch-all are NOT addressed by this bead.

**Rationale:** The bead scope is "digest covers wait semantics." A broader fix should be a follow-up bead.

**Source refs:** `digest_step_primitive` catch-all arm

## Contract Acceptance Criteria

| Clause | Acceptance Test | Status |
|--------|----------------|--------|
| C1 | Different `event` values → different digests | Requires test + fix |
| C1 | Different `timeout` values → different digests | Requires test + fix |
| C2 | WaitUntil ≠ WaitEvent with same timeout text | Requires test + fix |
| C3 | WaitEvent with timeout=None ≠ WaitEvent with timeout=Some("none") | Requires test + fix |
| C4 | Same source → same digest (existing test must pass) | Regression guard |
| C5 | Both copies produce identical digest for Wait workflows | Requires test + fix |
| C6 | All existing digest stability tests pass | Regression guard |
