# Proof Strategy: Wait Digest Coverage

**Bead:** vb-xi2f.32
**Date:** 2026-05-24
**State:** proof-planner (State 4)
**Schema:** proof-strategy/v1

## 1. Strategy Overview

This bead fixes a digest collision bug: `digest_step_primitive()` hashes only the string `"wait"` for `StepPrimitive::Wait`, ignoring the `event` and `timeout` fields. The fix adds an explicit `Wait { event, timeout }` match arm that hashes a discriminator (`"wait_until"` vs `"wait_event"`) plus field values, including a `"none"` sentinel for absent optional fields.

**Proof posture:** This is a P1 (Priority 1, high consequence) scope. The digest is a pure function (no I/O, no time, no concurrency, no unsafe). The proof strategy is layered:

1. **Kani** — bounded panic-freedom of the new match arm (ps-wait-008), bounded equivalence of both copies of `digest_step_primitive` (ps-wait-005), and bounded collision-freedom for small slot domains (ps-wait-006).
2. **proptest** — broad input-space testing for digest sensitivity (ps-wait-001 through ps-wait-006), determinism regression (ps-wait-004, ps-wait-007), and cross-path consistency (ps-wait-005, ps-wait-009).
3. **cargo-fuzz** — adversarial collision hunting for crafted wait field values (ps-wait-001, ps-wait-003, ps-wait-006).
4. **Regression guard** — existing test suite (ps-wait-007).

**Explicitly not applied:** TLA+ (no temporal behavior), Verus (overkill for pure-function fix in P1 scope), Flux (no refinement-type predicates), Loom (no concurrency), Miri (no unsafe).

## 2. Risk Classification

| Risk Tag | Class | Primary Lane | Why |
|----------|-------|-------------|-----|
| digest_collision | semantic-integrity | proptest + cargo-fuzz + Kani | Must prove that different Wait fields → different digests |
| semantic_integrity | behavior_affecting | proptest | Broad property: digest sensitivity to field changes |
| deterministic_regression | regression | proptest | Existing test must still pass |
| panic_freedom | bounded | Kani | Prove new match arm doesn't panic |
| duplicate_code_divergence | duplicate_code | proptest + Kani | Prove both copies produce identical output |
| sentinel_ambiguity | behavior_affecting | proptest + cargo-fuzz | Prove sentinel `"none"` is unambiguous |

## 3. Layered Defense

### Layer 1: Kani (Bounded Proof)
- **ps-wait-008:** Panic-freedom of the new `Wait` match arm in both copies.
- **ps-wait-005:** Bounded equivalence of both `digest_step_primitive` copies for all Wait configurations with slot text up to 16 chars.
- **ps-wait-006:** Bounded collision-freedom: all 3 legal Wait shapes produce distinct digests for small alphabet (a-z) field values up to length 4.

### Layer 2: proptest (Broad Input Coverage)
- **ps-wait-001:** `forall a,b with different wait fields ⇒ digest(workflow_with(a)) != digest(workflow_with(b))`
- **ps-wait-002:** `forall timeout_text: digest(WaitUntil{t}) != digest(WaitEvent{e,t})` for any event string e.
- **ps-wait-003:** `forall e: digest(WaitEvent{e, None}) != digest(WaitEvent{e, Some("none")})`
- **ps-wait-004:** Existing test `proptest_equal_primitive_sources_compile_to_equal_digest_and_ir` continues to pass.
- **ps-wait-005:** Cross-path: same source → same digest via both `compile_source()` (cold) and `compile_workflow()` (warm).
- **ps-wait-006:** Pairwise distinct digests for distinct Wait configurations in otherwise-identical workflows.
- **ps-wait-007:** All existing stability tests pass.
- **ps-wait-009:** Structural equivalence of digest output between both copies.

### Layer 3: cargo-fuzz (Adversarial Input)
- **ps-wait-001:** Fuzz target: mutate event/timeout strings, assert digest differs.
- **ps-wait-003:** Fuzz target: mutate timeout strings near sentinel `"none"`, assert no collision.
- **ps-wait-006:** Fuzz target: generate two Wait workflows with differing fields, assert digests differ.

### Layer 4: Existing Regression Tests
- `compiled_digest_is_deterministic` (error_variant_tests.rs:765)
- `proptest_equal_primitive_sources_compile_to_equal_digest_and_ir` (v1_primitive_lowering.rs:828)
- All existing engine integration tests for WaitUntil/WaitEvent behavior.

## 4. Lane Selection Rationale

| Lane | Decision | Rationale |
|------|----------|-----------|
| TLA+ | not_applicable | Digest is a pure function with no temporal behavior, retries, leases, queues, or interleavings. The fix is a synchronous hash update. |
| Verus | not_applicable | P1 scope. The digest function is a simple match-arm + hasher.update() sequence. Verus would require rewriting both copies in Verus-subset Rust, which is disproportionate. Kani + proptest provide stronger coverage for the real risks (collision, panic). |
| Kani | required | The new match arm introduces a branch and string operations. Kani proves panic-freedom for all legal inputs and bounded collision-freedom. |
| Flux | not_applicable | No refinement-type predicates to prove. The `event`/`timeout` Option<String> validation is handled by `validate_wait_shape`, not by digest logic. |
| Loom | not_applicable | No threads, atomics, channels, or concurrent interleavings in digest computation. |
| Miri | not_applicable | Zero unsafe code in digest path. `#![forbid(unsafe_code)]` enforced. |
| proptest | required | Primary lane. The digest sensitivity property is naturally expressible as a property test: different inputs → different outputs. |
| cargo-fuzz | required | Defense-in-depth. While blake3 is collision-resistant, fuzzing proves the fix doesn't introduce structural collisions (e.g., discriminator vs field-value ambiguity). |

## 5. Assumptions and Bounds

1. **blake3 collision resistance** is assumed. We do not prove blake3's cryptographic properties. We prove that the fix correctly feeds distinct inputs into blake3.
2. **Validation gate** is trusted. `validate_wait_shape` rejects `(None, None)` before digest is computed. We do not prove that validation cannot be bypassed.
3. **String content** is valid UTF-8 (guaranteed by YAML parser).
4. **Bounded slot text** for Kani: slot expression text is bounded to 16 chars for Kani proofs. Real slot text is typically `"0"` to `"255"` (3 chars) — this bound is honest.
5. **Both copies fixed identically** — the proof strategy assumes this discipline. If one copy diverges, the cross-path proptest will catch it.
6. **Hash ordering is stable** — blake3's `update()` treats separate calls as domain-separated inputs.

## 6. Non-Goals (Out of Scope)

- Proving that `canonical_digest == compute_compiled_digest` (explicitly out of scope per C7).
- Fixing other primitives (Ask, Do, Save, etc.) — follow-up bead per C8.
- Deduplicating the two copies of `canonical_digest` — follow-up bead per DD-3.
- Proving blake3's cryptographic properties.
- Proving that validation cannot be bypassed.

## 7. Failure Boundaries

If Kani discovers a panic in the new match arm → fix the implementation, not the harness (GOD RULE 4).
If proptest discovers a collision → fix the implementation (likely discriminator or ordering bug).
If cargo-fuzz discovers a collision → fix the implementation, not the fuzz target.
If cross-path proptest discovers divergence → apply fix to both copies identically.
