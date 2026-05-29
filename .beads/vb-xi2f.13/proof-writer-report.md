# Proof Writer Report: vb-xi2f.13

**Bead:** vb-xi2f.13 — Nested choose lowering fix
**State:** 5 (proof-writer) → awaiting proof-reviewer
**Date:** 2026-05-29

## Summary

Wrote production bug fixes and verification artifacts for the planned nested-choose lowering obligations. Kani harnesses are split into source-length-compliant modules under `crates/vb_compile/src/mod_compile_lowering/kani/`, one Verus artifact verifies with the installed one-off `verus` command, and Flux artifacts exist but were not executed in this pass. Kani execution is blocked by pre-existing compilation errors in unrelated `vb_compile` harness files before choose-specific harnesses can run.

## Obligations Touched

| Obligation | Verifier | Artifact | Status |
|---|---|---|---|
| PO-KANI-001 | kani | `crates/vb_compile/src/mod_compile_lowering/kani/kani_choose_width.rs` | Written |
| PO-KANI-002 | kani | `crates/vb_compile/src/mod_compile_lowering/kani/kani_choose_body.rs` | Written |
| PO-KANI-003 | kani | `crates/vb_compile/src/mod_compile_lowering/kani/kani_choose_width.rs` | Written |
| PO-KANI-004 | kani | `crates/vb_compile/src/mod_compile_lowering/kani/kani_choose_width.rs` | Written |
| PO-KANI-005 | kani | `crates/vb_compile/src/mod_compile_lowering/kani/kani_choose_body.rs` | Written |
| PO-KANI-006 | kani | `crates/vb_compile/src/mod_compile_lowering/kani/kani_choose_slots.rs` | Written |
| PO-KANI-007 | kani | `crates/vb_compile/src/mod_compile_lowering/kani/kani_choose_slots.rs` | Written |
| PO-KANI-008 | kani | `crates/vb_compile/src/mod_compile_lowering/kani/kani_choose_width.rs` | Written |
| PO-KANI-009 | kani | (vb_core, not in scope for this bead's source edits) | Deferred to existing harnesses |
| PO-KANI-010 | kani | (vb_core, not in scope) | Deferred |
| PO-KANI-011 | kani | `crates/vb_compile/src/mod_compile_lowering/kani/kani_choose_slots.rs` | Written |
| PO-KANI-012 | kani | `crates/vb_compile/src/mod_compile_lowering/kani/kani_choose_body.rs` | Written |
| PO-KANI-013 | kani | `crates/vb_compile/src/mod_compile_lowering/kani/kani_choose_slots.rs` | Written |
| PO-VERUS-001 | verus | `verification/verus/vb_compile/src/choose_bool_invariant.rs` | Written; one-off Verus pass |
| PO-FLUX-001 | flux-rs | `verification/flux/vb_compile/src/choose_slot_count.rs` | Written |
| PO-FLUX-002 | flux-rs | `verification/flux/vb_compile/src/choose_slot_disjoint.rs` | Written |
| PO-PROPTEST-001 | proptest | Not written by proof-writer | Deferred to test-writer |
| PO-PROPTEST-002 | proptest | Not written | Deferred |
| PO-PROPTEST-003 | proptest | Not written | Deferred |
| PO-PROPTEST-004 | proptest | Not written | Deferred |
| PO-PROPTEST-005 | proptest | Not written | Deferred |
| PO-FUZZ-001 | cargo-fuzz | Not written by proof-writer | Deferred to fuzz writer |
| PO-FUZZ-002 | cargo-fuzz | Not written | Deferred |

## Production Code Fixes

### 1. `choose_width` (part_01.rs, lines 117-132)
**Bug:** Hardcoded to `Ok(1)`, ignoring branch body steps.
**Fix:** Computes `1 + sum(body_width(&branch.steps, 0))` using `checked_add`.
**Evidence:** `rtk cargo test -p vb_compile` passes with 662 tests passed and 5 ignored. `choose_width_counts_branch_body_steps` verifies width includes body nodes.

### 2. `lower_canonical_choose` (`part_14.rs`)
**Bug:** Rejected non-empty branch bodies with `UnsupportedStepPrimitive` error.
**Fix:** 
- Added cursor-based body step tracking
- Empty bodies: target = common_next (unchanged behavior)
- Non-empty bodies: target = first body step
- Emits the `ChooseSlot` before generated body nodes so `StepIdx` values match node table positions
- Body steps chain correctly: intermediate → next step, last → common_next

### 3. `emit_choose_branch_body` (`part_14.rs`, re-exported through `part_06.rs`)
**New function:** Emits Set and Do body steps for choose branches.
- Handles 0..N body steps per branch
- Correctly chains next pointers
- Returns node count for cursor advancement
- Supports only Set and Do primitives (canonical pathway)

## Commands Run

### Compilation & Tests (PASS)
```bash
rtk cargo fmt --check
bash scripts/check-source-length.sh
rtk cargo test -p vb_compile  # 662 passed, 5 ignored
moon ci                       # 32 completed (4 cached), 9m 57s 296ms
```
Moon CI raw output: `/home/lewis/.local/share/opencode/tool-output/tool_e74e68f1b001LCKcBMlrqNIFXT`.

### Kani Execution (BLOCKED)
```bash
cargo kani -p vb_compile --harness kani_choose_body_fallthrough --unwind 256
```
**Result:** blocked before the target harness by pre-existing `vb_compile` Kani compile errors in unrelated legacy harnesses (`WorkflowSourceParts` gating/private constructor issues and legacy `kani::Arbitrary` gaps).

### Verus (PASS)
```bash
verus --crate-type=lib verification/verus/vb_compile/src/choose_bool_invariant.rs
```
**Result:** `verification results:: 2 verified, 0 errors`.

### Flux (BLOCKED_TOOLING)
```bash
flux verification/flux/vb_compile/src/choose_slot_count.rs
```
**Result:** Not executed in this implementation pass. Files contain refinement contracts and runtime tests as fallback evidence.

## Blocker Evidence

| Blocker | Cause | Impact |
|---|---|---|
| BLOCKED_KANI_PREEXISTING | Pre-existing harness files reference removed `WorkflowSourceParts` type | All Kani executions for vb_compile fail. Not caused by this bead. |
| FLUX_NOT_EXECUTED | Flux lane not run in this pass | Refinement files written but not verifier-closed. |

## Trusted Base

See `trusted-base-ledger.jsonl` for the complete ledger. Key trust assumptions:
1. `SlotCompiler` internals assumed correct (slot monotonic tracking)
2. YAML parser produces valid AST
3. `vb_validate::shared::validate` correctly rejects invalid IR
4. `lower_choose` and `validate_branch_route` correctly handle SlotBranch entries
5. Replay engine (`replay_choose_slot`) dispatches to per-branch targets correctly

## Residual Obligations

1. **[DEFERRED]** PO-PROPTEST-001 through PO-PROPTEST-005: Proptest properties to be written by test-writer
2. **[DEFERRED]** PO-FUZZ-001, PO-FUZZ-002: Fuzz targets to be written by fuzz writer
3. **[DEFERRED]** PO-KANI-009, PO-KANI-010: vb_core replay harnesses exist but need verification re-run
4. **[PENDING]** All Kani harnesses need execution after pre-existing compilation issues are resolved
5. **[DONE]** Verus artifact verifies with `verus --crate-type=lib verification/verus/vb_compile/src/choose_bool_invariant.rs`
6. **[PENDING]** Flux refinements need flux execution
