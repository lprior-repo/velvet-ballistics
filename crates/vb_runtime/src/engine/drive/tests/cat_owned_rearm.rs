#![forbid(unsafe_code)]
//! Adversarial validated-IR driver checks, not source-language acceptance.
//! The only injected state is a later Waiting body node after a natural pause.

mod expected;
mod fixture;

use super::common::mkr;
use crate::engine::drive::drive_deterministic_full;
use crate::engine::types::{EvidenceCollector, RetryPolicy, RuntimeEngineError, RuntimeSignal};
use crate::primitives::collect::CollectStates;
use fixture::{FAMILIES, Family, build};
use vb_core::capability::CapabilitySet;
use vb_core::engine::{EngineError, StepBudget};
use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::{ListId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

fn checked<T, E: std::fmt::Debug>(result: Result<T, E>) -> Result<T, String> {
    result.map_err(|error| format!("{error:?}"))
}

struct Scenario {
    workflow: CompiledWorkflow,
    run: RunFrame,
    store: ValueStore,
    source: ListId,
    payload: Vec<SlotValue>,
    evidence: EvidenceCollector,
    collect: CollectStates,
}

impl Scenario {
    fn new(family: Family, productive: bool) -> Result<Self, String> {
        let workflow = build(family, productive)?;
        let mut run = mkr(5, 6)?;
        let mut store = ValueStore::new();
        let payload = if productive {
            vec![SlotValue::I64(7), SlotValue::I64(9)]
        } else {
            vec![SlotValue::I64(7)]
        };
        let source = checked(store.insert_list(payload.clone().into_boxed_slice()))?;
        checked(run.write_slot_with_taint(
            SlotIdx::new(0),
            SlotValue::List(source),
            Taint::Secret,
        ))?;
        checked(run.write_slot_with_taint(SlotIdx::new(5), SlotValue::Bool(true), Taint::Clean))?;
        Ok(Self {
            workflow,
            run,
            store,
            source,
            payload,
            evidence: EvidenceCollector::new(),
            collect: CollectStates::new(),
        })
    }

    fn drive(&mut self, steps: u64) -> Result<RuntimeSignal, RuntimeEngineError> {
        drive_deterministic_full(
            &self.workflow,
            &mut self.run,
            &mut StepBudget::new(steps),
            &mut self.store,
            &[],
            RetryPolicy::NEVER,
            &mut self.evidence,
            &mut self.collect,
            &CapabilitySet::empty(),
            1,
        )
    }

    fn pause_and_inject_waiting(&mut self) -> Result<(), String> {
        assert_eq!(self.drive(2), Ok(RuntimeSignal::StepBudgetExhausted));
        assert_eq!(self.run.pc(), StepIdx::new(3));
        assert_eq!(
            checked(self.run.step_state(StepIdx::new(1)))?,
            StepState::Succeeded
        );
        for id in [2, 3, 4] {
            assert_eq!(
                checked(self.run.step_state(StepIdx::new(id)))?,
                StepState::Pending
            );
        }
        // Setup evidence is separate from the single-latch event oracle below.
        assert!(!self.evidence.drain().is_empty());
        checked(self.run.mark_running(StepIdx::new(2)))?;
        checked(self.run.mark_waiting(StepIdx::new(2)))
    }

    fn assert_boundary(&self, before: &RunFrame, productive: bool) -> Result<(), String> {
        assert_eq!(self.run.pc(), StepIdx::new(if productive { 1 } else { 4 }));
        for (id, state) in [
            (1, StepState::Succeeded),
            (2, StepState::Waiting),
            (3, StepState::Succeeded),
        ] {
            assert_eq!(checked(self.run.step_state(StepIdx::new(id)))?, state);
        }
        for id in [0, 1, 2, 4] {
            assert_eq!(
                self.run.step_state(StepIdx::new(id)),
                before.step_state(StepIdx::new(id))
            );
        }
        for id in [4, 5] {
            assert_eq!(
                self.run.read_slot(SlotIdx::new(id)),
                before.read_slot(SlotIdx::new(id))
            );
            assert_eq!(
                self.run.read_taint(SlotIdx::new(id)),
                before.read_taint(SlotIdx::new(id))
            );
        }
        let executed = before
            .executed()
            .checked_add(1)
            .ok_or("test executed overflow")?;
        assert_eq!(self.run.executed(), executed);
        assert_eq!(self.run.parallel_in_flight(), before.parallel_in_flight());
        assert_eq!(
            checked(self.store.list(self.source))?,
            self.payload.as_slice()
        );
        Ok(())
    }
}

fn check_case(family: Family, productive: bool) -> Result<(), String> {
    let mut scenario = Scenario::new(family, productive)?;
    let original = scenario.workflow.to_parts();
    scenario.pause_and_inject_waiting()?;
    let before = scenario.run.clone();
    let result = scenario.drive(1);
    let expected = if productive {
        Err(RuntimeEngineError::Core(
            EngineError::InvalidCompiledWorkflow {
                reason: "loop body step not in rearmable state",
            },
        ))
    } else {
        Ok(RuntimeSignal::StepBudgetExhausted)
    };
    assert_eq!(result, expected, "{family:?}, productive={productive}");
    // Prior primitive slot/PC/evidence effects commit; no whole-drive rollback claim.
    scenario.assert_boundary(&before, productive)?;
    assert_eq!(
        scenario.workflow.to_parts(),
        original,
        "compiled IR changed"
    );
    let events = scenario.evidence.drain();
    expected::verify(
        family,
        productive,
        &scenario.run,
        &scenario.store,
        scenario.source,
        &scenario.collect,
        &events,
    )
}

#[test]
fn productive_bad_later_state_does_not_reset_earlier_body() -> Result<(), String> {
    for family in FAMILIES {
        check_case(family, true).map_err(|error| format!("{family:?}: {error}"))?;
    }
    Ok(())
}

#[test]
fn exhausted_control_does_not_rearm_even_with_waiting_body() -> Result<(), String> {
    for family in FAMILIES {
        check_case(family, false).map_err(|error| format!("{family:?}: {error}"))?;
    }
    Ok(())
}
