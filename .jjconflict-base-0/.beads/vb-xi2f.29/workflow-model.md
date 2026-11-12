# Workflow Model: Digest Computation with Together Coverage

bead_id: vb-xi2f.29
bead_title: P1: digest covers together semantics
phase: 3 (rust-contract)
created_at: 2026-05-24

## Current State (Buggy)

```
compile_source(source)
  └─ canonical_digest(source)                         // part_05.rs:116
       ├─ hasher.update(version)
       ├─ hasher.update(name)
       ├─ hasher.update(trigger details)
       └─ for step in source.steps()                  // TOP-LEVEL ONLY
            ├─ hasher.update(step.id)
            └─ digest_step_primitive(hasher, &step.primitive)
                 ├─ Set    → hash "set" + output + value
                 ├─ Finish → hash "finish" + result
                 └─ OTHER  → hash canonical_primitive_name(primitive)
                      └─ Together → "parallel"  ← BUG: should be "together"
```

**Defect summary:**
1. Only top-level `step.id` and the string `"parallel"` enter the digest for a Together step.
2. Branch `label` strings, branch count, branch ordering, and sub-step contents are entirely absent.
3. Changing branch count 2→3, renaming branches, or modifying sub-step primitives does not change the digest.

## Target State (After Fix)

```
canonical_digest(source)
  ├─ hasher.update(version)
  ├─ hasher.update(name)
  ├─ hasher.update(trigger details)
  └─ for step in source.steps()                       // top-level
       ├─ hasher.update(step.id)
       └─ digest_step_primitive(hasher, &step.primitive)
            ├─ Set         → hash "set" + output + value
            ├─ Finish      → hash "finish" + result
            ├─ Together { branches }
            │    ├─ hash canonical_primitive_name → "together"
            │    ├─ hash branch_count as u16 LE bytes
            │    └─ for branch in branches (in order):
            │         ├─ hasher.update(branch.label)
            │         └─ for sub_step in branch.steps:
            │              └─ digest_sub_step(hasher, &sub_step)
            │                   ├─ hasher.update(sub_step.id)
            │                   └─ digest_step_primitive → (RECURSIVE)
            └─ OTHER → hash canonical_primitive_name
```

## State Machine: Digest Computation

### States

| State | Description |
|-------|-------------|
| **START** | `canonical_digest()` invoked with a `WorkflowSource` |
| **METADATA_HASHED** | Version, name, trigger hashed |
| **STEP_ITERATION** | Iterating over `source.steps()` |
| **PRIMITIVE_HASHED** | Current step's primitive has been fed to the hasher |
| **TOGETHER_BRANCHES_HASHED** | All branches of a together step have been recursively hashed |
| **SUB_STEP_HASHED** | A nested sub-step within a branch has been hashed |
| **DONE** | `WorkflowDigest::from_bytes(hasher.finalize())` called and returned |

### Transitions

```
START ───[hash metadata]──────────► METADATA_HASHED
METADATA_HASHED ───[begin step loop]──► STEP_ITERATION
STEP_ITERATION
  ├──[hash step.id]────────────────────► STEP_ITERATION (advance to primitive)
  ├──[hash primitive: non-Together]───► PRIMITIVE_HASHED ──► STEP_ITERATION (next step)
  ├──[hash primitive: Together]───────► PRIMITIVE_HASHED (begin branch loop)
  │     ├──[hash canonical name "together"]───►
  │     ├──[hash branch_count]───────────────►
  │     └──[for each branch:]
  │           ├──[hash branch.label]─────────►
  │           └──[for each sub_step:]
  │                 └──[hash sub_step.id, sub_step.primitive]──► SUB_STEP_HASHED
  │                       └──[if sub_step.primitive is Together → RECURSE]
  │
  └──[steps exhausted]────────────────► DONE
DONE ──► return WorkflowDigest
```

### Guards

- **G-001**: `canonical_digest` is only called during compilation (`compile_source`, `YamlCompiler::compile`). It is not called at runtime.
- **G-002**: `source.steps()` is non-empty for any valid workflow (at least one step exists). Empty step lists are valid and hashable (produce a deterministic digest).
- **G-003**: Branch count is bounded by `limits::MAX_TOGETHER_BRANCHES` (≤ `u16::MAX`). No overflow possible when hashing `len() as u16`.
- **G-004**: Recursion depth is bounded by `limits::MAX_CONSTRUCT_DEPTH` (currently 32). No infinite recursion.
- **G-005**: Together step with zero branches is rejected during validation/compilation before digest is computed (`validate_together_start_edges` rejects empty branch lists).

### Outcomes

| Outcome | Condition |
|---------|-----------|
| **Deterministic digest** | Normal path — same source → same digest |
| **Different digest** | Semantically different source → different digest (the contract being established) |
| **Compilation error before digest** | Invalid together structure (zero branches, out-of-bounds index) rejected by validation |
| **Digest panic** | MUST NEVER happen. All hashing operations are infallible. |

## Digest Sub-Step Traversal (New Logic)

### Algorithm

```
fn digest_sub_step(hasher: &mut blake3::Hasher, step: &StepAst):
    hasher.update(step.id.as_bytes())
    digest_step_primitive(hasher, &step.primitive)
    // If step.primitive is Together, digest_step_primitive recurses into branches.
    // If step.primitive contains other sub-steps (for_each, collect, etc.),
    //    they would need similar recursive treatment. NOT IN SCOPE for this bead.
```

### Termination Proof Sketch

- Each recursive call corresponds to a step at a deeper nesting level.
- Maximum legal nesting depth is `MAX_CONSTRUCT_DEPTH` (32).
- Base case: a step whose primitive is not `Together` (or other scoped primitives in scope) → no further recursion.
- Since the YAML AST is a finite tree (parsing rejects cyclic references), the recursion always terminates.

## Integration Points

1. **`compile_source()` in `part_01.rs:46`**: assigns `digest: canonical_digest(source)` to `WorkflowParts`. Must pick up the fixed digest.
2. **`YamlCompiler::compile()` in `mod_compile_core.rs:30`**: calls `compile_source()`.
3. **`lower_canonical_step()` → `lower_canonical_parallel()`**: happens AFTER digest computation. Not affected by changes.
4. **`compute_compiled_digest()` in `mod_compile_core.rs:114`**: byte-level digest. Unaffected.

## Verification Paths

- **Unit test**: Construct two `WorkflowSource` values differing only in together branches; assert digest inequality.
- **Proptest**: Randomly generate together branch configurations; assert digest determinism and branch-config sensitivity.
- **Kani**: Verify `canonical_primitive_name(Together) == "together"` (already harnessed, currently failing).
- **Kani**: Verify `canonical_digest` is deterministic for symbolic `WorkflowSource` with bounded together structures.
