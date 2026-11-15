#![forbid(unsafe_code)]

use super::event_to_json;
use serde_json::json;
use vb_core::{ActionId, ActionTicket, RunId, SeqNo, SlotIdx, StepIdx, Taint, WorkflowDigest};
use vb_storage::{DurableActionOutcome, EventSeq, JournalEvent};

#[test]
fn suspension_and_action_events_preserve_distinct_ids_and_attempts() {
    let run = RunId::new(51);
    let seq = EventSeq::new(61);
    let step = StepIdx::new(7);
    let action = ActionId::new(9);
    let attempt = 3;
    for (event, name) in [
        (
            JournalEvent::WaitScheduledEvent {
                run,
                seq,
                step,
                attempt,
            },
            "WaitScheduled",
        ),
        (
            JournalEvent::WaitResolvedEvent {
                run,
                seq,
                step,
                attempt,
            },
            "WaitResolved",
        ),
        (
            JournalEvent::AskScheduledEvent {
                run,
                seq,
                step,
                attempt,
            },
            "AskScheduled",
        ),
        (
            JournalEvent::AskAnsweredEvent {
                run,
                seq,
                step,
                attempt,
            },
            "AskAnswered",
        ),
        (
            JournalEvent::AskTimedOutEvent {
                run,
                seq,
                step,
                attempt,
            },
            "AskTimedOut",
        ),
        (
            JournalEvent::RetryScheduledEvent {
                run,
                seq,
                step,
                attempt,
            },
            "RetryScheduled",
        ),
    ] {
        assert_eq!(
            event_to_json(&event),
            json!({"type":name,"run":51,"seq":61,"step":7,"attempt":3})
        );
    }
    for (event, name) in [
        (
            JournalEvent::ActionScheduled {
                run,
                seq,
                step,
                action,
                attempt,
            },
            "ActionScheduled",
        ),
        (
            JournalEvent::ActionCompletedEvent {
                run,
                seq,
                step,
                action,
                attempt,
            },
            "ActionCompleted",
        ),
        (
            JournalEvent::ActionFailedEvent {
                run,
                seq,
                step,
                action,
                attempt,
            },
            "ActionFailed",
        ),
    ] {
        assert_eq!(
            event_to_json(&event),
            json!({"type":name,"run":51,"seq":61,"step":7,"action":9,"attempt":3})
        );
    }
}

fn ticket() -> ActionTicket {
    ActionTicket {
        run: RunId::new(71),
        step: StepIdx::new(5),
        seq: SeqNo::new(73),
        action: ActionId::new(9),
        run_attempt: 2,
        attempt: 3,
        idempotency_key: 75,
        capacity: 4,
        fencing_token: 77,
    }
}

#[test]
fn action_envelopes_retain_payload_ticket_and_distinct_digests() {
    let ticket = ticket();
    let run = ticket.run;
    let seq = EventSeq::new(81);
    let input = SlotIdx::new(11);
    let output = SlotIdx::new(13);
    let action_abi_digest = WorkflowDigest::from_bytes([0x31; 32]);
    let value_digest = [0x47; 32];
    let cases = [
        (
            JournalEvent::ActionScheduledTicket {
                run,
                seq,
                ticket,
                input,
                output,
                action_abi_digest,
            },
            json!({"type":"ActionScheduledTicket","run":71,"seq":81,"attempt":2,"ticket":format!("{ticket:?}"),"input":11,"output":13,"action_abi_digest":format!("{action_abi_digest:?}")}),
        ),
        (
            JournalEvent::ActionCompletedEnvelope {
                run,
                seq,
                ticket,
                output,
                outcome: DurableActionOutcome::Ready,
                value: vec![3, 5, 7],
                encoded_len: 3,
                taint: Taint::Secret,
                value_digest,
                action_abi_digest,
            },
            json!({"type":"ActionCompletedEnvelope","run":71,"seq":81,"attempt":2,"ticket":format!("{ticket:?}"),"output":13,"outcome":"Ready","value":[3,5,7],"encoded_len":3,"taint":"Secret","value_digest":format!("{value_digest:?}"),"action_abi_digest":format!("{action_abi_digest:?}")}),
        ),
        (
            JournalEvent::ActionAbandoned { run, seq, ticket },
            json!({"type":"ActionAbandoned","run":71,"seq":81,"attempt":2,"ticket":format!("{ticket:?}")}),
        ),
    ];
    for (event, expected) in cases {
        assert_eq!(event_to_json(&event), expected);
    }
}

#[test]
fn budget_and_checkpoint_events_project_current_data() -> Result<(), Box<dyn std::error::Error>> {
    use vb_core::{AggregateResourceBudget, CapabilitySet, RuntimePolicy, TenantId};
    use vb_storage::{RunInputBinding, RunPolicyPin};
    let artifact = WorkflowDigest::from_bytes([0x11; 32]);
    let input = RunInputBinding::Digest(WorkflowDigest::from_bytes([0x22; 32]));
    let pin = RunPolicyPin::Pinned(WorkflowDigest::from_bytes([0x33; 32]));
    let workflow = vb_compile::compile_workflow(b"version: velvet-ballistics/v1\nname: projection_budget\nwhen: {manual: {}}\nsteps:\n  - id: value\n    set: {output: answer, value: '7'}\n  - id: done\n    finish: {result: answer}\n")?;
    let budget = AggregateResourceBudget::from_workflow(&workflow)?;
    let capabilities = CapabilitySet::empty();
    let policy = RuntimePolicy::Strict;
    let run = RunId::new(91);
    let seq = EventSeq::new(93);
    let cases = [
        (
            JournalEvent::RunAccepted {
                run,
                seq,
                workflow: artifact,
            },
            json!({"type":"RunAccepted","run":91,"seq":93,"workflow":format!("{artifact:?}")}),
        ),
        (
            JournalEvent::RunAdmissionBudget {
                run,
                seq,
                artifact_digest: artifact,
                input_digest: input,
                policy_pin: pin,
                granted_capabilities: capabilities.clone(),
                policy,
                budget,
            },
            json!({"type":"RunAdmissionBudget","run":91,"seq":93,"artifact_digest":format!("{artifact:?}"),"input_digest":format!("{input:?}"),"policy_pin":format!("{pin:?}"),"granted_capabilities":format!("{capabilities:?}"),"policy":format!("{policy:?}"),"budget":format!("{budget:?}")}),
        ),
        (
            JournalEvent::RetryCheckpoint {
                run,
                seq,
                attempt: 4,
                step: StepIdx::new(5),
                payload_digest: [0x55; 32],
                payload: vec![2, 4, 6],
            },
            json!({"type":"RetryCheckpoint","run":91,"seq":93,"attempt":4,"step":5,"payload_digest":format!("{:?}",[0x55;32]),"payload":[2,4,6]}),
        ),
        (
            JournalEvent::TenantAdmissionEvent {
                run,
                seq,
                tenant: TenantId::new(17),
                cost_units: 19,
            },
            json!({"type":"TenantAdmissionEvent","run":91,"seq":93,"tenant":17,"cost_units":19}),
        ),
        (
            JournalEvent::RunBudgetOriginEvent {
                run,
                seq,
                started_millis: 1234567,
            },
            json!({"type":"RunBudgetOriginEvent","run":91,"seq":93,"started_millis":1234567}),
        ),
    ];
    for (event, expected) in cases {
        assert_eq!(event_to_json(&event), expected);
    }
    Ok(())
}

#[test]
fn policy_compatibility_and_retry_timer_records_are_retained() {
    use vb_core::policy::{
        CurrentPolicyIdentity, DecisionCode, DecisionRecord, InfluenceState, PinnedAdmissionPolicy,
        PolicyDigest, PolicyLayer,
    };
    use vb_storage::compat::{
        CompatBoundary, CompatDecision, CompatDigests, CompatInputs, SchemaVersion,
    };
    let ticket = ticket();
    let run = ticket.run;
    let seq = EventSeq::new(101);
    let decision = DecisionRecord {
        ticket,
        admission: PinnedAdmissionPolicy::Artifact(PolicyDigest::from_bytes([0x13; 32])),
        current_live: CurrentPolicyIdentity::Configured {
            digest: PolicyDigest::from_bytes([0x27; 32]),
        },
        input_digest: PolicyDigest::from_bytes([0x35; 32]),
        taint: Taint::DerivedFromSecret,
        outcome: DecisionCode::Suspended,
        layer: PolicyLayer::Emergency,
        emergency_influence: InfluenceState::NotEvaluated,
        compat_influence: InfluenceState::NotEvaluated,
    };
    assert_eq!(
        event_to_json(&JournalEvent::PolicyDecisionEvent {
            run,
            seq,
            decision: decision.clone()
        }),
        json!({"type":"PolicyDecisionEvent","run":71,"seq":101,"decision":format!("{decision:?}")})
    );
    let decision = CompatDecision::decide(
        CompatInputs {
            boundary: CompatBoundary::IrVersion,
            held: SchemaVersion::new(1, 2, 3),
            required: SchemaVersion::new(2, 3, 4),
            migration_available: true,
            replay_only: false,
        },
        CompatDigests {
            rule_digest: [1; 32],
            old_schema_digest: [2; 32],
            new_schema_digest: [3; 32],
        },
    );
    assert_eq!(
        event_to_json(&JournalEvent::CompatibilityDecisionEvent { run, seq, decision }),
        json!({"type":"CompatibilityDecisionEvent","run":71,"seq":101,"decision":format!("{decision:?}")})
    );
    let timer = vb_storage::retry_timer::RetryTimerV1 {
        version: 1,
        failed_ticket: ticket,
        next_attempt: 4,
        generation: 103,
        scheduled_unix_ms: 200,
        deadline_unix_ms: 250,
        checkpoint_digest: [0x17; 32],
        checkpoint: vec![2, 5, 8],
    };
    assert_eq!(
        event_to_json(&JournalEvent::RetryTimerScheduled {
            run,
            seq,
            step: StepIdx::new(5),
            attempt: 2,
            timer: timer.clone()
        }),
        json!({"type":"RetryTimerScheduled","run":71,"seq":101,"step":5,"attempt":2,"timer":format!("{timer:?}")})
    );
}
