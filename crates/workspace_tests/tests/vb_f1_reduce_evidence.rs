#![forbid(unsafe_code)]
//! F1 reduce evidence oracles for vb-8eocj.3.
//!
//! Strong actual EvidenceEvent::SlotWritten assertions for numeric input slot 8
//! with documented fixture, across empty/single/two-element lists.
//! Clean taint only; frozen vb_f2 covers the three taint levels.

use vb_compile::compile_workflow;
use vb_core::{
    CapabilitySet, CompiledNodeKind, ConstValue, RunFrame, RunId, SlotIdx, SlotValue,
    StepBudget, Taint, ValueStore, new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

const HEADER: &str = "version: velvet-ballistics/v1\nname: f1_reduce_evidence\nwhen:\n  manual: {}\nsteps:\n";
const SENTINELS: &str = "  - id: first\n    set:\n      output: answer\n      value: \"42\"\n  - id: second\n    set:\n      output: sentinel\n      value: \"99\"\n";
const REDUCE: &str = "  - id: bounded_fold\n    reduce:\n      variable: acc\n      input: \"8\"\n      initial: \"10\"\n      steps:\n        - id: fold_value\n          set:\n            output: fold_result\n            value: \"7\"\n";

fn source(composites: &str) -> String {
    format!("{HEADER}{SENTINELS}{composites}  - id: done\n    finish:\n      result: answer\n")
}

struct SourceRun {
    workflow: vb_core::CompiledWorkflow,
    frame: RunFrame,
    store: ValueStore,
    evidence: EvidenceCollector,
    collect_states: CollectStates,
}

impl SourceRun {
    fn new(source: &str) -> Result<Self, String> {
        let workflow = compile_workflow(source.as_bytes())
            .map_err(|error| format!("canonical source compilation failed: {error:?}"))?;
        let frame = new_run_frame(RunId::new(1), &workflow)
            .map_err(|error| format!("frame creation failed: {error:?}"))?;
        Ok(Self {
            workflow,
            frame,
            store: ValueStore::new(),
            evidence: EvidenceCollector::new(),
            collect_states: CollectStates::new(),
        })
    }

    fn drive(&mut self, steps: u64) -> Result<RuntimeSignal, String> {
        drive_deterministic_full(
            &self.workflow,
            &mut self.frame,
            &mut StepBudget::new(steps),
            &mut self.store,
            &[],
            RetryPolicy::NEVER,
            &mut self.evidence,
            &mut self.collect_states,
            &CapabilitySet::empty(),
            1,
        )
        .map_err(|error| format!("unchanged source IR execution failed: {error:?}"))
    }
}

/// Find the ReduceStart node and extract its slots.
fn find_reduce_start(workflow: &vb_core::CompiledWorkflow) -> Option<(SlotIdx, SlotIdx, Option<SlotIdx>)> {
    let parts = workflow.to_parts();
    parts.nodes.iter().find_map(|node| match &node.kind {
        CompiledNodeKind::ReduceStart { input, accumulator, .. } => {
            Some((*input, *accumulator, node.output))
        }
        _ => None,
    })
}

/// Find the ReduceNext node and extract its slots.
fn find_reduce_next(workflow: &vb_core::CompiledWorkflow) -> Option<(SlotIdx, SlotIdx, Option<SlotIdx>)> {
    let parts = workflow.to_parts();
    parts.nodes.iter().find_map(|node| match &node.kind {
        CompiledNodeKind::ReduceNext { iterator_slot, accumulator, .. } => {
            Some((*iterator_slot, *accumulator, node.output))
        }
        _ => None,
    })
}

/// Find the body step's output slot (SetConst with value 7).
fn find_body_output(workflow: &vb_core::CompiledWorkflow) -> Option<SlotIdx> {
    let parts = workflow.to_parts();
    parts.nodes.iter().find_map(|node| match &node.kind {
        CompiledNodeKind::SetConst { value } => {
            if workflow.constant(*value) == Some(&ConstValue::I64(7)) {
                node.output
            } else {
                None
            }
        }
        _ => None,
    })
}

/// Count SlotWritten events for a specific slot.
fn count_slot_writes(events: &[EvidenceEvent], slot: SlotIdx) -> usize {
    events.iter().filter(|event| match event {
        EvidenceEvent::SlotWritten { slot: s, .. } => *s == slot,
        _ => false,
    }).count()
}

fn assert_reduce_evidence(n: usize, items: &[SlotValue]) -> Result<(), String> {
    let mut run = SourceRun::new(&source(REDUCE))?;
    let original_parts = run.workflow.to_parts();

    // Derive all slots from the compiled workflow, never invent them.
    let (input_slot, accum_slot, iterator_slot_opt) = find_reduce_start(&run.workflow)
        .ok_or("missing compiled ReduceStart")?;
    let (next_iterator_slot, next_accum_slot, next_item_slot_opt) = find_reduce_next(&run.workflow)
        .ok_or("missing compiled ReduceNext")?;
    let next_item_slot = next_item_slot_opt.ok_or("ReduceNext lacks private item output")?;
    let iterator_slot = iterator_slot_opt.ok_or("ReduceStart lacks private iterator output")?;

    // Verify slot consistency between Start and Next.
    assert_eq!(iterator_slot, next_iterator_slot, "iterator slot mismatch");
    assert_eq!(accum_slot, next_accum_slot, "accumulator slot mismatch");

    // The body writes to a private slot via SetConst 7.
    let body_output_slot = find_body_output(&run.workflow)
        .ok_or("missing body SetConst 7 output")?;
    let body_output_slot = body_output_slot.ok_or("body step has no output slot")?;

    // Seed the input list into slot 8.
    let list = run.store.insert_list(items.to_vec().into_boxed_slice())
        .map_err(|e| format!("input list creation failed: {e:?}"))?;
    run.frame.write_slot_with_taint(input_slot, SlotValue::List(list), Taint::Clean)
        .map_err(|e| format!("input slot write failed: {e:?}"))?;

    // Drive the workflow with a generous step budget.
    let signal = run.drive(128)?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(42)), "workflow did not finish at 42");

    // Drain actual evidence events for per-step write assertions.
    let events = run.evidence.drain();

    // Body execution count must equal the number of items.
    let body_write_count = count_slot_writes(&events, body_output_slot);
    assert_eq!(body_write_count, n, "body execution count mismatch");

    // Iterator write count: Start writes 1 (except empty), each continuing Next writes 1.
    // Empty list: 0 writes. Single item: 1 (Start). Two items: 2 (Start + 1 Next).
    let iterator_write_count = count_slot_writes(&events, iterator_slot);
    assert_eq!(iterator_write_count, n, "iterator write count mismatch");

    // Next item write count: only continuing Next steps write the item slot.
    // Empty: 0. Single: 0. Two: 1. Three: 2. General: n - 1.
    let next_item_write_count = count_slot_writes(&events, next_item_slot);
    assert_eq!(next_item_write_count, n.saturating_sub(1), "Next item write count mismatch");

    // Verify sentinels are preserved after execution.
    for expected in [42, 99] {
        let sentinel_slot = run.workflow.to_parts().nodes.iter().find_map(|node| match &node.kind {
            CompiledNodeKind::SetConst { value } => {
                if run.workflow.constant(*value) == Some(&ConstValue::I64(expected)) {
                    node.output
                } else {
                    None
                }
            }
            _ => None,
        });
        if let Some(Some(slot)) = sentinel_slot {
            let value = run.frame.read_slot(slot)
                .map_err(|e| format!("sentinel {expected} read failed: {e:?}"))?;
            assert_eq!(*value, SlotValue::I64(expected), "sentinel {expected} was clobbered");
            let taint = run.frame.read_taint(slot)
                .map_err(|e| format!("sentinel {expected} taint read failed: {e:?}"))?;
            assert_eq!(taint, Taint::Clean, "sentinel {expected} taint changed");
        }
    }

    // Verify input list is unchanged.
    let input_value = run.frame.read_slot(input_slot)
        .map_err(|e| format!("input slot read failed: {e:?}"))?;
    assert_eq!(*input_value, SlotValue::List(list), "input list was modified");
    let input_taint = run.frame.read_taint(input_slot)
        .map_err(|e| format!("input taint read failed: {e:?}"))?;
    assert_eq!(input_taint, Taint::Clean, "input taint changed");

    // Verify compiled IR was not mutated during execution.
    assert_eq!(run.workflow.to_parts(), original_parts, "compiled IR was modified");

    Ok(())
}

#[test]
fn reduce_empty_list_evidence() -> Result<(), String> {
    assert_reduce_evidence(0, &[])
}

#[test]
fn reduce_single_item_evidence() -> Result<(), String> {
    assert_reduce_evidence(1, &[SlotValue::I64(3)])
}

#[test]
fn reduce_two_items_evidence() -> Result<(), String> {
    assert_reduce_evidence(2, &[SlotValue::I64(3), SlotValue::I64(5)])
}
