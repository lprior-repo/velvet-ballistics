P1-12r simulate-structured: Add kind (StepKind enum) field to SimulationStep; rename kind_label to kind_label_text (3-field baseline, not 4)

# Verification excerpts (read-before-write)

## crates/vb_cli/src/commands_workflow/mod.rs (143 lines)
- Line 17-21: `pub(crate) struct SimulationStep { pub index: usize, pub kind_label: String, pub description: String }` — EXACTLY 3 FIELDS, not 4.
- Line 23-28: `pub(crate) struct SimulationResult { pub steps: Vec<SimulationStep>, pub total_steps: usize, pub action_count: usize, pub branch_count: usize }`.
- Line 30-60: `pub(crate) fn simulate_workflow(workflow: &CompiledWorkflow) -> SimulationResult` — iterates `workflow.node_count()`, calls `node_kind_label(&node.kind)` (line 43), `describe_node_for_simulate(&node.kind, ...)` (line 44-45), pushes a `SimulationStep` (line 47-51).
- The `node_kind_label` function is in `dot.rs` (line 11 re-export).

## Master doc §75 (lines 4133-4170)
- Master doc shows simulate output as `events: [{seq, kind, step, action, source, slot, value_summary, ...}, ...]` — a list of event records.
- Master does NOT define a `SimulationStep` struct by that name; it shows the wire-format output (events array).
- The `SimulationStep` struct in the codebase is a LOCAL Rust struct, not a master doc spec.

# Round-2 corrections applied (from black-hat review)

The round-2 bead claimed the baseline had 4 fields — it has 3. The round-2 bead proposed adding 4 new fields for a total of 8 — this is wrong.

The new spec:
- Baseline: 3 fields (`index, kind_label, description`).
- Add: 1 NEW field (`kind: StepKind` enum).
- Resolve conflict: rename `kind_label: String` to `kind_label_text: String` to free up the `kind` name.

Total: 4 fields (`index, kind_label_text, kind, description`).

# Scope (verified, no fabrication)

Modify `SimulationStep` in `commands_workflow/mod.rs:17-21`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepKind {
    Nop,
    SetConst,
    Copy,
    EvalExpr,
    BuildObject,
    BuildList,
    Do,
    Choose,
    ForEach,
    Together,
    Collect,
    Reduce,
    Repeat,
    WaitUntil,
    WaitEvent,
    Ask,
    RetryCheck,
    ErrorHandler,
    Finish,
    // ... add as needed
}

pub(crate) struct SimulationStep {
    pub index: usize,
    pub kind_label_text: String,    // RENAMED from kind_label
    pub kind: StepKind,              // NEW
    pub description: String,
}
```

Populate `kind` in `simulate_workflow` (line 30-60) by adding a helper `node_kind_to_step_kind(&node.kind) -> StepKind` that maps `CompiledNodeKind` to `StepKind`. `kind_label_text` continues to come from `node_kind_label(&node.kind).to_string()`.

# Acceptance test

```rust
#[test]
fn simulate_do_step_has_kind_do() {
    // Build a workflow with a single Do { action: ActionId::new(7) } step.
    // Call simulate_workflow.
    // Assert steps[0].kind == StepKind::Do.
    // Assert steps[0].kind_label_text == "Do".
    // Assert steps[0].description contains "Do action 7".
}

#[test]
fn simulate_set_step_has_kind_setconst() {
    // Build a workflow with SetConst { value: 0, output: 0 }.
    // Assert steps[0].kind == StepKind::SetConst.
    // Assert steps[0].description == "Set constant value".
}
```

# Anti-hallucination guards

- DO NOT claim 4 baseline fields — there are 3.
- DO NOT add `action_id`, `mock_output`, or `suspension_reason` to SimulationStep — these are not requested. The 3-round spec is to add ONLY `kind: StepKind` and rename `kind_label` to `kind_label_text`.
- DO NOT claim master §75 specifies SimulationStep fields — master §75 specifies the wire-format `events` output, which is a different concern.

# Kani harness (skipped — simulate is a dry-run cold path; no arithmetic)

Coverage comes from unit tests. No Kani needed.

# Dependency

This bead has NO dependencies. (The round-2 bead was P1 with no deps; we preserve that.)
