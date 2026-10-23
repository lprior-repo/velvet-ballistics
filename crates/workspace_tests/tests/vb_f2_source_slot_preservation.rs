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
    EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

const HEADER: &str = "version: velvet-ballistics/v1\nname: f2_preservation\nwhen:\n  manual: {}\nsteps:\n";
const SENTINELS: &str = "  - id: first\n    set:\n      output: answer\n      value: \"42\"\n  - id: second\n    set:\n      output: sentinel\n      value: \"99\"\n";
const TOGETHER: &str = "  - id: fanout\n    together:\n      branches:\n        - label: left\n          steps:\n            - id: left_value\n              set:\n                output: left_result\n                value: \"7\"\n        - label: right\n          steps:\n            - id: right_value\n              set:\n                output: right_result\n                value: \"11\"\n";
const REPEAT: &str = "  - id: bounded_repeat\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: body_value\n          set:\n            output: body_result\n            value: \"13\"\n";

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
}

fn source(composites: &str, finish: &str) -> String {
    format!("{HEADER}{SENTINELS}{composites}  - id: done\n    finish:\n      result: {finish}\n")
}

fn assert_finished_preserving(composites: &str, finish: &str, expected: i64) -> Result<(), String> {
    let mut run = SourceRun::new(&source(composites, finish))?;
    let original = run.workflow.to_parts();
    assert_eq!(run.drive(128)?, RuntimeSignal::Finished(SlotValue::I64(expected)));
    run.assert_sentinels()?;
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
    assert_finished_preserving(TOGETHER, "answer", 42)
}

#[test]
fn together_finishes_with_prior_sentinel_99() -> Result<(), String> {
    assert_finished_preserving(TOGETHER, "sentinel", 99)
}

#[test]
fn repeat_finishes_with_prior_answer_42() -> Result<(), String> {
    assert_finished_preserving(REPEAT, "answer", 42)
}

#[test]
fn repeat_finishes_with_prior_sentinel_99() -> Result<(), String> {
    assert_finished_preserving(REPEAT, "sentinel", 99)
}

#[test]
fn sequential_together_repeat_preserve_prior_source_values() -> Result<(), String> {
    assert_finished_preserving(&format!("{TOGETHER}{REPEAT}"), "answer", 42)
}

#[test]
fn sequential_repeat_together_preserve_prior_source_values() -> Result<(), String> {
    assert_finished_preserving(&format!("{REPEAT}{TOGETHER}"), "sentinel", 99)
}
