# Proof-to-Implementation Input: vb-xi2f.13

## Bridge Overview

This document maps every planned proof obligation to concrete Rust implementation targets, behavior tests, and evidence commands. The `proof-to-implementation` agent uses this input to create the bridge artifacts that connect proof claims to production code.

## Production Code Targets

### File: `crates/vb_compile/src/mod_compile_lowering/part_01.rs`

| Function | Lines (current) | Change | Proof Obligations | Behavior Tests Needed |
|---|---|---|---|---|
| `choose_width` | 117-122 | Replace hardcoded `Ok(1)` with `1 + sum(body_width)` using `checked_add` | PO-KANI-001, PO-KANI-004, PO-KANI-012, PO-PROPTEST-001, PO-PROPTEST-005 | Unit: `choose_width_empty_branches_returns_1`, `choose_width_single_branch_2_steps_returns_3`, `choose_width_multi_branch_returns_correct_sum`, `choose_width_overflow_returns_error` |
| `canonical_step_width` | 86-146 | Dispatch to `choose_width` already exists; fix is in callee | (indirect) | Regression: existing tests pass unchanged |
| `canonical_layout` | (unchanged) | Calls `choose_width` — receives fixed width | (indirect) | Integration: layout precomputation allocates correct width |

### File: `crates/vb_compile/src/mod_compile_lowering/part_02.rs`

| Function | Lines (current) | Change | Proof Obligations | Behavior Tests Needed |
|---|---|---|---|---|
| `lower_canonical_choose` | 216-293 | Remove lines 251-259 (body rejection); add body lowering with per-branch target assignment | PO-KANI-002, PO-KANI-003, PO-KANI-005, PO-KANI-006, PO-KANI-007, PO-KANI-012, PO-KANI-013, PO-PROPTEST-002, PO-PROPTEST-003, PO-PROPTEST-005 | Unit: `lower_choose_empty_body_targets_next`, `lower_choose_single_body_set_targets_body_start`, `lower_choose_multi_body_steps_chain_correctly`, `lower_choose_unsupported_primitive_in_body_returns_error`, `lower_choose_body_overflows_stepidx`; Integration: `compile_yaml_with_choose_body_produces_valid_ir`; Regression: all existing choose tests pass unchanged |

### File: `crates/vb_compile/src/mod_compile_lowering/part_05.rs`

| Function | Lines | Change | Proof Obligations | Behavior Tests Needed |
|---|---|---|---|---|
| `slot_from_text` | (unchanged) | No change | PO-KANI-011, PO-FUZZ-001 | Fuzz: already handled by fuzz target |

### File: `crates/vb_compile/src/mod_compile_lowering/part_06.rs`

| Function | Lines | Change | Proof Obligations | Behavior Tests Needed |
|---|---|---|---|---|
| `lower_choose` | (unchanged) | No change | PO-KANI-008 (fanout re-verification) | Regression: existing fanout tests pass |

### File: `crates/vb_core/src/replay/choose.rs`

| Function | Lines | Change | Proof Obligations | Behavior Tests Needed |
|---|---|---|---|---|
| `replay_choose_slot` | (unchanged) | No change | PO-KANI-009, PO-KANI-010 | Regression: existing 7 replay tests pass; New: `replay_choose_with_body_steps_executes_correct_nodes` |

## Behavior Test Inventory

### Unit Tests (vb_compile)

| Test Name | Source File | Accepts | Rejects | Maps to AC |
|---|---|---|---|---|
| `choose_width_empty_branches_returns_1` | `tests.rs` (new) | All branches empty → 1 | — | AC2 |
| `choose_width_single_branch_2_steps_returns_3` | `tests.rs` (new) | 1 branch × 2 Set steps → 3 | — | AC1 |
| `choose_width_multi_branch_returns_correct_sum` | `tests.rs` (new) | 2 branches × mixed steps → correct sum | — | AC1 |
| `choose_width_overflow_returns_error` | `tests.rs` (new) | — | Width overflow → StepIndexOutOfRange | AC1 |
| `lower_choose_empty_body_targets_next` | `tests.rs` (new) | Empty branch → target = next | — | AC8 |
| `lower_choose_single_body_set_targets_body_start` | `tests.rs` (new) | 1 branch × 1 Set → target = id + 1 | — | AC3, AC4 |
| `lower_choose_multi_body_steps_chain_correctly` | `tests.rs` (new) | 1 branch × 3 steps → all chained, last → next | — | AC4, AC5 |
| `lower_choose_two_branches_mixed_bodies` | `tests.rs` (new) | Branch 0 empty, branch 1 with body → correct targets | — | AC4 |
| `lower_choose_unsupported_primitive_in_body_returns_error` | `tests.rs` (new) | — | Body step with ForEach → UnsupportedStepPrimitive | AC3 |
| `lower_choose_body_overflows_stepidx` | `tests.rs` (new) | — | Body too many steps → StepIndexOutOfRange | AC3 |
| `lower_choose_condition_slots_distinct_from_body_slots` | `tests.rs` (new) | Condition + body slots disjoint | — | AC6, AC9 |
| `lower_choose_no_yaml_strings_in_slotbranch_condition` | `tests.rs` (new) | All conditions are SlotIdx | — | AC9 |

### Regression Tests (vb_compile)

| Test Name | File | Purpose |
|---|---|---|
| `lower_canonical_choose_accepts_two_branches` | `tests.rs` (existing) | Existing test must pass unchanged |
| `lower_canonical_choose_rejects_unknown_otherwise_label` | `tests.rs` (existing) | Existing test must pass unchanged |
| `lower_canonical_choose_rejects_65_branches` | `tests.rs` (existing) | Fanout limit preserved (AC10) |

### Integration Tests (workspace_tests)

| Test Name | File | Purpose |
|---|---|---|
| `compile_yaml_with_choose_single_body_set` | `integration_choose.rs` (new) | YAML with choose + `Set` body → valid `CompiledWorkflow` |
| `compile_yaml_with_choose_multi_body_steps` | `integration_choose.rs` (new) | YAML with choose + `Set` + `Do` body → valid |
| `compile_yaml_with_choose_body_and_otherwise` | `integration_choose.rs` (new) | YAML with choose + body + otherwise → valid, otherwise after body |
| `replay_compiled_choose_with_body_executes_nodes` | `integration_choose.rs` (new) | Compile + replay: choose body nodes execute correctly |
| `existing_choose_fixtures_still_compile` | `integration_choose.rs` | Regression: `choose_true.yaml`, `3step_choose.yaml` produce same IR (AC8) |

### YAML Test Fixtures Needed

| Fixture | Description |
|---|---|
| `fixtures/valid/choose_body_set.yaml` | Choose with 1 branch, body has single `Set` step |
| `fixtures/valid/choose_body_set_do.yaml` | Choose with 1 branch, body has `Set` + `Do` steps |
| `fixtures/valid/choose_body_multi_branch.yaml` | Choose with 2 branches, mixed empty/non-empty bodies |
| `fixtures/valid/choose_body_otherwise.yaml` | Choose with body + otherwise label |
| `fixtures/invalid/choose_body_unsupported_prim.yaml` | Choose with body containing `ForEach` (should error) |
| `fixtures/invalid/choose_body_too_many_steps.yaml` | Choose with body exceeding step budget (should error) |

## Proof-to-Source Tracing

| Proof Obligation | Production Function | Source File | Line(s) After Fix |
|---|---|---|---|
| PO-KANI-001 | `choose_width`, `lower_canonical_choose` | `part_01.rs`, `part_02.rs` | ~117-132, ~216-293 |
| PO-KANI-002 | `lower_canonical_choose` | `part_02.rs` | ~216-293 |
| PO-KANI-003 | `lower_canonical_choose` | `part_02.rs` | ~262-283 (otherwise resolution) |
| PO-KANI-004 | `choose_width`, `body_width` | `part_01.rs` | ~117-132 |
| PO-KANI-005 | `step_idx`, `lower_canonical_choose` | `part_01.rs`, `part_02.rs` | ~99, ~216-293 |
| PO-KANI-006 | `SlotCompiler::record_slot` | `part_05.rs` (unchanged) | N/A |
| PO-KANI-007 | `slot_from_text`, `record_slot` | `part_05.rs` (unchanged) | N/A |
| PO-KANI-008 | `lower_canonical_choose`, `lower_choose` | `part_02.rs`, `part_06.rs` | ~226, ~26 |
| PO-KANI-009 | `replay_choose_slot` | `replay/choose.rs` (unchanged) | 12-58 |
| PO-KANI-010 | `replay_choose_slot` | `replay/choose.rs` (unchanged) | 12-58 |
| PO-KANI-011 | `slot_from_text` | `part_05.rs` (unchanged) | ~29 |
| PO-KANI-012 | `choose_width`, `lower_canonical_choose` | `part_01.rs`, `part_02.rs` | ~117-132, ~216-293 |
| PO-KANI-013 | `lower_canonical_choose` | `part_02.rs` | ~216-293 |
| PO-VERUS-001 | `lower_canonical_choose` (model) | `verus/choose_bool_invariant.rs` | N/A (spec artifact) |
| PO-FLUX-001 | `record_slot` (refinement) | `part_05.rs`, `flux/choose_slot_count.rs` | N/A |
| PO-FLUX-002 | `slot_from_text`, `record_slot` | `part_05.rs`, `flux/choose_slot_disjoint.rs` | N/A |
| PO-PROPTEST-001 | `choose_width` | `part_01.rs` | ~117-132 |
| PO-PROPTEST-002 | `lower_canonical_choose` | `part_02.rs` | ~216-293 |
| PO-PROPTEST-003 | `lower_canonical_choose` | `part_02.rs` | ~262-283 |
| PO-PROPTEST-004 | `compile_source` | `compile/mod.rs` | ~25-27 |
| PO-PROPTEST-005 | `choose_width`, `lower_canonical_choose` | `part_01.rs`, `part_02.rs` | ~117-132, ~216-293 |
| PO-FUZZ-001 | `slot_from_text`, `compile_source` | `part_05.rs`, `compile/mod.rs` | ~29, ~25-27 |
| PO-FUZZ-002 | `compile_source` | `compile/mod.rs` | ~25-27 |

## Default Command Evidence

### Kani
```bash
# All vb_compile kani harnesses
cargo kani -p vb_compile --harness <harness_name> --unwind <N>

# vb_core replay harnesses
cargo kani -p vb_core --harness <harness_name> --unwind <N>
```

### Verus
```bash
bash scripts/verify-verus.sh verification/verus/vb_compile/src/choose_bool_invariant.rs
```

### Flux
```bash
flux verification/flux/vb_compile/src/choose_slot_count.rs
flux verification/flux/vb_compile/src/choose_slot_disjoint.rs
```

### Proptest
```bash
cargo test -p vb_compile --test <test_name> -- --nocapture
```

### Fuzz
```bash
cargo fuzz run fuzz_choose_when_parse -- -max_len=256 -runs=100000
cargo fuzz run fuzz_choose_depth -- -max_len=65536 -runs=50000
```

## Implementation Constraints from Proof Plan

1. **`choose_width` MUST use `checked_add`** — not `+` or `+=`. PO-KANI-004 verifies overflow behavior.
2. **Body lowering MUST assign `next = common_next` for LAST body step** — not `next = next_body_step + 1`. PO-KANI-002 verifies fallthrough.
3. **Body lowering MUST NOT mutate branch order** — branches lowered in order; condition slots resolved in order. PO-KANI-007 verifies slot disjointness.
4. **`slot_from_text` MUST be called for EVERY `branch.when`** — no bypass, no direct string usage. PO-KANI-013 verifies no YAML in IR.
5. **`builder.record_slot` MUST be called for EVERY body `Set` output** — no manual slot assignment. PO-KANI-006 verifies slot uniqueness.
6. **Fanout check MUST remain at BOTH `lower_canonical_choose` and `lower_choose`** — defense in depth. PO-KANI-008 verifies both checkpoints.
7. **Empty-body case MUST be behaviorally identical** — regression tests must pass. PO-KANI-012 verifies width=1 for all-empty.
8. **No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`** — Holzman Rust rules. All error paths use `Result<_, CompileErrors>`.

## Risk-Register for Implementation

| Risk | Mitigation | Proof Gate |
|---|---|---|
| Forgetting to update `choose_width` → layout mismatch | `canonical_layout` calls `choose_width` which must return correct width | PO-KANI-012 |
| Body target = next instead of body_start for non-empty branch | Code review: check slot_branch.target assignment | PO-KANI-002 |
| Body nodes' `next` field set to adjacent branch start instead of common_next | Code review: check last body node's next assignment | PO-KANI-002 |
| Slot allocation skipped for body `Set` outputs | Code review: ensure `record_slot` called per body Set | PO-KANI-006 |
| YAML `when` string surviving into IR | Type system prevents this; verify no manual construction of SlotBranch with string | PO-KANI-013 |
| Regression: existing tests break | Run full test suite before claiming fix | AC10 (all existing tests pass) |
