use vb_core::{
    CapabilitySet, CompiledWorkflow, RunFrame, RunId, SlotValue, StepBudget, Taint, ValueStore,
    new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) struct Probe {
    pub(super) plan: CompiledWorkflow,
    pub(super) frame: RunFrame,
    pub(super) store: ValueStore,
    pub(super) events: Vec<EvidenceEvent>,
    pub(super) peak: u16,
}

impl Probe {
    pub(super) fn new(source: &str) -> TestResult<Self> {
        let plan = vb_compile::compile_workflow(source.as_bytes())?;
        vb_validate::shared::validate(&plan.to_parts())?;
        let frame = new_run_frame(RunId::new(1), &plan)?;
        let mut store = ValueStore::with_max_slots(plan.resource_contract().max_slots);
        if let Some(metadata) = plan.source_metadata() {
            vb_core::workflow::source_metadata::seed_source_symbols(metadata, &mut store)?;
        }
        Ok(Self {
            plan,
            frame,
            store,
            events: Vec::new(),
            peak: 0,
        })
    }

    pub(super) fn seed(&mut self, value: SlotValue, taint: Taint) -> TestResult {
        let input = self.plan.input_slots().first().ok_or("missing input")?;
        self.frame.write_slot_with_taint(input.slot, value, taint)?;
        Ok(())
    }

    pub(super) fn budget(&self) -> TestResult<vb_core::budget::WholeWorkflowBudget> {
        let parts = self.plan.to_parts();
        Ok(vb_core::budget::WholeWorkflowBudget::compute(
            &parts.nodes,
            parts.entry,
            &parts.resource_contract,
        )?)
    }

    pub(super) fn finish(&mut self, quantum: u64) -> TestResult<SlotValue> {
        let original = self.plan.to_parts();
        let mut evidence = EvidenceCollector::new();
        let mut collect = CollectStates::new();
        for _ in 0..512 {
            let signal = drive_deterministic_full(
                &self.plan,
                &mut self.frame,
                &mut StepBudget::new(quantum),
                &mut self.store,
                &[],
                RetryPolicy::NEVER,
                &mut evidence,
                &mut collect,
                &CapabilitySet::empty(),
                1,
            )?;
            self.events.extend(evidence.drain());
            self.peak = self.peak.max(self.frame.parallel_in_flight());
            if let RuntimeSignal::Finished(value) = signal {
                assert_eq!(self.frame.parallel_in_flight(), 0);
                assert_eq!(self.plan.to_parts(), original);
                return Ok(value);
            }
            assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
        }
        Err("workflow did not finish".into())
    }

    pub(super) fn list(&self, value: SlotValue) -> TestResult<&[SlotValue]> {
        let SlotValue::List(id) = value else {
            return Err(format!("expected list, got {value:?}").into());
        };
        Ok(self.store.list(id)?)
    }

    pub(super) fn assert_failure_evidence(&self) -> TestResult {
        let failed: Vec<_> = self
            .events
            .iter()
            .filter_map(|event| match event {
                EvidenceEvent::StepFailed { step } => Some(*step),
                _ => None,
            })
            .collect();
        assert_eq!(failed.len(), 1);
        let step = *failed.first().ok_or("missing failure")?;
        assert_eq!(self.frame.step_state(step)?, vb_core::StepState::Failed);
        assert!(!self.events.iter().any(|event| matches!(event, EvidenceEvent::StepSucceeded { step: succeeded, .. } if *succeeded == step)));
        let slot = self
            .plan
            .node(step)
            .and_then(|node| node.output.slot())
            .ok_or("missing failed output")?;
        assert_eq!(
            self.frame.read_slot(slot),
            Err(vb_core::EngineError::SlotUninitialized { slot })
        );
        assert!(!self.events.iter().any(|event| matches!(event, EvidenceEvent::SlotWritten { slot: written, .. } if *written == slot)));
        let started = self.events.iter().position(|event| matches!(event, EvidenceEvent::StepStarted { step: started } if *started == step)).ok_or("failure missing start evidence")?;
        let failed = self.events.iter().position(|event| matches!(event, EvidenceEvent::StepFailed { step: failed } if *failed == step)).ok_or("missing failed evidence")?;
        assert!(started < failed);
        Ok(())
    }

    pub(super) fn assert_result_evidence(&self, value: SlotValue, taint: Taint) -> TestResult {
        let parts = self.plan.to_parts();
        let output = parts
            .nodes
            .iter()
            .find_map(|node| match node.kind {
                vb_core::CompiledNodeKind::Finish { result } => Some(result),
                _ => None,
            })
            .ok_or("missing finish")?;
        assert_eq!(self.frame.read_taint(output)?, taint);
        assert!(self.events.iter().any(|event| matches!(event, EvidenceEvent::SlotWritten { slot, value: written, taint: written_taint, .. } if *slot == output && *written == value && *written_taint == taint)));
        Ok(())
    }
}
