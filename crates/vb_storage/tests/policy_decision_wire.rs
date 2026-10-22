//! Integration tests for PolicyDecisionEvent wire serialization.
//!
//! Covers: all PolicyLayer/DecisionCode combinations, flag permutations,
//! FjallJournal append/reopen/read equality, malformed payload handling,
//! and legacy tag stability.

use vb_core::ids::RunId;
use vb_core::ids::WorkflowDigest;
use vb_core::policy::{DecisionCode, PolicyDigest, PolicyLayer};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

fn make_policy_digest(byte: u8) -> PolicyDigest {
    PolicyDigest(WorkflowDigest::from_bytes([byte; 32]))
}

/// Test all PolicyLayer values serialize and deserialize correctly.
#[test]
fn test_policy_layers_all() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(10);
    let seq = EventSeq::new(0);
    let policy_digest = make_policy_digest(1);
    let input_digest = make_policy_digest(2);

    let layers = [
        PolicyLayer::Admission,
        PolicyLayer::Replay,
        PolicyLayer::LiveEffect,
        PolicyLayer::EmergencyDeny,
    ];

    for layer in layers {
        let event = JournalEvent::PolicyDecisionEvent {
            run,
            seq,
            layer,
            code: DecisionCode::Allowed,
            policy_digest,
            input_digest,
            emergency_deny_active: false,
            compatibility_influence: false,
        };

        let wire = postcard::to_allocvec(&event)?;
        let decoded: JournalEvent = postcard::from_bytes(&wire)?;

        assert_eq!(decoded, event);
    }

    Ok(())
}

/// Test all DecisionCode values serialize and deserialize correctly.
#[test]
fn test_decision_codes_all() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(11);
    let seq = EventSeq::new(0);
    let policy_digest = make_policy_digest(3);
    let input_digest = make_policy_digest(4);

    let codes = [
        DecisionCode::Allowed,
        DecisionCode::Denied,
        DecisionCode::Suspended,
    ];

    for code in codes {
        let event = JournalEvent::PolicyDecisionEvent {
            run,
            seq,
            layer: PolicyLayer::Admission,
            code,
            policy_digest,
            input_digest,
            emergency_deny_active: false,
            compatibility_influence: false,
        };

        let wire = postcard::to_allocvec(&event)?;
        let decoded: JournalEvent = postcard::from_bytes(&wire)?;

        assert_eq!(decoded, event);
    }

    Ok(())
}

/// Test flag permutations for emergency_deny_active and compatibility_influence.
#[test]
fn test_flag_permutations() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(12);
    let seq = EventSeq::new(0);
    let policy_digest = make_policy_digest(5);
    let input_digest = make_policy_digest(6);

    let flag_combos = [(false, false), (true, false), (false, true), (true, true)];

    for (emergency, compat) in flag_combos {
        let event = JournalEvent::PolicyDecisionEvent {
            run,
            seq,
            layer: PolicyLayer::LiveEffect,
            code: DecisionCode::Denied,
            policy_digest,
            input_digest,
            emergency_deny_active: emergency,
            compatibility_influence: compat,
        };

        let wire = postcard::to_allocvec(&event)?;
        let decoded: JournalEvent = postcard::from_bytes(&wire)?;

        assert_eq!(decoded, event);
    }

    Ok(())
}

/// Test FjallJournal append/reopen/read equality with distinct non-zero digests.
#[test]
fn test_fjall_journal_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let journal = FjallJournal::open(temp.path(), None)?;

    let run = RunId::new(13);
    let seq = EventSeq::new(0);
    let policy_digest = make_policy_digest(7);
    let input_digest = make_policy_digest(8);

    let event = JournalEvent::PolicyDecisionEvent {
        run,
        seq,
        layer: PolicyLayer::Admission,
        code: DecisionCode::Allowed,
        policy_digest,
        input_digest,
        emergency_deny_active: true,
        compatibility_influence: false,
    };

    journal.append_strict(&event)?;
    drop(journal);

    let reopened = FjallJournal::open(temp.path(), None)?;
    let read_events = reopened.events_for_run(run)?;
    drop(reopened);

    assert_eq!(read_events, vec![event]);
    Ok(())
}

/// Test multiple policy events in same run preserve distinct digests.
#[test]
fn test_multiple_events_distinct_digests() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let journal = FjallJournal::open(temp.path(), None)?;

    let run = RunId::new(14);

    let events = vec![
        JournalEvent::PolicyDecisionEvent {
            run,
            seq: EventSeq::new(0),
            layer: PolicyLayer::Admission,
            code: DecisionCode::Allowed,
            policy_digest: make_policy_digest(9),
            input_digest: make_policy_digest(10),
            emergency_deny_active: false,
            compatibility_influence: false,
        },
        JournalEvent::PolicyDecisionEvent {
            run,
            seq: EventSeq::new(1),
            layer: PolicyLayer::LiveEffect,
            code: DecisionCode::Denied,
            policy_digest: make_policy_digest(11),
            input_digest: make_policy_digest(12),
            emergency_deny_active: true,
            compatibility_influence: true,
        },
    ];

    for event in &events {
        journal.append_strict(event)?;
    }
    drop(journal);

    let reopened = FjallJournal::open(temp.path(), None)?;
    let read_events = reopened.events_for_run(run)?;
    drop(reopened);

    assert_eq!(read_events, events);
    Ok(())
}

/// Test malformed payload returns error, not panic or garbage.
#[test]
fn test_malformed_payload() -> Result<(), Box<dyn std::error::Error>> {
    let wire = postcard::to_allocvec(&JournalEvent::PolicyDecisionEvent {
        run: RunId::new(15),
        seq: EventSeq::new(0),
        layer: PolicyLayer::Admission,
        code: DecisionCode::Allowed,
        policy_digest: make_policy_digest(13),
        input_digest: make_policy_digest(14),
        emergency_deny_active: false,
        compatibility_influence: false,
    })?;

    let end = wire.len().checked_sub(4).ok_or("wire too short")?;
    let truncated = wire.get(..end).ok_or("slice out of bounds")?.to_vec();
    let result: Result<JournalEvent, _> = postcard::from_bytes(&truncated);

    assert!(result.is_err());
    Ok(())
}

/// Test tag stability: RunAccepted still uses its established tag.
#[test]
fn test_legacy_tag_stability() -> Result<(), Box<dyn std::error::Error>> {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(16),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([15; 32]),
    };

    let wire = postcard::to_allocvec(&event)?;
    assert_eq!(wire.first().copied(), Some(10));

    let decoded: JournalEvent = postcard::from_bytes(&wire)?;
    assert_eq!(decoded, event);
    Ok(())
}
