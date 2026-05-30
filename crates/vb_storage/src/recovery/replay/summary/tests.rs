#[cfg(test)]
mod tests {
    use super::*;
    use crate::EventSeq;
    use crate::recovery::types::RecoveryTerminalState;
    use vb_core::replay::SuspensionKind;
    use vb_core::{
        ActionId, CapabilitySet, FiniteF64, ListId, ObjectId, RunId, RuntimePolicy, SlotIdx,
        StepIdx, Taint, WorkflowDigest,
    };

    fn fresh_summary() -> RecoveryRuntimeSummary {
        RecoveryRuntimeSummary {
            run: RunId::new(1),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(0),
            workflow: None,
            steps_started: 0,
            steps_succeeded: 0,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        }
    }

    fn assert_counters(
        summary: &RecoveryRuntimeSummary,
        steps_started: u64,
        steps_succeeded: u64,
        actions_scheduled: u64,
        actions_resolved: u64,
        suspensions: u64,
        slots_written: u64,
    ) {
        assert_eq!(summary.steps_started, steps_started, "steps_started");
        assert_eq!(summary.steps_succeeded, steps_succeeded, "steps_succeeded");
        assert_eq!(
            summary.actions_scheduled, actions_scheduled,
            "actions_scheduled"
        );
        assert_eq!(
            summary.actions_resolved, actions_resolved,
            "actions_resolved"
        );
        assert_eq!(summary.suspensions, suspensions, "suspensions");
        assert_eq!(summary.slots_written, slots_written, "slots_written");
    }

    #[test]
    fn ask_answered_event_is_no_op() {
        let mut summary = fresh_summary();
        let event = JournalEvent::AskAnsweredEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        apply_summary_event(&mut summary, &event);
        assert_counters(&summary, 0, 0, 0, 0, 0, 0);
    }

    #[test]
    fn action_failed_event_increments_actions_resolved_only() {
        let mut summary = fresh_summary();
        let event = JournalEvent::ActionFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            action: ActionId::new(0),
            attempt: 1,
        };
        apply_summary_event(&mut summary, &event);
        assert_counters(&summary, 0, 0, 0, 1, 0, 0);
    }

    #[test]
    fn slot_written_event_increments_slots_written_only() {
        let mut summary = fresh_summary();
        let event = JournalEvent::SlotWrittenEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            slot: SlotIdx::new(0),
            value: None,
            extra: None,
            attempt: 1,
        };
        apply_summary_event(&mut summary, &event);
        assert_counters(&summary, 0, 0, 0, 0, 0, 1);
    }

    #[test]
    fn wait_scheduled_event_increments_suspensions() {
        let mut summary = fresh_summary();
        let event = JournalEvent::WaitScheduledEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        apply_summary_event(&mut summary, &event);
        assert_counters(&summary, 0, 0, 0, 0, 1, 0);
    }

    #[test]
    fn summary_events_cover_workflow_admission_retry_and_terminals() {
        let mut summary = fresh_summary();
        let run = RunId::new(1);
        let workflow = digest(7);
        let events = [
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(1),
                workflow,
            },
            JournalEvent::RunAdmission {
                run,
                seq: EventSeq::new(2),
                artifact_digest: digest(8),
                granted_capabilities: CapabilitySet::empty(),
                policy: RuntimePolicy::Strict,
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(1),
                attempt: 1,
            },
            JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(4),
                step: StepIdx::new(1),
                output: SlotIdx::new(2),
            },
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(5),
                step: StepIdx::new(1),
                action: ActionId::new(3),
                attempt: 1,
            },
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(6),
                step: StepIdx::new(1),
                action: ActionId::new(3),
                attempt: 1,
            },
            JournalEvent::RetryScheduledEvent {
                run,
                seq: EventSeq::new(7),
                step: StepIdx::new(1),
                attempt: 1,
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(8),
                result: SlotIdx::new(2),
                attempt: 1,
            },
        ];

        events
            .iter()
            .for_each(|event| apply_summary_event(&mut summary, event));

        assert_eq!(summary.workflow, Some(workflow));
        assert_counters(&summary, 1, 1, 1, 1, 1, 0);
        assert_eq!(
            summary.terminal,
            Some(RecoveryTerminalState::Finished {
                result: SlotIdx::new(2),
            })
        );

        apply_summary_event(
            &mut summary,
            &JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(9),
                attempt: 1,
                reason: None,
            },
        );
        assert_eq!(summary.terminal, Some(RecoveryTerminalState::Cancelled));

        apply_summary_event(
            &mut summary,
            &JournalEvent::RunFailedEvent {
                run,
                seq: EventSeq::new(10),
                attempt: 1,
            },
        );
        assert_eq!(summary.terminal, Some(RecoveryTerminalState::Failed));
    }

    #[test]
    fn recover_run_admission_returns_latest_metadata_or_none() {
        let run = RunId::new(12);
        let first = digest(1);
        let latest = digest(2);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: digest(9),
            },
            JournalEvent::RunAdmission {
                run,
                seq: EventSeq::new(1),
                artifact_digest: first,
                granted_capabilities: CapabilitySet::empty(),
                policy: RuntimePolicy::Relaxed,
            },
            JournalEvent::RunAdmission {
                run,
                seq: EventSeq::new(2),
                artifact_digest: latest,
                granted_capabilities: CapabilitySet::empty(),
                policy: RuntimePolicy::Journaled,
            },
        ];

        let recovered = recover_run_admission_from_events(&events);
        assert!(matches!(
            recovered,
            Some(RecoveredRunAdmission {
                artifact_digest,
                run_id,
                policy: RuntimePolicy::Journaled,
                ..
            }) if artifact_digest == latest && run_id == run
        ));
        assert_eq!(recover_run_admission_from_events(&events[0..1]), None);
    }

    #[test]
    fn replayed_object_slots_are_explicitly_unsupported() {
        let slots = recovered_single_slot(SlotValue::Object(ObjectId::new(7)));

        assert_eq!(slots, RecoveredSlots::unsupported());
    }

    #[test]
    fn replayed_list_slots_are_explicitly_unsupported() {
        let slots = recovered_single_slot(SlotValue::List(ListId::new(8)));

        assert_eq!(slots, RecoveredSlots::unsupported());
    }

    #[test]
    fn replayed_scalar_slots_remain_supported() {
        let slots = recovered_single_slot(SlotValue::I64(7));

        assert!(slots.fully_supported);
        assert_eq!(slots.entries.len(), 1);
    }

    #[test]
    fn replayed_all_scalar_slot_variants_remain_supported() {
        [
            SlotValue::Null,
            SlotValue::Bool(true),
            SlotValue::F64(finite_f64(1.25)),
            SlotValue::Symbol(vb_core::SymbolId::new(4)),
        ]
        .into_iter()
        .for_each(|value| {
            let slots = recovered_single_slot(value);
            assert!(slots.fully_supported, "scalar slot must be supported");
            assert_eq!(slots.entries.len(), 1);
        });
    }

    #[test]
    fn summarize_recovery_events_empty_returns_exact_no_recovery_data() {
        let result = summarize_recovery_events(&[]);

        assert!(matches!(
            result,
            Err(RecoveryError::NoRecoveryData { run }) if run == RunId::new(0)
        ));
    }

    #[test]
    fn frame_seed_empty_events_returns_exact_no_recovery_data() {
        let result = recover_runtime_frame_seed_from_events(&[]);

        assert!(matches!(
            result,
            Err(RecoveryError::NoRecoveryData { run }) if run == RunId::new(0)
        ));
    }

    #[test]
    fn frame_seed_builder_without_workflow_delegates_to_event_recovery() {
        let events = vec![JournalEvent::StepStarted {
            run: RunId::new(21),
            seq: EventSeq::new(0),
            step: StepIdx::new(5),
            attempt: 1,
        }];

        let direct = recover_runtime_frame_seed_from_events(&events);
        let built = RecoveryFrameSeedBuilder::new().build(&events);

        assert!(matches!((direct, built), (Ok(a), Ok(b)) if a == b));
    }

    #[test]
    fn workflow_digest_rejection_reports_exact_mismatch_and_accepts_match() {
        let expected = digest(11);
        let found = digest(12);
        let events = [JournalEvent::RunAccepted {
            run: RunId::new(31),
            seq: EventSeq::new(0),
            workflow: found,
        }];

        assert!(matches!(
            reject_workflow_digest_mismatch(&events, expected),
            Err(RecoveryError::CompiledIrDigestMismatch { expected: e, found: f })
                if e == expected && f == found
        ));
        assert_eq!(
            reject_workflow_digest_mismatch(&events, found).ok(),
            Some(())
        );
        assert_eq!(
            reject_workflow_digest_mismatch(&[], expected).ok(),
            Some(())
        );
    }

    #[test]
    fn frame_seed_slot_dimension_overflow_reports_exact_variant() {
        let run = RunId::new(41);
        let events = [JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            output: SlotIdx::MAX,
        }];

        // Frame seed now handles large slot indices gracefully instead of erroring
        let result = recover_runtime_frame_seed_from_events(&events);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let seed = result.unwrap();
        assert_eq!(seed.step_count, 1);
        assert!(
            seed.steps.iter().any(|entry| entry.step == StepIdx::new(0)
                && entry.state == RecoveredStepState::Succeeded)
        );
    }

    #[test]
    fn event_slot_values_cover_valid_corrupt_and_missing_frame_paths() {
        let run = RunId::new(51);
        let valid_bytes = encoded_slot_value(SlotValue::Bool(true));
        let events = vec![
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(0),
                slot: SlotIdx::new(0),
                value: Some(valid_bytes),
                extra: None,
                attempt: 1,
            },
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(1),
                slot: SlotIdx::new(1),
                value: Some(vec![255, 0, 255]),
                extra: None,
                attempt: 1,
            },
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(2),
                slot: SlotIdx::new(2),
                value: None,
                extra: None,
                attempt: 1,
            },
        ];

        let seed = recover_runtime_frame_seed_from_events(&events);

        let recovered = seed.expect("should recover successfully");
        assert!(
            recovered
                .slots
                .iter()
                .any(|entry| entry.slot == SlotIdx::new(0)
                    && entry.value == SlotValue::Bool(true)
                    && entry.taint == Taint::DerivedFromSecret),
            "Expected slot 0 with Bool(true) and DerivedFromSecret taint"
        );
        assert!(
            recovered.unsupported.slot_values,
            "Expected slot_values unsupported"
        );
    }

    #[test]
    fn unresolved_action_marks_pending_action_recovery_unsupported() {
        let run = RunId::new(61);
        let events = [JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(3),
            action: ActionId::new(9),
            attempt: 1,
        }];

        let seed = recover_runtime_frame_seed_from_events(&events);

        assert!(
            matches!(seed, Ok(recovered) if recovered.pending_actions.iter().any(|entry|
            entry.step == StepIdx::new(3) && entry.action == ActionId::new(9)
        ) && recovered.unsupported.pending_actions)
        );
    }

    #[test]
    fn replay_step_not_found_maps_to_exact_recovery_error() {
        assert_replay_divergence(
            ReplayError::StepNotFound {
                step: StepIdx::new(9),
            },
            StepIdx::new(9),
            "replay step not found in compiled workflow",
        );
    }

    #[test]
    fn replay_non_deterministic_maps_to_exact_recovery_error() {
        assert_replay_divergence(
            ReplayError::NonDeterministicStep {
                step: StepIdx::new(4),
                kind: SuspensionKind::AskPending,
            },
            StepIdx::new(4),
            "replay blocked by non-deterministic Ask step",
        );
    }

    #[test]
    fn replay_non_deterministic_mapping_uses_all_typed_kind_names() {
        let cases = [
            (
                SuspensionKind::ActionPending,
                "replay blocked by non-deterministic Do step",
            ),
            (
                SuspensionKind::AskPending,
                "replay blocked by non-deterministic Ask step",
            ),
            (
                SuspensionKind::WaitUntil,
                "replay blocked by non-deterministic WaitUntil step",
            ),
            (
                SuspensionKind::WaitEvent,
                "replay blocked by non-deterministic WaitEvent step",
            ),
        ];

        cases.into_iter().for_each(|(kind, detail)| {
            assert_replay_divergence(
                ReplayError::NonDeterministicStep {
                    step: StepIdx::new(4),
                    kind,
                },
                StepIdx::new(4),
                detail,
            );
        });
    }

    #[test]
    fn replay_slot_not_available_maps_to_exact_recovery_error() {
        assert_replay_divergence(
            ReplayError::SlotNotAvailable {
                slot: SlotIdx::new(3),
            },
            StepIdx::ZERO,
            "replay required unavailable slot SlotIdx(3)",
        );
    }

    #[test]
    fn replay_expression_error_maps_to_exact_recovery_error() {
        assert_replay_divergence(
            ReplayError::ExpressionEvalFailed {
                step: StepIdx::new(6),
            },
            StepIdx::new(6),
            "replay expression evaluation failed",
        );
    }

    #[test]
    fn replay_internal_error_maps_to_exact_recovery_error() {
        assert_replay_divergence(
            ReplayError::Internal {
                reason: "arena handle recovery unsupported",
            },
            StepIdx::ZERO,
            "arena handle recovery unsupported",
        );
    }

    #[test]
    fn recovery_summary_multiple_runs_reports_exact_divergence_detail() {
        let events = two_run_events();

        assert!(matches!(
            summarize_recovery_events(&events),
            Err(RecoveryError::ReplayDivergence { step, detail })
                if step == StepIdx::ZERO
                    && detail == "recovery summary received events for multiple runs"
        ));
    }

    #[test]
    fn frame_seed_multiple_runs_reports_exact_divergence_detail() {
        let events = two_run_events();

        assert!(matches!(
            recover_runtime_frame_seed_from_events(&events),
            Err(RecoveryError::ReplayDivergence { step, detail })
                if step == StepIdx::ZERO
                    && detail == "frame seed recovery received events for multiple runs"
        ));
    }

    fn recovered_single_slot(value: SlotValue) -> RecoveredSlots {
        RecoveredSlots::from_replayed(vec![RecoveredSlotEntry {
            slot: SlotIdx::new(0),
            value,
            taint: Taint::Secret,
        }])
    }

    fn digest(byte: u8) -> WorkflowDigest {
        WorkflowDigest::from_bytes([byte; 32])
    }

    fn encoded_slot_value(value: SlotValue) -> Vec<u8> {
        match postcard::to_allocvec(&value) {
            Ok(bytes) => bytes,
            Err(error) => panic!("slot value encoding failed: {error}"),
        }
    }

    fn finite_f64(value: f64) -> FiniteF64 {
        match FiniteF64::new(value) {
            Ok(finite) => finite,
            Err(error) => panic!("finite test value rejected: {error}"),
        }
    }

    fn two_run_events() -> [JournalEvent; 2] {
        [
            JournalEvent::StepStarted {
                run: RunId::new(1),
                seq: EventSeq::new(0),
                step: StepIdx::new(0),
                attempt: 1,
            },
            JournalEvent::StepStarted {
                run: RunId::new(2),
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 1,
            },
        ]
    }

    fn assert_replay_divergence(error: ReplayError, step: StepIdx, detail: &str) {
        assert!(matches!(
            replay_error_to_recovery(error),
            RecoveryError::ReplayDivergence { step: s, detail: d }
                if s == step && d == detail
        ));
    }
}