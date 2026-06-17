#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables,
)]
#![forbid(unsafe_code)]

use tempfile::TempDir;
use vb_core::{CapabilitySet, RunId, RuntimePolicy, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::recovery::{
    ActionReplayTracker, RecoveryHydration, RecoveryTerminalState, recover_full_journal,
    recover_runtime_summary,
};
use vb_storage::{EventSeq, FjallConfig, FjallJournal, JournalError, JournalEvent};

fn test_digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

fn open_journal(dir: &TempDir) -> Result<FjallJournal, String> {
    FjallJournal::open(dir.path(), Some(FjallConfig::default())).map_err(|error| error.to_string())
}

fn test_admission_event(run: RunId, seq: EventSeq, digest: WorkflowDigest) -> JournalEvent {
    JournalEvent::RunAdmission {
        run,
        seq,
        artifact_digest: digest,
        granted_capabilities: CapabilitySet::empty(),
        policy: RuntimePolicy::Relaxed,
    }
}

fn write_events_strict(journal: &FjallJournal, events: &[JournalEvent]) -> Result<(), String> {
    for event in events {
        journal
            .append_strict(event)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn resumed_run_events(run: RunId, digest: WorkflowDigest) -> Vec<JournalEvent> {
    vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        test_admission_event(run, EventSeq::new(1), digest),
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            attempt: 1,
            deadline_ms: 30000,
        },
        JournalEvent::RetryScheduledEvent {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(5),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(6),
            slot: SlotIdx::new(2),
            value: None,
            extra: None,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(7),
            step: StepIdx::new(1),
            output: SlotIdx::new(2),
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(8),
            result: SlotIdx::new(2),
            attempt: 1,
        },
    ]
}

#[test]
fn resume_tail_replays_exactly_when_journal_is_reopened() -> Result<(), String> {
    let dir = TempDir::new().map_err(|error| error.to_string())?;
    let run = RunId::new(16_200);
    let digest = test_digest(0x16);
    let expected = resumed_run_events(run, digest);

    {
        let journal = open_journal(&dir)?;
        write_events_strict(&journal, &expected)?;
    }

    let journal = open_journal(&dir)?;
    let recovered = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        recovered, expected,
        "reopened journal replay must preserve every pre-resume, resume-marker, and post-resume event"
    );

    let mut tracker = ActionReplayTracker::new();
    let full_replay = recover_full_journal(&journal, run, &mut tracker, &[], &[])
        .map_err(|error| error.to_string())?;
    assert_eq!(
        full_replay, expected,
        "full recovery replay must match the exact durable resume journal"
    );
    assert_eq!(
        tracker.is_resolved(vb_core::ActionId::new(1), StepIdx::ZERO),
        false,
        "timer/step resume replay must not invent resolved external actions"
    );

    let hydration = recover_runtime_summary(&journal, run).map_err(|error| error.to_string())?;
    let RecoveryHydration::Summary(summary) = hydration else {
        return Err(format!("expected summary hydration, got {hydration:?}"));
    };
    assert_eq!(summary.run, run);
    assert_eq!(summary.first_seq, EventSeq::new(0));
    assert_eq!(summary.last_seq, EventSeq::new(8));
    assert_eq!(summary.workflow, Some(digest));
    assert_eq!(summary.steps_started, 2);
    assert_eq!(summary.steps_succeeded, 1);
    assert_eq!(summary.suspensions, 2);
    assert_eq!(summary.slots_written, 1);
    assert_eq!(
        summary.terminal,
        Some(RecoveryTerminalState::Finished {
            result: SlotIdx::new(2)
        })
    );
    Ok(())
}

#[test]
fn resume_tail_replay_is_deterministic_when_read_twice() -> Result<(), String> {
    let dir = TempDir::new().map_err(|error| error.to_string())?;
    let run = RunId::new(16_201);
    let expected = resumed_run_events(run, test_digest(0x17));

    {
        let journal = open_journal(&dir)?;
        write_events_strict(&journal, &expected)?;
    }

    let (replay_a, full_a, action_resolved_a) = {
        let journal = open_journal(&dir)?;
        let replay = journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?;
        let mut tracker = ActionReplayTracker::new();
        let full = recover_full_journal(&journal, run, &mut tracker, &[], &[])
            .map_err(|error| error.to_string())?;
        let action_resolved = tracker.is_resolved(vb_core::ActionId::new(99), StepIdx::new(1));
        (replay, full, action_resolved)
    };

    let (replay_b, full_b, action_resolved_b) = {
        let journal = open_journal(&dir)?;
        let replay = journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?;
        let mut tracker = ActionReplayTracker::new();
        let full = recover_full_journal(&journal, run, &mut tracker, &[], &[])
            .map_err(|error| error.to_string())?;
        let action_resolved = tracker.is_resolved(vb_core::ActionId::new(99), StepIdx::new(1));
        (replay, full, action_resolved)
    };

    assert_eq!(replay_a, expected);
    assert_eq!(replay_b, expected);
    assert_eq!(replay_a, replay_b);
    assert_eq!(full_a, full_b);
    assert_eq!(action_resolved_a, action_resolved_b);
    Ok(())
}

#[test]
fn resume_tail_replay_rejects_sequence_gap_before_resume_continuation() -> Result<(), String> {
    let dir = TempDir::new().map_err(|error| error.to_string())?;
    let run = RunId::new(16_202);
    let digest = test_digest(0x18);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
            deadline_ms: 30000,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(1),
            attempt: 1,
        },
    ];

    {
        let journal = open_journal(&dir)?;
        write_events_strict(&journal, &events)?;
    }

    let journal = open_journal(&dir)?;
    let result = journal.events_for_run(run);
    let Err(JournalError::SequenceGap { expected, actual }) = result else {
        return Err(format!("expected SequenceGap, got {result:?}"));
    };
    assert_eq!(expected, EventSeq::new(2));
    assert_eq!(actual, EventSeq::new(3));

    let mut tracker = ActionReplayTracker::new();
    let full_result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);
    let Err(vb_storage::recovery::RecoveryError::Journal(JournalError::SequenceGap {
        expected,
        actual,
    })) = full_result
    else {
        return Err(format!(
            "expected recovery SequenceGap, got {full_result:?}"
        ));
    };
    assert_eq!(expected, EventSeq::new(2));
    assert_eq!(actual, EventSeq::new(3));
    Ok(())
}
