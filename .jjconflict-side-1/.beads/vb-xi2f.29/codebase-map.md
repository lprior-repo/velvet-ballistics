# Codebase Map: vb-xi2f.29 — Digest Covers Together Semantics

**Exploration Date:** 2026-05-24
**Bead:** vb-xi2f.29
**State:** 2 (explore)
**Source:** /home/lewis/src/velvet-ballistics

---

## 1. Scope Summary

The bead requires ensuring the compiler's digest (compiled artifact hash/checksum) is sensitive to `together` primitive semantics. When together properties change (branch labels, branch count, sub-step contents), the digest must change.

## 2. Architecture Overview

### 2.1 Compilation Pipeline (Active)

```
YamlCompiler::compile()                     [mod_compile_core.rs:30]
  → vb_yaml::parse_workflow_source()        [vb_yaml crate]
  → mod_compile_lowering::compile_source()  [part_01.rs:16]
    → canonical_digest(source)              [part_05.rs:116]  ← DIGEST COMPUTED HERE
    → lower_canonical_step() for each step   [part_02.rs:16]
      → Together → lower_canonical_parallel() [part_03.rs:15]
```

### 2.2 Dead Code Warning

`crates/vb_compile/src/compile/mod.rs` exists but is **NOT declared in `lib.rs`** — it is dead code. It contains duplicate implementations of `canonical_digest()`, `digest_step_primitive()`, `lower_together()`, etc. that are never compiled. All active logic lives in `mod_compile_lowering/`.

---

## 3. Key Files

### 3.1 Digest Computation

| File | Symbol | Role |
|------|--------|------|
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs:116` | `canonical_digest(source)` | **Primary digest function.** Hashes version, name, trigger, each step ID, each step primitive. |
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs:140` | `digest_step_primitive(hasher, prim)` | Hashes Set/Finish details; for all other primitives (including Together), calls `canonical_primitive_name()`. |
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs:98` | `canonical_primitive_name(prim)` | Maps `Together` → `"parallel"` (bug: should be `"together"`). |
| `crates/vb_compile/src/mod_compile_core.rs:114` | `compute_compiled_digest(source)` | **Byte-level** blake3 hash of raw source bytes. Different from `canonical_digest()`; used for external serialized-artifact comparison. |
| `crates/vb_compile/src/compile/mod.rs:220` | `canonical_digest(source)` | **DEAD CODE** — duplicate, not linked into binary. |
| `crates/vb_compile/src/compile/mod.rs:243` | `digest_step_primitive()` | **DEAD CODE** — duplicate. |

### 3.2 Together Lowering/Compilation

| File | Symbol | Role |
|------|--------|------|
| `crates/vb_compile/src/mod_compile_lowering/part_01.rs:16` | `compile_source()` | Entry: builds WorkflowParts with `digest: canonical_digest(source)` at line 46. |
| `crates/vb_compile/src/mod_compile_lowering/part_02.rs:16` | `lower_canonical_step()` | Routes `Together` to `lower_canonical_parallel()`. |
| `crates/vb_compile/src/mod_compile_lowering/part_03.rs:15` | `lower_canonical_parallel()` | Creates TogetherStart, TogetherBranch, TogetherJoin nodes. |
| `crates/vb_compile/src/mod_compile_lowering/part_03.rs:92` | `emit_together_branches()` | Emits branch nodes and body steps. |
| `crates/vb_compile/src/mod_compile_lowering/part_03.rs:82` | `together_join_offset()` | Calculates join node offset. |
| `crates/vb_compile/src/mod_compile_lowering/part_01.rs:114` | `together_width()` | Returns total width including branches. |
| `crates/vb_compile/src/mod_compile_lowering/part_06.rs:87` | `lower_together()` | Public helper for direct API lowering (used by external callers like tests). |

### 3.3 Core IR Types (vb_core)

| File | Symbol | Role |
|------|--------|------|
| `crates/vb_core/src/workflow/mod.rs:620` | `CompiledNodeKind::TogetherStart` | Start node with branch targets and join. |
| `crates/vb_core/src/workflow/mod.rs:625` | `CompiledNodeKind::TogetherBranch` | Single branch execution node. |
| `crates/vb_core/src/workflow/mod.rs:633` | `CompiledNodeKind::TogetherJoin` | Join node with branch_count and accumulator. |
| `crates/vb_core/src/workflow/mod.rs:97` | `CompiledWorkflow` | Contains `digest: WorkflowDigest`. |
| `crates/vb_core/src/workflow/mod.rs:277` | `WorkflowParts.digest` | Public digest field. |
| `crates/vb_core/src/ids/mod.rs:330` | `WorkflowDigest` | 32-byte blake3 digest wrapper. |
| `crates/vb_core/src/ids/mod.rs:75` | `BranchIdx` | Branch index within Together (not used in digest). |

### 3.4 YAML AST Types (vb_yaml)

| File | Symbol | Role |
|------|--------|------|
| `crates/vb_yaml/src/ast/types.rs:201` | `StepPrimitive::Together` | AST variant: `Together { branches: Vec<TogetherBranch> }` |
| `crates/vb_yaml/src/ast/types.rs:283` | `TogetherBranch` | `{ label: String, steps: Vec<StepAst> }` |
| `crates/vb_yaml/src/ast/types.rs:12` | `WorkflowSource` | Top-level AST. `steps()` returns `&[StepAst]` — **flat top-level list, does not include nested branch sub-steps**. |
| `crates/vb_yaml/src/ast/parse_steps.rs:188` | `parse_parallel()` | Parses YAML `parallel:` into `StepPrimitive::Together`. |

---

## 4. Digest Gap Analysis

### 4.1 What IS Hashed for a Together Step

The `canonical_digest()` in `part_05.rs:116` loops over `source.steps()` (only top-level steps):

```
for step in source.steps() {
    hasher.update(step.id.as_bytes());           // e.g. "fanout"
    digest_step_primitive(&mut hasher, &step.primitive);  // calls canonical_primitive_name → "parallel"
}
```

For a Together step, the digest includes:
- The step's `id` (e.g., `"fanout"`)
- The string `"parallel"` (the canonical_primitive_name)

### 4.2 What Is NOT Hashed

The Together step's `StepPrimitive::Together { branches }` contains `Vec<TogetherBranch>` where each `TogetherBranch` has:
- `label: String` — **NOT hashed**
- `steps: Vec<StepAst>` — **NOT hashed** (these are nested, not in `source.steps()`)

The following together-specific properties are absent from the digest:
1. **Branch labels** (e.g., `"left"`, `"right"`)
2. **Branch count** (2 branches vs 3 branches → same digest)
3. **Sub-step contents** (IDs, primitives, values inside branches)
4. **Branch ordering** (swap two branches → same digest)
5. **Condition fields** on branches (`TogetherBranch.condition` would be unhashed if set)

### 4.3 Also: Canonical Name Bug

`canonical_primitive_name(Together)` returns `"parallel"` instead of `"together"`. A Kani harness (`kani_canonical_name.rs:42`) proves this is buggy. The digest would change from `"parallel"` to `"together"` if fixed — a material semantic change. The fix should be coordinated with digest coverage improvements.

---

## 5. Existing Test Coverage

### 5.1 Digest-Specific Tests

| Test | File:Line | What It Tests |
|------|-----------|---------------|
| `compiled_digest_is_deterministic` | `error_variant_tests.rs:765` | Same source → same digest |
| `different_sources_produce_different_digests` | `error_variant_tests.rs:781` | Different workflow names → different digests |
| `proptest_equal_primitive_sources_compile_to_equal_digest_and_ir` | `v1_primitive_lowering.rs:828` | Proptest: same case → same digest (64 cases, includes parallel case) |
| `workflow_digest_from_bytes_creates_digest` | `error_variant_tests.rs:682` | Type construction smoke test |
| Various `WorkflowDigest` unit tests | `ids/mod.rs:603-977` | Roundtrip, equality, inequality |

### 5.2 Together-Specific Tests

| Test | File:Line | What It Tests |
|------|-----------|---------------|
| `compile_workflow_emits_supported_ir_when_each_scoped_primitive_is_valid` | `v1_primitive_lowering.rs:114` | Parallel primitive emits correct node kinds |
| `public_lowering_helpers_return_exact_range_and_workflow_errors` | `v1_primitive_lowering.rs:652` | Too-many-branches error shape |
| `proptest_scoped_primitives_never_return_unsupported_step_primitive` | `v1_primitive_lowering.rs:837` | Proptest: all scoped primitives (including parallel) compile |
| `validate_transition_target_rejects_together_start_branch_out_of_bounds` | `section36_mandatory_coverage.rs:1566` | Bound validation for TogetherStart |
| `validate_transition_target_rejects_together_branch_join_out_of_bounds` | `section36_mandatory_coverage.rs:1634` | Bound validation for TogetherBranch |
| `prop_together_start_counts_fanout` | `vb_qi37_2_4_state8_tests.rs:614` | Proptest: TogetherStart fanout budget counting |
| `prop_together_fanout_exceeded_includes_diagnostics` | `vb_qi37_2_4_state8_tests.rs:952` | Proptest: fanout exceeded diagnostic |

### 5.3 Kani Harnesses (Digest-Relevant)

| Harness | File:Line | Status |
|---------|-----------|--------|
| `canonical_name_together_harness` | `kani_canonical_name.rs:42` | **PROVES BUG**: expects `"together"`, gets `"parallel"` |
| `canonical_name_all_harness` | `kani_canonical_name.rs:121` | **PROVES BUG**: same assertion for all variants |

### 5.4 Gap: No Together-Digest Change Tests

**No existing test verifies that changes to together semantics produce different digests.** Specifically missing:
- Changing a branch label → digest changes
- Adding/removing branches → digest changes
- Changing sub-step contents inside a branch → digest changes
- Reordering branches → digest changes

---

## 6. Dependencies

```
vb_compile
  ├── vb_yaml (AST parsing, StepPrimitive, TogetherBranch)
  ├── vb_core (CompiledNodeKind, CompiledWorkflow, WorkflowDigest, WorkflowParts)
  ├── vb_validate (IR validation)
  ├── blake3 (digest hashing)
  └── postcard (serialization)
```

---

## 7. Risks

| Risk | Severity | Description |
|------|----------|-------------|
| **Digest insensitivity** | HIGH | Changing together branch labels/steps/configurations does not change the compiled workflow digest. |
| **Canonical name bug** | MEDIUM | `Together` maps to `"parallel"` instead of `"together"`. Fixing this changes digests for all together workflows. |
| **Dead code confusion** | LOW | `compile/mod.rs` has duplicate digest logic but is not compiled. Must not confuse implementers. |
| **Two digest functions** | LOW | `compute_compiled_digest` (byte-level) vs `canonical_digest` (structure-level) serve different purposes. |
| **Nested step handling** | HIGH | `source.steps()` returns only top-level steps. Sub-steps inside foreach/parallel/collect/etc. are NOT iterated during digest computation. |
| **Grouped primitives at risk** | HIGH | The same gap likely affects `for_each`, `collect`, `aggregate`, `repeat` — all have nested sub-steps that are invisible to the digest. |

## 8. Recommended Approach

1. **Fix `canonical_primitive_name`** to return `"together"` for Together (not `"parallel"`).
2. **Enhance `canonical_digest`** to recursively hash nested step trees for Together branches (and other scoped primitives). Include:
   - Branch labels
   - Branch count
   - Sub-step IDs and primitives within each branch
3. **Add proptest coverage** for together-specific digest sensitivity:
   - Same workflow with different branch labels → different digest
   - Same workflow with different branch count → different digest
   - Same workflow with different sub-step contents → different digest
4. **Delete dead code** in `crates/vb_compile/src/compile/mod.rs` to eliminate confusion.
5. **Update Kani harnesses** to reflect the fix and verify digest coverage.

---

## 9. Open Questions

1. **UNKNOWN**: Should `canonical_digest` use a recursive traversal or a flat serialization of the entire AST?
2. **UNKNOWN**: Should the digest include TogetherBranch `condition` fields (currently optional in the type but not parsed in all paths)?
3. **UNKNOWN**: Should the same nested-step fix be applied to `for_each`, `collect`, `aggregate`, `repeat` simultaneously?
4. **UNKNOWN**: Is there a canonical flattening step that already produces a complete step list before digest computation?
