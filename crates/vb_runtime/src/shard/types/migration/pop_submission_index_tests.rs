mod pop_submission_index_tests {
    use super::*;
    use vb_core::workflow::{
        CompiledInputConstraint, CompiledInputSlot, CompiledNodeKind, InputSlotKind,
        ResourceContract, WorkflowParts,
    };
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn submission() -> Result<(ShardCommand, PendingQuotaReservation), Box<dyn std::error::Error>> {
        let action = ActionId::new(0);
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            semantic_source_identity: None,
            source_metadata: None,
            name: Box::from("queue_submission_custody"),
            digest: WorkflowDigest::from_bytes([97; 32]),
            nodes: Box::from([vb_core::workflow::node(
                StepIdx::ZERO,
                CompiledNodeKind::Do {
                    action,
                    input: SlotIdx::ZERO,
                },
                Some(SlotIdx::ZERO),
                None,
            )]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
            input_slots: Box::from([CompiledInputSlot {
                slot: SlotIdx::ZERO,
                kind: InputSlotKind::Any,
                constraint: CompiledInputConstraint::required(),
            }]),
        })
        .map_err(|error| format!("{error:?}"))?;
        let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&workflow)
            .map_err(|error| format!("{error:?}"))?
            .max_cost_units;
        assert!(cost > 0);
        let tenant = vb_core::ids::TenantId::new(97);
        Ok((
            ShardCommand::SubmitTenantWithInputsAndContracts {
                run: RunId::new(82_201),
                workflow,
                inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
                caps: crate::shard::tests::test_fixtures::contract_capability_for(action),
                action_contracts: crate::shard::tests::test_fixtures::contracts_through(action),
                tenant,
            },
            PendingQuotaReservation {
                tenant: Some(tenant),
                cost_units: cost,
            },
        ))
    }

    fn assert_submission_count(queue: &ShardCommandQueue, run: RunId, count: Option<u32>) {
        assert_eq!(
            ShardCommandQueue::lock_or_recover(&queue.pending_submissions)
                .get(&run)
                .copied(),
            count
        );
    }

    #[test]
    fn real_submit_dequeue_decrements_duplicate_index_without_releasing_custody() -> TestResult {
        let (command, quota) = submission()?;
        let run = RunId::new(82_201);
        let queue = ShardCommandQueue::new(3)?;
        let ledger = crate::quota::SharedQuotaLedger::new();
        ledger.set_limits(quota.cost_units, quota.cost_units)?;
        for _ in 0..2 {
            queue.enqueue_with_reservation(command.clone(), Some(3), Some(quota), Some(&ledger))?;
        }
        assert_submission_count(&queue, run, Some(2));
        assert_eq!(queue.pending_for(run), 2);
        assert_eq!(ledger.global_usage(), quota.cost_units);
        let first = queue
            .pop_unfenced_queued_observed(|| {
                assert!(matches!(
                    queue.pending_submissions.try_lock(),
                    Err(std::sync::TryLockError::WouldBlock)
                ));
            })
            .ok_or("missing first submission")?;
        assert_eq!(first.command, command);
        assert_eq!(first.quota, Some(quota));
        assert_submission_count(&queue, run, Some(1));
        assert_eq!(queue.pending_for(run), 1);
        assert_eq!(ledger.global_usage(), quota.cost_units);
        // A duplicate arriving while one remains must still be unreserved.
        queue.enqueue_with_reservation(command.clone(), Some(3), Some(quota), Some(&ledger))?;
        assert_submission_count(&queue, run, Some(2));
        for remaining in [Some(1), None] {
            let next = queue.pop_queued().ok_or("missing unreserved duplicate")?;
            assert_eq!(next.command, command);
            assert_eq!(next.quota, None);
            assert_submission_count(&queue, run, remaining);
            assert_eq!(ledger.global_usage(), quota.cost_units);
        }
        assert_eq!(queue.pending_for(run), 0);
        assert!(
            ShardCommandQueue::lock_or_recover(&queue.admitted_depth)
                .get(&run)
                .is_none()
        );
        assert!(queue.is_empty());
        // Dequeue returned responsibility to this test, not to a fake runtime.
        let held = first.quota.ok_or("missing returned custody")?;
        ledger.release(held.tenant, held.cost_units)?;
        assert_eq!(ledger.global_usage(), 0);
        queue.enqueue_with_reservation(command.clone(), Some(3), Some(quota), Some(&ledger))?;
        assert_submission_count(&queue, run, Some(1));
        assert_eq!(ledger.global_usage(), quota.cost_units);
        let fresh = queue
            .pop_queued()
            .ok_or("missing fresh reserved submission")?;
        assert_eq!(fresh.command, command);
        assert_eq!(fresh.quota, Some(quota));
        assert_submission_count(&queue, run, None);
        ledger.release(quota.tenant, quota.cost_units)?;
        assert_eq!(ledger.global_usage(), 0);
        Ok(())
    }
}
