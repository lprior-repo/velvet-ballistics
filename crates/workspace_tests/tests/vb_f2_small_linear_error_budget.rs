use std::{num::NonZeroUsize, sync::Arc};
use vb_core::{
    ActionId, CapabilitySet, CompiledWorkflow, RunId, RuntimePolicy, SlotValue, StepIdx,
    WholeWorkflowBudget,
    action::{ActionContract, ActionFailureCode, ActionName, Idempotency, RetrySafety, SideEffect},
    workflow::NodeContinuation,
};
use vb_runtime::{
    journal::{RuntimeJournalEvent as E, VolatileRuntimeJournal},
    runtime::{
        Runtime,
        completion_observation::{
            CompletionObservationSlot, CompletionValueObservation, OwnedCompletionScalar,
        },
    },
    shard::ShardConfig,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn real_failed_action_executes_error_only_finish_within_the_cold_budget() -> TestResult {
    for quantum in [1, 64] {
        let workflow = vb_compile::compile_workflow(
            br#"
version: velvet-ballistics/v1
name: error_only_small_graph
when: {manual: {}}
inputs: {value: number}
steps:
  - id: work
    on_error: {handler: recovered}
    do: {action: '0', input: '$input.value'}
  - id: recovered
    finish: {result: '$input.value'}
"#,
        )?;
        let mut parts = workflow.to_parts();
        assert_eq!(parts.nodes.len(), 2);
        let first = parts.nodes.first_mut().ok_or("first node")?;
        assert_eq!(first.routing.error.handler(), Some(StepIdx::new(1)));
        first.routing.next = NodeContinuation::End;
        let budget =
            WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract)?;
        let workflow = CompiledWorkflow::try_from_parts(parts)?;
        vb_validate::shared::validate(&workflow.to_parts())?;
        let input = workflow.input_slots().first().ok_or("input")?.slot;
        let journal = Arc::new(VolatileRuntimeJournal::new());
        let mut runtime = Runtime::new(
            NonZeroUsize::MIN,
            ShardConfig {
                policy: RuntimePolicy::Relaxed,
                step_budget_per_tick: quantum,
                ..ShardConfig::default()
            },
            journal.clone(),
        );
        let run = RunId::new(71);
        runtime.submit_direct_with_inputs_grants_and_contracts(
            run,
            workflow,
            Box::from([(input, SlotValue::I64(42))]),
            CapabilitySet::empty(),
            Box::from([ActionContract {
                id: ActionId::new(0),
                name: ActionName::new("handled-action")?,
                input_slot_count: 1,
                output_slot_count: 1,
                max_input_bytes: 1024,
                max_output_bytes: 1024,
                timeout_ms: 1000,
                idempotency: Idempotency::DeterministicPure,
                side_effect: SideEffect::None,
                retry_safety: RetrySafety::Safe,
                required_capabilities: Box::new([]),
            }]),
        )?;
        runtime.tick_all()?;
        let ticket = runtime
            .lookup_pending_action_ticket(run, 0)
            .ok_or("actual action ticket")?;
        assert_eq!(ticket.step, StepIdx::new(0));
        runtime.fail_action(ticket, ActionFailureCode::Rejected.into())?;
        let mut observations = [CompletionObservationSlot::default()];
        let outcome = runtime
            .tick_all_observed(&mut observations)
            .map_err(|error| format!("observation: {error:?}"))?;
        outcome.execution?;
        outcome
            .observation
            .map_err(|error| format!("observation: {error:?}"))?;
        assert_eq!(
            observations
                .first()
                .and_then(|slot| slot.first)
                .ok_or("recovery completion")?
                .result,
            CompletionValueObservation::Available(OwnedCompletionScalar::I64(42)),
        );
        assert_eq!(runtime.counters_snapshot().runs_completed, 1);
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        assert_eq!(runtime.lookup_pending_action_ticket(run, 0), None);
        let events = journal.snapshot()?;
        let started: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                E::StepStarted {
                    run: observed,
                    step,
                    ..
                } if *observed == run => Some(*step),
                _ => None,
            })
            .collect();
        assert_eq!(started, vec![StepIdx::new(0), StepIdx::new(1)]);
        let failures: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                E::ActionFailed {
                    run: observed,
                    step,
                    ..
                } if *observed == run => Some(*step),
                _ => None,
            })
            .collect();
        assert_eq!(failures, vec![StepIdx::new(0)]);
        let failed = events
            .iter()
            .position(
                |event| matches!(event, E::ActionFailed { run: observed, .. } if *observed == run),
            )
            .ok_or("failure event")?;
        let recovered = events.iter().position(|event| matches!(event, E::StepStarted { run: observed, step, .. } if *observed == run && *step == StepIdx::new(1))).ok_or("handler start")?;
        assert!(failed < recovered);
        assert_eq!(budget.max_total_steps, u64::try_from(started.len())?);
        assert_eq!(budget.max_action_tickets, 1);
        assert_eq!(budget.max_timer_entries, 0);
    }
    Ok(())
}
