# Codebase Map: digest coverage of wait semantics

**Bead:** vb-xi2f.32
**Date:** 2026-05-24
**Status:** DISCOVERED — digest gap confirmed

## 1. Scope Summary

The compiler computes a `WorkflowDigest` (blake3 hash) from parsed workflow source. This digest is embedded in `CompiledWorkflow` and used by the engine for identity, idempotency, and integrity checks.

**Finding:** The internal `canonical_digest()` function does NOT hash the fields of `Wait` primitives (`event`, `timeout`). It only hashes the string `"wait"` as a primitive name. Therefore, two workflows with different wait conditions (different event slots, different timeout durations) produce the **same digest** — the digest is insensitive to wait semantics.

## 2. Key Files — Digest Computation

### Primary: `vb_compile` — digest logic (TWO copies exist)

| File | Symbol | Role |
|------|--------|------|
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs:116` | `pub(super) fn canonical_digest` | Internal digest used by `compile_source()` in part_01.rs. This is the canonical copy used by the active cold-path compiler. |
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs:140` | `pub(super) fn digest_step_primitive` | Dispatches per-primitive hashing. **Wait falls to catch-all `other =>` arm (line 158-160).** |
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs:98` | `pub(super) fn canonical_primitive_name` | Returns name string per primitive enum variant. Wait → `"wait"`. |
| `crates/vb_compile/src/compile/mod.rs:220` | `fn canonical_digest` | **DUPLICATE** — legacy copy used by the warm-path YamlCompiler codepath. Same bug: Wait not hashed. |
| `crates/vb_compile/src/compile/mod.rs:243` | `fn digest_step_primitive` | **DUPLICATE** — same bug at line 257-259. |
| `crates/vb_compile/src/compile/mod.rs:203` | `fn canonical_primitive_name` | **DUPLICATE** — returns `"wait"` for Wait. |
| `crates/vb_compile/src/mod_compile_core.rs:114` | `pub fn compute_compiled_digest` | Public API: hashes raw `source: &[u8]` bytes with blake3. **Not affected** — this always changes when source bytes change. |

### Digest type definition

| File | Symbol | Role |
|------|--------|------|
| `crates/vb_core/src/ids/mod.rs:342` | `pub struct WorkflowDigest([u8; 32])` | 32-byte blake3 hash. `#[repr(transparent)]`, `Copy`, `Eq`, `Hash`. |
| `crates/vb_core/src/ids/mod.rs:344` | `impl WorkflowDigest` | `from_bytes`, `as_bytes` constructors. |

### WorkflowParts — digest carrier

| File | Symbol | Role |
|------|--------|------|
| `crates/vb_core/src/workflow/mod.rs:278` | `pub digest: WorkflowDigest` | Field in `WorkflowParts`. |
| `crates/vb_core/src/workflow/mod.rs:101` | `pub const fn digest(&self) -> WorkflowDigest` | Accessor on `CompiledWorkflow`. |

## 3. Key Files — Wait Lowering/Compilation

### YAML AST definition

| File | Symbol | Role |
|------|--------|------|
| `crates/vb_yaml/src/ast/types.rs:238` | `StepPrimitive::Wait { event: Option<String>, timeout: Option<String> }` | Parsed YAML fields. `event` = wait for event slot; `timeout` = deadline (when `event` is `None`) or timeout (when `event` is `Some`). |

### Canonical lowering pipeline (active cold-path)

| File | Symbol | Role |
|------|--------|------|
| `crates/vb_compile/src/mod_compile_lowering/part_01.rs:16` | `pub fn compile_source` | Main entry: validates, lays out steps, iterates, calls `canonical_digest(source)` at line 46. |
| `crates/vb_compile/src/mod_compile_lowering/part_02.rs:59` | `lower_canonical_step` (Wait arm) | Dispatches to `lower_canonical_wait` with `event.as_deref()` and `timeout.as_deref()`. |
| `crates/vb_compile/src/mod_compile_lowering/part_04.rs:135` | `pub(super) fn lower_canonical_wait` | Parses slots from text, determines WaitKind (Until vs Event), calls `lower_wait`. |
| `crates/vb_compile/src/mod_compile_lowering/part_07.rs:84` | `pub fn lower_wait` | Emits `CompiledNodeKind::WaitUntil` or `WaitEvent` nodes. |
| `crates/vb_compile/src/mod_compile_lowering/part_07.rs:73` | `pub enum WaitKind { Until { deadline }, Event { event, timeout } }` | Type-safe discriminator for the two legal wait shapes. |

### IR node kinds (runtime engine)

| File | Symbol | Role |
|------|--------|------|
| `crates/vb_core/src/nodes.rs:155` | `CompiledNodeKind::WaitUntil { deadline_slot: SlotIdx }` | Deadline wait IR. |
| `crates/vb_core/src/nodes.rs:157` | `CompiledNodeKind::WaitEvent { event: SlotIdx, timeout_slot: Option<SlotIdx> }` | Event wait IR. |

### Engine wait handling

| File | Symbol | Role |
|------|--------|------|
| `crates/vb_core/src/engine/step.rs:89` | `Ok(EngineSignal::AwaitingWait)` | Wait signal emitted by step engine. |
| `crates/vb_core/src/engine/signals.rs:110` | `EngineSignal::AwaitingWait` | Signal variant for wait suspension. |
| `crates/vb_core/src/frame.rs:23` | "Step is suspended on a wait primitive" | Frame docs. |
| `crates/vb_core/src/frame.rs:414` | "Marks a step waiting" | Frame operation. |

### Validation

| File | Symbol | Role |
|------|--------|------|
| `crates/vb_compile/src/mod_compile_validation/part_03.rs:186` | `validate_wait_shape` | Validates YAML wait fields: requires `until` XOR `event` (± optional `timeout`). |
| `crates/vb_compile/src/mod_compile_validation/part_02.rs:208` | `StepPrimitive::Wait` | Validation enum variant. |

### Compile AST (warm-path, older)

| File | Symbol | Role |
|------|--------|------|
| `crates/vb_compile/src/ast/types.rs:175` | `StepKindAst::Wait { slot, timeout, is_event }` | Compile AST representation. |
| `crates/vb_compile/src/ast/parse.rs:387` | `parse_wait` | Parses `until` or `event` + optional `timeout`. |

## 4. The Gap — Explained

### What the digest currently hashes for Wait

In both copies of `digest_step_primitive`, the Wait match arm falls through to:

```rust
other => {
    hasher.update(canonical_primitive_name(other).as_bytes());
    // `canonical_primitive_name(Wait{..})` returns "wait"
}
```

This means only the string `"wait"` is hashed. The fields `event: Option<String>` and `timeout: Option<String>` are **completely ignored**.

### What the digest DOES hash for comparison

For `Set`, the digest includes `output` and `value` fields.
For `Finish`, the digest includes the `result` value.

### What `compute_compiled_digest` does

The public API function `compute_compiled_digest(source: &[u8])` hashes the raw source bytes. This **is** sensitive to source changes (including wait field changes), but it is a separate function from `canonical_digest` — they produce different hash values for the same workflow.

### Concrete example of the gap

Two workflows like these produce the **same** `canonical_digest`:

```yaml
# Workflow A: wait for event on slot 0, 30s timeout
- id: wait_for_event
  wait:
    event: "0"
    timeout: "30"

# Workflow B: wait until deadline on slot 5
- id: wait_for_deadline
  wait:
    timeout: "5"
```

Both produce the hash of `"wait"` for their wait step — no difference in the digest. But the compiled IR nodes are completely different (`WaitEvent{event:0, timeout:Some(30)}` vs `WaitUntil{deadline_slot:5}`).

## 5. Broader Digest Coverage Gap (context)

This gap is NOT limited to Wait. All primitives except `Set` and `Finish` fall through to the `other =>` arm in `digest_step_primitive`:

- **Hashed with fields:** `Set` (output, value), `Finish` (result)
- **Name only (NO fields):** `Wait`, `Ask`, `Do`, `Save`, `Choose`, `ForEach`, `Together`, `Parallel`, `Collect`, `Aggregate`, `Repeat`

For the bead scope (Wait), the fix would add a match arm for `StepPrimitive::Wait` that hashes both `event` and `timeout` fields.

## 6. Existing Test Coverage

### Tests for digest computation

| File | Test | What it proves |
|------|------|---------------|
| `crates/vb_compile/src/tests/error_variant_tests.rs:765` | `compiled_digest_is_deterministic` | Same source → same `compute_compiled_digest` result <br/>**NOTE:** Tests only the public byte-hashing API, NOT `canonical_digest`. |
| `crates/vb_compile/src/tests/error_variant_tests.rs:781` | `different_sources_produce_different_digests` | Different source name → different digest <br/>**NOTE:** Tests only the public byte-hashing API. |
| `crates/vb_compile/tests/v1_primitive_lowering.rs:828` | `proptest_equal_primitive_sources_compile_to_equal_digest_and_ir` | Proptest: same source → same `canonical_digest` <br/>**NOTE:** Only tests equality, not sensitivity to wait field changes. |
| `crates/vb_core/src/ids/mod.rs:898` | `workflow_digest_equality` | Basic equality/inequality of WorkflowDigest. |
| `crates/vb_core/src/ids/mod.rs:977` | `workflow_digest_hash_consistency` | Hash consistency for HashMap use. |
| `crates/vb_core/tests/vb_core_yaml_e2e_chain_strict_yaml.rs:64` | (inline assertion) | `workflow.digest() == workflow.to_parts().digest` roundtrip. |

### Tests for wait compilation

| File | Test | What it proves |
|------|------|---------------|
| `crates/vb_compile/tests/v1_primitive_lowering.rs:113` | `compile_workflow_emits_supported_ir...` | Wait compiles to `["WaitEvent", "Finish"]` node sequence. |
| `crates/vb_compile/tests/v1_primitive_lowering.rs:231` | `compile_workflow_emits_exact_wait_until_shape...` | Wait with deadline-only → `["WaitUntil", "Finish"]`. |
| `crates/vb_compile/tests/v1_primitive_lowering.rs:246` | `compile_workflow_returns_step_field_shape...` (wait case) | Empty `wait.event` → validation error. |
| `crates/vb_core/src/engine/tests/integration_step_behavior.rs:519` | `wait_until_returns_awaiting_wait...` | Engine emits AwaitingWait for WaitUntil. |
| `crates/vb_core/src/engine/tests/integration_step_behavior.rs:555` | `wait_event_returns_awaiting_wait...` | Engine emits AwaitingWait for WaitEvent. |
| `crates/vb_core/tests/section36_mandatory_coverage.rs:1056` | `waiting_step_can_resume_to_running` | Waiting→Running state transition. |

### MISSING: No test for digest sensitivity to wait field changes

**There is no test that verifies the digest changes when wait fields (`event`, `timeout`) change.** The proptest at line 828 only tests digest stability (same input → same digest), not sensitivity (different input → different digest).

## 7. Crates and Dependency Graph

```
vb_yaml (AST types for YAML parsing)
    ↓
vb_compile (cold-path compiler, uses vb_yaml::ast::StepPrimitive)
    ↓ depends on
vb_core (WorkflowDigest, WorkflowParts, CompiledNodeKind, CompiledWorkflow)
    ↓ depends on
vb_validate (shared workflow validation)
vb_codegen (Rust code generation, stub)
```

- `vb_compile` is the sole crate that computes `canonical_digest`.
- `vb_core` defines `WorkflowDigest` and `WorkflowParts` (the digest carrier).
- `vb_yaml` defines `StepPrimitive::Wait { event, timeout }` — the type whose fields are being ignored.

## 8. Risks

| Risk | Severity | Detail |
|------|----------|--------|
| Digest collision | **HIGH** | Two workflows with different wait semantics produce identical digests. This could cause integrity checks to pass incorrectly, idempotency gates to incorrectly deduplicate distinct workflows, and replay verification to treat different workflows as identical. |
| Inconsistency between `canonical_digest` and `compute_compiled_digest` | MEDIUM | These two functions produce different digests for the same workflow. The internal `canonical_digest` ignores wait fields; the public byte-hashing API does not. |
| Duplicated digest code | MEDIUM | Identical `canonical_digest` logic exists in both `compile/mod.rs` and `mod_compile_lowering/part_05.rs`. Both have the same bug. Both need fixing. |
| Broader gap | LOW (out of scope) | 7 other primitives also only hash their name, not their fields. This may be intentional for some (body-containing primitives like ForEach are recursively covered by hashing step IDs), but Ask, Do, and others may also need field-level hashing. |

## 9. Recommended Fix Path

The fix is in TWO files (both copies of `digest_step_primitive`):

1. **`crates/vb_compile/src/mod_compile_lowering/part_05.rs:140`** — Add a `Wait` match arm that hashes:
   - `"wait"` label
   - `event.as_deref().unwrap_or("until")` (or a null marker for absent event)
   - `timeout.as_deref().unwrap_or("none")` (or a null marker for absent timeout)

2. **`crates/vb_compile/src/compile/mod.rs:243`** — Same change in the duplicate.

New tests needed:
- A test that verifies different wait event slots produce different digests.
- A test that verifies different wait timeouts produce different digests.
- A test that verifies `WaitUntil` vs `WaitEvent` produce different digests.
- A test that verifies absent timeout vs present timeout produce different digests.

## 10. Open Questions

1. **Should both `canonical_digest` and `compute_compiled_digest` produce the same result?** Currently they don't. The bead doesn't specify.
2. **Should the broader gap (other primitives) be addressed in this bead or in follow-up beads?** Scope suggests Wait only.
3. **Is the duplicate digest code intended to converge?** Both copies have identical bugs. A refactoring bead may be needed.

## 11. Downstream Handoff

- **rust-contract:** Model wait digest semantics. Define what wait properties must affect the digest.
- **proof-planner:** Plan Kani/proptest coverage for digest sensitivity to wait fields.
- **test-writer:** Write missing test: "different wait conditions → different digests."
- **holzman-rust:** Fix `digest_step_primitive` in both copies with full match arm for Wait.
