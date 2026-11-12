# Hazard Analysis: Wait Digest Coverage

**Bead:** vb-xi2f.32
**Date:** 2026-05-24
**State:** rust-contract (State 3)
**Schema:** hazard-analysis/v1

## 1. Temporal Hazards

### TH-1: Time-of-Check to Time-of-Use (Digest Stability)
- **Hazard:** Workflow source is validated, then `canonical_digest` is computed from the parsed AST, then steps are lowered. If the source object were mutable between digest computation and lowering, the digest could reflect a different state than what was compiled.
- **Mitigation:** `compile_source` borrows `source` immutably. No mutation possible. The AST is frozen after parsing.
- **Risk:** NONE (currently)

### TH-2: Order-Dependent Hash
- **Hazard:** If the order of step hashing changes relative to step lowering, the digest could change even when the content is semantically identical.
- **Mitigation:** `canonical_digest` iterates `source.steps()` in source order. Step iteration order is deterministic.
- **Risk:** NONE (currently)

### TH-3: Post-Fix Digest Breaks Persisted Artifacts
- **Hazard:** After the fix, `canonical_digest` will produce different values for workflows containing wait steps. Previously persisted `CompiledWorkflow` artifacts will have invalid digests.
- **Mitigation:** Recompilation solves this. Versioning the digest algorithm would allow graceful migration.
- **Risk:** MEDIUM (operational). Out of scope for this bead but documented.

## 2. Rust-Core Invariant Hazards

### RCIH-1: Digest Collision (THE ACTIVE BUG)
- **Hazard:** `digest_step_primitive` hashes only `"wait"` for all Wait primitives, ignoring `event` and `timeout` fields. Two workflows with different wait conditions produce the same `WorkflowDigest`.
- **Impact:** Digest-based identity checks pass for semantically different workflows. Idempotency gates incorrectly deduplicate distinct workflows. Integrity checks report false positives.
- **Severity:** HIGH — behavior affecting.
- **Fix:** Add `Wait { event, timeout }` match arm in `digest_step_primitive` in both copies.

### RCIH-2: Post-Fix Hash Collision Between WaitUntil and WaitEvent
- **Hazard:** If the discriminator is not included, a WaitUntil with timeout=`"slot_0"` could hash identically to a WaitEvent with event=`"slot_0"` and no timeout (if the timeout=None sentinel collides with some event value).
- **Mitigation:** Explicit discriminator (`"wait_until"` vs `"wait_event"`) prevents ambiguity. The three-argument hash for WaitUntil (`wait_until` + timeout bytes) cannot collide with the four-argument hash for WaitEvent (`wait_event` + event bytes + timeout-or-none).
- **Risk:** NONE (if discriminator is correctly implemented)

### RCIH-3: Hash Ordering Collision
- **Hazard:** If `event` and `timeout` are hashed without separators or discriminators, `(event="abc", timeout="def")` could hash identically to `(event="abcd", timeout="ef")`.
- **Mitigation:** Each field is a separate `hasher.update()` call. The blake3 state machine processes each update independently, treating them as distinct inputs (equivalent to hashing the concatenation with length framing). No ambiguity.
- **Risk:** NONE (blake3's `update` API provides domain separation automatically for separate calls)

### RCIH-4: Duplicate Code Divergence
- **Hazard:** If only one copy of `digest_step_primitive` is fixed, the cold-path and warm-path compilers produce different digests for the same source.
- **Mitigation:** Apply the exact same fix to BOTH copies.
- **Severity:** HIGH — behavior affecting.
- **Risk:** Requires discipline in implementation. Both copies must be fixed identically.

### RCIH-5: Empty Steps Digest
- **Hazard:** A workflow with zero steps hashes only version + name + trigger. Two empty workflows with different names but same trigger produce different digests (name is hashed). No additional risk.
- **Risk:** NONE

## 3. Bounded State Hazards

### BSH-1: Hash Function Explosion
- **Hazard:** `digest_step_primitive` could be called an unbounded number of times for workflows with many steps.
- **Mitigation:** The number of steps is bounded by validation and memory limits. `blake3` hash updates are O(n) in input length. No risk of hash state explosion.
- **Risk:** NONE

### BSH-2: Wait Field Length
- **Hazard:** `event` and `timeout` fields are `Option<String>` — strings from YAML. They could be arbitrarily long.
- **Mitigation:** The compiler already stores these strings. Hashing them adds no new memory pressure. The AST strings exist regardless.
- **Risk:** NONE (same memory bound as existing code)

### BSH-3: Step Index Overflow
- **Hazard:** `canonical_digest` iterates `source.steps()` and hashes each step ID. With enough steps, the hash computation could take significant time.
- **Mitigation:** Step count is bounded by validation and practical limits.
- **Risk:** NONE

## 4. Refinement Hazards

### RH-1: WaitKind vs Digest Discrimination Mismatch
- **Hazard:** `WaitKind::Until` and `WaitKind::Event` are discriminators used during compilation/IR generation. The digest must produce the SAME discrimination for the same input. If the digest's `event.is_none()` check disagrees with `lower_canonical_wait`'s `(None, Some(t))` match, the digest would classify differently than the compiler.
- **Mitigation:** Both the digest and `lower_canonical_wait` use `event` and `timeout` fields from the same `StepPrimitive::Wait` struct. They see the same data. No inconsistency possible.
- **Risk:** NONE

### RH-2: Validation Guarded by Type Check
- **Hazard:** The `Wait { event: None, timeout: None }` case is rejected by validation. If validation is bypassed, `digest_step_primitive` would encounter this illegal state.
- **Mitigation:** `compile_source` calls `validate_canonical_compile_scope` before `canonical_digest`. If validation is bypassed, the panic would occur in lowering, not in digest. The digest should defensively handle the (None, None) case by hashing a sentinel, but it will never be reached in practice.
- **Risk:** LOW (requires validation bypass)

## 5. Concurrency Hazards

### CH-1: Shared Indices in Compiler
- **Hazard:** `lower_canonical_wait` uses `builder: &mut SlotCompiler` to record slots. If digest computation shared mutable state with the builder, ordering could matter.
- **Mitigation:** `canonical_digest` is called BEFORE lowering (line 46 in part_01.rs, before the step loop). It does not share any mutable state with lowering.
- **Risk:** NONE (sequential call order)

### CH-2: Concurrent Compilation Sessions
- **Hazard:** Two concurrent compilation sessions for the same workflow source produce different digests if compilation is non-deterministic.
- **Mitigation:** `canonical_digest` is a pure function. Two concurrent calls with identical input produce identical output.
- **Risk:** NONE

## 6. Unsafe / Provenance Hazards

### UPH-1: None
- No `unsafe` code exists in the digest computation path.
- `WorkflowDigest([u8; 32])` is `repr(transparent)` but uses a safe constructor `from_bytes(bytes: [u8; 32])`.
- `blake3::Hasher` is a safe Rust library.
- **Risk:** NONE

## 7. Hostile Input Hazards

### HIH-1: Crafted Wait Fields Causing Hash Collision
- **Hazard:** An attacker could craft `event` and `timeout` strings that produce the same hash as a different wait configuration, bypassing digest-based integrity checks.
- **Mitigation:** blake3 is collision-resistant. The probability of a crafted collision is negligible (2^-128 for a 256-bit output space). The attacker would need control over the workflow source, which is already trusted.
- **Risk:** NONE (blake3 cryptographic strength)

### HIH-2: Wait Fields with Non-UTF-8 Content
- **Hazard:** `event: Option<String>` and `timeout: Option<String>` are Rust `String` types — always valid UTF-8.
- **Mitigation:** YAML parsing guarantees valid UTF-8 for String fields.
- **Risk:** NONE

### HIH-3: Extremely Long Wait Field Strings
- **Hazard:** Maliciously long `event` or `timeout` strings could consume memory during hashing.
- **Mitigation:** The strings already exist in the AST. Hashing reads them borrow-checked; no additional allocation.
- **Risk:** NONE

## 8. Performance Hazards

### PH-1: Hash Computation Overhead
- **Hazard:** Adding field hashing for Wait increases the digest computation time.
- **Mitigation:** The additional fields are small strings (slot reference text like `"0"`, `"5"`, `"30"`). The overhead is negligible — blake3 processes 12+ GB/s on modern hardware.
- **Risk:** NONE

### PH-2: Branch in Hot Path
- **Hazard:** The match on `event.is_some()` adds a branch in `digest_step_primitive`.
- **Mitigation:** `digest_step_primitive` is called once per step at compile time. It is not a hot runtime path.
- **Risk:** NONE

## 9. Release / API Hazards

### RAH-1: Incompatible Digest Change
- **Hazard:** After the fix, workflows that compiled before will produce different digests. Any system that stores or compares `WorkflowDigest` values will see a change.
- **Mitigation:** This is a bug fix, not a breaking change. The old behavior was incorrect. Systems that persist digests must recompile affected workflows.
- **Risk:** MEDIUM (operational impact). Documented in the bead.
- **Affected:** `CompiledWorkflow` artifacts in Fjall storage. `vb_storage` artifact store.

### RAH-2: Proptest Regression
- **Hazard:** The existing proptest `proptest_equal_primitive_sources_compile_to_equal_digest_and_ir` tests stability (same → same). After the fix, this test should still pass for Wait primitives — two identical Wait asts produce identical digests.
- **Mitigation:** The fix adds sensitivity (different → different) while preserving stability (same → same).
- **Risk:** NONE (backward compatible test behavior)

## 10. Hazard Severity Summary

| Hazard | Severity | Behavior Affecting | Currently Mitigated? |
|--------|----------|--------------------|---------------------|
| RCIH-1: Digest Collision | **HIGH** | YES | **NO — this is the bug** |
| RCIH-4: Duplicate Code Divergence | **HIGH** | YES | **PARTIAL — both copies must be fixed** |
| TH-3: Post-Fix Persistence Break | MEDIUM | YES | YES (recompilation) |
| RAH-1: Incompatible Digest Change | MEDIUM | YES | YES (documentation) |
| RCIH-2: WaitUntil vs WaitEvent Collision | LOW | YES | YES (discriminator design) |
| RH-1: Discrimination Mismatch | LOW | NO | YES (same data source) |
| RH-2: Validation Bypass | LOW | NO | YES (validation precedes digest) |
| All others | NONE | — | YES |
