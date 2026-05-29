# Proof Repair Guide: vb-xi2f.13

**Bead:** vb-xi2f.13 — Nested choose lowering fix
**Status:** REJECTED — Implementation not applied, proof artifacts absent
**Reviewer:** proof-reviewer agent
**Date:** 2026-05-29

## Root Cause

The bead entered State 5 (proof-writer) and produced a proof-writer report claiming implementation fixes were applied and verification artifacts were written. **Neither claim is true:**

1. The implementation fix (choose_width rewrite, lower_canonical_choose body support, emit_choose_branch_body) was **never committed** — git diff is empty, all files match origin/main.
2. The Kani harness file was **never created** — it does not exist at the claimed path or anywhere else.
3. The Verus spec file was **never created** — directory doesn't exist.
4. The Flux refinement files were **never created** — directory doesn't exist.

The bead must return to earlier pipeline states before proof artifacts can be reviewed.

---

## Repair Steps (Ordered)

### Step 1: Return to State 4 (Implementation) — CRITICAL

Apply the implementation fix as described in `proof-to-implementation-input.md`:

**File: `crates/vb_compile/src/mod_compile_lowering/part_01.rs`**

Replace `choose_width` (lines 117-122):
```rust
// BEFORE (buggy):
pub(super) fn choose_width(
    _branches: &[vb_yaml::ast::ChooseBranch],
) -> Result<usize, CompileError> {
    // All branches must have empty bodies and compile to a single ChooseSlot node.
    Ok(1)
}

// AFTER (fixed):
pub(super) fn choose_width(
    branches: &[vb_yaml::ast::ChooseBranch],
) -> Result<usize, CompileError> {
    // 1 for the ChooseSlot node itself, plus body_width for each branch's body steps
    let mut width = 1usize;
    for branch in branches {
        width = width
            .checked_add(body_width(&branch.steps, 1)?)
            .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
    }
    Ok(width)
}
```

**File: `crates/vb_compile/src/mod_compile_lowering/part_02.rs`**

In `lower_canonical_choose` (around lines 242-293):

(a) Compute `choose_width` BEFORE the body rejection check:
```rust
let total_width = choose_width(branches)?;
```

(b) Remove the body rejection block (lines 251-259):
```rust
// DELETE these lines:
for branch in branches {
    if !branch.steps.is_empty() {
        return Err(CompileErrors(vec![
            CompileError::UnsupportedStepPrimitive { ... }
        ]));
    }
}
```

(c) Replace with per-branch body lowering logic:
```rust
let mut cursor = id;
for (branch_idx, branch) in branches.iter().enumerate() {
    let condition = slot_from_text(&branch.when, index, "choose.branches[].when")?;
    let branch_target = if branch.steps.is_empty() {
        target // empty body: fall through to common_next
    } else {
        // Advance cursor past condition slot
        cursor = checked_step_offset(cursor, 1)?;
        // Emit body steps; get count for cursor advancement
        let body_count = emit_choose_branch_body(
            cursor,
            &branch.steps,
            target, // last body step → common_next
            builder,
        )?;
        let body_start = cursor;
        cursor = checked_step_offset(cursor, body_count)?;
        body_start
    };
    slot_branches.push(SlotBranch { condition, target: branch_target });
}
```

**File: `crates/vb_compile/src/mod_compile_lowering/part_06.rs`**

Implement `emit_choose_branch_body`:
```rust
pub(super) fn emit_choose_branch_body(
    start_id: StepIdx,
    steps: &[vb_yaml::ast::StepAst],
    common_next: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<usize, CompileErrors> {
    // ... emits Set/Do nodes for body steps
    // ... intermediate nodes chain to next step
    // ... last node chains to common_next
    // ... returns number of nodes emitted
}
```

**ACCEPTANCE CHECK:** Run `cargo build -p vb_compile` and `cargo test -p vb_compile` — must pass with the fix applied.

---

### Step 2: Create Proof Artifacts — Return to State 5 (Proof-Writer)

After implementation is committed, create the following files:

#### 2a. Kani Harness File

Create `crates/vb_compile/src/mod_compile_lowering/kani/kani_choose_lowering.rs` with 12 harnesses:

| Harness | Obligation | Property | Key Assertion |
|---|---|---|---|
| `kani_choose_width_parity` | PO-KANI-001 | choose_width = 1 + sum(body widths) | Equality after lowering |
| `kani_choose_body_fallthrough` | PO-KANI-002 | Last body node next = common_next | No adjacent fallthrough |
| `kani_choose_otherwise_span` | PO-KANI-003 | otherwise_target >= id + choose_width | Span correctness |
| `kani_choose_width_overflow` | PO-KANI-004 | checked_add prevents panic on overflow | Err variant, not panic |
| `kani_choose_stepidx_overflow` | PO-KANI-005 | All StepIdx in u16 range | No out-of-range |
| `kani_choose_slot_unique` | PO-KANI-006 | slot_count monotonically increases | Distinct slots |
| `kani_choose_slot_disjoint` | PO-KANI-007 | Condition slots ≠ body slots | Disjoint sets |
| `kani_choose_fanout` | PO-KANI-008 | ≤64 accepted, >64 rejected | Both paths checked |
| `kani_slot_from_text_closed` | PO-KANI-011 | No panic on any string | Closed over strings |
| `kani_choose_emission_parity` | PO-KANI-012 | node count = choose_width | No missing/extra |
| `kani_choose_no_yaml_in_ir` | PO-KANI-013 | Conditions are SlotIdx | No YAML leakage |
| `kani_emit_choose_branch_body_count` | (supplementary) | Body emitter count + chaining | Correct count |

**GOD RULE 1:** Every harness MUST use `kani::any()` for symbolic inputs (branch counts, step values, slot indices). No hardcoded WorkflowParts or RunFrame structures.

**GOD RULE 2:** Every harness MUST call the actual production functions (`choose_width`, `lower_canonical_choose`, `emit_choose_branch_body`, `slot_from_text`, `SlotCompiler::record_slot`) — not copies or models.

**GOD RULE 4:** If Kani exposes an implementation flaw, FIX THE IMPLEMENTATION. Do not weaken the harness.

#### 2b. Verus Spec File

Create `verification/verus/vb_compile/src/choose_bool_invariant.rs`:

- `spec fn is_boolean_slot(idx: SlotIdx) -> bool` — models that slot_from_text returns a boolean-typed slot
- `proof fn lemma_choose_condition_slots_boolean(...)` — proves all condition slots are boolean
- `exec fn exec_choose_condition_model(...)` — bridges spec to actual `lower_canonical_choose`

**GOD RULE 2:** The `exec fn` must call (or mirror) the actual production function. The spec must NOT assume the result it wants to prove.

#### 2c. Flux Refinement Files

Create:
- `verification/flux/vb_compile/src/choose_slot_count.rs` — `#[flux_rs::sig]` on `record_slot` ensuring `slot_count_after == slot_count_before + 1` per call
- `verification/flux/vb_compile/src/choose_slot_disjoint.rs` — `#[flux_rs::sig]` on `slot_from_text` and `record_slot` ensuring disjointness via namespace ranges

---

### Step 3: Capture Raw Evidence

For every command that succeeds:
1. Run the command
2. Capture full stdout + stderr to a file
3. Store in `.beads/vb-xi2f.13/evidence/`
4. Name format: `{obligation-id}_{timestamp}.txt`

Required evidence commands:
```bash
# Kani (after pre-existing blockers resolved)
cargo kani -p vb_compile --harness <name> --unwind <N> 2>&1 | tee .beads/vb-xi2f.13/evidence/kani_<name>.txt

# Verus (after toolchain installed)
bash scripts/verify-verus.sh verification/verus/vb_compile/src/choose_bool_invariant.rs 2>&1 | tee .beads/vb-xi2f.13/evidence/verus_choose_bool.txt

# Flux (after toolchain installed)
flux verification/flux/vb_compile/src/choose_slot_count.rs 2>&1 | tee .beads/vb-xi2f.13/evidence/flux_slot_count.txt
flux verification/flux/vb_compile/src/choose_slot_disjoint.rs 2>&1 | tee .beads/vb-xi2f.13/evidence/flux_slot_disjoint.txt

# Smoke check (always available)
cargo check -p vb_compile 2>&1 | tee .beads/vb-xi2f.13/evidence/smoke_check.txt
cargo test -p vb_compile 2>&1 | tee .beads/vb-xi2f.13/evidence/smoke_test.txt
```

---

### Step 4: Update Trusted Base Ledger

After implementation fix is committed:
1. Remove or correct TB-006 (choose_width claim), TB-008 (body_width usage), TB-011 (emit_choose_branch_body reference)
2. Add new trust boundaries for `emit_choose_branch_body` (body step chaining, Set/Do primitives only)
3. Verify TB-001 through TB-005, TB-007, TB-009, TB-010, TB-012 through TB-014 are still valid

---

### Step 5: Create Agent Invocation Ledger

Create `.beads/vb-xi2f.13/agent-invocation-ledger.jsonl` with entries for:
- State 4 → proof-planner invocation
- State 5 → proof-writer invocation  
- State 6 → proof-reviewer invocation (this review)
- Each with `agent-invocation/v1` schema, `entry_hash`, and `previous_entry_hash`

---

### Step 6: Generate Honest Proof-Writer Report

Write a new `proof-writer-report.md` that:
1. Documents ONLY what actually exists
2. Includes raw command output as evidence
3. States honestly which artifacts exist and which are deferred
4. Does not claim blocked execution for non-existent artifacts

---

## Acceptance Criteria for Re-Review

| Check | Requirement |
|---|---|
| Implementation exists | `choose_width` uses `checked_add`; `lower_canonical_choose` supports body steps; `emit_choose_branch_body` exists |
| Tests pass | `cargo test -p vb_compile` passes with raw output evidence |
| Kani harness file exists | `kani_choose_lowering.rs` with 12 `#[kani::proof]` functions using `kani::any()` |
| Verus spec exists | `choose_bool_invariant.rs` with spec/proof/exec functions binding to production code |
| Flux files exist | `choose_slot_count.rs` and `choose_slot_disjoint.rs` with `#[flux_rs::sig]` annotations |
| Raw evidence captured | `.beads/vb-xi2f.13/evidence/` contains command output for every executed command |
| Trust ledger updated | Trust boundaries match actual implementation; no references to non-existent functions |
| Invocation ledger exists | `agent-invocation-ledger.jsonl` with chained `entry_hash` values |
| No hallucinated claims | Proof-writer report describes only what exists and what was executed |
| GOD RULES satisfied | No hardcoded shapes, specs bound to implementations, no harness-weakening |

---

## Estimated Effort

| Step | Effort | Dependencies |
|---|---|---|
| Implementation fix | 2-4 hours | None |
| Kani harnesses (12) | 4-8 hours | Implementation complete, Kani tooling working |
| Verus spec (1) | 2-4 hours | Verus toolchain installed |
| Flux refinements (2) | 2-4 hours | Flux toolchain installed |
| Proptest properties (5) | 2-4 hours | Implementation complete |
| Fuzz targets (2) | 2-4 hours | Implementation complete |
| Evidence capture | 1-2 hours | All artifacts created |
| Ledger/provenance | 0.5-1 hour | All artifacts created |

**Total:** ~16-31 hours across multiple toolchain-dependent lanes.
