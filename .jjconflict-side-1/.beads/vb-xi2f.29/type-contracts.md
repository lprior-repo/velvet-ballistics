# Type Contracts: Digest Coverage for Together

bead_id: vb-xi2f.29
bead_title: P1: digest covers together semantics
phase: 3 (rust-contract)
created_at: 2026-05-24

## Contract 1: Canonical Name for Together

### Signature
```rust
pub(super) fn canonical_primitive_name(primitive: &vb_yaml::ast::StepPrimitive) -> &'static str;
```

### Contract
- **PRE-NAME-001**: `primitive` is a valid `StepPrimitive` variant.
- **POST-NAME-001**: When `primitive` is `StepPrimitive::Together { .. }`, the return value MUST be `"together"`.
- **POST-NAME-002**: When `primitive` is `StepPrimitive::Together { .. }`, the return value MUST NOT be `"parallel"`.
- **POST-NAME-003**: All other primitives return their documented canonical name as per the current implementation (Set → `"set"`, Save → `"save"`, Do → `"do"`, Choose → `"choose"`, ForEach → `"for_each"`, Collect → `"collect"`, Aggregate → `"aggregate"`, Repeat → `"repeat"`, Wait → `"wait"`, Ask → `"ask"`, Finish → `"finish"`, unknown → `"unknown"`).
- **POST-NAME-004**: The match is total; no panics for any `StepPrimitive` variant.

### Source Locations
- Active: `crates/vb_compile/src/mod_compile_lowering/part_05.rs:98-114`
- Dead duplicate: `crates/vb_compile/src/compile/mod.rs:203-218` (do not fix)

## Contract 2: Canonical Digest Must Include Together Semantics

### Signature
```rust
pub(super) fn canonical_digest(source: &vb_yaml::ast::WorkflowSource) -> WorkflowDigest;
```

### Contract
- **PRE-DIGEST-001**: `source` is a valid, parsed `WorkflowSource`.
- **POST-DIGEST-001**: The returned `WorkflowDigest` is deterministic for the same `source`.
- **POST-DIGEST-002**: For any `source` containing a `Together` step, the digest includes:
  1. The step's `id` (already hashed)
  2. The canonical name `"together"` (requires fix of canonical_primitive_name)
  3. The count of branches (`branches.len()`)
  4. Each branch's `label` string
  5. Each branch's `steps` — recursively: each sub-step's `id` and `primitive`
  6. Branch ordering (branches are hashed in array order)
- **POST-DIGEST-003**: Changing a branch label, adding/removing/reordering branches, or modifying sub-step contents within a branch MUST change the digest.
- **POST-DIGEST-004**: Empty branch steps (a branch with zero steps) is a valid and hashable state.
- **POST-DIGEST-005**: The digest traverses nested steps to a bounded depth (at least `MAX_CONSTRUCT_DEPTH + 1` to cover legally valid workflows).

### Source Locations
- Active: `crates/vb_compile/src/mod_compile_lowering/part_05.rs:116-138`
- Dead duplicate: `crates/vb_compile/src/compile/mod.rs:220-241` (do not fix)

## Contract 3: Digest Step Primitive for Together

### Signature
```rust
pub(super) fn digest_step_primitive(hasher: &mut blake3::Hasher, primitive: &vb_yaml::ast::StepPrimitive);
```

### Contract
- **PRE-DSP-001**: `hasher` is a valid `blake3::Hasher` in a writable state.
- **PRE-DSP-002**: `primitive` is a valid `StepPrimitive` variant.
- **POST-DSP-001**: For `StepPrimitive::Together { branches }`:
  1. Hashes the canonical name `"together"` (via `canonical_primitive_name`)
  2. Hashes `branches.len() as u16` as little-endian bytes
  3. For each `TogetherBranch` in order:
     - Hashes `branch.label.as_bytes()`
     - Recursively calls `digest_sub_step` for each `StepAst` in `branch.steps`
- **POST-DSP-002**: For `StepPrimitive::Set` and `StepPrimitive::Finish`, behavior is unchanged from current implementation.
- **POST-DSP-003**: For all other primitives, behavior is unchanged (calls `canonical_primitive_name` only).

### Source Locations
- Active: `crates/vb_compile/src/mod_compile_lowering/part_05.rs:140-162`
- Dead duplicate: `crates/vb_compile/src/compile/mod.rs:243-261` (do not fix)

## Contract 4: Recursive Sub-Step Digest (New)

### Proposed Signature
```rust
pub(super) fn digest_sub_step(hasher: &mut blake3::Hasher, step: &vb_yaml::ast::StepAst);
```

### Contract
- **PRE-SUB-001**: `hasher` is a valid `blake3::Hasher`.
- **PRE-SUB-002**: `step` is a valid `StepAst`.
- **POST-SUB-001**: Hashes `step.id.as_bytes()`.
- **POST-SUB-002**: Calls `digest_step_primitive(hasher, &step.primitive)`, which for `Together` recursively processes branches.
- **POST-SUB-003**: The recursion terminates because the AST is a tree with finite depth (bounded by `limits::MAX_CONSTRUCT_DEPTH`).
- **POST-SUB-004**: Does not hash `step.name`, `step.condition`, `step.with`, `step.retry`, `step.on_error`, or `step.then`. These fields are not in current scope.

## Contract 5: Type-Level Guarantees for Digest Integrity

### WorkflowDigest (vb_core::ids)
- **TYPE-001**: `WorkflowDigest` is `Copy`, `Eq`, `Hash` — safe to compare and use as map keys.
- **TYPE-002**: `WorkflowDigest::from_bytes([u8; 32])` is a pure constructor; no validation beyond length.
- **TYPE-003**: `WorkflowDigest` is `#[repr(transparent)]` over `[u8; 32]` — memory layout guaranteed.

### WorkflowSource Steps
- **TYPE-004**: `WorkflowSource::steps()` returns `&[StepAst]` — flat, top-level only. Consumers of this method for digest purposes MUST also traverse nested step structures.
- **TYPE-005**: `StepPrimitive::Together` has `branches: Vec<TogetherBranch>` — the `Vec` field makes branch count a runtime integer, not a type-level constant. Digest must handle variable-length branch lists.
- **TYPE-006**: `TogetherBranch` has `label: String` and `steps: Vec<StepAst>` — both are `Clone, PartialEq, Eq` for test comparison.

## Contract 6: Non-Goals (Type Level)

- **NON-001**: Step-level `condition`, `with`, `retry`, `on_error`, `then` fields remain unhashed.
- **NON-002**: `compute_compiled_digest` (byte-level) is not modified.
- **NON-003**: `for_each`, `collect`, `aggregate`, `repeat` nested-step digestion is NOT addressed here; they have the same defect but are out of scope.
- **NON-004**: The dead code in `compile/mod.rs` is not fixed; it remains dead.
