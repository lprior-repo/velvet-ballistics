# Proof Repair Guide: vb-qi37.14.1

## Bead
- **Bead ID**: vb-qi37.14.1
- **Title**: cli: Add single-step run command
- **Reviewer**: proof-reviewer
- **Date**: 2026-05-18

---

## Critical Directive

GOD RULE #2 MUST BE SATISFIED: "Verus proof fn and spec fn models MUST mathematically bind to the actual Rust implementations (exec fn) inside the production codebase."

The current Verus artifacts do NOT satisfy this rule. Repair must establish mechanical binding.

---

## BLOCKER-NEW-1: Verus Production Binding (CRITICAL)

### Problem
The Verus spec files (`verification/verus/step_state_machine.rs`, `signals_invariant.rs`, `run_frame_invariant.rs`) define shadow spec types (`SpecEngineSignal`, `SpecStepState`, `SpecTaint`) and prove properties about spec functions. They do NOT import or verify the actual production types/functions.

### Required Fix: Two Options

**Option A (Preferred): Use `#[extern_spec]` to bind production functions**

In `verification/verus/step_state_machine.rs`, add:

```verus
#[extern_spec]
impl crate::EngineSignal {
    pub open spec fn mark_step_after_signal_spec(&self) -> crate::StepState {
        match self {
            crate::EngineSignal::AwaitingWait => crate::StepState::Waiting,
            crate::EngineSignal::AwaitingAsk => crate::StepState::Asking,
            crate::EngineSignal::AwaitingAction | crate::EngineSignal::StepBudgetExhausted => crate::StepState::Running,
            crate::EngineSignal::Continue | crate::EngineSignal::Finished(_, _) => crate::StepState::Succeeded,
        }
    }
}

// Then prove equivalence:
pub proof fn proof_mark_step_after_signal_binding(signal: crate::EngineSignal)
    ensures
        crate::engine::step::mark_step_after_signal_spec(signal)
            == spec_mark_step_after_signal(signal.into_spec()),
{
    // match on signal variant, prove each arm matches
}
```

**Option B: Rewrite specs AS ghost functions inside production source**

In `crates/vb_core/src/engine/step.rs`, add:

```rust
verus! {

pub spec fn spec_mark_step_after_signal(signal: &EngineSignal) -> StepState {
    match signal {
        EngineSignal::AwaitingWait => StepState::Waiting,
        EngineSignal::AwaitingAsk => StepState::Asking,
        EngineSignal::AwaitingAction | EngineSignal::StepBudgetExhausted => StepState::Running,
        EngineSignal::Continue | EngineSignal::Finished(_, _) => StepState::Succeeded,
    }
}

proof fn proof_mark_step_after_signal_total(signal: EngineSignal)
    ensures
        spec_mark_step_after_signal(&signal).is_valid(),
{
    // exhaustiveness proof
}

} // verus!
```

Then verify in `crates/vb_core/src/engine/step.rs` using `cargo verus`.

**Same pattern for**:
- `step_once` PC bounds → add ghost spec to `step.rs` and verify with `#[extern_spec]`
- `write_slot_with_taint` taint validity → add ghost spec to `frame.rs` and verify with `#[extern_spec]`

### Verification Command
```bash
verus crates/vb_core/src/engine/step.rs
verus crates/vb_core/src/frame.rs
```

---

## BLOCKER-NEW-2: Vacuous Assertion in `step_once_slot_init_harness`

### Location
`crates/vb_core/src/kani_step_harnesses.rs:266-268`

### Problem
```rust
kani::assert(
    read_result.is_err() || read_result.is_ok(),  // ALWAYS TRUE
    "read_slot returns Result (not panic)",
);
```

### Required Fix
Replace with a meaningful invariant. The INV-003 claim is: "No slot is read that was not first written in the same step execution."

```rust
// After step_once, try to read each slot
// If step wrote to slot_idx, read should return Ok
// If step did NOT write to slot_idx, read should return Err(SlotUninitialized)
// This harness checks that read_slot does NOT panic either way

let slot_idx = SlotIdx::new(kani::any::<u16>() % slot_count.max(1));
let read_result = run.read_slot(slot_idx);

// INV-003: read_slot must return Err (SlotUninitialized) for slots
// that were not written by the step, NOT panic
if read_result.is_err() {
    // Expected: slot was not written, returned SlotUninitialized error
    kani::assert(true, "uninitialized slot returns error, not panic");
} else {
    // Slot was written by the step — also valid
    kani::assert(true, "initialized slot returns value");
}
```

Or stronger: add a cover statement to verify that BOTH paths are reachable:
```rust
kani::cover!(read_result.is_err(), "uninitialized slot path reachable");
kani::cover!(read_result.is_ok(), "initialized slot path reachable");
```

---

## BLOCKER-NEW-3: Tautological Assertion in `step_once_pc_bounds_harness`

### Location
`crates/vb_core/src/kani_step_harnesses.rs:328-331`

### Problem
```rust
kani::assert(
    pc_u16 >= 0,  // u16 is always >= 0 — TAUTOLOGY
    "PC >= 0 after step_once",
);
```

### Required Fix
Delete the tautological assertion. The meaningful bound is `pc_u16 < step_count`:

```rust
// INV-004: PC ∈ [0, step_count) after step_once
let pc = run.pc();
let pc_usize = pc.as_usize();
kani::assert(
    pc_usize < step_count as usize,
    "PC < step_count after step_once",
);
// Note: pc_u16 >= 0 is guaranteed by type (u16), no assertion needed
```

---

## BLOCKER-NEW-4: Kani Timeout — No Verification Results

### Problem
`cargo kani --harness step_once_bounds_harness --package vb_core` times out at 300s. All 6 harnesses likely have the same issue.

### Required Fix: Simplify harnesses

The timeout is caused by too many symbolic variables (`WorkflowParts` is extremely large). Reduce complexity:

**Option A: Reduce symbolic abstraction**
```rust
// Instead of arbitrary WorkflowParts, construct a minimal valid workflow
let nodes = vec![CompiledNode::nop()];  // minimal 1-node workflow
let workflow = CompiledWorkflow::new_simple("test", nodes, 0, 1)?;
// Now step_once is tractable to verify
```

**Option B: Increase unwind bound per-harness**
```rust
#[kani::unwind(10)]  // increase from 6
fn step_once_bounds_harness() { ... }
```

**Option C: Add --max-slice-size to limit symbolic exploration**
```bash
cargo kani --harness step_once_bounds_harness --package vb_core -- --max-slice-size 4
```

**Option D: Run with very long timeout and capture output**
```bash
timeout 3600 cargo kani --harness step_once_bounds_harness --package vb_core 2>&1 | tee kani-results/step_once_bounds.log
```

### Minimum Acceptable Evidence
For each harness: `Kani: 0 failures, N checks verified` — not just "artifact exists."

---

## Summary of Required Changes

| File | Issue | Fix |
|-------|-------|-----|
| `verification/verus/step_state_machine.rs` | No production binding | Add `#[extern_spec]` for `mark_step_after_signal` or rewrite ghost fns in `step.rs` |
| `verification/verus/signals_invariant.rs` | No production binding | Add `#[extern_spec]` for `step_once` PC bounds |
| `verification/verus/run_frame_invariant.rs` | No production binding | Add `#[extern_spec]` for `write_slot_with_taint` |
| `crates/vb_core/src/kani_step_harnesses.rs:266-268` | Vacuous `is_err \|\| is_ok` | Replace with cover statements for both init/uninit paths |
| `crates/vb_core/src/kani_step_harnesses.rs:328-331` | Tautological `pc_u16 >= 0` | Delete the tautology; keep `pc_u16 < step_count` |
| All 6 harnesses | Timeout | Simplify workflow construction or increase timeout |

---

## Re-run Targets

After repair, re-run these commands and include raw output in proof-evidence.md:

```bash
# Verus (must show 0 errors)
verus verification/verus/step_state_machine.rs
verus verification/verus/signals_invariant.rs
verus verification/verus/run_frame_invariant.rs

# Kani (must show 0 failures)
cargo kani --harness step_once_bounds_harness --package vb_core
cargo kani --harness step_once_state_mapping_harness --package vb_core
cargo kani --harness step_once_slot_init_harness --package vb_core
cargo kani --harness step_once_pc_bounds_harness --package vb_core
cargo kani --harness taint_validity_harness --package vb_core
cargo kani --harness step_once_error_harness --package vb_core
```
