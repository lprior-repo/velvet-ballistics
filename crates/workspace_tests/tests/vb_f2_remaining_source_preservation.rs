#![forbid(unsafe_code)]
//! F2 source-to-runtime regression oracles for vb-8eocj.1.
//!
//! Tests a numeric-source subset of the workflow language: choose with
//! slot-index conditions, collect with slot-index sources, and ask with
//! host-driven resume. Each test compiles canonical source, executes the
//! unchanged IR, and verifies slot preservation, body execution counts,
//! and IR immutability.

use vb_compile::compile_workflow;
use vb_core::{
    CapabilitySet, CompiledNodeKind, ConstValue, RunId, SlotIdx, SlotValue, StepBudget, Taint,
    ValueStore, new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

const HEADER: &str =
    "version: velvet-ballistics/v1\nname: f2_preservation\nwhen:\n  manual: {}\nsteps:\n";
const SENTINELS: &str = "  - id: first\n    set:\n      output: answer\n      value: \"42\"\n  - id: second\n    set:\n      output: sentinel\n      value: \"99\"\n";

struct SourceRun {
    workflow: vb_core::CompiledWorkflow,
    frame: vb_core::RunFrame,
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
            let value = self
                .frame
                .read_slot(slot)
                .map_err(|error| format!("sentinel {expected} unreadable: {error:?}"))?;
            let taint = self
                .frame
                .read_taint(slot)
                .map_err(|error| format!("sentinel {expected} taint unreadable: {error:?}"))?;
            assert_eq!(
                *value,
                SlotValue::I64(expected),
                "source output {expected} was clobbered"
            );
            assert_eq!(
                taint,
                Taint::Clean,
                "source output {expected} taint changed"
            );
        }
        Ok(())
    }

    fn assert_body_counts(&mut self, expected: &[(i64, usize)]) {
        let events = self.evidence.drain();
        for (body_value, count) in expected {
            let actual = events
                .iter()
                .filter(|event| match event {
                    EvidenceEvent::StepSucceeded { step, .. } => matches!(
                        self.workflow.node(*step).map(|node| &node.kind),
                        Some(CompiledNodeKind::SetConst { value })
                            if self.workflow.constant(*value) == Some(&ConstValue::I64(*body_value))
                    ),
                    _ => false,
                })
                .count();
            assert_eq!(
                actual, *count,
                "source body value {body_value} execution count"
            );
        }
    }
}

fn source(composites: &str, finish: &str) -> String {
    format!("{HEADER}{SENTINELS}{composites}  - id: done\n    finish:\n      result: {finish}\n")
}

#[test]
fn choose_true_branch_executes() -> Result<(), String> {
    let composites = "  - id: branch\n    choose:\n      branches:\n        - when: \"8\"\n          steps:\n            - id: body\n              set:\n                output: branch_result\n                value: \"7\"\n      otherwise: done\n";
    let mut run = SourceRun::new(&source(composites, "answer"))?;
    let original = run.workflow.to_parts();
    let condition_slot = run
        .workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| match &node.kind {
            CompiledNodeKind::ChooseSlot { branches, .. }
                if branches.first().map(|b| b.condition) == Some(SlotIdx::new(8)) =>
            {
                branches.first().map(|b| b.condition)
            }
            _ => None,
        })
        .ok_or("missing choose condition slot")?;
    run.frame
        .write_slot(condition_slot, SlotValue::Bool(true))
        .map_err(|error| format!("condition write failed: {error:?}"))?;
    assert_eq!(run.drive(128)?, RuntimeSignal::Finished(SlotValue::I64(42)));
    run.assert_sentinels()?;
    run.assert_body_counts(&[(7, 1)]);
    assert_eq!(
        run.workflow.to_parts(),
        original,
        "compiled IR was modified during execution"
    );
    Ok(())
}

#[test]
fn choose_false_branch_executes() -> Result<(), String> {
    let composites = "  - id: branch\n    choose:\n      branches:\n        - when: \"8\"\n          steps:\n            - id: body\n              set:\n                output: branch_result\n                value: \"7\"\n      otherwise: done\n";
    let mut run = SourceRun::new(&source(composites, "answer"))?;
    let original = run.workflow.to_parts();
    let condition_slot = run
        .workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| match &node.kind {
            CompiledNodeKind::ChooseSlot { branches, .. }
                if branches.first().map(|b| b.condition) == Some(SlotIdx::new(8)) =>
            {
                branches.first().map(|b| b.condition)
            }
            _ => None,
        })
        .ok_or("missing choose condition slot")?;
    run.frame
        .write_slot(condition_slot, SlotValue::Bool(false))
        .map_err(|error| format!("condition write failed: {error:?}"))?;
    assert_eq!(run.drive(128)?, RuntimeSignal::Finished(SlotValue::I64(42)));
    run.assert_sentinels()?;
    run.assert_body_counts(&[(7, 0)]);
    assert_eq!(
        run.workflow.to_parts(),
        original,
        "compiled IR was modified during execution"
    );
    Ok(())
}

#[test]
fn collect_empty_list() -> Result<(), String> {
    let composites = "  - id: gather\n    collect:\n      source: \"8\"\n      variable: page\n      pages: 3\n      items: 5\n      steps:\n        - id: body\n          set:\n            output: collect_result\n            value: \"7\"\n";
    let mut run = SourceRun::new(&source(composites, "answer"))?;
    let original = run.workflow.to_parts();
    let input_slot = run
        .workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| match &node.kind {
            CompiledNodeKind::CollectStart { source, .. } if *source == SlotIdx::new(8) => {
                Some(*source)
            }
            _ => None,
        })
        .ok_or("missing collect input slot")?;
    let list = run
        .store
        .insert_list(Box::<[SlotValue]>::from([]))
        .map_err(|e| format!("{e:?}"))?;
    run.frame
        .write_slot(input_slot, SlotValue::List(list))
        .map_err(|error| format!("collect input write failed: {error:?}"))?;
    assert_eq!(run.drive(128)?, RuntimeSignal::Finished(SlotValue::I64(42)));
    run.assert_sentinels()?;
    run.assert_body_counts(&[(7, 0)]);
    assert_eq!(
        run.workflow.to_parts(),
        original,
        "compiled IR was modified during execution"
    );
    Ok(())
}

#[test]
fn collect_three_items() -> Result<(), String> {
    let composites = "  - id: gather\n    collect:\n      source: \"8\"\n      variable: page\n      pages: 3\n      items: 5\n      steps:\n        - id: body\n          set:\n            output: collect_result\n            value: \"7\"\n";
    let mut run = SourceRun::new(&source(composites, "answer"))?;
    let original = run.workflow.to_parts();
    let input_slot = run
        .workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| match &node.kind {
            CompiledNodeKind::CollectStart { source, .. } if *source == SlotIdx::new(8) => {
                Some(*source)
            }
            _ => None,
        })
        .ok_or("missing collect input slot")?;
    let items: Vec<SlotValue> = vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)];
    let list = run
        .store
        .insert_list(Box::<[SlotValue]>::from(items))
        .map_err(|e| format!("{e:?}"))?;
    run.frame
        .write_slot(input_slot, SlotValue::List(list))
        .map_err(|error| format!("collect input write failed: {error:?}"))?;
    assert_eq!(run.drive(128)?, RuntimeSignal::Finished(SlotValue::I64(42)));
    run.assert_sentinels()?;
    run.assert_body_counts(&[(7, 1)]);
    assert_eq!(
        run.workflow.to_parts(),
        original,
        "compiled IR was modified during execution"
    );
    Ok(())
}

#[test]
fn ask_with_timeout_resumes() -> Result<(), String> {
    let composites = "  - id: query\n    ask:\n      prompt: \"0\"\n      timeout: \"1\"\n";
    let mut run = SourceRun::new(&source(composites, "answer"))?;
    let original = run.workflow.to_parts();
    assert_eq!(run.drive(128)?, RuntimeSignal::AwaitingAsk);
    run.assert_sentinels()?;
    let (answer_slot, output_slot, resume_step) = run
        .workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| {
            if matches!(&node.kind, CompiledNodeKind::AskResume { .. }) {
                Some((node.output?, node.output?, node.id))
            } else {
                None
            }
        })
        .ok_or("missing ask resume node")?;
    run.frame
        .write_slot(answer_slot, SlotValue::I64(17))
        .map_err(|error| format!("answer write failed: {error:?}"))?;
    run.frame
        .set_pc(resume_step)
        .map_err(|error| format!("resume pc set failed: {error:?}"))?;
    assert_eq!(run.drive(128)?, RuntimeSignal::Finished(SlotValue::I64(42)));
    run.assert_sentinels()?;
    let answer_value = run
        .frame
        .read_slot(output_slot)
        .map_err(|error| format!("answer read failed: {error:?}"))?;
    assert_eq!(
        *answer_value,
        SlotValue::I64(17),
        "ask answer was not preserved"
    );
    assert_eq!(
        run.workflow.to_parts(),
        original,
        "compiled IR was modified during execution"
    );
    Ok(())
}

#[test]
fn ask_without_timeout_resumes() -> Result<(), String> {
    let composites = "  - id: query\n    ask:\n      prompt: \"0\"\n";
    let mut run = SourceRun::new(&source(composites, "sentinel"))?;
    let original = run.workflow.to_parts();
    assert_eq!(run.drive(128)?, RuntimeSignal::AwaitingAsk);
    run.assert_sentinels()?;
    let (answer_slot, output_slot, resume_step) = run
        .workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| {
            if matches!(&node.kind, CompiledNodeKind::AskResume { .. }) {
                Some((node.output?, node.output?, node.id))
            } else {
                None
            }
        })
        .ok_or("missing ask resume node")?;
    run.frame
        .write_slot(answer_slot, SlotValue::I64(17))
        .map_err(|error| format!("answer write failed: {error:?}"))?;
    run.frame
        .set_pc(resume_step)
        .map_err(|error| format!("resume pc set failed: {error:?}"))?;
    assert_eq!(run.drive(128)?, RuntimeSignal::Finished(SlotValue::I64(99)));
    run.assert_sentinels()?;
    let answer_value = run
        .frame
        .read_slot(output_slot)
        .map_err(|error| format!("answer read failed: {error:?}"))?;
    assert_eq!(
        *answer_value,
        SlotValue::I64(17),
        "ask answer was not preserved"
    );
    assert_eq!(
        run.workflow.to_parts(),
        original,
        "compiled IR was modified during execution"
    );
    Ok(())
}

