#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use proptest::proptest;
    use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::value::ConstValue;
    use vb_storage::JournalEvent;
    use vb_storage::types::EventSeq;

    fn make_run_id(v: u64) -> RunId {
        RunId::new(v)
    }

    fn make_event_seq(v: u64) -> EventSeq {
        EventSeq::new(v)
    }

    fn make_step_idx(v: u16) -> StepIdx {
        StepIdx::new(v)
    }

    fn make_action_id(v: u16) -> ActionId {
        ActionId::new(v)
    }

    fn make_slot_idx(v: u16) -> SlotIdx {
        SlotIdx::new(v)
    }

    fn dummy_digest() -> WorkflowDigest {
        WorkflowDigest::from_bytes([0xAB_u8; 32])
    }

    // -------------------------------------------------------------------------
    // build_trace — pure behavior
    // -------------------------------------------------------------------------

    #[test]
    fn build_trace_returns_identical_output_for_identical_events() {
        let events = [
            JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(0),
                workflow: dummy_digest(),
            },
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(1),
                step: make_step_idx(0),
                attempt: 1,
            },
            JournalEvent::StepSucceeded {
                run: make_run_id(1),
                seq: make_event_seq(2),
                step: make_step_idx(0),
                output: make_slot_idx(0),
            },
        ];
        let trace = build_trace(&events);
        assert_eq!(trace.len(), 3);
        assert_eq!(trace[0].event_type, "RunAccepted");
        assert_eq!(trace[0].step, None);
        assert_eq!(trace[1].event_type, "StepStarted");
        assert_eq!(trace[1].step, Some(0));
        assert_eq!(trace[2].event_type, "StepSucceeded");
        assert_eq!(trace[2].step, Some(0));
    }

    #[test]
    fn build_trace_preserves_event_order() {
        let events = [
            JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(0),
                workflow: dummy_digest(),
            },
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(1),
                step: make_step_idx(0),
                attempt: 1,
            },
            JournalEvent::StepSucceeded {
                run: make_run_id(1),
                seq: make_event_seq(2),
                step: make_step_idx(0),
                output: make_slot_idx(0),
            },
            JournalEvent::ActionScheduled {
                run: make_run_id(1),
                seq: make_event_seq(3),
                step: make_step_idx(1),
                action: make_action_id(1),
                attempt: 1,
            },
        ];
        let trace = build_trace(&events);
        for (i, entry) in trace.iter().enumerate() {
            assert_eq!(entry.index, i, "index must match position for entry {i}");
        }
    }

    #[test]
    fn build_trace_empty_input_returns_empty_output() {
        let events: [JournalEvent; 0] = [];
        let trace = build_trace(&events);
        assert!(trace.is_empty());
    }

    #[test]
    fn build_trace_step_events_have_step_value() {
        let events = [
            JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(0),
                workflow: dummy_digest(),
            },
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(1),
                step: make_step_idx(5),
                attempt: 1,
            },
        ];
        let trace = build_trace(&events);
        assert_eq!(trace[1].step, Some(5));
    }

    #[test]
    fn build_trace_run_level_events_have_no_step() {
        let events = [
            JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(0),
                workflow: dummy_digest(),
            },
            JournalEvent::RunFinished {
                run: make_run_id(1),
                seq: make_event_seq(1),
                result: make_slot_idx(0),
                attempt: 1,
            },
        ];
        let trace = build_trace(&events);
        assert_eq!(trace[0].step, None);
        assert_eq!(trace[1].step, None);
    }

    #[test]
    fn build_trace_maps_seq_field_correctly() {
        let events = [JournalEvent::RunAccepted {
            run: make_run_id(1),
            seq: make_event_seq(42),
            workflow: dummy_digest(),
        }];
        let trace = build_trace(&events);
        assert_eq!(trace[0].seq, 42);
    }

    #[test]
    fn filter_trace_by_step_preserves_original_indices() {
        let events = [
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(1),
                step: make_step_idx(1),
                attempt: 1,
            },
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(2),
                step: make_step_idx(7),
                attempt: 1,
            },
        ];
        let filtered = filter_trace(
            build_trace(&events),
            TraceFilters {
                step: Some(7),
                ..TraceFilters::default()
            },
        );

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].index, 1);
        assert_eq!(filtered[0].step, Some(7));
    }

    #[test]
    fn filter_trace_by_action_matches_action_id_only() {
        let events = [
            JournalEvent::ActionScheduled {
                run: make_run_id(1),
                seq: make_event_seq(1),
                step: make_step_idx(2),
                action: make_action_id(3),
                attempt: 1,
            },
            JournalEvent::ActionFailedEvent {
                run: make_run_id(1),
                seq: make_event_seq(2),
                step: make_step_idx(2),
                action: make_action_id(9),
                attempt: 1,
            },
        ];
        let filtered = filter_trace(
            build_trace(&events),
            TraceFilters {
                action: Some(9),
                ..TraceFilters::default()
            },
        );

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].action, Some(9));
        assert_eq!(filtered[0].event_type, "ActionFailed");
    }

    #[test]
    fn filter_trace_by_status_uses_event_status_categories() {
        let events = [
            JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(0),
                workflow: dummy_digest(),
            },
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(1),
                step: make_step_idx(0),
                attempt: 1,
            },
            JournalEvent::RunFinished {
                run: make_run_id(1),
                seq: make_event_seq(2),
                result: make_slot_idx(0),
                attempt: 1,
            },
        ];
        let filtered = filter_trace(
            build_trace(&events),
            TraceFilters {
                status: Some(TraceStatus::Completed),
                ..TraceFilters::default()
            },
        );

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].event_type, "RunFinished");
        assert_eq!(filtered[0].status, Some(TraceStatus::Completed));
    }

    #[test]
    fn filter_trace_limit_applies_after_filters() {
        let events = [
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(1),
                step: make_step_idx(0),
                attempt: 1,
            },
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(2),
                step: make_step_idx(0),
                attempt: 1,
            },
            JournalEvent::RunFinished {
                run: make_run_id(1),
                seq: make_event_seq(3),
                result: make_slot_idx(0),
                attempt: 1,
            },
        ];
        let filtered = filter_trace(
            build_trace(&events),
            TraceFilters {
                status: Some(TraceStatus::Active),
                limit: Some(1),
                ..TraceFilters::default()
            },
        );

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].seq, 1);
    }

    #[test]
    fn filter_trace_by_sequence_range_is_inclusive() {
        let events = [
            JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(9),
                workflow: dummy_digest(),
            },
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(10),
                step: make_step_idx(0),
                attempt: 1,
            },
            JournalEvent::StepSucceeded {
                run: make_run_id(1),
                seq: make_event_seq(20),
                step: make_step_idx(0),
                output: make_slot_idx(0),
            },
            JournalEvent::RunFinished {
                run: make_run_id(1),
                seq: make_event_seq(21),
                result: make_slot_idx(0),
                attempt: 1,
            },
        ];

        let filtered = filter_trace(
            build_trace(&events),
            TraceFilters {
                since_seq: Some(10),
                until_seq: Some(20),
                ..TraceFilters::default()
            },
        );

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].seq, 10);
        assert_eq!(filtered[1].seq, 20);
    }

    #[test]
    fn build_trace_run_resumed_uses_event_seq() {
        let events = [JournalEvent::RunResumed {
            run: make_run_id(1),
            seq: make_event_seq(20),
            timestamp: Utc::now(),
        }];
        let trace = build_trace(&events);
        assert_eq!(trace[0].seq, 20);
        assert_eq!(trace[0].event_type, "RunResumed");
    }

    #[test]
    fn build_trace_length_matches_input() {
        let events: Vec<JournalEvent> = (0..10)
            .map(|i| JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(i),
                workflow: dummy_digest(),
            })
            .collect();
        let trace = build_trace(&events);
        assert_eq!(trace.len(), events.len());
    }

    // -------------------------------------------------------------------------
    // trace_one — per-variant correctness
    // -------------------------------------------------------------------------

    #[test]
    fn trace_one_run_accepted_maps_correct_fields() {
        let event = JournalEvent::RunAccepted {
            run: make_run_id(1),
            seq: make_event_seq(5),
            workflow: dummy_digest(),
        };
        let entry = trace_one(0, &event);
        assert_eq!(entry.event_type, "RunAccepted");
        assert_eq!(entry.seq, 5);
        assert_eq!(entry.step, None);
        assert_eq!(entry.index, 0);
    }

    #[test]
    fn trace_one_run_admission_maps_correct_fields() {
        let event = JournalEvent::RunAdmission {
            run: make_run_id(2),
            seq: make_event_seq(3),
            artifact_digest: dummy_digest(),
            granted_capabilities: vb_core::CapabilitySet::empty(),
            policy: vb_core::RuntimePolicy::Strict,
        };
        let entry = trace_one(1, &event);
        assert_eq!(entry.event_type, "RunAdmission");
        assert_eq!(entry.seq, 3);
        assert_eq!(entry.step, None);
        assert_eq!(entry.index, 1);
    }

    #[test]
    fn trace_one_step_started_maps_correct_fields() {
        let event = JournalEvent::StepStarted {
            run: make_run_id(1),
            seq: make_event_seq(7),
            step: make_step_idx(3),
            attempt: 2,
        };
        let entry = trace_one(2, &event);
        assert_eq!(entry.event_type, "StepStarted");
        assert_eq!(entry.seq, 7);
        assert_eq!(entry.step, Some(3));
        assert_eq!(entry.index, 2);
    }

    #[test]
    fn trace_one_step_succeeded_maps_correct_fields() {
        let event = JournalEvent::StepSucceeded {
            run: make_run_id(1),
            seq: make_event_seq(8),
            step: make_step_idx(3),
            output: make_slot_idx(1),
        };
        let entry = trace_one(3, &event);
        assert_eq!(entry.event_type, "StepSucceeded");
        assert_eq!(entry.seq, 8);
        assert_eq!(entry.step, Some(3));
        assert_eq!(entry.index, 3);
    }

    #[test]
    fn trace_one_action_scheduled_maps_correct_fields() {
        let event = JournalEvent::ActionScheduled {
            run: make_run_id(1),
            seq: make_event_seq(9),
            step: make_step_idx(2),
            action: make_action_id(4),
            attempt: 1,
        };
        let entry = trace_one(4, &event);
        assert_eq!(entry.event_type, "ActionScheduled");
        assert_eq!(entry.seq, 9);
        assert_eq!(entry.step, Some(2));
        assert_eq!(entry.index, 4);
    }

    #[test]
    fn trace_one_action_completed_maps_correct_fields() {
        let event = JournalEvent::ActionCompletedEvent {
            run: make_run_id(1),
            seq: make_event_seq(10),
            step: make_step_idx(2),
            action: make_action_id(4),
            attempt: 1,
        };
        let entry = trace_one(5, &event);
        assert_eq!(entry.event_type, "ActionCompleted");
        assert_eq!(entry.seq, 10);
        assert_eq!(entry.step, Some(2));
        assert_eq!(entry.index, 5);
    }

    #[test]
    fn trace_one_action_failed_maps_correct_fields() {
        let event = JournalEvent::ActionFailedEvent {
            run: make_run_id(1),
            seq: make_event_seq(11),
            step: make_step_idx(2),
            action: make_action_id(4),
            attempt: 1,
        };
        let entry = trace_one(6, &event);
        assert_eq!(entry.event_type, "ActionFailed");
        assert_eq!(entry.seq, 11);
        assert_eq!(entry.step, Some(2));
        assert_eq!(entry.index, 6);
    }

    #[test]
    fn trace_one_slot_written_maps_correct_fields() {
        let event = JournalEvent::SlotWrittenEvent {
            run: make_run_id(1),
            seq: make_event_seq(12),
            slot: make_slot_idx(3),
            value: Some(vec![0x01, 0x02]),
            extra: None,
            attempt: 1,
        };
        let entry = trace_one(7, &event);
        assert_eq!(entry.event_type, "SlotWritten");
        assert_eq!(entry.seq, 12);
        assert_eq!(entry.step, None);
        assert_eq!(entry.index, 7);
    }

    #[test]
    fn trace_one_wait_scheduled_maps_correct_fields() {
        let event = JournalEvent::WaitScheduledEvent {
            run: make_run_id(1),
            seq: make_event_seq(13),
            step: make_step_idx(4),
            attempt: 1,
        };
        let entry = trace_one(8, &event);
        assert_eq!(entry.event_type, "WaitScheduled");
        assert_eq!(entry.seq, 13);
        assert_eq!(entry.step, Some(4));
        assert_eq!(entry.index, 8);
    }

    #[test]
    fn trace_one_ask_scheduled_maps_correct_fields() {
        let event = JournalEvent::AskScheduledEvent {
            run: make_run_id(1),
            seq: make_event_seq(14),
            step: make_step_idx(5),
            attempt: 1,
        };
        let entry = trace_one(9, &event);
        assert_eq!(entry.event_type, "AskScheduled");
        assert_eq!(entry.seq, 14);
        assert_eq!(entry.step, Some(5));
        assert_eq!(entry.index, 9);
    }

    #[test]
    fn trace_one_ask_answered_maps_correct_fields() {
        let event = JournalEvent::AskAnsweredEvent {
            run: make_run_id(1),
            seq: make_event_seq(15),
            step: make_step_idx(5),
            attempt: 1,
        };
        let entry = trace_one(10, &event);
        assert_eq!(entry.event_type, "AskAnswered");
        assert_eq!(entry.seq, 15);
        assert_eq!(entry.step, Some(5));
        assert_eq!(entry.index, 10);
    }

    #[test]
    fn trace_one_retry_scheduled_maps_correct_fields() {
        let event = JournalEvent::RetryScheduledEvent {
            run: make_run_id(1),
            seq: make_event_seq(16),
            step: make_step_idx(3),
            attempt: 1,
        };
        let entry = trace_one(11, &event);
        assert_eq!(entry.event_type, "RetryScheduled");
        assert_eq!(entry.seq, 16);
        assert_eq!(entry.step, Some(3));
        assert_eq!(entry.index, 11);
    }

    #[test]
    fn trace_one_run_cancelled_maps_correct_fields() {
        let event = JournalEvent::RunCancelled {
            run: make_run_id(1),
            seq: make_event_seq(17),
            attempt: 1,
            reason: Some("user requested".to_string()),
        };
        let entry = trace_one(12, &event);
        assert_eq!(entry.event_type, "RunCancelled");
        assert_eq!(entry.seq, 17);
        assert_eq!(entry.step, None);
        assert_eq!(entry.index, 12);
    }

    #[test]
    fn trace_one_run_finished_maps_correct_fields() {
        let event = JournalEvent::RunFinished {
            run: make_run_id(1),
            seq: make_event_seq(18),
            result: make_slot_idx(7),
            attempt: 1,
        };
        let entry = trace_one(13, &event);
        assert_eq!(entry.event_type, "RunFinished");
        assert_eq!(entry.seq, 18);
        assert_eq!(entry.step, None);
        assert_eq!(entry.index, 13);
    }

    #[test]
    fn trace_one_run_failed_maps_correct_fields() {
        let event = JournalEvent::RunFailedEvent {
            run: make_run_id(1),
            seq: make_event_seq(19),
            attempt: 1,
        };
        let entry = trace_one(14, &event);
        assert_eq!(entry.event_type, "RunFailed");
        assert_eq!(entry.seq, 19);
        assert_eq!(entry.step, None);
        assert_eq!(entry.index, 14);
    }

    #[test]
    fn trace_one_run_resumed_uses_event_seq() {
        let event = JournalEvent::RunResumed {
            run: make_run_id(1),
            seq: make_event_seq(20),
            timestamp: Utc::now(),
        };
        let entry = trace_one(15, &event);
        assert_eq!(entry.event_type, "RunResumed");
        assert_eq!(entry.seq, 20);
        assert_eq!(entry.step, None);
        assert_eq!(entry.index, 15);
    }

    #[test]
    fn trace_one_run_retried_uses_event_seq() {
        let event = JournalEvent::RunRetried {
            run: make_run_id(1),
            seq: make_event_seq(21),
            timestamp: Utc::now(),
        };
        let entry = trace_one(16, &event);
        assert_eq!(entry.event_type, "RunRetried");
        assert_eq!(entry.seq, 21);
        assert_eq!(entry.step, None);
        assert_eq!(entry.index, 16);
    }

    #[test]
    fn trace_one_run_answered_uses_event_seq() {
        let event = JournalEvent::RunAnswered {
            run: make_run_id(1),
            seq: make_event_seq(22),
            slot_idx: make_slot_idx(2),
            answer: ConstValue::Null,
            timestamp: Utc::now(),
        };
        let entry = trace_one(17, &event);
        assert_eq!(entry.event_type, "RunAnswered");
        assert_eq!(entry.seq, 22);
        assert_eq!(entry.step, None);
        assert_eq!(entry.index, 17);
    }

    // -------------------------------------------------------------------------
    // Invariants — deterministic, length-preserving, index-correct
    // -------------------------------------------------------------------------

    #[test]
    fn build_trace_is_deterministic() {
        // Test with a mixed vector of events
        let events = vec![
            JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(0),
                workflow: dummy_digest(),
            },
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(1),
                step: make_step_idx(0),
                attempt: 1,
            },
            JournalEvent::StepSucceeded {
                run: make_run_id(1),
                seq: make_event_seq(2),
                step: make_step_idx(0),
                output: make_slot_idx(0),
            },
            JournalEvent::RunFinished {
                run: make_run_id(1),
                seq: make_event_seq(3),
                result: make_slot_idx(0),
                attempt: 1,
            },
        ];
        let trace1 = build_trace(&events);
        let trace2 = build_trace(&events);
        assert_eq!(trace1, trace2, "build_trace must be deterministic");
    }

    #[test]
    fn build_trace_preserves_length_empty() {
        let events: Vec<JournalEvent> = vec![];
        let trace = build_trace(&events);
        assert_eq!(trace.len(), 0);
    }

    #[test]
    fn build_trace_preserves_length_small() {
        let events = vec![
            JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(0),
                workflow: dummy_digest(),
            },
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(1),
                step: make_step_idx(0),
                attempt: 1,
            },
        ];
        let trace = build_trace(&events);
        assert_eq!(trace.len(), 2);
    }

    #[test]
    fn build_trace_preserves_length_large() {
        let events: Vec<JournalEvent> = (0..50)
            .map(|i| JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(i),
                workflow: dummy_digest(),
            })
            .collect();
        let trace = build_trace(&events);
        assert_eq!(trace.len(), 50);
    }

    proptest! {
        #[test]
        fn trace_entry_index_matches_position(idx in 0usize..1000_usize) {
            // Create a deterministic event for this index
            let event = JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(idx as u64),
                workflow: dummy_digest(),
            };
            let entry = trace_one(idx, &event);
            assert_eq!(entry.index, idx, "trace_one index must match provided index");
        }
    }
}
