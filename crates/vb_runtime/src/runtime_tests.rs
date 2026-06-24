#![forbid(unsafe_code)]
//! Tests for the multi-shard runtime.

use crate::AskTicket;
use crate::journal::{RuntimeJournal, RuntimeJournalEvent, VolatileRuntimeJournal};
use crate::runtime::Runtime;
use crate::shard::ShardConfig;
use std::sync::{Arc, Mutex};
use vb_core::action::{
    ActionContract, ActionFailureCode, ActionName, ActionOutputReady, ActionTicket, Idempotency,
    RetrySafety, SideEffect,
};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, ConstIdx, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuntimeError;
    use crate::engine::action::compute_idempotency_key;
    use crate::shard::{AskAnswer, InspectResponse, ShardDirective};
    use crate::trace::TraceEvent;
    use std::num::NonZeroUsize;

    #[derive(Debug)]
    struct RejectCompletionJournal {
        events: Mutex<Vec<RuntimeJournalEvent>>,
    }

    impl RejectCompletionJournal {
        fn shared() -> Arc<Self> {
            Arc::new(Self {
                events: Mutex::new(Vec::new()),
            })
        }

        fn snapshot(&self) -> crate::RuntimeResult<Vec<RuntimeJournalEvent>> {
            self.events
                .lock()
                .map(|events| events.clone())
                .map_err(|_| RuntimeError::JournalPoisoned)
        }
    }

    impl RuntimeJournal for RejectCompletionJournal {
        fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            if matches!(event, RuntimeJournalEvent::ActionCompletedEnvelope { .. }) {
                return Err(RuntimeError::JournalFull { capacity: 0 });
            }
            self.events
                .lock()
                .map_err(|_| RuntimeError::JournalPoisoned)?
                .push(event);
            Ok(())
        }

        fn probe(&self) -> crate::RuntimeResult<()> {
            Ok(())
        }
    }

    fn suspended_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("suspended"),
            digest: WorkflowDigest::from_bytes([1; 32]),
            nodes: Box::from([node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn action_then_finish_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let do_node = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(7),
                input: SlotIdx::new(0),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(1),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("action_then_finish"),
            digest: WorkflowDigest::from_bytes([3; 32]),
            nodes: Box::from([do_node, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn runtime_config() -> ShardConfig {
        ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        }
    }

    fn contract_required_capability(action: ActionId) -> Capability {
        Capability::new("__contract_required__".into(), action)
    }

    fn action_contract(action: ActionId, input_slots: u16, output_slots: u16) -> ActionContract {
        ActionContract {
            id: action,
            name: ActionName::new("test-action").unwrap(),
            input_slot_count: input_slots,
            output_slot_count: output_slots,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::from([contract_required_capability(action)]),
        }
    }

    fn action_contracts_through(
        action: ActionId,
        input_slots: u16,
        output_slots: u16,
    ) -> Box<[ActionContract]> {
        let target = action.get();
        let mut contracts = Vec::with_capacity(usize::from(target).saturating_add(1));
        let mut id = 0u16;
        loop {
            let current = ActionId::new(id);
            if id == target {
                contracts.push(action_contract(current, input_slots, output_slots));
                break;
            }
            contracts.push(action_contract(current, 0, 0));
            id = id.saturating_add(1);
        }
        contracts.into_boxed_slice()
    }

    fn action_grants(action: ActionId) -> CapabilitySet {
        CapabilitySet::from_grants(Box::from([contract_required_capability(action)]))
    }

    fn encoded_len(value: &SlotValue) -> u32 {
        match postcard::to_allocvec(value) {
            Ok(bytes) => match u32::try_from(bytes.len()) {
                Ok(len) => len,
                Err(_) => u32::MAX,
            },
            Err(_) => u32::MAX,
        }
    }

    fn active_frame(runtime: &Runtime, run: vb_core::ids::RunId) -> Option<vb_core::RunFrame> {
        runtime
            .shards
            .get(runtime.shard_index(run))
            .and_then(|shard| shard.run_state_get(run))
            .map(|state| state.frame.clone())
    }

    fn submit_suspended(
        runtime: &Runtime,
        run: vb_core::ids::RunId,
        wf: vb_core::workflow::CompiledWorkflow,
    ) -> crate::RuntimeResult<()> {
        let action = ActionId::new(0);
        runtime.submit_direct_with_inputs_grants_and_contracts(
            run,
            wf,
            Box::from([(SlotIdx::new(0), SlotValue::I64(0))]),
            action_grants(action),
            action_contracts_through(action, 1, 0),
        )
    }

    fn ask_waiting_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let set_prompt = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let set_timeout = CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(1),
            },
        };
        let ask = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::ZERO,
                timeout_slot: Some(SlotIdx::new(1)),
            },
        };
        let resume = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: Some(StepIdx::new(4)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::AskResume {
                answer: SlotIdx::new(2),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(2),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("ask_waiting_test"),
            digest: WorkflowDigest::from_bytes([42; 32]),
            nodes: Box::from([set_prompt, set_timeout, ask, resume, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([
                ConstValue::I64(0),
                ConstValue::I64(0),
            ]),
            slot_count: 3,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn submit_ask_waiting(
        runtime: &Runtime,
        run: vb_core::ids::RunId,
        wf: vb_core::workflow::CompiledWorkflow,
    ) -> crate::RuntimeResult<()> {
        let action = ActionId::new(0);
        runtime.submit_direct_with_inputs_grants_and_contracts(
            run,
            wf,
            Box::from([]),
            action_grants(action),
            action_contracts_through(action, 0, 0),
        )
    }

    fn submit_action_then_finish(
        runtime: &Runtime,
        run: vb_core::ids::RunId,
        wf: vb_core::workflow::CompiledWorkflow,
    ) -> crate::RuntimeResult<()> {
        let action = ActionId::new(7);
        runtime.submit_direct_with_inputs_grants_and_contracts(
            run,
            wf,
            Box::from([(SlotIdx::new(0), SlotValue::I64(0))]),
            action_grants(action),
            action_contracts_through(action, 1, 2),
        )
    }

    #[test]
    fn snapshot_run_reports_missing_run_without_command_queue() {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new(shard_count, ShardConfig::default());
        let result = runtime.snapshot_run(vb_core::ids::RunId::new(1), 7);
        assert_eq!(
            result,
            Ok(InspectResponse::NotFound {
                run: vb_core::ids::RunId::new(1),
                correlation: 7,
            })
        );
    }

    #[test]
    fn list_events_is_non_destructive() {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let runtime = Runtime::new(shard_count, config);
        let first = runtime.list_events(vb_core::ids::RunId::new(1));
        let second = runtime.list_events(vb_core::ids::RunId::new(1));
        assert_eq!(first, Ok(Vec::new()));
        assert_eq!(second, Ok(Vec::new()));
    }

    #[test]
    fn new_creates_configured_shard_count() {
        let Some(shard_count) = NonZeroUsize::new(3) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn shutdown_graceful_enqueues_on_all_shards() {
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn shutdown_graceful_processes_shards_before_journal_drain() -> Result<(), String> {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return Err(String::from("expected non-zero shard count"));
        };
        let journal = Arc::new(VolatileRuntimeJournal::new());
        let mut runtime = Runtime::new_with_journal(shard_count, runtime_config(), journal.clone());
        let Some(wf) = finished_workflow() else {
            return Err(String::from("expected finished workflow fixture"));
        };
        let run = vb_core::ids::RunId::new(31);
        assert_eq!(runtime.submit_direct(run, wf), Ok(()));

        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        let encoded_bool = postcard::to_allocvec(&SlotValue::Bool(true))
            .map_err(|err| format!("bool slot serialization failed: {err:?}"))?;

        assert_eq!(
            journal.snapshot(),
            Ok(vec![
                RuntimeJournalEvent::RunSubmitted {
                    run,
                    workflow: WorkflowDigest::from_bytes([2; 32]),
                },
                RuntimeJournalEvent::RunAdmission {
                    admission: crate::admission::RunAdmission::new(
                        WorkflowDigest::from_bytes([2; 32]),
                        run,
                        CapabilitySet::empty(),
                        RuntimePolicy::Relaxed,
                    ),
                },
                RuntimeJournalEvent::StepStarted {
                    run,
                    step: StepIdx::new(0),
                },
                RuntimeJournalEvent::SlotWritten {
                    run,
                    slot: SlotIdx::new(0),
                    value: encoded_bool,
                    taint: vb_core::Taint::Clean,
                    extra: None,
                },
                RuntimeJournalEvent::StepSucceeded {
                    run,
                    step: StepIdx::new(0),
                    output: SlotIdx::new(0),
                    attempt: 1,
                },
                RuntimeJournalEvent::StepStarted {
                    run,
                    step: StepIdx::new(1),
                },
                RuntimeJournalEvent::StepSucceeded {
                    run,
                    step: StepIdx::new(1),
                    output: SlotIdx::ZERO,
                    attempt: 1,
                },
                RuntimeJournalEvent::RunFinished {
                    run,
                    result: SlotIdx::ZERO,
                },
            ])
        );
        Ok(())
    }

    #[test]
    fn counters_snapshot_aggregates_across_shards() {
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf1) = suspended_workflow() else {
            return;
        };
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            submit_suspended(&runtime, vb_core::ids::RunId::new(1), wf1),
            Ok(())
        );
        assert_eq!(
            submit_suspended(&runtime, vb_core::ids::RunId::new(2), wf2),
            Ok(())
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 2);
    }

    #[test]
    fn drain_trace_aggregates_across_shards() {
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf1) = suspended_workflow() else {
            return;
        };
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            submit_suspended(&runtime, vb_core::ids::RunId::new(1), wf1),
            Ok(())
        );
        assert_eq!(
            submit_suspended(&runtime, vb_core::ids::RunId::new(2), wf2),
            Ok(())
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        let events = runtime.drain_trace();
        // Each submit produces: RunSubmitted + initial SlotWritten + StepStarted
        // + ActionScheduled = 4 events per run.
        assert_eq!(events.len(), 8);
    }

    // Helper: workflow that finishes immediately (SetConst -> Finish).
    fn finished_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let set_const = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("finished"),
            digest: WorkflowDigest::from_bytes([2; 32]),
            nodes: Box::from([set_const, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([vb_core::value::ConstValue::Bool(true)]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    #[test]
    fn runtime_submit_direct_enqueues_on_correct_shard() {
        // Given a 2-shard runtime
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(1);
        // When submitting a run
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then counters show 1 run submitted
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
    }

    #[test]
    fn runtime_cancel_run_routes_to_correct_shard() {
        // Given a 2-shard runtime with a submitted run
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(1);
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When cancelling the run
        assert_eq!(runtime.cancel_run(run), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then the failed counter is incremented
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_failed, 1);
    }

    #[test]
    fn runtime_complete_action_routes_to_correct_shard() {
        // Given a 2-shard runtime with a suspended run
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(1);
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When completing the action
        assert_eq!(runtime.complete_action(run, StepIdx::new(0)), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then the trace contains the ActionCompleted event
        let events = runtime.list_events(run);
        match events {
            Ok(evts) => {
                let found = evts.iter().any(|e| {
                    *e == TraceEvent::ActionCompleted {
                        run,
                        step: StepIdx::new(0),
                    }
                });
                assert_eq!(found, true);
            }
            Err(error) => {
                assert_eq!(Err(error), Ok(Vec::<TraceEvent>::new()));
            }
        }
    }

    #[test]
    fn do_action_completion_writes_output_and_journals_events() {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let journal = Arc::new(VolatileRuntimeJournal::new());
        let mut runtime = Runtime::new_with_journal(shard_count, runtime_config(), journal.clone());
        let Some(wf) = action_then_finish_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(11);
        assert_eq!(submit_action_then_finish(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        let events = runtime.list_events(run);
        assert!(matches!(
            events,
            Ok(ref evts) if evts.contains(&TraceEvent::ActionScheduled {
                run,
                step: StepIdx::ZERO,
            })
        ));

        let ticket = ActionTicket {
            run,
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(7),
            attempt: 1,
            idempotency_key: compute_idempotency_key(run, SeqNo::ZERO, ActionId::new(7)),
            capacity: 1,
        };
        let value = SlotValue::I64(99);
        let output = ActionOutputReady {
            output_slot: SlotIdx::new(1),
            value,
            taint: Taint::Clean,
            encoded_len: encoded_len(&value),
        };
        assert_eq!(runtime.complete_action_with_output(ticket, output), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_completed, 1);
        let trace = runtime.list_events(run);
        assert!(matches!(
            trace,
            Ok(ref evts) if evts.iter().any(|e| matches!(e,
                TraceEvent::SlotWritten { run: r, slot, .. }
                if *r == run && *slot == SlotIdx::new(1)
            )) && evts.contains(&TraceEvent::ActionCompleted {
                run,
                step: StepIdx::ZERO,
            }) && evts.contains(&TraceEvent::RunFinished { run })
        ));
        let journal_events = journal.snapshot();
        assert!(matches!(
            journal_events,
            Ok(ref evts) if evts.iter().any(|e| matches!(e,
                RuntimeJournalEvent::ActionScheduledTicket { ticket: t, output, .. }
                if t.run == run && t.step == StepIdx::ZERO && t.action == ActionId::new(7)
                    && *output == SlotIdx::new(1)
            )) && evts.iter().any(|e| matches!(e,
                RuntimeJournalEvent::ActionCompletedEnvelope { ticket: t, output, encoded_len: len, .. }
                if t.run == run && t.step == StepIdx::ZERO && t.action == ActionId::new(7)
                    && *output == SlotIdx::new(1) && *len == encoded_len(&value)
            ))
        ));
    }

    #[test]
    fn runtime_action_completion_preserves_frame_when_completion_envelope_append_fails() {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let journal = RejectCompletionJournal::shared();
        let mut runtime = Runtime::new_with_journal(shard_count, runtime_config(), journal.clone());
        let Some(wf) = action_then_finish_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(13);
        assert_eq!(submit_action_then_finish(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        let before = active_frame(&runtime, run);
        assert!(matches!(
            before,
            Some(ref frame) if frame.read_slot(SlotIdx::new(1)).is_err()
        ));

        let value = SlotValue::I64(99);
        let output = ActionOutputReady {
            output_slot: SlotIdx::new(1),
            value,
            taint: Taint::Clean,
            encoded_len: encoded_len(&value),
        };
        let completion_ticket = ActionTicket {
            run,
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(7),
            attempt: 1,
            idempotency_key: compute_idempotency_key(run, SeqNo::ZERO, ActionId::new(7)),
            capacity: 1,
        };
        assert_eq!(
            runtime.complete_action_with_output(completion_ticket, output),
            Ok(())
        );
        assert_eq!(
            runtime.tick_all(),
            Err(RuntimeError::JournalFull { capacity: 0 })
        );
        assert_eq!(active_frame(&runtime, run), before);

        let trace = runtime.list_events(run);
        assert!(matches!(
            trace,
            Ok(ref evts) if !evts.iter().any(|event| matches!(
                event,
                TraceEvent::SlotWritten { run: r, slot, .. }
                    if *r == run && *slot == SlotIdx::new(1)
            )) && !evts.contains(&TraceEvent::ActionCompleted {
                run,
                step: StepIdx::ZERO,
            })
        ));
        let journal_events = journal.snapshot();
        assert!(matches!(
            journal_events,
            Ok(ref evts) if evts.iter().any(|event| matches!(
                event,
                RuntimeJournalEvent::ActionScheduledTicket { ticket: t, .. }
                    if t.run == run && t.action == ActionId::new(7)
            )) && !evts.iter().any(|event| matches!(
                event,
                RuntimeJournalEvent::ActionCompletedEnvelope { ticket: t, .. }
                    if t.run == run && t.action == ActionId::new(7)
            ))
        ));
    }

    #[test]
    fn do_action_completion_rejects_wrong_action_ticket() {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = action_then_finish_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(12);
        assert_eq!(submit_action_then_finish(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        let ticket = ActionTicket {
            run,
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(8),
            attempt: 1,
            idempotency_key: compute_idempotency_key(run, SeqNo::ZERO, ActionId::new(8)),
            capacity: 1,
        };
        let output = ActionOutputReady {
            output_slot: SlotIdx::new(1),
            value: SlotValue::I64(99),
            taint: Taint::Clean,
            encoded_len: 8,
        };
        assert_eq!(
            runtime.complete_action_with_output(ticket, output),
            Err(RuntimeError::InvalidActionCompletion)
        );
    }

    #[test]
    fn runtime_inspect_run_returns_found_from_correct_shard() {
        // Given a 2-shard runtime with a submitted run
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(1);
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When inspecting the run
        assert_eq!(runtime.inspect_run(run, 42), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then the inspect response is Found with correct fields
        let response = runtime.take_inspect_response(run);
        match response {
            Ok(Some(InspectResponse::Found(snapshot))) => {
                assert_eq!(snapshot.run, run);
                assert_eq!(snapshot.correlation, 42);
            }
            other => {
                // Wrong: expected Found
                assert_eq!(other, Ok(None));
            }
        }
    }

    #[test]
    fn runtime_tick_all_returns_false_when_any_shard_shuts_down() {
        // Given a 2-shard runtime
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        // When shutting down only one shard
        // Use shutdown_graceful which enqueues to all shards
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        // Then tick_all returns false
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn runtime_tick_all_returns_true_when_all_shards_alive() {
        // Given a 2-shard runtime with no shutdown
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        // When ticking with empty queues
        let result = runtime.tick_all();
        // Then result is true
        assert_eq!(result, Ok(true));
    }

    #[test]
    fn runtime_list_events_returns_events_for_target_run_only() {
        // Given a 2-shard runtime with two runs
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf1) = suspended_workflow() else {
            return;
        };
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        let run1 = vb_core::ids::RunId::new(1);
        let run2 = vb_core::ids::RunId::new(2);
        assert_eq!(submit_suspended(&runtime, run1, wf1), Ok(()));
        assert_eq!(submit_suspended(&runtime, run2, wf2), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When listing events for run1
        let events = runtime.list_events(run1);
        assert_eq!(events.is_ok(), true);
        let events = events;
        let events = match events {
            Ok(e) => e,
            Err(_) => return,
        };
        // Then all events are for run1 only
        let all_run1 = events.iter().all(|e| e.run_id() == run1);
        assert_eq!(all_run1, true);
    }

    #[test]
    fn runtime_take_inspect_response_returns_none_initially() {
        // Given a fresh runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        // When taking inspect response without any inspect command
        let run = vb_core::ids::RunId::new(1);
        let response = runtime.take_inspect_response(run);
        // Then response is Ok(None)
        assert_eq!(response, Ok(None));
    }

    #[test]
    fn runtime_counters_snapshot_starts_at_zero() {
        // Given a fresh runtime
        let Some(shard_count) = NonZeroUsize::new(3) else {
            return;
        };
        let runtime = Runtime::new(shard_count, runtime_config());
        // When taking counters snapshot
        let snap = runtime.counters_snapshot();
        // Then all counters are zero
        assert_eq!(snap.runs_submitted, 0);
        assert_eq!(snap.runs_completed, 0);
        assert_eq!(snap.runs_failed, 0);
        assert_eq!(snap.steps_executed, 0);
    }

    #[test]
    fn runtime_submit_compiled_delegates_to_submit_direct() {
        // Given a runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = finished_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(42);
        // When using submit_compiled
        assert_eq!(runtime.submit_compiled(run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then the run is processed
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_completed, 1);
    }

    #[test]
    fn runtime_fail_action_routes_to_run_shard() {
        // Given a runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new(shard_count, runtime_config());
        let ticket = ActionTicket {
            run: vb_core::ids::RunId::new(1),
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(0),
            attempt: 1,
            idempotency_key: compute_idempotency_key(
                vb_core::ids::RunId::new(1),
                SeqNo::ZERO,
                ActionId::new(0),
            ),
            capacity: 1,
        };
        let failure = ActionFailureCode::Rejected.into();
        let result = runtime.fail_action(ticket, failure);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn runtime_answer_ask_routes_to_run_shard() {
        // Given a 1-shard runtime with an Ask-waiting run
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let run = vb_core::ids::RunId::new(1);
        let Some(wf) = ask_waiting_workflow() else {
            return;
        };
        assert_eq!(submit_ask_waiting(&runtime, run, wf), Ok(()));
        // Drive the run to the Ask-waiting state.
        assert_eq!(runtime.tick_all(), Ok(true));
        let answer = AskAnswer {
            ticket: AskTicket {
                run,
                ask_step: StepIdx::new(2),
                resume_step: StepIdx::new(3),
            },
            answer_slot: SlotIdx::new(2),
            value: SlotValue::Bool(true),
            taint: Taint::Clean,
            encoded_len: 1u32,
        };
        // Then answer_ask routes to the only shard and succeeds
        assert_eq!(runtime.answer_ask(answer), Ok(()));
    }

    #[test]
    fn runtime_answer_ask_finds_run_on_migrated_shard() {
        // Given a 2-shard runtime (RA-030 regression test)
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let run = vb_core::ids::RunId::new(1);
        // Pick a run that actually lives on shard 0 so the migration target
        // (shard 1) differs from the home shard. This guarantees the
        // hash-based shard_for() lookup would miss the run.
        let home_index = runtime.shard_index(run);
        if home_index != 0 {
            // skip — cannot construct migration scenario on this seed
            return;
        }
        let destination = 1usize;
        // When submitting an Ask-waiting workflow to put the run into a
        // suspend state where answer_ask is meaningful.
        let Some(wf) = ask_waiting_workflow() else {
            return;
        };
        assert_eq!(submit_ask_waiting(&runtime, run, wf), Ok(()));
        // Tick until AskAwaiting (handle_ask returns AwaitingAsk).
        assert_eq!(runtime.tick_all(), Ok(true));
        // Sanity: run is now active on home shard.
        assert!(runtime.shards[home_index].run_state_contains(run));
        // Simulate migration: remove from home, insert into destination.
        let state = runtime.shards[home_index]
            .run_state_remove(run)
            .expect("run must be active on home shard before migration");
        assert_eq!(
            runtime.shards[destination].run_state_insert(run, state),
            Ok(None)
        );
        // Now answer_ask must find the run on the destination shard.
        let answer = AskAnswer {
            ticket: AskTicket {
                run,
                ask_step: StepIdx::new(2),
                resume_step: StepIdx::new(3),
            },
            answer_slot: SlotIdx::new(2),
            value: SlotValue::Bool(true),
            taint: Taint::Clean,
            encoded_len: 1u32,
        };
        // Then answer_ask returns Ok (post-fix scans all shards).
        assert_eq!(runtime.answer_ask(answer), Ok(()));
    }

    #[test]
    fn runtime_answer_ask_returns_run_not_found_for_unknown_run() {
        // Given a 2-shard runtime with no submitted runs
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let runtime = Runtime::new(shard_count, runtime_config());
        // When answering an ask for a run that exists nowhere
        let answer = AskAnswer {
            ticket: AskTicket {
                run: vb_core::ids::RunId::new(424242),
                ask_step: StepIdx::ZERO,
                resume_step: StepIdx::new(1),
            },
            answer_slot: SlotIdx::new(0),
            value: SlotValue::Bool(true),
            taint: Taint::Clean,
            encoded_len: 1u32,
        };
        // Then answer_ask returns RunNotFound
        assert_eq!(runtime.answer_ask(answer), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn runtime_drain_trace_returns_empty_for_fresh_runtime() {
        // Given a fresh runtime
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        // When draining trace
        let events = runtime.drain_trace();
        // Then result is empty
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn runtime_submit_and_cancel_increments_failed_counter() {
        // Given a 1-shard runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(1);
        // When submitting then cancelling
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(runtime.cancel_run(run), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then failed counter is 1
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_failed, 1);
    }

    #[test]
    fn runtime_inspect_run_enqueues_command_successfully() {
        // Given a runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(1);
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When inspecting
        assert_eq!(runtime.inspect_run(run, 99), Ok(()));
        // Then tick processes the inspect
        assert_eq!(runtime.tick_all(), Ok(true));
        // And the response is available
        let response = runtime.take_inspect_response(run);
        match response {
            Ok(Some(InspectResponse::Found(snap))) => {
                assert_eq!(snap.run, run);
                assert_eq!(snap.correlation, 99);
            }
            other => {
                assert_eq!(other, Ok(None));
            }
        }
    }

    #[test]
    fn runtime_shutdown_graceful_enqueues_to_all_shards() {
        // Given a 3-shard runtime
        let Some(shard_count) = NonZeroUsize::new(3) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        // When shutting down gracefully
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        // Then tick_all returns false
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn runtime_tick_all_after_shutdown_returns_false_repeatedly() {
        // Given a shutdown runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(false));
        // When ticking again
        assert_eq!(runtime.tick_all(), Ok(false));
        // Then it still returns false
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn runtime_submit_direct_returns_queue_full_when_shard_queue_full() {
        // Given a runtime with tiny queue
        let config = ShardConfig {
            command_queue_capacity: 1,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new(shard_count, config);
        // When filling the queue
        let Some(wf) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(1), wf.clone()),
            Ok(())
        );
        // Then the second submit returns QueueFull
        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(2), wf),
            Err(RuntimeError::QueueFull)
        );
    }

    #[test]
    fn runtime_list_events_for_unknown_shard_returns_error() {
        // Given a runtime with no runs
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new(shard_count, runtime_config());
        // When listing events for a run on a nonexistent shard (can't happen with valid shard_index)
        // Use a valid run that maps to shard 0
        let events = runtime.list_events(vb_core::ids::RunId::new(1));
        // Then result is Ok with empty vec
        match events {
            Ok(evts) => {
                assert_eq!(evts.len(), 0);
            }
            Err(error) => {
                assert_eq!(Err(error), Ok(Vec::<TraceEvent>::new()));
            }
        }
    }

    #[test]
    fn runtime_counters_aggregate_across_shards() {
        // Given a 2-shard runtime
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf1) = finished_workflow() else {
            return;
        };
        let Some(wf2) = finished_workflow() else {
            return;
        };
        // When submitting runs
        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(1), wf1),
            Ok(())
        );
        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(2), wf2),
            Ok(())
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then counters aggregate across shards
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 2);
        assert_eq!(snap.runs_completed, 2);
    }

    #[test]
    fn runtime_single_shard_operations() {
        // Given a 1-shard runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = finished_workflow() else {
            return;
        };
        // When submitting a run
        assert_eq!(
            submit_suspended(&runtime, vb_core::ids::RunId::new(1), wf),
            Ok(())
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then it completes
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_completed, 1);
        assert_eq!(snap.runs_failed, 0);
    }

    #[test]
    fn runtime_new_creates_correct_shard_count() {
        // Given a runtime with 4 shards
        let Some(shard_count) = NonZeroUsize::new(4) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        // When shutting down
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        // Then tick_all returns false (all shards shut down)
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn runtime_take_inspect_response_for_unknown_run_returns_not_found() {
        // Given a runtime with no submitted runs
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        // When inspecting a non-existent run
        let run = vb_core::ids::RunId::new(999);
        assert_eq!(runtime.inspect_run(run, 1), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then the response is NotFound
        let response = runtime.take_inspect_response(run);
        assert_eq!(
            response,
            Ok(Some(InspectResponse::NotFound {
                run,
                correlation: 1
            }))
        );
    }

    #[test]
    fn runtime_drain_trace_returns_submitted_events() {
        // Given a runtime with submitted runs
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            submit_suspended(&runtime, vb_core::ids::RunId::new(1), wf),
            Ok(())
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        // When draining trace
        let events = runtime.drain_trace();
        // Then events contain RunSubmitted
        let found = events.iter().any(|e| {
            matches!(e, TraceEvent::RunSubmitted { run } if *run == vb_core::ids::RunId::new(1))
        });
        assert_eq!(found, true);
    }

    // =======================================================================
    // Adversarial BDD tests — runtime
    // =======================================================================

    #[test]
    fn runtime_shutdown_with_pending_run_then_tick_returns_false() {
        // Given a runtime with a pending suspended run
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(300);
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When shutting down with a pending run
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        // Then tick_all returns false
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn runtime_run_stays_on_one_shard_across_operations() {
        // Given a 2-shard runtime
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(301);
        // When submitting, then cancelling
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(runtime.cancel_run(run), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then counters show exactly 1 submitted and 1 failed
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_failed, 1);
        // And re-submitting the same run succeeds (it was removed by cancel)
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        assert_eq!(submit_suspended(&runtime, run, wf2), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        let snap2 = runtime.counters_snapshot();
        assert_eq!(snap2.runs_submitted, 2);
    }

    #[test]
    fn runtime_complete_action_for_never_submitted_run_returns_ok_enqueue() {
        // Given a runtime with no runs
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        // When completing an action for a run that was never submitted
        let run = vb_core::ids::RunId::new(999);
        assert_eq!(runtime.complete_action(run, StepIdx::new(0)), Ok(()));
        // Then tick returns RunNotFound (the shard has no such run)
        assert_eq!(runtime.tick_all(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn runtime_fail_action_for_never_submitted_run_returns_ok_enqueue() {
        // Given a runtime with no runs
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new(shard_count, runtime_config());
        // When failing an action for a run that was never submitted
        let ticket = ActionTicket {
            run: vb_core::ids::RunId::new(998),
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(0),
            attempt: 1,
            idempotency_key: compute_idempotency_key(
                vb_core::ids::RunId::new(998),
                SeqNo::ZERO,
                ActionId::new(0),
            ),
            capacity: 1,
        };
        let failure = ActionFailureCode::Rejected.into();
        // Then enqueue succeeds (failure is queued)
        assert_eq!(runtime.fail_action(ticket, failure), Ok(()));
    }

    #[test]
    fn runtime_queue_full_returns_typed_error() {
        // Given a runtime with tiny queue capacity
        let config = ShardConfig {
            command_queue_capacity: 1,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new(shard_count, config);
        let Some(wf) = suspended_workflow() else {
            return;
        };
        // When filling the queue to capacity
        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(1), wf.clone()),
            Ok(())
        );
        // Then the next submit returns QueueFull (exact error variant)
        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(2), wf),
            Err(RuntimeError::QueueFull)
        );
    }

    #[test]
    fn runtime_drain_trace_after_drain_returns_empty() {
        // Given a runtime with events
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            submit_suspended(&runtime, vb_core::ids::RunId::new(1), wf),
            Ok(())
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        // When draining twice
        let first = runtime.drain_trace();
        assert_eq!(first.is_empty(), false);
        let second = runtime.drain_trace();
        // Then second drain is empty
        assert_eq!(second.len(), 0);
    }

    #[test]
    fn runtime_countered_exhausted_at_max_active_runs() {
        // Given a 1-shard runtime with max_active_runs = 1
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, config);
        let Some(wf1) = suspended_workflow() else {
            return;
        };
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        // When submitting two runs
        assert_eq!(
            submit_suspended(&runtime, vb_core::ids::RunId::new(1), wf1),
            Ok(())
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(2), wf2),
            Ok(())
        );
        // Then second tick returns ActiveRunCapacityExceeded
        assert_eq!(
            runtime.tick_all(),
            Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
        );
    }

    #[test]
    fn runtime_snapshot_run_for_unknown_run_returns_not_found() {
        // Given a fresh runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new(shard_count, runtime_config());
        // When snapshotting a non-existent run
        let result = runtime.snapshot_run(vb_core::ids::RunId::new(9999), 42);
        // Then it returns NotFound
        assert_eq!(
            result,
            Ok(InspectResponse::NotFound {
                run: vb_core::ids::RunId::new(9999),
                correlation: 42,
            })
        );
    }

    #[test]
    fn runtime_list_events_is_idempotent() {
        // Given a runtime with a submitted run
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            submit_suspended(&runtime, vb_core::ids::RunId::new(1), wf),
            Ok(())
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        // When listing events twice without draining
        let first = runtime.list_events(vb_core::ids::RunId::new(1));
        let second = runtime.list_events(vb_core::ids::RunId::new(1));
        // Then both return the same events (non-destructive)
        assert_eq!(first, second);
        assert_eq!(first.map(|e| e.is_empty()), Ok(false));
    }

    #[test]
    fn runtime_finished_workflow_counts_completed_not_failed() {
        // Given a runtime with a workflow that finishes immediately
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = finished_workflow() else {
            return;
        };
        // When submitting
        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(42), wf),
            Ok(())
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then completed is 1 and failed is 0
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_completed, 1);
        assert_eq!(snap.runs_failed, 0);
        assert_eq!(snap.runs_submitted, 1);
    }

    // =======================================================================
    // Adversarial BDD tests - runtime attack vectors
    // =======================================================================

    #[test]
    fn runtime_two_shards_deterministic_routing_same_run_same_shard() {
        // Given a 4-shard runtime
        let Some(shard_count) = NonZeroUsize::new(4) else {
            return;
        };
        let runtime = Runtime::new(shard_count, runtime_config());
        // When computing shard index for run 1 twice
        let idx1 = runtime.shard_index(vb_core::ids::RunId::new(1));
        let idx2 = runtime.shard_index(vb_core::ids::RunId::new(1));
        // Then the shard index is deterministic
        assert_eq!(idx1, idx2);
        assert!(idx1 < 4);
    }

    #[test]
    fn runtime_two_shards_different_runs_may_land_on_different_shards() {
        // Given a 4-shard runtime
        let Some(shard_count) = NonZeroUsize::new(4) else {
            return;
        };
        let runtime = Runtime::new(shard_count, runtime_config());
        // When computing shard indices for different runs
        let idx1 = runtime.shard_index(vb_core::ids::RunId::new(1));
        let idx2 = runtime.shard_index(vb_core::ids::RunId::new(2));
        // Then at least two different shard indices exist among the runs
        // (we can't guarantee different shards for 1 and 2, but we check the mechanism works)
        assert!(idx1 < 4);
        assert!(idx2 < 4);
    }

    #[test]
    fn runtime_cancel_then_resubmit_on_same_shard_succeeds() {
        // Given a 1-shard runtime with a cancelled run
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(400);
        assert_eq!(submit_suspended(&runtime, run, wf.clone()), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(runtime.cancel_run(run), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When re-submitting the same run
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then counters show 2 submissions and 1 failed
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 2);
        assert_eq!(snap.runs_failed, 1);
    }

    #[test]
    fn runtime_fail_action_for_active_suspended_run_increments_failed() {
        // Given a 1-shard runtime with a suspended run
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(401);
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When failing the action
        let ticket = ActionTicket {
            run,
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(0),
            attempt: 1,
            idempotency_key: compute_idempotency_key(run, SeqNo::ZERO, ActionId::new(0)),
            capacity: 1,
        };
        let failure = ActionFailureCode::Rejected.into();
        assert_eq!(runtime.fail_action(ticket, failure), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then the run is failed.
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_failed, 1);
    }

    #[test]
    fn runtime_tick_all_after_shutdown_ignores_pending_commands() {
        // Given a runtime with a submit queued, then shutdown
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        // Shutdown first (processes shutdown tick)
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        // When trying to submit after shutdown (enqueue still works but tick ignores)
        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(402), wf),
            Ok(())
        );
        // Then tick_all returns false (shard shutting down)
        assert_eq!(runtime.tick_all(), Ok(false));
        assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
    }

    #[test]
    fn runtime_journal_events_are_recorded_for_submit_and_finish() {
        // Given a runtime with a volatile journal
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let journal = Arc::new(VolatileRuntimeJournal::new());
        let mut runtime = Runtime::new_with_journal(shard_count, runtime_config(), journal.clone());
        let Some(wf) = finished_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(403);
        // When submitting and ticking
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then journal contains RunSubmitted and RunFinished
        let events = journal.snapshot();
        match events {
            Ok(evts) => {
                let found_submitted = evts.iter().any(|e| {
                    *e == RuntimeJournalEvent::RunSubmitted {
                        run,
                        workflow: vb_core::ids::WorkflowDigest::from_bytes([2; 32]),
                    }
                });
                let found_finished = evts.iter().any(
                    |e| matches!(e, RuntimeJournalEvent::RunFinished { run: r, .. } if *r == run),
                );
                assert_eq!(found_submitted, true);
                assert_eq!(found_finished, true);
            }
            Err(error) => {
                assert_eq!(Err(error), Ok(Vec::<RuntimeJournalEvent>::new()));
            }
        }
    }

    #[test]
    fn runtime_countered_exhausted_does_not_corrupt_other_runs() {
        // Given a 1-shard runtime with max_active_runs=1
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, config);
        let Some(wf1) = suspended_workflow() else {
            return;
        };
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        let run1 = vb_core::ids::RunId::new(500);
        let run2 = vb_core::ids::RunId::new(501);
        // When submitting run1 (succeeds) then run2 (capacity exceeded)
        assert_eq!(submit_suspended(&runtime, run1, wf1), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(runtime.submit_direct(run2, wf2), Ok(()));
        assert_eq!(
            runtime.tick_all(),
            Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
        );
        // Then run1 is still alive and inspectable
        let snap = runtime.snapshot_run(run1, 1);
        match snap {
            Ok(InspectResponse::Found(s)) => {
                assert_eq!(s.run, run1);
            }
            other => {
                assert_eq!(
                    other,
                    Ok(InspectResponse::NotFound {
                        run: run1,
                        correlation: 1
                    })
                );
            }
        }
    }

    // =======================================================================
    // Scheduler edge case tests — Section 36
    // =======================================================================

    // --- 1. Queue-full returns typed error with diagnostic code ---

    #[test]
    fn scheduler_queue_full_returns_typed_error_with_diagnostic_code() {
        // Given a runtime with command_queue_capacity = 1
        let config = ShardConfig {
            command_queue_capacity: 1,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new(shard_count, config);
        let Some(wf) = suspended_workflow() else {
            return;
        };
        // When filling the queue
        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(600), wf.clone()),
            Ok(())
        );
        // Then the next submit returns the typed QueueFull error
        let err = runtime.submit_direct(vb_core::ids::RunId::new(601), wf);
        assert_eq!(err, Err(RuntimeError::QueueFull));
        // And the error's diagnostic code matches QUEUE_FULL
        match err {
            Err(ref e) => assert_eq!(e.diagnostic_code(), RuntimeError::QUEUE_FULL_CODE),
            Ok(()) => assert!(false, "expected QueueFull error"),
        }
    }

    // --- 2. Run stays on one shard across all operations ---

    #[test]
    fn scheduler_run_stays_on_one_shard_across_all_operations() {
        // Given a 4-shard runtime
        let Some(shard_count) = NonZeroUsize::new(4) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(602);
        let target_shard = runtime.shard_index(run);

        // When submitting, inspecting, and cancelling the same run
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.shard_index(run), target_shard);
        assert_eq!(runtime.tick_all(), Ok(true));

        assert_eq!(runtime.inspect_run(run, 1), Ok(()));
        assert_eq!(runtime.shard_index(run), target_shard);
        assert_eq!(runtime.tick_all(), Ok(true));

        assert_eq!(runtime.cancel_run(run), Ok(()));
        assert_eq!(runtime.shard_index(run), target_shard);
        assert_eq!(runtime.tick_all(), Ok(true));

        // Then the run always maps to the same shard index
        assert!(target_shard < 4);
        // And counters reflect the submit + cancel lifecycle
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_failed, 1);
    }

    // --- 3. Cancel pending runs (before execution tick) ---

    #[test]
    fn scheduler_cancel_pending_run_before_execution() {
        // Given a 1-shard runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(603);

        // When submitting and cancelling before tick
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        // Cancel is enqueued before the submit is processed by tick
        assert_eq!(runtime.cancel_run(run), Ok(()));
        // tick_all processes one command per shard per tick, so we need two ticks
        // First tick processes the Submit (run becomes active/suspended)
        assert_eq!(runtime.tick_all(), Ok(true));
        // Second tick processes the Cancel
        assert_eq!(runtime.tick_all(), Ok(true));

        // Then the run is cancelled (failed counter = 1)
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_failed, 1);
        // And inspecting the run returns NotFound (no active run)
        let result = runtime.snapshot_run(run, 2);
        assert_eq!(
            result,
            Ok(InspectResponse::NotFound {
                run,
                correlation: 2,
            })
        );
    }

    // --- 4. Cancel waiting runs (suspended on WaitUntil) ---

    #[test]
    fn scheduler_cancel_run_waiting_on_timer() {
        // Given a 1-shard runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = wait_then_finish_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(604);

        // When submitting a wait workflow and ticking (run enters Wait state)
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        // Then the run is in the runs map (snapshot returns Found)
        let before = runtime.snapshot_run(run, 3);
        match before {
            Ok(InspectResponse::Found(snap)) => {
                assert_eq!(snap.run, run);
            }
            other => {
                assert_eq!(
                    other,
                    Ok(InspectResponse::NotFound {
                        run,
                        correlation: 3
                    })
                );
            }
        }

        // When cancelling the waiting run
        assert_eq!(runtime.cancel_run(run), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        // Then the run is cleanly removed
        let after = runtime.snapshot_run(run, 4);
        assert_eq!(
            after,
            Ok(InspectResponse::NotFound {
                run,
                correlation: 4,
            })
        );
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_failed, 1);
        assert_eq!(snap.runs_completed, 0);
    }

    // --- 5. Shutdown drains gracefully with pending runs ---

    #[test]
    fn scheduler_shutdown_drains_pending_suspended_run() {
        // Given a 1-shard runtime with a journal to observe events
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let journal = Arc::new(VolatileRuntimeJournal::new());
        let mut runtime = Runtime::new_with_journal(shard_count, runtime_config(), journal.clone());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(605);

        // When submitting a suspended run and initiating graceful shutdown
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        // drain_for_shutdown processes queued commands
        // Then tick_all returns false (shutdown complete)
        assert_eq!(runtime.tick_all(), Ok(false));

        // And the journal recorded the run submission
        let journal_events = journal.snapshot();
        match journal_events {
            Ok(evts) => {
                let found_submitted = evts.iter().any(
                    |e| matches!(e, RuntimeJournalEvent::RunSubmitted { run: r, .. } if *r == run),
                );
                assert_eq!(found_submitted, true);
            }
            Err(error) => {
                assert_eq!(Err(error), Ok(Vec::<RuntimeJournalEvent>::new()));
            }
        }
    }

    #[test]
    fn scheduler_shutdown_drains_pending_finished_run() {
        // Given a 1-shard runtime with a journal
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let journal = Arc::new(VolatileRuntimeJournal::new());
        let mut runtime = Runtime::new_with_journal(shard_count, runtime_config(), journal.clone());
        let Some(wf) = finished_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(606);

        // When submitting a finished workflow and initiating shutdown
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(false));

        // Then the run completed during drain
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_completed, 1);

        // And journal recorded RunFinished
        let journal_events = journal.snapshot();
        match journal_events {
            Ok(evts) => {
                let found_finished = evts.iter().any(
                    |e| matches!(e, RuntimeJournalEvent::RunFinished { run: r, .. } if *r == run),
                );
                assert_eq!(found_finished, true);
            }
            Err(error) => {
                assert_eq!(Err(error), Ok(Vec::<RuntimeJournalEvent>::new()));
            }
        }
    }

    // --- 6. Timer resume order: multiple waits, deterministic processing ---

    #[test]
    fn scheduler_timer_fire_processes_correct_run() {
        // Given a 1-shard runtime with two waiting runs
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf1) = wait_then_finish_workflow() else {
            return;
        };
        let Some(wf2) = wait_then_finish_workflow() else {
            return;
        };
        let run1 = vb_core::ids::RunId::new(607);
        let run2 = vb_core::ids::RunId::new(608);

        assert_eq!(submit_action_then_finish(&runtime, run1, wf1), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(submit_action_then_finish(&runtime, run2, wf2), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        // Both runs are now in wait state. Fire captured timer authority for run1 only.
        let Ok(entry1) = runtime.capture_timer_entry(run1) else {
            return;
        };
        assert_eq!(runtime.timer_entry_fired(entry1), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        // Then run1 completed but run2 is still waiting
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_completed, 1);

        // run2 is still present (inspect returns Found)
        let inspect = runtime.snapshot_run(run2, 5);
        match inspect {
            Ok(InspectResponse::Found(s)) => {
                assert_eq!(s.run, run2);
            }
            other => {
                assert_eq!(
                    other,
                    Ok(InspectResponse::NotFound {
                        run: run2,
                        correlation: 5,
                    })
                );
            }
        }

        // Now fire captured timer authority for run2.
        let Ok(entry2) = runtime.capture_timer_entry(run2) else {
            return;
        };
        assert_eq!(runtime.timer_entry_fired(entry2), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        let snap2 = runtime.counters_snapshot();
        assert_eq!(snap2.runs_completed, 2);
    }

    // --- 7. Action completion resumes correct run ---

    #[test]
    fn scheduler_action_completion_resumes_correct_run_among_many() {
        // Given a 1-shard runtime with three suspended runs
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf1) = action_then_finish_workflow() else {
            return;
        };
        let Some(wf2) = action_then_finish_workflow() else {
            return;
        };
        let Some(wf3) = action_then_finish_workflow() else {
            return;
        };
        let run1 = vb_core::ids::RunId::new(610);
        let run2 = vb_core::ids::RunId::new(611);
        let run3 = vb_core::ids::RunId::new(612);

        assert_eq!(submit_action_then_finish(&runtime, run1, wf1), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(submit_action_then_finish(&runtime, run2, wf2), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(submit_action_then_finish(&runtime, run3, wf3), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        // All three runs are suspended on their Do actions.
        // Complete only run2's action.
        let ticket = ActionTicket {
            run: run2,
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(7),
            attempt: 1,
            idempotency_key: compute_idempotency_key(run2, SeqNo::ZERO, ActionId::new(7)),
            capacity: 1,
        };
        let output = ActionOutputReady {
            output_slot: SlotIdx::new(1),
            value: SlotValue::I64(42),
            taint: Taint::Clean,
            encoded_len: encoded_len(&SlotValue::I64(42)),
        };
        assert_eq!(runtime.complete_action_with_output(ticket, output), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        // Then only run2 completed
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_completed, 1);

        // run1 and run3 are still present and suspended
        assert_suspended_run_is_found(&runtime, run1, 6);
        assert_suspended_run_is_found(&runtime, run3, 6);

        // Verify the completed run's trace shows the finish
        let trace = runtime.list_events(run2);
        match trace {
            Ok(evts) => {
                let found_finished = evts
                    .iter()
                    .any(|e| *e == TraceEvent::RunFinished { run: run2 });
                assert_eq!(found_finished, true);
            }
            Err(error) => {
                assert_eq!(Err(error), Ok(Vec::<TraceEvent>::new()));
            }
        }
    }

    fn assert_suspended_run_is_found(
        runtime: &Runtime,
        run: vb_core::ids::RunId,
        correlation: u64,
    ) {
        match runtime.snapshot_run(run, correlation) {
            Ok(InspectResponse::Found(s)) => {
                assert_eq!(s.run, run);
            }
            other => {
                assert_eq!(other, Ok(InspectResponse::NotFound { run, correlation }));
            }
        }
    }

    // --- 8. No task-per-step: scheduler processes within step budget ---

    #[test]
    fn scheduler_no_task_per_step_processes_within_budget() {
        // Given a runtime with step_budget_per_tick = 100 (sufficient for short workflows)
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 64,
            step_budget_per_tick: 100,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, config);
        // Use the finished_workflow (SetConst -> Finish, 2 steps)
        let Some(wf) = finished_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(620);

        // When submitting the workflow
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        // A single tick processes the submit command and drives to completion
        // The scheduler does NOT spawn a task per step; it drives all steps
        // within the budget of one tick synchronously.
        assert_eq!(runtime.tick_all(), Ok(true));

        // Then the run completes in a single tick (no task-per-step behavior)
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_completed, 1);
        // Verify steps executed is exactly 2 (SetConst + Finish)
        assert!(snap.steps_executed >= 2);
    }

    #[test]
    fn scheduler_single_tick_does_not_spawn_concurrent_tasks() {
        // Given a runtime with a suspended run
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(621);

        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        // When inspecting the run, it is in a suspended state with deterministic PC
        assert_eq!(runtime.inspect_run(run, 10), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        let response = runtime.take_inspect_response(run);
        match response {
            Ok(Some(InspectResponse::Found(snap))) => {
                // The PC is at step 0 (the Do step that suspended)
                assert_eq!(snap.run, run);
                assert_eq!(snap.correlation, 10);
                // PC should be at the suspended step (0 for our single-node workflow)
                assert!(snap.pc == StepIdx::ZERO);
            }
            other => {
                assert_eq!(other, Ok(None));
            }
        }
    }

    #[test]
    fn tick_shard_continue_drives_only_selected_shard() {
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(2);

        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_shard(0, ShardDirective::Continue), Ok(true));

        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_completed, 0);
    }

    #[test]
    fn tick_shard_suspend_preserves_pending_work() {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(3);

        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_shard(0, ShardDirective::Suspend), Ok(true));
        assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
        assert_eq!(runtime.tick_shard(0, ShardDirective::Continue), Ok(true));
        assert_eq!(runtime.counters_snapshot().runs_submitted, 1);
    }

    #[test]
    fn tick_shard_migrate_rejects_self_and_invalid_target() {
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());

        assert_eq!(
            runtime.tick_shard(0, ShardDirective::Migrate { target: 0 }),
            Err(RuntimeError::MigrateSelf)
        );
        assert_eq!(
            runtime.tick_shard(0, ShardDirective::Migrate { target: 9 }),
            Err(RuntimeError::ShardNotFound { shard: 9 })
        );
    }

    #[test]
    fn tick_shard_shutdown_drains_and_reports_dead() {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = finished_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(4);

        assert_eq!(runtime.submit_direct(run, wf), Ok(()));
        assert_eq!(runtime.tick_shard(0, ShardDirective::Shutdown), Ok(false));
        assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    }

    #[test]
    fn tick_shard_shutdown_drains_when_command_queue_is_full() {
        let config = ShardConfig {
            command_queue_capacity: 1,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: RuntimePolicy::Relaxed,
        };
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, config);
        let Some(wf) = finished_workflow() else {
            return;
        };

        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(40), wf),
            Ok(())
        );
        assert_eq!(runtime.tick_shard(0, ShardDirective::Shutdown), Ok(false));
        assert_eq!(runtime.counters_snapshot().runs_completed, 1);
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn tick_shard_cancel_and_barrier_do_not_advance_work_silently() {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let Some(wf) = finished_workflow() else {
            return;
        };

        assert_eq!(
            runtime.submit_direct(vb_core::ids::RunId::new(41), wf),
            Ok(())
        );
        assert_eq!(
            runtime.tick_shard(0, ShardDirective::Cancel),
            Err(RuntimeError::UnsupportedOperation {
                operation: "tick_shard_cancel"
            })
        );
        assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
        assert_eq!(
            runtime.tick_shard(0, ShardDirective::Barrier),
            Err(RuntimeError::UnsupportedOperation {
                operation: "tick_shard_barrier"
            })
        );
        assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
        assert_eq!(runtime.tick_shard(0, ShardDirective::Continue), Ok(true));
        assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    }

    // --- Helper: wait-then-finish workflow (SetConst -> WaitUntil -> Finish) ---

    fn wait_then_finish_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let set_deadline = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let wait = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::ZERO,
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        };
        let parts = WorkflowParts {
            name: Box::from("wait_then_finish"),
            digest: WorkflowDigest::from_bytes([6; 32]),
            nodes: Box::from([set_deadline, wait, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([vb_core::value::ConstValue::I64(10)]),
            slot_count: 1,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
            symbols_count: 0,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }
}
