#![forbid(unsafe_code)]

use vb_compile::compile_workflow;
use vb_core::{
    CapabilitySet, CompiledWorkflow, InputSlotKind, ListId, RunFrame, RunId, SlotIdx, SlotValue,
    StepBudget, Taint, ValueStore, new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, RetryPolicy, RuntimeEngineError, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

pub(super) const BODY: &str = "        - id: copy_first\n          set:\n            output: copied\n            value: '$entry'\n        - id: copy_second\n          set:\n            output: echoed\n            value: '$copied'\n";

pub(super) const FINAL: &str = "  - id: done\n    finish:\n      result: anchor\n";

pub(super) fn source(
    schema: &str,
    at_once: Option<u32>,
    variable: &str,
    body: &str,
    tail: &str,
) -> String {
    let at_once_line = match at_once {
        Some(n) => format!("      at_once: {n}\n"),
        None => String::new(),
    };
    format!(
        "version: velvet-ballistics/v1\nname: f1_bounded_foreach\nwhen:\n  manual: {{}}\ninputs:\n  entries: {schema}\nsteps:\n  - id: seed\n    set:\n      output: anchor\n      value: '42'\n  - id: bounded\n    for_each:\n      input: '$input.entries'\n      variable: {variable}\n{at_once_line}      steps:\n{body}{tail}"
    )
}

pub(super) fn compile(source: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(source.as_bytes())
        .map_err(|error| format!("workflow compilation failed: {error:?}"))
}

pub(super) struct SourceRun {
    pub(super) workflow: CompiledWorkflow,
    pub(super) frame: RunFrame,
    pub(super) store: ValueStore,
    pub(super) evidence: EvidenceCollector,
    pub(super) collect_states: CollectStates,
    pub(super) input_slot: SlotIdx,
    pub(super) input_list: ListId,
}

impl SourceRun {
    pub(super) fn new(source: &str, values: &[SlotValue], taint: Taint) -> Result<Self, String> {
        let workflow = compile(source)?;
        let inputs = workflow.input_slots();
        if inputs.len() != 1 {
            return Err(format!("expected 1 input, found {}", inputs.len()));
        }
        let input = inputs.first().ok_or("missing declared input")?;
        if input.kind != InputSlotKind::List {
            return Err(format!("expected list input, found {:?}", input.kind));
        }
        let input_slot = input.slot;
        let mut frame = new_run_frame(RunId::new(1), &workflow)
            .map_err(|error| format!("frame creation failed: {error:?}"))?;
        let mut store = ValueStore::new();
        let input_list = store
            .insert_list(values.to_vec().into_boxed_slice())
            .map_err(|error| format!("list insertion failed: {error:?}"))?;
        frame
            .write_slot_with_taint(input_slot, SlotValue::List(input_list), taint)
            .map_err(|error| format!("input slot write failed: {error:?}"))?;
        Ok(Self {
            workflow,
            frame,
            store,
            evidence: EvidenceCollector::new(),
            collect_states: CollectStates::new(),
            input_slot,
            input_list,
        })
    }

    pub(super) fn drive(&mut self, steps: u64) -> Result<RuntimeSignal, RuntimeEngineError> {
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
    }

    pub(super) fn assert_input(&self, values: &[SlotValue], taint: Taint) -> Result<(), String> {
        let slot_value = self
            .frame
            .read_slot(self.input_slot)
            .map_err(|error| format!("input slot read failed: {error:?}"))?;
        match slot_value {
            SlotValue::List(id) if *id == self.input_list => {}
            other => {
                return Err(format!(
                    "input slot changed: expected List({:?}), found {:?}",
                    self.input_list, other
                ));
            }
        }
        let slot_taint = self
            .frame
            .read_taint(self.input_slot)
            .map_err(|error| format!("input taint read failed: {error:?}"))?;
        if slot_taint != taint {
            return Err(format!(
                "input taint changed: expected {:?}, found {:?}",
                taint, slot_taint
            ));
        }
        let stored = self
            .store
            .list(self.input_list)
            .map_err(|error| format!("list read failed: {error:?}"))?;
        if stored.len() != values.len() {
            return Err(format!(
                "list length changed: expected {}, found {}",
                values.len(),
                stored.len()
            ));
        }
        for (i, (expected, actual)) in values.iter().zip(stored.iter()).enumerate() {
            if expected != actual {
                return Err(format!("list element {} changed", i));
            }
        }
        Ok(())
    }

    pub(super) fn output(&self, step: u16) -> Result<SlotIdx, String> {
        let node = self
            .workflow
            .node(vb_core::StepIdx::new(step))
            .ok_or_else(|| format!("step {} not found", step))?;
        node.output
            .ok_or_else(|| format!("step {} has no output slot", step))
    }
}
