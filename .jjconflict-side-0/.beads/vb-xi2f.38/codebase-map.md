# Codebase Map: Collect Digest Semantics (vb-xi2f.38)

## Bead Overview
- **Bead ID**: vb-xi2f.38
- **Title**: P1: digest covers collect semantics
- **Parent**: vb-xi2f (P0: complete compiler source-to-final-IR lowering for all v1 primitives)
- **Scope**: digest coverage for collect primitive; must map collect primitive/digest mapping, where collect appears in workflows, how digest is computed for collect-shaped inputs/outputs

## Executive Summary

The `collect` primitive is **NOT fully covered** in the source digest computation. The `digest_step_primitive` function in `vb_compile/src/mod_compile_lowering/part_05.rs` (and `vb_compile/src/compile/mod.rs`) only hashes the primitive **name** `"collect"` for `StepPrimitive::Collect`, but does NOT include the fields:
- `variable` (String)
- `source` (String)
- `pages` (Option<u32>)
- `items` (Option<u32>)
- `body` (Vec<StepAst>)

This means two workflows with identical step IDs but different collect parameters will produce the **same source digest**, which is a digest coverage defect.

---

## 1. Collect Primitive Definition

### 1.1 YAML AST (`vb_yaml/src/ast/types.rs` lines 207-218)
```rust
Collect {
    variable: String,       // Loop variable name
    source: String,          // Source expression
    pages: Option<u32>,     // Maximum pages (optional)
    items: Option<u32>,     // Items per page (optional)
    body: Vec<StepAst>,     // Body steps
}
```

### 1.2 YAML Parser (`vb_yaml/src/ast/parse_steps.rs` lines 206-220)
```rust
fn parse_collect(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    let variable = require_str_in(node, "variable", "collect.variable")?;
    let source = require_str_in(node, "source", "collect.source")?;
    let pages = opt_u32(node, "pages");
    let items = opt_u32(node, "items");
    let body = parse_body_steps(node)?;
    Ok(StepPrimitive::Collect { variable, source, pages, items, body })
}
```

### 1.3 CompiledNodeKind IR (`vb_core/src/workflow/mod.rs`)
- `CollectStart { source: SlotIdx, limit: u32, page_size: u32, body: StepIdx, done: StepIdx }`
- `CollectPage { collector_slot: SlotIdx, body: StepIdx, done: StepIdx }`
- `CollectFinish { collector_slot: SlotIdx }`

### 1.4 Lowering (`vb_compile/src/mod_compile_lowering/part_03.rs` lines 159-212)
The `lower_canonical_collect` function emits exactly **4 nodes**:
1. **Node 0**: `CollectStart` { source, limit: pages.unwrap_or(1), page_size: items.unwrap_or(1), body: id+1, done: id+3 }
2. **Node 1**: `SetConst` (from body Set step)
3. **Node 2**: `CollectPage` { collector_slot: source, body: id+1, done: id+3 }
4. **Node 3**: `CollectFinish` { collector_slot: source }

---

## 2. Digest Computation Architecture

### 2.1 Two-Stage Digest System

#### Stage 1: Source Digest (`canonical_digest`)
**File**: `vb_compile/src/mod_compile_lowering/part_05.rs` (lines 116-138) and `vb_compile/src/compile/mod.rs` (lines 220-241)

```rust
pub(super) fn canonical_digest(source: &vb_yaml::ast::WorkflowSource) -> WorkflowDigest {
    let mut hasher = blake3::Hasher::new();
    hasher.update(source.version().as_bytes());
    hasher.update(source.name().as_bytes());
    // ... trigger handling ...
    for step in source.steps() {
        hasher.update(step.id.as_bytes());
        digest_step_primitive(&mut hasher, &step.primitive);
    }
    WorkflowDigest::from_bytes(hasher.finalize().into())
}
```

#### Stage 2: Compiled Artifact Digest (`compute_compiled_digest`)
**File**: `vb_compile/src/mod_compile_core.rs` (lines 114-116) and `vb_compile/src/compile/mod.rs` (lines 709-711)

```rust
pub fn compute_compiled_digest(source: &[u8]) -> WorkflowDigest {
    WorkflowDigest::from_bytes(blake3::hash(source).into())
}
```

The compiled artifact is a postcard-serialized `WorkflowParts` containing the source digest.

### 2.2 Digest Step Primitive Coverage (CRITICAL FINDING)

**File**: `vb_compile/src/mod_compile_lowering/part_05.rs` (lines 140-161) and `vb_compile/src/compile/mod.rs` (lines 243-261)

```rust
fn digest_step_primitive(hasher: &mut blake3::Hasher, primitive: &vb_yaml::ast::StepPrimitive) {
    match primitive {
        vb_yaml::ast::StepPrimitive::Set { output, value } => {
            hasher.update(b"set");
            hasher.update(output.as_bytes());
            hasher.update(value.as_bytes());
        }
        vb_yaml::ast::StepPrimitive::Finish { result } => {
            hasher.update(b"finish");
            match result {
                vb_yaml::ast::ScalarValue::String(value) => hasher.update(value.as_bytes()),
                vb_yaml::ast::ScalarValue::Integer(value) => hasher.update(&value.to_le_bytes()),
            };
        }
        other => {
            hasher.update(canonical_primitive_name(other).as_bytes());
        }
    }
}
```

**DEFECT**: For `StepPrimitive::Collect`, only `canonical_primitive_name(other)` returning `"collect"` is hashed. The fields `variable`, `source`, `pages`, `items`, and `body` are **NOT included** in the digest.

---

## 3. Where Collect Appears in Workflows

### 3.1 Validation Gates
**File**: `vb_validate/src/gates.rs` (lines 456-548)
- `CollectStart { body, done }`: validates body/done step indices are in range, validates loop span
- `CollectPage { body, done }`: validates body/done step indices
- `CollectNext { body, done }`: validates body/done step indices
- `CollectFinish`: validates matching done from `CollectStart`

### 3.2 Runtime Primitives
**File**: `vb_runtime/src/primitives/collect.rs`
- `collect_start`: Reads source list, writes first page, jumps to body or done
- `collect_page`: Re-entry point for body processing
- `collect_next`: Advances pagination cursor
- `collect_finish`: Writes final result to output slot

### 3.3 Replay/Recovery
**File**: `vb_core/src/replay/tests.rs` (lines 1100-1950)
- `replay_collect_pages_with_taint`
- `replay_collect_start_page_bound`
- `replay_collect_start_empty_source_jumps_done`
- `replay_collect_start_rejects_zero_page_size`
- `replay_collect_start_rejects_source_over_limit`
- `replay_collect_start_rejects_non_list_source`
- `replay_collect_page_rejects_non_list_collector`
- `replay_collect_start_rejects_missing_source_list`
- `replay_collect_start_reports_page_insert_failure`

---

## 4. Verification Artifacts for Collect

### 4.1 TLA+ Models
- `verification/tla/collect_body_model.tla`: Models 4-node emission sequence for collect lowering (PO-001)

### 4.2 Verus Proofs
- `verification/verus/collect_ir_structure.rs`: PO-012 (lower_canonical_collect IR struct field refinement)
- `verification/verus/collect_lowering.rs`: PO-002 (lower_canonical_collect pre/post conditions)
- `verification/verus/try_from_parts.rs`: PO-021 (CompiledWorkflow::try_from_parts validation)

### 4.3 Kani Harnesses
- `verification/kani/collect_try_from_parts.rs`: PO-022 (try_from_parts panic-free for valid Collect IR)
- `verification/kani/collect_node_bounds_harness.rs`: Node offset bounds for CollectStart
- `verification/kani/collect_budget_harness.rs`: CollectStart budget verification
- `verification/kani/emit_single_body_set_all_calls.rs`: Emits 4 nodes for collect

### 4.4 Fuzz Targets
- `fuzz/src/lib.rs` (lines 2945-3016): `fuzz_collect_page_pagination`

### 4.5 Integration Tests
- `crates/workspace_tests/benches/collect_page.rs`
- `crates/workspace_tests/benches/collect_page_root_migrated.rs`

---

## 5. Key Files and APIs

### 5.1 Digest Computation
| File | Function | Purpose |
|------|----------|---------|
| `vb_compile/src/mod_compile_lowering/part_05.rs` | `canonical_digest` | Computes source digest from YAML AST |
| `vb_compile/src/mod_compile_lowering/part_05.rs` | `digest_step_primitive` | Hashes primitive name for Collect |
| `vb_compile/src/compile/mod.rs` | `canonical_digest` | Canonical source digest (same impl) |
| `vb_compile/src/compile/mod.rs` | `digest_step_primitive` | Digest step primitive (same impl) |
| `vb_compile/src/mod_compile_core.rs` | `compute_compiled_digest` | BLAKE3 of serialized artifact |
| `vb_compile/src/compile/mod.rs` | `compute_compiled_digest` | Artifact digest (same impl) |

### 5.2 Collect Lowering
| File | Function | Purpose |
|------|----------|---------|
| `vb_compile/src/mod_compile_lowering/part_03.rs` | `lower_canonical_collect` | Emits 4-node collect IR |
| `vb_compile/src/mod_compile_lowering/part_02.rs` | step lowering dispatch | Calls lower_canonical_collect |
| `vb_compile/src/mod_compile_lowering/part_06.rs` | `lower_collect` | Full collect lowering |
| `vb_compile/src/mod_compile_lowering/part_10.rs` | `lower_collect` | Another lowering path |

### 5.3 Collect Runtime
| File | Function | Purpose |
|------|----------|---------|
| `vb_runtime/src/primitives/collect.rs` | `collect_start` | Execute CollectStart |
| `vb_runtime/src/primitives/collect.rs` | `collect_page` | Execute CollectPage re-entry |
| `vb_runtime/src/primitives/collect.rs` | `collect_next` | Advance pagination cursor |
| `vb_runtime/src/primitives/collect.rs` | `collect_finish` | Write final result |

### 5.4 Collect Validation
| File | Function | Purpose |
|------|----------|---------|
| `vb_validate/src/gates.rs` | `validate_node_pairing` | Validates collect node pairing |
| `vb_validate/src/gates.rs` | `is_matching_collect_start` | Checks CollectStart/CollectPage/CollectFinish pairing |
| `vb_validate/src/gates.rs` | `is_collect_start_done` | Validates done target |

---

## 6. Identified Defect: Incomplete Collect Digest Coverage

### 6.1 The Problem

The `digest_step_primitive` function treats `Collect` as a "catch-all" case, only hashing the primitive name:

```rust
other => {
    hasher.update(canonical_primitive_name(other).as_bytes());
}
```

For `Collect`, this means only `"collect"` (6 bytes) are hashed, ignoring:
- `variable` (loop variable name)
- `source` (source expression string)
- `pages` (page limit)
- `items` (items per page)
- `body` (body steps content)

### 6.2 Impact

Two workflows with:
```yaml
# Workflow A
- id: step1
  collect:
    variable: x
    source: "list_a"
    pages: 10
    items: 5
    steps:
      - id: body1
        set:
          output: result
          value: "1"

# Workflow B
- id: step1
  collect:
    variable: y  
    source: "completely_different_list"
    pages: 999
    items: 100
    steps:
      - id: totally_different_body
        finish:
          result: "999"
```

These have **identical source digests** because only the step ID (`"step1"`) and primitive name (`"collect"`) are hashed.

### 6.3 Required Fix

The `digest_step_primitive` function needs a `Collect` case that includes all semantically significant fields:

```rust
vb_yaml::ast::StepPrimitive::Collect { variable, source, pages, items, body } => {
    hasher.update(b"collect");
    hasher.update(variable.as_bytes());
    hasher.update(source.as_bytes());
    if let Some(p) = pages {
        hasher.update(&p.to_le_bytes());
    }
    if let Some(i) = items {
        hasher.update(&i.to_le_bytes());
    }
    // Body steps should recursively digest their primitives
    for step in body {
        hasher.update(step.id.as_bytes());
        digest_step_primitive(hasher, &step.primitive);
    }
}
```

---

## 7. Scoped Files for Delivery

### 7.1 Must Fix (Digest Implementation)
- `vb_compile/src/mod_compile_lowering/part_05.rs` - `digest_step_primitive` function
- `vb_compile/src/compile/mod.rs` - `digest_step_primitive` function (duplicate)

### 7.2 Must Verify (No Regression)
- `crates/vb_compile/src/tests/error_variant_tests.rs` - `compute_compiled_digest` determinism tests
- `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs` - Cross-run digest determinism
- `crates/vb_storage/tests/vb_core_atomic_admission_red.rs` - Artifact digest admission

### 7.3 Related Infrastructure
- `vb_core/src/ids/mod.rs` - `WorkflowDigest` type definition
- `vb_core/src/workflow/mod.rs` - `CompiledWorkflow`, `WorkflowParts`
- `vb_yaml/src/ast/types.rs` - `StepPrimitive::Collect` definition
- `vb_yaml/src/ast/parse_steps.rs` - `parse_collect`

### 7.4 Tests Requiring Updates
- `crates/vb_compile/src/proptest_collect.rs` - Property tests for collect lowering
- `verification/kani/collect_try_from_parts.rs` - Kani harness for collect IR

---

## 8. Dependency Graph

```
vb_yaml::ast::StepPrimitive::Collect
         │
         ▼
vb_yaml::ast::parse_steps::parse_collect
         │
         ▼
vb_compile::mod_compile_lowering::part_02::lower_canonical_*
         │
         ▼
vb_compile::mod_compile_lowering::part_03::lower_canonical_collect
         │
         ├──► vb_core::CompiledNodeKind::CollectStart
         ├──► vb_core::CompiledNodeKind::CollectPage  
         └──► vb_core::CompiledNodeKind::CollectFinish
         │
         ▼
vb_compile::compile::canonical_digest (MISSING: collect field hashing)
         │
         ▼
vb_core::WorkflowParts::digest
         │
         ▼
vb_compile::compile::emit_compiled_artifact
         │
         ▼
vb_compile::mod_compile_core::compute_compiled_digest (BLAKE3 of artifact)
```
