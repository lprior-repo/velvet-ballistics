use super::*;

fn completion_ticket(step: u16, action: u16) -> ActionTicket {
    let run = RunId::new(1);
    ActionTicket {
        run,
        step: StepIdx::new(step),
        seq: SeqNo::new(2),
        action: ActionId::new(action),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 1,
        capacity: 1,
        fencing_token: 1,
    }
}

fn completed_envelope(step: u16, action: u16) -> JournalEvent {
    let ticket = completion_ticket(step, action);
    JournalEvent::ActionCompletedEnvelope {
        run: ticket.run,
        seq: EventSeq::new(1),
        ticket,
        action_abi_digest: vb_core::WorkflowDigest::from_bytes([0; 32]),
        output: vb_core::SlotIdx::new(1),
        outcome: crate::DurableActionOutcome::Ready,
        value: Vec::new(),
        encoded_len: 0,
        taint: vb_core::Taint::Clean,
        value_digest: *blake3::hash(&[]).as_bytes(),
    }
}

#[test]
fn completion_envelope_emits_one_confirmed_side_effect_with_ticket_identity() {
    let analysis = analyze_incident_events(&[completed_envelope(7, 700)]);
    let effects: Vec<_> = analysis
        .side_effects
        .iter()
        .map(|effect| {
            (
                effect.step,
                effect.action,
                matches!(effect.certainty, SideEffectCertainty::Confirmed),
            )
        })
        .collect();

    assert_eq!(effects, [(7, 700, true)]);
}

#[test]
fn mixed_completion_order_and_failure_guidance_are_preserved_without_mutation() {
    let events = vec![
        action_completed(3, 30),
        completed_envelope(7, 700),
        run_failed(),
    ];
    let original_events = events.clone();
    let analysis = analyze_incident_events(&events);
    let effects: Vec<_> = analysis
        .side_effects
        .iter()
        .map(|effect| (effect.step, effect.action))
        .collect();

    assert!(analysis.failure_found);
    assert_eq!(effects, [(3, 30), (7, 700)]);
    assert!(analysis.side_effects.iter().any(
        |effect| effect.step == 7 && matches!(effect.certainty, SideEffectCertainty::Confirmed)
    ));
    assert!(
        build_repair_hints(
            &analysis.failure_code,
            &analysis.side_effects,
            analysis.failed_at_step
        )
        .iter()
        .any(|hint| hint
            == "review side effects that completed before failure for compensating actions")
    );
    assert_eq!(events, original_events);
}

#[test]
fn reopened_journal_envelope_identity_reaches_incident_analysis()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let run = RunId::new(1);
    let journal = crate::FjallJournal::open(temp.path(), None)?;
    let events = [
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: vb_core::WorkflowDigest::from_bytes([1; 32]),
        },
        completed_envelope(7, 700),
    ];
    journal.append_strict_batch(&events)?;
    drop(journal);

    let reopened = crate::FjallJournal::open(temp.path(), None)?;
    let replayed = reopened.events_for_run(run)?;
    let analysis = analyze_incident_events(&replayed);
    let effects: Vec<_> = analysis
        .side_effects
        .iter()
        .map(|effect| {
            (
                effect.step,
                effect.action,
                matches!(effect.certainty, SideEffectCertainty::Confirmed),
            )
        })
        .collect();

    assert_eq!(effects, [(7, 700, true)]);
    Ok(())
}
