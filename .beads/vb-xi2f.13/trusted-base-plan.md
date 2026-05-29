# Trusted Base Plan: vb-xi2f.13

## Principle

The nested choose lowering fix touches exactly two functions (`choose_width`, `lower_canonical_choose`) in one crate (`vb_compile`). Everything else in the compilation pipeline is trusted base — it does not change and its correctness is assumed for this bead.

## Trusted Artifacts (Not Changed by This Bead)

### 1. vb_core — IR Types (UNCHANGED)

| Artifact | File | Trust Assumption | Rationale |
|---|---|---|---|
| `CompiledNodeKind::ChooseSlot` | `crates/vb_core/src/nodes.rs` | Already supports `SlotBranch.target` per branch | IR layer designed for per-branch targeting from the start |
| `SlotBranch { condition, target }` | `crates/vb_core/src/workflow/mod.rs` | `condition: SlotIdx` is numeric, not string | Anti-hallucination invariant enforced by types |
| `CompiledNode` | `crates/vb_core/src/workflow/mod.rs` | `id: StepIdx`, `kind`, `next: Option<StepIdx>` | Standard IR node structure |
| `StepIdx(u16)` | `crates/vb_core/src/workflow/mod.rs` | `StepIdx::new(v)` fails for `v > u16::MAX` | Checked constructor; trusted for overflow prevention |
| `SlotIdx(u16)` | `crates/vb_core/src/workflow/mod.rs` | Wraps `u16` | Trusted namespace for slot indices |
| `ConstIdx(u16)` | `crates/vb_core/src/workflow/mod.rs` | Wraps `u16` | Trusted namespace for constant indices |

### 2. vb_compile — Slot Compiler (UNCHANGED)

| Artifact | File | Trust Assumption | Rationale |
|---|---|---|---|
| `SlotCompiler::record_slot` | `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | Allocates unique, monotonically increasing `SlotIdx` | Trusted for slot uniqueness invariant |
| `SlotCompiler::slot_count` | `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | Returns total allocated slots | Trusted for post-condition verification |
| `SlotCompiler::push_node` | `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | Appends node with correct step index | Trusted for node emission |
| `slot_from_text` | `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | Resolves `&str` → `Result<SlotIdx, CompileError>` | Trusted for when-string resolution (unchanged) |
| `checked_step_offset` | `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | Bounds-checked step index arithmetic | Trusted for step pointer advancement |

### 3. vb_compile — Lowering Utilities (UNCHANGED)

| Artifact | File | Trust Assumption | Rationale |
|---|---|---|---|
| `lower_choose` | `crates/vb_compile/src/mod_compile_lowering/part_06.rs` | Assembles `ChooseSlot` node from `SlotBranch[]` | Already supports per-branch targets; calls `validate_branch_route` |
| `validate_branch_route` | `crates/vb_compile/src/mod_compile_lowering/part_08.rs` | Checks empty branch table + missing otherwise | Existing validation reused |
| `canonical_layout` | `crates/vb_compile/src/mod_compile_lowering/part_01.rs` | Precomputes step widths via `step_width` dispatch | Calls `choose_width` — this will call the FIXED version |
| `canonical_step_names` | `crates/vb_compile/src/mod_compile_lowering/part_01.rs` | Maps step names to StepIdx | Trusted for label resolution |
| `body_width` | `crates/vb_compile/src/mod_compile_lowering/part_01.rs` | Computes width of body step sequence | Already uses `checked_add`; trusted for arithmetic safety |
| `step_idx` | `crates/vb_compile/src/mod_compile_lowering/part_01.rs` | Converts `u16` to `Result<StepIdx, CompileError>` | Trusted for overflow checking |

### 4. vb_validate — Validation (UNCHANGED)

| Artifact | File | Trust Assumption | Rationale |
|---|---|---|---|
| `vb_validate::shared::validate` | `crates/vb_validate/src/shared.rs` | Full IR validation (graph, nodes, reachability, slots) | Trusted for acceptance criterion AC7 |
| `validate_slot_choose` | `crates/vb_core/src/validation/nodes.rs` | Checks per-branch slot and target validity | Already validates per-branch targets |
| `validate_choice` | `crates/vb_core/src/validation/graph.rs` | Checks forward edges for all branches | Already validates per-branch target edges |
| `validate_forward_target` | `crates/vb_core/src/validation/graph.rs` | `target.as_usize() > ci` | Trusted for forward-edge guarantee |
| `validate_reachability` | `crates/vb_core/src/validation/graph.rs` | All nodes reachable from entry | Body nodes must be reachable through ChooseSlot |

### 5. vb_yaml — YAML AST & Parsing (UNCHANGED)

| Artifact | File | Trust Assumption | Rationale |
|---|---|---|---|
| `ChooseBranch { when, steps }` | `crates/vb_yaml/src/ast/types.rs` | `steps: Vec<StepAst>` already present | AST supports body steps |
| `StepAst { id, primitive }` | `crates/vb_yaml/src/ast/types.rs` | Standard step AST node | Trusted for lowering |
| `parse_choose` | `crates/vb_yaml/src/ast/parse_steps.rs` | Parses choose YAML including branch steps | Already parses body steps |
| `parse_body_steps` | `crates/vb_yaml/src/ast/parse_steps.rs` | Generic body step parser | Trusted for YAML step parsing |
| `DepthLimit` / `NodeLimit` | `crates/vb_yaml/src/ast/limits.rs` | Enforces YAML depth limit | Trusted for hostile input defense |

### 6. vb_core — Replay Engine (UNCHANGED)

| Artifact | File | Trust Assumption | Rationale |
|---|---|---|---|
| `replay_choose_slot` | `crates/vb_core/src/replay/choose.rs` | Dispatches to per-branch target | Already uses `SlotBranch.target`; no runtime change needed |
| `choose_slot_branch` | `crates/vb_core/src/engine/step.rs` | Engine dispatch to choose | Trusted for runtime execution |

### 7. vb_compile — Body Lowering Pattern (REFERENCE)

| Artifact | File | Trust Assumption | Rationale |
|---|---|---|---|
| `emit_single_body_set` | `crates/vb_compile/src/mod_compile_lowering/part_04.rs` | Pattern for lowering Set/Do body steps | Reference implementation for multi-step body lowering |
| `body_constant_index` | `crates/vb_compile/src/mod_compile_lowering/part_04.rs` | Parses constant value → `ConstIdx` | Trusted for constant resolution in body Set steps |

## Trust Boundary Stubs

The following artifacts are trusted and their implementation details are NOT re-verified by this bead's proof obligations:

1. **`SlotCompiler` internals** — assumed to allocate unique, monotonically increasing slots. Only the post-condition (slot_count increase) is verified.

2. **YAML parser (`parse_choose`, `parse_body_steps`)** — assumed to produce valid `ChooseBranch` AST. Only the lowering from AST→IR is verified. The parser is tested by existing YAML tests.

3. **`vb_validate::shared::validate`** — assumed to correctly reject invalid IR. Not re-proven. The IR produced by the fixed lowering is asserted to pass validation (AC7), which is a behavior test, not a proof.

4. **Replay engine (`replay_choose_slot`)** — assumed to correctly dispatch to per-branch targets. PS-TYPE-001 and PS-LIVENESS-001 verify specific replay behaviors, but the replay engine as a whole is trusted.

## Assumptions Requiring Verification

These assumptions are NOT in the trusted base and MUST be verified by proof obligations:

| Assumption | Verified By | Risk if False |
|---|---|---|
| `choose_width` returns exact node count | PO-KANI-001, PO-KANI-012, PO-PROPTEST-001, PO-PROPTEST-005 | CRITICAL: layout/width mismatch |
| Body edge nodes chain to `common_next` | PO-KANI-002, PO-PROPTEST-002 | HIGH: execution path corruption |
| Otherwise target is after all body nodes | PO-KANI-003, PO-PROPTEST-003 | MEDIUM: otherwise lands in body range |
| `checked_add` prevents width overflow | PO-KANI-004 | LOW: silent overflow |
| Generated StepIdx values stay in u16 range | PO-KANI-005 | LOW: node ID corruption |
| Slot indices are unique | PO-KANI-006, PO-FLUX-001 | CRITICAL: slot reuse corruption |
| Condition slots and body slots are disjoint | PO-KANI-007, PO-FLUX-002 | HIGH: condition overwrite |
| Fanout limits enforced at both checkpoints | PO-KANI-008 | LOW: limit bypass |
| Boolean condition slots at runtime | PO-KANI-009, PO-VERUS-001 | HIGH: Internal replay error |
| All-false-no-otherwise returns Internal error | PO-KANI-010 | MEDIUM: runtime error |
| `slot_from_text` fails closed | PO-KANI-011, PO-FUZZ-001 | MEDIUM: slot access violation |
| Deep nesting produces graceful error | PO-PROPTEST-004, PO-FUZZ-002 | LOW: OOM/stack overflow |
| No YAML strings in IR | PO-KANI-013 | HIGH: anti-hallucination violation |
