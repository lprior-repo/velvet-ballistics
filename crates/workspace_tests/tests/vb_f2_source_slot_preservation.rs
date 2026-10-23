#![forbid(unsafe_code)]
//! F2 source-to-runtime regression oracles for vb-8eocj.1.
//!
//! Every workflow comes from compile_workflow and is executed unchanged.
//! Entry-only checks isolate clobbering from other incomplete body wiring;
//! terminal checks separately require the emitted workflow to finish.
//! Named/nested source coverage blocked by F1 is not replaced with manual IR.

use vb_compile::compile_workflow;
use vb_core::{
    CapabilitySet, CompiledNodeKind, CompiledWorkflow, ConstValue, RunFrame, RunId,
    SlotIdx, SlotValue, StepBudget, Taint, ValueStore, new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

const HEADER: &str = "version: velvet-ballistics/v1\nname: f2_preservation\nwhen:\n  manual: {}\nsteps:\n";
const SENTINELS: &str = "  - id: first\n    set:\n      output: answer\n      value: \"42\"\n  - id: second\n    set:\n      output: sentinel\n      value: \"99\"\n";
const TOGETHER: &str = "  - id: fanout\n    together:\n      branches:\n        - label: left\n          steps:\n            - id: left_value\n              set:\n                output: left_result\n                value: \"7\"\n        - label: right\n          steps:\n            - id: right_value\n              set:\n                output: right_result\n                value: \"11\"\n";
const REPEAT: &str = "  - id: bounded_repeat\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: body_value\n          set:\n            output: body_result\n            value: \"13\"\n";
const FOR_EACH: &str = "  - id: bounded_loop\n    for_each:\n      variable: item\n      input: \"8\"\n      at_once: 3\n      steps:\n        - id: item_value\n          set:\n            output: item_result\n            value: \"7\"\n";
const REDUCE: &str = "  - id: bounded_fold\n    reduce:\n      variable: acc\n      input: \"8\"\n      initial: \"10\"\n      steps:\n        - id: fold_value\n          set:\n            output: fold_result\n            value: \"7\"\n";

struct SourceRun {
    workflow: CompiledWorkflow,
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

    fn constant_output(&self, expected: i64) -> Result<SlotIdx, String> {
        self.workflow
            .to_parts()
            .nodes
            .iter()
            .find_map(|node| match &node.kind {
                CompiledNodeKind::SetConst { value }
                    if self.workflow.constant(*value) == Some(&ConstValue::I64(expected)) =>
                {
                    node.output
                }
                _ => None,
            })
            .ok_or_else(|| format!("missing source output for sentinel {expected}"))
    }

    fn assert_sentinels(&self) -> Result<(), String> {
        for expected in [42, 99] {
            let slot = self.constant_output(expected)?;
            let value = self.frame.read_slot(slot)
                .map_err(|error| format!("sentinel {expected} unreadable: {error:?}"))?;
            let taint = self.frame.read_taint(slot)
                .map_err(|error| format!("sentinel {expected} taint unreadable: {error:?}"))?;
            assert_eq!(*value, SlotValue::I64(expected), "source output {expected} was clobbered");
            assert_eq!(taint, Taint::Clean, "source output {expected} taint changed");
        }
        Ok(())
    }

    fn assert_body_counts(&mut self, expected: &[(i64, usize)]) {
        let events = self.evidence.drain();
        for (body_value, count) in expected {
            let actual = events.iter().filter(|event| match event {
                EvidenceEvent::StepSucceeded { step, .. } => matches!(
                    self.workflow.node(*step).map(|node| &node.kind),
                    Some(CompiledNodeKind::SetConst { value })
                        if self.workflow.constant(*value) == Some(&ConstValue::I64(*body_value))
                ),
                _ => false,
            }).count();
            assert_eq!(actual, *count, "source body value {body_value} execution count");
        }
    }
}

fn source(composites: &str, finish: &str) -> String {
    format!("{HEADER}{SENTINELS}{composites}  - id: done\n    finish:\n      result: {finish}\n")
}

fn assert_finished_preserving(
    composites: &str,
    finish: &str,
    expected: i64,
    body_counts: &[(i64, usize)],
) -> Result<(), String> {
    let mut run = SourceRun::new(&source(composites, finish))?;
    let original = run.workflow.to_parts();
    assert_eq!(run.drive(128)?, RuntimeSignal::Finished(SlotValue::I64(expected)));
    run.assert_sentinels()?;
    run.assert_body_counts(body_counts);
    assert_eq!(run.workflow.to_parts(), original, "compiled IR was modified during execution");
    Ok(())
}

#[test]
fn together_entry_preserves_prior_source_values() -> Result<(), String> {
    let mut run = SourceRun::new(&source(TOGETHER, "answer"))?;
    assert_eq!(run.drive(2)?, RuntimeSignal::StepBudgetExhausted);
    run.assert_sentinels()?;
    assert_eq!(run.drive(1)?, RuntimeSignal::StepBudgetExhausted);
    run.assert_sentinels()
}

#[test]
fn together_finishes_with_prior_answer_42() -> Result<(), String> {
    assert_finished_preserving(TOGETHER, "answer", 42, &[(7, 1), (11, 1)])
}

#[test]
fn together_finishes_with_prior_sentinel_99() -> Result<(), String> {
    assert_finished_preserving(TOGETHER, "sentinel", 99, &[(7, 1), (11, 1)])
}

#[test]
fn repeat_finishes_with_prior_answer_42() -> Result<(), String> {
    assert_finished_preserving(REPEAT, "answer", 42, &[(13, 3)])
}

#[test]
fn repeat_finishes_with_prior_sentinel_99() -> Result<(), String> {
    assert_finished_preserving(REPEAT, "sentinel", 99, &[(13, 3)])
}

#[test]
fn sequential_together_repeat_preserve_prior_source_values() -> Result<(), String> {
    assert_finished_preserving(&format!("{TOGETHER}{REPEAT}"), "answer", 42, &[(7, 1), (11, 1), (13, 3)])
}

#[test]
fn sequential_repeat_together_preserve_prior_source_values() -> Result<(), String> {
    assert_finished_preserving(&format!("{REPEAT}{TOGETHER}"), "sentinel", 99, &[(7, 1), (11, 1), (13, 3)])
}

// Numeric source operands are the existing authoring subset, not F1 named
// input binding or full admission coverage. Their values are initialized only
// after checking that emitted state cannot own the authored operand slot.
fn assert_role_isolation(run: &SourceRun, numeric_input: Option<SlotIdx>) -> Result<(), String> {
    let mut protected = vec![run.constant_output(42)?, run.constant_output(99)?];
    protected.extend(numeric_input);
    for node in &run.workflow.to_parts().nodes {
        let mut writes = Vec::new();
        let sentinel_definition = matches!(
            &node.kind,
            CompiledNodeKind::SetConst { value }
                if matches!(run.workflow.constant(*value), Some(ConstValue::I64(42 | 99)))
        );
        if !sentinel_definition {
            writes.extend(node.output);
        }
        writes.extend(node.error_slot);
        match &node.kind {
            CompiledNodeKind::ForEachStart { item_slot, .. } => {
                let state = node.output.ok_or("ForEachStart lacks iterator output")?;
                assert_ne!(state, *item_slot, "item aliases iterator state");
                writes.push(*item_slot);
            }
            CompiledNodeKind::ForEachNext { iterator_slot, .. } => writes.push(*iterator_slot),
            CompiledNodeKind::ReduceStart { input, accumulator, .. } => {
                let state = node.output.ok_or("ReduceStart lacks iterator output")?;
                assert_ne!(state, *input, "reduce item aliases iterator state");
                assert_ne!(state, *accumulator, "reduce accumulator aliases iterator state");
                assert_ne!(*input, *accumulator, "reduce item aliases accumulator");
                writes.extend([*input, *accumulator]);
            }
            CompiledNodeKind::ReduceNext { iterator_slot, .. } => writes.push(*iterator_slot),
            CompiledNodeKind::TogetherBranch { accumulator, .. }
            | CompiledNodeKind::TogetherJoin { accumulator, .. } => {
                if let Some(output) = node.output {
                    assert_ne!(output, *accumulator, "branch result aliases accumulator");
                }
                writes.push(*accumulator);
            }
            CompiledNodeKind::RepeatCheck { attempt_slot, .. } => writes.push(*attempt_slot),
            CompiledNodeKind::RepeatStart { .. } => {
                node.output.ok_or("RepeatStart lacks attempt state")?;
            }
            _ => {}
        }
        for slot in writes {
            assert!(!protected.contains(&slot), "generated {:?} overwrites protected {slot:?}", node.kind);
        }
    }
    Ok(())
}

fn assert_numeric_list_preserved(composite: &str, taint: Taint) -> Result<(), String> {
    let mut run = SourceRun::new(&source(composite, "answer"))?;
    let operand = SlotIdx::new(8);
    let parts = run.workflow.to_parts();
    assert!(parts.nodes.iter().any(|node| match &node.kind {
        CompiledNodeKind::ForEachStart { input, .. }
        | CompiledNodeKind::ReduceStart { input, .. } => *input == operand,
        CompiledNodeKind::Copy { source } => *source == operand,
        _ => false,
    }), "the authored numeric source operand was lost");
    assert_role_isolation(&run, Some(operand))?;
    let items = [SlotValue::I64(3), SlotValue::I64(5), SlotValue::I64(8)];
    let list = run.store.insert_list(Box::<[SlotValue]>::from(items))
        .map_err(|error| format!("input list creation failed: {error:?}"))?;
    run.frame.write_slot_with_taint(operand, SlotValue::List(list), taint)
        .map_err(|error| format!("numeric source initialization failed: {error:?}"))?;
    assert_eq!(run.drive(128)?, RuntimeSignal::Finished(SlotValue::I64(42)));
    run.assert_sentinels()?;
    run.assert_body_counts(&[(7, 3)]);
    assert_eq!(*run.frame.read_slot(operand).map_err(|e| format!("{e:?}"))?, SlotValue::List(list));
    assert_eq!(run.frame.read_taint(operand).map_err(|e| format!("{e:?}"))?, taint);
    assert_eq!(run.store.list(list).map_err(|e| format!("{e:?}"))?, &items);
    assert_eq!(run.workflow.to_parts(), parts, "numeric-source IR was replaced");
    Ok(())
}

#[test]
fn generated_composite_roles_do_not_alias_source_outputs() -> Result<(), String> {
    for (label, composite) in [("Together", TOGETHER), ("Repeat", REPEAT)] {
        let run = SourceRun::new(&source(composite, "answer"))?;
        assert_role_isolation(&run, None).map_err(|error| format!("{label}: {error}"))?;
    }
    Ok(())
}

#[test]
fn numeric_source_for_each_preserves_42_99_and_list_taint() -> Result<(), String> {
    for taint in [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
        assert_numeric_list_preserved(FOR_EACH, taint)?;
    }
    Ok(())
}

#[test]
fn numeric_source_reduce_preserves_42_99_and_list_taint() -> Result<(), String> {
    for taint in [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
        assert_numeric_list_preserved(REDUCE, taint)?;
    }
    Ok(())
}
