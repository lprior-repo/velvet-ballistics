// SPDX-License-Identifier: MIT
//
// Proptest: every RuntimeJournalEvent variant routes through storage_event
// without fabricating RunFailedEvent. Only a RunFailed input maps to
// JournalEvent::RunFailedEvent; every other variant maps to its own storage
// event. The enum has 24 variants.

use proptest::prelude::*;

const VARIANT_COUNT: u8 = 24;

const RUN_FAILED_TAG: u8 = 6;

const EXPECTED_KINDS: [&str; 24] = [
    "RunSubmitted",
    "RunAdmission",
    "CompatibilityDecision",
    "TenantAdmission",
    "RetryCheckpoint",
    "RunFinished",
    "RunFailed",
    "RunCancelled",
    "RunKilled",
    "ActionScheduled",
    "ActionCompleted",
    "ActionScheduledTicket",
    "ActionCompletedEnvelope",
    "ActionFailed",
    "ActionAbandoned",
    "WaitScheduled",
    "WaitResolved",
    "AskScheduled",
    "AskAnswered",
    "AskTimedOut",
    "SlotWritten",
    "StepStarted",
    "StepSucceeded",
    "Resumed",
];

fn ticket_fixture() -> vb_core::action::ActionTicket {
    vb_core::action::ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: vb_core::ids::SeqNo::new(0),
        action: ActionId::new(1),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        fencing_token: 0,
    }
}

fn admission_fixture() -> crate::admission::RunAdmission {
    crate::admission::RunAdmission::new(
        WorkflowDigest::from_bytes([42; 32]),
        RunId::new(1),
        vb_core::capability::CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
    )
}

fn compat_decision_fixture() -> vb_storage::compat::CompatDecision {
    let held = vb_storage::compat::SchemaVersion::new(1, 0, 0);
    vb_storage::compat::decide_boundary(
        vb_storage::compat::artifact_ir_version_inputs(held),
        vb_storage::compat::schema_version_digest(held),
        vb_storage::compat::schema_version_digest(vb_storage::compat::CURRENT_IR_VERSION),
    )
}

fn event_for_tag(tag: u8) -> Option<RuntimeJournalEvent> {
    let run = RunId::new(1);
    let workflow = WorkflowDigest::from_bytes([42; 32]);
    let step = StepIdx::new(0);
    let action = ActionId::new(1);
    match tag {
        0 => Some(RuntimeJournalEvent::RunSubmitted { run, workflow }),
        1 => Some(RuntimeJournalEvent::RunAdmission {
            admission: admission_fixture(),
        }),
        2 => Some(RuntimeJournalEvent::CompatibilityDecision {
            run,
            decision: compat_decision_fixture(),
        }),
        3 => Some(RuntimeJournalEvent::TenantAdmission {
            run,
            tenant: vb_core::ids::TenantId::new(7),
            cost_units: 42,
        }),
        4 => Some(RuntimeJournalEvent::RetryCheckpoint {
            run,
            attempt: 1,
            step,
            payload_digest: [0; 32],
            payload: Vec::new(),
        }),
        5 => Some(RuntimeJournalEvent::RunFinished {
            run,
            result: SlotIdx::new(0),
            attempt: 1,
        }),
        6 => Some(RuntimeJournalEvent::RunFailed { run, attempt: 1 }),
        7 => Some(RuntimeJournalEvent::RunCancelled {
            run,
            reason: None,
            attempt: 1,
        }),
        8 => Some(RuntimeJournalEvent::RunKilled { run, attempt: 1 }),
        9 => Some(RuntimeJournalEvent::ActionScheduled { run, step, action }),
        10 => Some(RuntimeJournalEvent::ActionCompleted { run, step, action }),
        11 => Some(RuntimeJournalEvent::ActionScheduledTicket {
            ticket: ticket_fixture(),
            input: SlotIdx::new(0),
            output: SlotIdx::new(1),
            action_abi_digest: workflow,
        }),
        12 => Some(RuntimeJournalEvent::ActionCompletedEnvelope {
            ticket: ticket_fixture(),
            output: SlotIdx::new(1),
            value: Vec::new(),
            encoded_len: 0,
            taint: Taint::Clean,
            value_digest: [0; 32],
            action_abi_digest: workflow,
        }),
        13 => Some(RuntimeJournalEvent::ActionFailed {
            run,
            step,
            action,
            attempt: 1,
        }),
        14 => Some(RuntimeJournalEvent::ActionAbandoned {
            ticket: ticket_fixture(),
        }),
        15 => Some(RuntimeJournalEvent::WaitScheduled { run, step }),
        16 => Some(RuntimeJournalEvent::WaitResolved { run, step }),
        17 => Some(RuntimeJournalEvent::AskScheduled { run, step }),
        18 => Some(RuntimeJournalEvent::AskAnswered {
            run,
            step,
            slot: SlotIdx::new(0),
        }),
        19 => Some(RuntimeJournalEvent::AskTimedOut { run, step }),
        20 => Some(RuntimeJournalEvent::SlotWritten {
            run,
            slot: SlotIdx::new(0),
            value: Vec::new(),
            taint: Taint::Clean,
            extra: None,
        }),
        21 => Some(RuntimeJournalEvent::StepStarted {
            run,
            step,
            attempt: 1,
        }),
        22 => Some(RuntimeJournalEvent::StepSucceeded {
            run,
            step,
            output: SlotIdx::new(1),
            attempt: 1,
        }),
        23 => Some(RuntimeJournalEvent::Resumed {
            run,
            timestamp: 1_700_000_000,
        }),
        _ => None,
    }
}

fn assert_variant_maps_without_fabrication(tag: u8) {
    let event = event_for_tag(tag).expect("tag maps to a fixture");
    assert_eq!(event.kind(), EXPECTED_KINDS[usize::from(tag)]);
    let seq = EventSeq::new(7);
    let mapped = StorageRuntimeJournal::storage_event(event, seq).expect("storage mapping succeeds");
    let is_run_failed = matches!(mapped, JournalEvent::RunFailedEvent { .. });
    assert_eq!(is_run_failed, tag == RUN_FAILED_TAG);
}

proptest! {
    #[test]
    fn no_variant_fabricates_run_failed(tag in 0u8..VARIANT_COUNT) {
        assert_variant_maps_without_fabrication(tag);
    }
}

#[test]
fn every_variant_maps_without_fabrication() {
    for tag in 0..VARIANT_COUNT {
        assert_variant_maps_without_fabrication(tag);
    }
}
