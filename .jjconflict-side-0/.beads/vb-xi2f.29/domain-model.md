# Domain Model: Digest Coverage for Together

bead_id: vb-xi2f.29
bead_title: P1: digest covers together semantics
phase: 3 (rust-contract)
created_at: 2026-05-24

## Ubiquitous Language

| Term | Definition |
|------|-----------|
| **WorkflowDigest** | A 32-byte blake3 hash that uniquely identifies the compiled semantics of a workflow source. Changes to the digest signal that the compiled artifact materially differs from a prior compilation. |
| **Canonical Name** | The stable semantic label for a step primitive independent of YAML keyword spellings. The canonical name for `parallel:` YAML is `"together"`, not `"parallel"`. |
| **Canonical Digest** | A structure-level digest (as opposed to byte-level `compute_compiled_digest`) that hashes version, name, trigger, step IDs, and step primitives in a deterministic traversal. |
| **Together Step** | A workflow step with YAML `parallel:` keyword, AST type `StepPrimitive::Together { branches: Vec<TogetherBranch> }`. Fan-out to N concurrent branches. |
| **Together Branch** | A named sub-workflow within a Together step. Has a `label` (e.g., `"left"`, `"right"`) and a list of `steps: Vec<StepAst>`. |
| **Top-Level Steps** | Steps reachable via `WorkflowSource::steps()`. Nested sub-steps inside `TogetherBranch.steps`, `ForEach.body`, `Collect.body`, etc. are NOT in this list. The digest ONLY hashes top-level step IDs. |
| **Digest Sensitivity** | A property meaning: any change to the workflow's semantics MUST produce a different WorkflowDigest. The inverse (different digest → different semantics) is not guaranteed, but same digest → same semantics is the contract. |
| **Nested Step Blindness** | The defect where scoped primitives (together, for_each, collect, aggregate, repeat) have nested sub-steps that are invisible to the digest. Only the parent step's ID and canonical primitive name are hashed. |

## Entities

### WorkflowDigest (Value Object)
- **Type**: `vb_core::ids::WorkflowDigest([u8; 32])`
- **Derived from**: blake3 hash of structured fields + nested step traversal
- **Invariants**: Deterministic for same source; changes when semantics change
- **Identity**: 32-byte value; equality is byte-for-byte

### WorkflowSource (Aggregate Root)
- **Type**: `vb_yaml::ast::WorkflowSource`
- **Fields**: version, name, trigger, inputs, vars, secrets, steps, result, examples
- **Key Method**: `steps()` returns `&[StepAst]` — only top-level steps
- **Gap**: No recursive step traversal method exists

### StepPrimitive::Together (Entity)
- **Fields**: `branches: Vec<TogetherBranch>`
- **In-scope struct fields for digest**: branch count, branch labels, sub-step IDs, sub-step primitives, branch ordering
- **Note**: All fields are currently unhashed. Only the parent step `id` and the string `"parallel"` (bug) enter the digest.

### TogetherBranch (Value Object)
- **Fields**: `label: String`, `steps: Vec<StepAst>`
- **All fields are digest-relevant**
- **Ordering matters**: `[branch_a, branch_b]` ≠ `[branch_b, branch_a]` for digest

### StepAst (Entity)
- **Fields**: `id: String`, `name: Option<String>`, `condition: Option<String>`, `primitive: StepPrimitive`, `with: Option<String>`, `retry: Option<RetryPolicy>`, `on_error: Option<ErrorHandlerAst>`, `then: Option<String>`
- **For digest**: only `id` and `primitive` are currently hashed. Future scope could consider `condition`, `with`, `retry`, `error_handler`, and `then` for completeness.

## Invariants

- **INV-001 (Digest Sensitivity)**: Changing any field of a `TogetherBranch` (label, steps) or adding/removing/reordering branches MUST produce a different `WorkflowDigest`.
- **INV-002 (Canonical Name)**: `canonical_primitive_name(Together)` MUST return `"together"`, not `"parallel"`.
- **INV-003 (Determinism)**: `canonical_digest(source)` is deterministic: same `WorkflowSource` → same `WorkflowDigest` every time.
- **INV-004 (Branch Ordering)**: The digest includes branch labels/contents in the order they appear in the `branches` vector. Reordering branches produces a different digest.
- **INV-005 (Non-Vacuity)**: A test with two `WorkflowSource` values that differ only in together semantics MUST compute different digests.
- **INV-006 (Recursive Completeness)**: All nested steps within a together branch are recursively hashed, including their nested sub-steps (e.g., a for_each inside a together branch).

## Forbidden States

- **FS-001**: Two `WorkflowSource` values with different together `branches` produce the same `WorkflowDigest`.
- **FS-002**: `canonical_primitive_name(Together)` returns `"parallel"` (current bug).
- **FS-003**: A digest that does not change when a branch label changes from `"left"` to `"right"`.
- **FS-004**: A digest that does not change when a new branch is added to a together step.
- **FS-005**: A digest that does not change when sub-step contents within a branch are modified.
- **FS-006**: Infinite recursion or stack overflow during nested step traversal for deeply nested together constructs.

## Scope and Non-Goals

**In scope for this bead:**
- Together step's branch labels, branch count, branch ordering, and sub-step IDs/primitives must affect the digest.

**Out of scope for this bead (may be future beads):**
- `for_each`, `collect`, `aggregate`, `repeat` — same nested-step-blindness defect exists but is not addressed here.
- Step-level `condition`, `with`, `retry`, `on_error`, `then` fields — not currently in digest scope.
- `compute_compiled_digest` (byte-level digest) — different purpose, not in scope.
- Deleting dead code in `compile/mod.rs`.
- Canonical name fix for `Aggregate` → `"reduce"` (separate issue).
