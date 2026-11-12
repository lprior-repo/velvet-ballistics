# Codebase Map — vb-xi2f.34: Digest covers finish semantics

**Bead**: vb-xi2f.34  
**Explore Date**: 2026-05-24  
**Scope**: Digest computation sensitivity to finish primitive semantics in `vb_compile` and `vb_core`

---

## 1. Architecture Overview

The compilation pipeline has a canonical active path and a legacy duplicate path:

```
YAML text bytes
  → YamlCompiler::compile()                    [mod_compile_core.rs:30]
    → mod_compile_lowering::compile_source()    [part_01.rs:16]           ← CANONICAL
      → canonical_digest(source)                [part_05.rs:116]
        → digest_step_primitive(&mut hasher, primitive)  [part_05.rs:140]
      → WorkflowParts { digest, ... }
      → CompiledWorkflow::from_parts_unchecked() / try_from_parts()
```

A **legacy parallel path** exists in `compile/mod.rs` (lines 25–110, 220–261, 692–702) with duplicate definitions of `compile_source`, `canonical_digest`, `digest_step_primitive`, and `lower_finish`. This path is used by proptest helpers but **not** by the main compilation flow.

---

## 2. Key Files

### 2.1 Digest Computation (canonical path)

| File | Lines | Item | Role |
|---|---|---|---|
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | 116–138 | `canonical_digest()` | Computes blake3 hash from parsed AST (version, name, trigger, step IDs, primitives). Returns `WorkflowDigest`. |
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | 140–162 | `digest_step_primitive()` | Hashes a single step primitive into the blake3 stream. Special-cases `Set` and `Finish`; others use `canonical_primitive_name()`. |
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | 150–157 | `Finish` arm in `digest_step_primitive` | Hashes `"finish"` then the result value: `String` → UTF-8 bytes, `Integer` → LE bytes, all other → `"unsupported"`. |
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | 98–113 | `canonical_primitive_name()` | Maps primitives to names for error messages. Used as fallback in digest for non-Set/non-Finish. |
| `crates/vb_compile/src/mod_compile_core.rs` | 114–116 | `compute_compiled_digest()` | **Different function** — raw blake3 hash of source `&[u8]` bytes. NOT the structural digest stored in the workflow. |

### 2.2 Digest Computation (legacy path)

| File | Lines | Item | Notes |
|---|---|---|---|
| `crates/vb_compile/src/compile/mod.rs` | 220–241 | `canonical_digest()` (legacy) | Duplicate of `part_05.rs:116`. Same logic. |
| `crates/vb_compile/src/compile/mod.rs` | 243–261 | `digest_step_primitive()` (legacy) | Duplicate of `part_05.rs:140`. No `_` arm in match (non-exhaustive). |
| `crates/vb_compile/src/compile/mod.rs` | 709–711 | `compute_compiled_digest()` (legacy) | Duplicate raw hash function. |

### 2.3 Finish Lowering (canonical path)

| File | Lines | Item | Role |
|---|---|---|---|
| `crates/vb_compile/src/mod_compile_lowering/part_02.rs` | 114–133 | `lower_canonical_finish()` | Validates finish is last step, calls `canonical_finish_slot()`, pushes `Finish` node via `lower_finish()`. |
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | 74–96 | `canonical_finish_slot()` | Resolves result — `String` → lookup in output map, `Integer` → direct slot index. |
| `crates/vb_compile/src/mod_compile_lowering/part_07.rs` | 154–165 | `lower_finish()` | Creates `CompiledNode { kind: CompiledNodeKind::Finish { result } }`. |
| `crates/vb_compile/src/mod_compile_lowering/part_01.rs` | 16–58 | `compile_source()` | Top-level compile function. Calls `canonical_digest(source)` at line 46 to set `WorkflowParts.digest`. Also calls `canonical_layout()` which sets `Finish` width to 1 (line 89). |

### 2.4 Finish Lowering (legacy path)

| File | Lines | Item | Notes |
|---|---|---|---|
| `crates/vb_compile/src/compile/mod.rs` | 73–84 | `Finish` lowering (legacy) | Inline in `compile_source` (legacy). |
| `crates/vb_compile/src/compile/mod.rs` | 692–702 | `lower_finish()` (legacy) | Duplicate of `part_07.rs:155`. |

### 2.5 Core Types

| File | Lines | Item | Role |
|---|---|---|---|
| `crates/vb_core/src/ids/mod.rs` | 340–356 | `WorkflowDigest` | `#[repr(transparent)]` newtype over `[u8; 32]`. `from_bytes()`, `as_bytes()`. |
| `crates/vb_core/src/workflow/mod.rs` | 727–731 | `CompiledNodeKind::Finish { result: SlotIdx }` | Terminal node in compiled IR. |
| `crates/vb_core/src/workflow/mod.rs` | 658–659 | `CompiledNodeKind::CollectFinish` | Collection variant with finish semantics. |
| `crates/vb_core/src/workflow/mod.rs` | 675–676 | `CompiledNodeKind::ReduceFinish` | Reduction variant with finish semantics. |
| `crates/vb_core/src/workflow/mod.rs` | 694–695 | `CompiledNodeKind::RepeatFinish` | Repeat variant with finish semantics. |
| `crates/vb_core/src/workflow/mod.rs` | 17–30 | `CompiledWorkflow` | Holds `digest: WorkflowDigest` field. Returns via `digest()` accessor at line 101. |
| `crates/vb_core/src/workflow/mod.rs` | 734–752 | `validate_parts()` | Validates nodes including Finish `result` slot bounds. |
| `crates/vb_yaml/src/ast/types.rs` | 263–270 | `ScalarValue` | `#[non_exhaustive]` enum with `String(String)` and `Integer(i64)` variants. |

### 2.6 Existing Tests (digest-related)

| File | Lines | Test | What it actually tests |
|---|---|---|---|
| `crates/vb_compile/src/tests/error_variant_tests.rs` | 682–686 | `workflow_digest_from_bytes_creates_digest` | Trivial — creates `WorkflowDigest` without panic. |
| `crates/vb_compile/src/tests/error_variant_tests.rs` | 765–777 | `compiled_digest_is_deterministic` | Tests `compute_compiled_digest()` (raw byte hash) — NOT `canonical_digest()`. |
| `crates/vb_compile/src/tests/error_variant_tests.rs` | 781–803 | `different_sources_produce_different_digests` | Tests `compute_compiled_digest()` with different workflow **names**. Changes only name between two YAML blobs. |
| `crates/vb_core/src/ids/mod.rs` | 893–910 | `workflow_digest_equality` / `inequality` | Tests `WorkflowDigest` equality — not the computation. |
| `crates/vb_core/src/ids/mod.rs` | 914–921 | `workflow_digest_single_byte_difference` | Verifies single byte difference produces inequality. |
| `crates/vb_core/src/ids/mod.rs` | 977–984 | `workflow_digest_hash_consistency` | Hash consistency of WorkflowDigest. |

### 2.7 Existing Tests (finish compilation)

| File | Lines | Test | What it tests |
|---|---|---|---|
| `crates/vb_compile/src/tests/error_variant_tests.rs` | 267–282 | `last_step_not_finish_rejected_with_last_step_must_finish` | Finish position constraint. |
| `crates/vb_compile/src/taint/tests/secret_finish_tests.rs` | 1–500+ | Multiple tests | Section 47 finish taint propagation (secret passthrough). NOT digest-sensitive. |
| `crates/vb_compile/src/kani_canonical_name.rs` | 167–168 | Kani harness `canonical_name_all_harness` | Verifies `canonical_primitive_name(Finish)` returns `"finish"`. Does NOT test digest. |
| `crates/vb_core/src/engine/validate/tests/red_phase_behavior_tests.rs` | 89+, 806+ | Multiple tests | Structural validation of Finish nodes in runtime. |

### 2.8 Proptest harnesses

| File | Item | Relevance |
|---|---|---|
| `crates/vb_compile/src/proptest_error_parity.rs` | Lines 68–69 | Generates `StepPrimitive::Finish { result: ... }` in property tests. |
| `crates/vb_compile/src/proptest_collect.rs` | Lines 183–184 | Asserts `CollectFinish` node kind in integration. |

---

## 3. Test Coverage Gaps

### GAP-1: No `canonical_digest()` unit tests
There are **zero** tests that call `canonical_digest()` directly. All existing digest tests call `compute_compiled_digest()` (raw blake3 of source bytes), which is a different function.

### GAP-2: No digest sensitivity to finish result value
No test verifies that changing:
- `finish.result` from `"my_output"` to `"other_output"` changes the digest
- `finish.result` from String to Integer type changes the digest
- `finish.result` from `1` to `2` changes the digest

### GAP-3: No digest sensitivity to finish step position
No test verifies that moving finish to a non-last position (which changes the step ID sequence) changes the digest — though this would be caught by compile validation before digest computation.

### GAP-4: No digest sensitivity to finish step ID
No test verifies that renaming the finish step's ID changes the digest. The `canonical_digest()` function hashes `step.id.as_bytes()` at line 134, so this IS covered by design — but no test exists.

### GAP-5: No test for `_` fallback in `digest_step_primitive`
The `_ => hasher.update(b"unsupported")` arm cannot be hit with current `ScalarValue` (only `String` and `Integer` exist), but there's no test documenting the behavior.

### GAP-6: No integration test for digest changing with finish semantics
No end-to-end test that:
1. Compiles a workflow with a specific finish
2. Changes the finish result value
3. Verifies the compiled workflow's `digest()` field has changed

---

## 4. Crate Dependency Graph (relevant subset)

```
vb_yaml (ScalarValue, StepPrimitive, WorkflowSource, StepAst)
  └─ vb_compile (canonical_digest, digest_step_primitive, lower_finish)
       └─ vb_core (CompiledWorkflow, CompiledNodeKind::Finish, WorkflowDigest)
            └─ vb_validate (WorkflowParts validation)
            └─ vb_codegen (Rust source emission)
       └─ vb_validate::shared
```

Touched crates:
- `vb_compile` — primary (digest computation, finish lowering)
- `vb_core` — secondary (WorkflowDigest type, CompiledNodeKind, CompiledWorkflow)
- `vb_yaml` — upstream dependency (ScalarValue, StepPrimitive types)
- `vb_validate` — downstream (uses WorkflowParts.digest for validation)

---

## 5. Risks

### RISK-1: Digest entropy for Finish result
`digest_step_primitive` hashes the result label/bytes. For `String` results (output references), changing from `"my_output"` to `"other"` produces a different hash. For `Integer` results, changing from `3` to `4` produces a different hash. But there's no explicit test proving this.

### RISK-2: Duplicate code divergence risk
`compile/mod.rs` and `mod_compile_lowering/part_05.rs` have identical copies of `canonical_digest()` and `digest_step_primitive()`. If one is updated and the other isn't, the legacy path would produce incorrect digests. The legacy `compile/mod.rs` version of `digest_step_primitive` lacks the `_` match arm (non-exhaustive match).

### RISK-3: ScalarValue extensibility
`ScalarValue` is `#[non_exhaustive]`. If a new variant is added (e.g., `Bool`), it would fall through to the `_` arm in `digest_step_primitive` (canonical path) or fail to compile in the legacy path. Either scenario would produce a digest that doesn't distinguish the new variant's value.

### RISK-4: canonical_primitive_name bugs
Known bugs from `kani_canonical_name.rs`:
- `Together` maps to `"parallel"` instead of `"together"`
- `Aggregate` maps to `"aggregate"` instead of `"reduce"`
These affect `canonical_primitive_name()` but NOT `digest_step_primitive()` which uses the own match arms for `Set` and `Finish`.

---

## 6. Recommended Downstream Work

1. **rust-contract**: Model the digest contract for finish semantics — what fields must be included, what changes must invalidate the digest
2. **test-planner**: Plan unit tests for `canonical_digest()` and `digest_step_primitive()` — specifically finish result changes, finish step ID changes
3. **proof-planner**: Evaluate whether Kani harnesses are needed to prove digest sensitivity to finish result variants
4. **holzman-rust (implementation)**: Add `canonical_digest()` tests; consider consolidating duplicate code in `compile/mod.rs` vs `mod_compile_lowering/`
5. **black-hat-reviewer**: Audit the `_` fallback arm in `digest_step_primitive` for hash collision risk across future `ScalarValue` variants
