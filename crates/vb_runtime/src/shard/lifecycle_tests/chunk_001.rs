    use vb_core::action::{
        ActionFailure, ActionFailureCode, ActionOutputReady, ActionTicket,
        RetryPolicy as VbRetryPolicy,
    };
    use vb_core::capability::CapabilitySet;
    use vb_core::ids::{ActionId, ConstIdx, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::value::{ConstValue, SlotValue, Taint};
    use vb_core::workflow::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
    };

    use crate::RuntimeError;
    use crate::journal::{RuntimeJournalEvent, SharedRuntimeJournal};
    use crate::trace::TraceEvent;

    use super::super::types::{
        AskAnswer, AskTicket, InspectResponse, PendingTimerKind, Shard, ShardCommand,
        ShardConfig,
    };

    fn timer_command(shard: &Shard, run: RunId) -> ShardCommand {
        let Some(entry) = shard.timer_entry(run) else {
            panic!("timer_command requires a live captured timer entry");
        };
        ShardCommand::TimerFired {
            run: entry.run,
            generation: entry.generation,
            deadline: entry.deadline,
            kind: entry.kind,
        }
    }

    fn invalid_timer_command(run: RunId) -> ShardCommand {
        ShardCommand::TimerFired {
            run,
            generation: 0,
            deadline: std::time::Instant::now(),
            kind: PendingTimerKind::Wait,
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
            input_slots: Box::from([]),
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn zero_retry_policy_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let set_policy = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let action = CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::ZERO,
            },
        };
        let retry = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RetryCheck {
                policy_slot: SlotIdx::new(1),
                body: StepIdx::new(1),
                exhausted: StepIdx::new(3),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        };
        let parts = WorkflowParts {
            name: Box::from("zero_retry_policy"),
            digest: WorkflowDigest::from_bytes([8; 32]),
            nodes: Box::from([set_policy, action, retry, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::I64(0)]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
            input_slots: Box::from([]),
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

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
            constants: Box::from([ConstValue::Bool(true)]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
            input_slots: Box::from([]),
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn error_handler_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let guard = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(2),
                error_slot: None,
            },
        };
        let action = CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
        };
        let handler = CompiledNode {
            id: StepIdx::new(2),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let finish_node = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("error_handler"),
            digest: WorkflowDigest::from_bytes([3; 32]),
            nodes: Box::from([guard, action, handler, finish_node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::Bool(false)]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
            input_slots: Box::from([]),
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn wait_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
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
        let finish_node = CompiledNode {
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
            digest: WorkflowDigest::from_bytes([4; 32]),
            nodes: Box::from([set_deadline, wait, finish_node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::I64(10)]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
            input_slots: Box::from([]),
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

// vb-3l40a: validate_submit_admission_with_contracts capability coverage tests
#[cfg(test)]
mod vb_3l40a_contract_validation_tests {
    use super::*;
    use crate::shard::Shard;
    use crate::shard::ShardConfig;
    use crate::RuntimeError;
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};
    // ResourceContract imported via vb_core::workflow::ResourceContract

    fn do_workflow() -> CompiledWorkflow {
        let node = CompiledNode {
            id: vb_core::ids::StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ids::ActionId::new(0),
                input: SlotIdx::ZERO,
            },
        };
        let parts = WorkflowParts {
            name: Box::from("test-do"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([3u8; 32]),
            nodes: Box::from([node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 1,
            symbols_count: 0,
            entry: vb_core::ids::StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
            input_slots: Box::from([]),
        };
        CompiledWorkflow::try_from_parts(parts)
            .expect("test workflow must compile")
    }

    fn make_contract(
        id: vb_core::ids::ActionId,
        name: &str,
        required_caps: Box<[vb_core::capability::Capability]>,
    ) -> vb_core::action::ActionContract {
        vb_core::action::ActionContract {
            id,
            name: vb_core::action::ActionName::new(name).expect("test name"),
            input_slot_count: 0,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 1000,
            idempotency: vb_core::action::Idempotency::DeterministicPure,
            side_effect: vb_core::action::SideEffect::None,
            retry_safety: vb_core::action::RetrySafety::Safe,
            required_capabilities: required_caps,
        }
    }

    #[test]
    fn validate_rejects_insufficient_capability() {
        let shard_config = ShardConfig {
            command_queue_capacity: 64,
            trace_capacity: 64,
            step_budget_per_tick: 4,
            max_active_runs: 8,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(shard_config);
        let run = vb_core::ids::RunId::new(1);
        let workflow = do_workflow();

        // Contract requires a cap that is not granted
        let cap = vb_core::capability::Capability::new("storage".into(), vb_core::ids::ActionId::new(0));
        let contract = make_contract(vb_core::ids::ActionId::new(0), "test", Box::new([cap.clone()]));

        // Grant a different cap
        let granted =
            vb_core::capability::CapabilitySet::from_grants(Box::new([
                vb_core::capability::Capability::new("other".into(), vb_core::ids::ActionId::new(0)),
            ]));

        let result = shard.validate_submit_admission_with_contracts(
            run, &workflow, granted, &vec![contract],
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            RuntimeError::AdmissionCapabilityDenied {
                required, granted: _, ..
            } => {
                assert_eq!(required.name(), "storage");
            }
            _ => panic!("expected AdmissionCapabilityDenied, got: {err:?}"),
        }
    }

    #[test]
    fn validate_accepts_sufficient_capability() {
        let shard_config = ShardConfig {
            command_queue_capacity: 64,
            trace_capacity: 64,
            step_budget_per_tick: 4,
            max_active_runs: 8,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(shard_config);
        let run = vb_core::ids::RunId::new(2);
        let workflow = do_workflow();

        // Contract requires cap that is granted
        let cap = vb_core::capability::Capability::new("storage".into(), vb_core::ids::ActionId::new(0));
        let contract = make_contract(vb_core::ids::ActionId::new(0), "test", Box::new([cap.clone()]));

        let granted =
            vb_core::capability::CapabilitySet::from_grants(Box::new([cap.clone()]));

        let result = shard.validate_submit_admission_with_contracts(
            run, &workflow, granted, &vec![contract],
        );

        assert!(result.is_ok(), "expected Ok, got: {result:?}");
    }
}
