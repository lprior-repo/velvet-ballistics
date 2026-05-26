# Codebase Map — vb-xi2f.28: Digest coverage of for_each semantics

**Bead:** vb-xi2f.28  
**Scope:** Ensure the compiler's digest is sensitive to for_each primitive semantics  
**Date:** 2026-05-25  
**Status:** EXPLORE COMPLETE — gap confirmed

---

## 1. Core Finding: Digest does NOT cover for_each semantics

The `canonical_digest` function (used to compute the source-level workflow digest) hashes only the primitive name string `"for_each"` for `ForEach` steps. It does NOT hash any for_each field values (input, at_once/limit, body steps, item slot, done target). This means:

- **Different for_each configurations produce identical digests.** Changing `for_each.input`, `for_each.at_once` (limit), the body step content, or any other for_each field does NOT change the digest.
- **This violates the acceptance criteria** which state: "Changing for_each input, item slot, fanout/limit, body entry, or done target changes compiled digest."
- The same gap exists for ALL non-Set/non-Finish primitives (collect, reduce, repeat, parallel, wait, ask, etc.).

### The two digest concepts

| Digest type | Function | Location | Sensitive to for_each? |
|---|---|---|---|
| **canonical_digest** (source-level) | `canonical_digest()` | Two copies: `compile/mod.rs:220` and `mod_compile_lowering/part_05.rs:116` | **NO** — only hashes `"for_each"` string |
| **compute_compiled_digest** (artifact-level) | `compute_compiled_digest()` | `mod_compile_core.rs:114` | **YES** — hashes postcard-serialized `WorkflowParts` (full IR) |

The `canonical_digest` is what gets stored in `WorkflowParts.digest` and returned by `CompiledWorkflow::digest()`. The `compute_compiled_digest` hashes the full byte-level serialized artifact, which does include for_each semantics — but this is a different layer of computation.

---

## 2. Relevant Source Files

### 2.1 Digest Computation (THE GAP IS HERE)

| File | Role | Key Symbols |
|---|---|---|
| `crates/vb_compile/src/compile/mod.rs:220-241` | `canonical_digest()` — older compilation path, hashes source AST | `canonical_digest()`, `digest_step_primitive()` |
| `crates/vb_compile/src/compile/mod.rs:243-261` | `digest_step_primitive()` — catch-all `other` arm only hashes primitive name | `digest_step_primitive()` |
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs:116-138` | `canonical_digest()` — primary lowering path, IDENTICAL GAP | `canonical_digest()` |
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs:140-162` | `digest_step_primitive()` — same catch-all approach | `digest_step_primitive()` |
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs:98-114` | `canonical_primitive_name()` — maps primitives to static name strings | `canonical_primitive_name()` |
| `crates/vb_compile/src/compile/mod.rs:203-218` | `canonical_primitive_name()` — older duplicate | `canonical_primitive_name()` |
| `crates/vb_compile/src/mod_compile_core.rs:114-116` | `compute_compiled_digest()` — blake3 over raw bytes (OK, different level) | `compute_compiled_digest()` |

### 2.2 for_each Lowering/Compilation

| File | Role | Key Symbols |
|---|---|---|
| `crates/vb_compile/src/mod_compile_lowering/part_01.rs:16-58` | Main `compile_source()` using canonical layout + lower_canonical_step | `compile_source()`, `canonical_layout()`, `canonical_step_width()` |
| `crates/vb_compile/src/mod_compile_lowering/part_01.rs:92` | `canonical_step_width()` — for_each width = `body_width(body, 2)` | `canonical_step_width()` |
| `crates/vb_compile/src/mod_compile_lowering/part_02.rs:16-79` | `lower_canonical_step()` — dispatches to `lower_canonical_for_each()` | `lower_canonical_step()` |
| `crates/vb_compile/src/mod_compile_lowering/part_02.rs:135-187` | **`lower_canonical_for_each()`** — builds ForEachStart + SetConst(body) + ForEachNext nodes | `lower_canonical_for_each()`, `emit_single_body_set()` |
| `crates/vb_compile/src/mod_compile_lowering/part_04.rs:195` | `emit_single_body_set()` — emits body SetConst for for_each | `emit_single_body_set()` |
| `crates/vb_compile/src/compile/mod.rs:374-414` | `lower_for_each()` — programmatic API (used by tests) | `lower_for_each()` |
| `crates/vb_compile/src/compile/mod.rs:97` | Older `compile_source()` using deprecated non-layout approach | `compile_source()` |

### 2.3 Core Types

| File | Role | Key Symbols |
|---|---|---|
| `crates/vb_core/src/ids/mod.rs:342-356` | `WorkflowDigest` — 32-byte BLAKE3 digest wrapper | `WorkflowDigest`, `from_bytes()`, `as_bytes()` |
| `crates/vb_core/src/compiled_workflow.rs:10-54` | `CompiledWorkflow` — immutable compiled IR with digest | `CompiledWorkflow`, `digest()`, `try_from_parts()` |
| `crates/vb_core/src/compiled_workflow.rs:211` | `WorkflowParts` — owns the `digest: WorkflowDigest` field | `WorkflowParts` |
| `crates/vb_core/src/nodes.rs` | `CompiledNodeKind::ForEachStart`, `ForEachNext`, `ForEachJoin` | `ForEachStart { input, item_slot, limit, body, done }`, `ForEachNext { iterator_slot, body, done }` |
| `crates/vb_yaml/src/ast.rs` | `StepPrimitive::ForEach { input, at_once, body, .. }` — YAML AST type | `StepPrimitive::ForEach` |

### 2.4 Validation (where digested IR is validated)

| File | Role | Key Symbols |
|---|---|---|
| `crates/vb_validate/` | Shared validation crate | `ValidationError`, `validate()` |
| `crates/vb_compile/src/mod_compile_validation/` | Compile-time validation (13 parts) | `reject_unsupported_for_each_fields()` |

---

## 3. Existing Test Coverage

### 3.1 Digest Tests (NONE cover for_each sensitivity)

| File | Test | What it covers | Gap |
|---|---|---|---|
| `crates/vb_compile/src/tests/error_variant_tests.rs:765` | `compiled_digest_is_deterministic` | Same source → same digest | Does NOT test for_each changes |
| `crates/vb_compile/src/tests/error_variant_tests.rs:781` | `different_sources_produce_different_digests` | Different workflow names → different digests | Does NOT test for_each changes |
| `crates/vb_compile/tests/v1_primitive_lowering.rs:828` | `proptest_equal_primitive_sources_compile_to_equal_digest_and_ir` | Same YAML → same digest | Only tests determinism, NOT sensitivity |
| `crates/vb_core/src/ids/mod.rs:895-991` | WorkflowDigest unit tests | Equality, inequality, hash | Type-level tests only |

### 3.2 for_each Compilation Tests (ZERO digest assertions)

| File | Tests | What they cover | Digest tested? |
|---|---|---|---|
| `crates/vb_compile/tests/vb_a001_for_each_topology.rs` | 11 tests (TEST-001 through TEST-011) | Node kind sequence, edge connectivity, slot count, target validity, validation rejection | **NO** — zero digest assertions |
| `crates/vb_compile/tests/v1_primitive_lowering.rs:68-72` | for_each in `PRIMITIVE_CASES` | Node kinds match `FOREACH_KINDS`, slot count = 2 | **NO** — only structural checks |
| `crates/vb_compile/src/kani_foreach_parity.rs` | KANI-001 through KANI-005 | Body SetConst.next edge, backward edges, reachability, malformed IR rejection | **NO** — structural proofs only |

### 3.3 Recovery/Storage Digest Tests (different layer)

| File | Tests | Layer |
|---|---|---|
| `crates/vb_storage/src/tests.rs:149` | `verify_digest_match` | Storage artifact verification |
| `crates/vb_storage/src/proptests.rs:798` | Stale digest detection | Storage admission |
| `crates/vb_storage/src/kani_digest_checks_vb_2bzz.rs` | Kani digests | Storage boundary |
| `crates/workspace_tests/idempotency_suite/tests/vb_ko29_5_public_idempotency.rs` | Digest mismatch errors | Admission boundary |

---

## 4. Dependency Map

```
vb_yaml (AST types: StepPrimitive::ForEach)
    ↓
vb_compile (YAML → IR lowering)
    ├── mod_compile_lowering/part_01 (compile_source, canonical_layout)
    ├── mod_compile_lowering/part_02 (lower_canonical_for_each)
    ├── mod_compile_lowering/part_04 (emit_single_body_set)
    ├── mod_compile_lowering/part_05 (canonical_digest ← THE GAP)
    ├── compile/mod.rs (older compile_source, canonical_digest, lower_for_each)
    └── mod_compile_core.rs (compute_compiled_digest)
    ↓
vb_core (WorkflowParts.digest, CompiledWorkflow.digest(), WorkflowDigest)
    ↓
vb_validate (validates IR with digest)
    ↓
vb_runtime / vb_storage (uses digest for artifact admission, recovery)
```

---

## 5. Risk Assessment

| Risk | Severity | Description |
|---|---|---|
| **Digest coverage gap** | **HIGH** | `canonical_digest` does not include for_each fields. Two semantically different for_each configurations produce identical digests. |
| **Duplicate code** | MEDIUM | `canonical_digest` and `digest_step_primitive` both exist in TWO places (`compile/mod.rs` and `mod_compile_lowering/part_05.rs`). Fix must be applied to both. |
| **Catch-all gap** | HIGH | The `other => { hasher.update(name) }` pattern affects ALL non-Set/non-Finish primitives (collect, reduce, repeat, parallel, wait, ask) — not just for_each. |
| **Two digest levels** | LOW | `compute_compiled_digest` hashes full serialized IR (correct), but `canonical_digest` (source-level) is what gets stored in `CompiledWorkflow.digest()`. The bead acceptance criteria appear to target the source-level digest. |
| **Test coverage void** | HIGH | No existing test verifies that changing any for_each property changes the digest. Tests only cover determinism (same source = same digest) and name-level differences. |

---

## 6. Recommended Fix Approach

The fix should expand `digest_step_primitive()` (in both locations) to hash for_each-specific fields:

**Required fields to hash for ForEach digest sensitivity:**
- `"for_each"` primitive name (already done)
- `input` (slot reference as string/bytes)
- `at_once` / limit (u32, as le_bytes)
- `variable` / item_slot (already implicit via slot allocation)
- Body step content (recursively hash each body step)
- Done target (implicit from step placement, but should be explicit)

**Files needing changes:**
1. `crates/vb_compile/src/mod_compile_lowering/part_05.rs:140-162` — `digest_step_primitive()` — ADD ForEach arm
2. `crates/vb_compile/src/compile/mod.rs:243-261` — `digest_step_primitive()` — ADD ForEach arm

**New tests needed:**
1. Test that changing `for_each.input` changes `canonical_digest`
2. Test that changing `for_each.at_once` changes `canonical_digest`
3. Test that changing body step content changes `canonical_digest`
4. Test that semantically identical for_each IR produces stable digest
5. Proptest: arbitrary for_each variations → digest uniqueness

---

## 7. Open Questions

| # | Question | Status |
|---|---|---|
| 1 | Should the fix also cover other primitives (collect, reduce, repeat, etc.) or only for_each? | Bead scope is for_each only, but identical gap exists |
| 2 | Is `canonical_digest` the right digest to fix, or should we switch to `compute_compiled_digest`? | Acceptance criteria says "compiled digest" — likely `canonical_digest` |
| 3 | Should the two `canonical_digest` duplicates be consolidated? | Recommended but out of scope |
| 4 | Does the runtime rely on `CompiledWorkflow::digest()` for recovery admission? | YES — `vb_storage::admission.rs`, `vb_runtime::recovery.rs` |

