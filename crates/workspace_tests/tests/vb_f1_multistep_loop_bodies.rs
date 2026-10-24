#![forbid(unsafe_code)]

use vb_compile::compile_workflow;
use vb_core::{
    CapabilitySet, CompiledNodeKind, CompiledWorkflow, ConstValue, RunFrame, RunId, SlotIdx,
    SlotValue, StepBudget, Taint, ValueStore, new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

const HEADER: &str =
    "version: velvet-ballistics/v1\nname: f1_multistep_loop_bodies\nwhen:\n  manual: {}\nsteps:\n";
const SENTINELS: &str = "  - id: first\n    set:\n      output: answer\n      value: \"42\"\n  - id: second\n    set:\n      output: sentinel\n      value: \"99\"\n";

fn repeat_source(max_attempts: u64) -> String {
    format!(
        "{HEADER}{SENTINELS}  - id: bounded_repeat\n    repeat:\n      max_attempts: {max_attempts}\n      steps:\n        - id: body_first\n          set:\n            output: body_first_result\n            value: \"7\"\n        - id: body_second\n          set:\n            output: body_second_result\n            value: \"11\"\n  - id: done\n    finish:\n      result: answer\n"
    )
}

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

    fn body_step_values(&mut self) -> Vec<i64> {
        let events = self.evidence.drain();
        let mut values = Vec::new();
        for event in events {
            if let EvidenceEvent::StepSucceeded { step, .. } = event {
                if let Some(node) = self.workflow.node(step) {
                    if let CompiledNodeKind::SetConst { value } = &node.kind {
                        if let Some(&ConstValue::I64(v)) = self.workflow.constant(*value) {
                            if v == 7 || v == 11 {
                                values.push(v);
                            }
                        }
                    }
                }
            }
        }
        values
    }
}

fn assert_multistep_repeat(max_attempts: u64, expected_body: &[i64]) -> Result<(), String> {
    let mut run = SourceRun::new(&repeat_source(max_attempts))?;
    let original = run.workflow.to_parts();
    assert_eq!(run.drive(128)?, RuntimeSignal::Finished(SlotValue::I64(42)));
    run.assert_sentinels()?;
    let body_values = run.body_step_values();
    assert_eq!(body_values, expected_body, "body step value sequence");
    assert_eq!(
        run.workflow.to_parts(),
        original,
        "compiled IR was modified during execution"
    );
    Ok(())
}

#[test]
fn repeat_max1_two_step_body_preserves_sentinels() -> Result<(), String> {
    assert_multistep_repeat(1, &[7, 11])
}

#[test]
fn repeat_max3_two_step_body_preserves_sentinels() -> Result<(), String> {
    assert_multistep_repeat(3, &[7, 11, 7, 11, 7, 11])
}
