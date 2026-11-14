use super::*;
use crate::{EventSeq, JournalEvent, JournalWriterQueue, StorageLimits};
use vb_core::{AggregateResourceBudget, CapabilitySet, RuntimePolicy};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn record(
    run: u64,
    inputs: Box<[(SlotIdx, SlotValue)]>,
) -> Result<RunInputsRecord, Box<dyn std::error::Error>> {
    let workflow = super::tests::workflow()?;
    let artifact =
        crate::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    let source = b"atomic admission retained source";
    let source_digest = WorkflowDigest::from_bytes(blake3::hash(source).into());
    Ok(RunInputsRecord::new(
        RunId::new(run),
        artifact.digest,
        source_digest,
        &workflow,
        inputs,
    )?)
}

fn receipt(record: &RunInputsRecord, seq: u64) -> Result<JournalEvent, Box<dyn std::error::Error>> {
    let workflow = workflow()?;
    let artifact = artifact()?;
    let budget = AggregateResourceBudget::from_workflow(&workflow)?;
    Ok(JournalEvent::RunAdmissionBudget {
        run: record.run(),
        seq: EventSeq::new(seq),
        artifact_digest: record.artifact_digest(),
        input_digest: RunInputBinding::Digest(record.input_digest()),
        policy_pin: RunPolicyPin::Pinned(artifact.policy_digest),
        granted_capabilities: CapabilitySet::empty(),
        policy: RuntimePolicy::Strict,
        budget,
    })
}

fn unbudgeted_receipt(record: &RunInputsRecord, seq: u64) -> JournalEvent {
    JournalEvent::RunAdmission {
        run: record.run(),
        seq: EventSeq::new(seq),
        artifact_digest: record.artifact_digest(),
        input_digest: RunInputBinding::Digest(record.input_digest()),
        policy_pin: RunPolicyPin::Unverified,
        granted_capabilities: CapabilitySet::empty(),
        policy: RuntimePolicy::Strict,
    }
}

fn deficient_budget_receipt(
    record: &RunInputsRecord,
    seq: u64,
    input_digest: RunInputBinding,
    policy_pin: RunPolicyPin,
) -> Result<JournalEvent, Box<dyn std::error::Error>> {
    let workflow = workflow()?;
    let budget = AggregateResourceBudget::from_workflow(&workflow)?;
    Ok(JournalEvent::RunAdmissionBudget {
        run: record.run(),
        seq: EventSeq::new(seq),
        artifact_digest: record.artifact_digest(),
        input_digest,
        policy_pin,
        granted_capabilities: CapabilitySet::empty(),
        policy: RuntimePolicy::Strict,
        budget,
    })
}

fn accepted(record: &RunInputsRecord, seq: u64, workflow: WorkflowDigest) -> JournalEvent {
    JournalEvent::RunAccepted {
        run: record.run(),
        seq: EventSeq::new(seq),
        workflow,
    }
}

fn workflow() -> Result<vb_core::CompiledWorkflow, Box<dyn std::error::Error>> {
    super::tests::workflow().map_err(Into::into)
}

fn artifact() -> Result<crate::AcceptedArtifact, Box<dyn std::error::Error>> {
    let workflow = workflow()?;
    crate::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])
        .map_err(Into::into)
}

// Test-only same-layout serde carrier for hostile serialized inputs.
// Field order and types must match RunInputsRecord exactly.
#[derive(serde::Serialize, serde::Deserialize)]
struct InputsCarrier {
    run: u64,
    artifact_digest: [u8; 32],
    workflow_digest: [u8; 32],
    source_digest: [u8; 32],
    input_digest: [u8; 32],
    inputs: Vec<(u32, SlotValue)>,
}

fn hostile_record(
    valid: &RunInputsRecord,
    mutated_value: SlotValue,
) -> Result<RunInputsRecord, Box<dyn std::error::Error>> {
    // Genuine record -> serialize -> decode to carrier -> mutate only value
    // -> re-serialize -> decode as real record. Stored digest X untouched.
    let valid_bytes = postcard::to_allocvec(valid)?;
    let mut carrier: InputsCarrier = postcard::from_bytes(&valid_bytes)?;

    // Prove identical layout: re-encoding unmodified carrier equals original.
    let roundtrip_bytes = postcard::to_allocvec(&carrier)?;
    assert_eq!(roundtrip_bytes, valid_bytes);

    // Mutate only the scalar input value; stored input_digest remains X.
    carrier.inputs[0].1 = mutated_value;
    let hostile_bytes = postcard::to_allocvec(&carrier)?;
    let hostile: RunInputsRecord = postcard::from_bytes(&hostile_bytes)?;
    Ok(hostile)
}

fn duplicate_slot_record(
    valid: &RunInputsRecord,
) -> Result<RunInputsRecord, Box<dyn std::error::Error>> {
    let valid_bytes = postcard::to_allocvec(valid)?;
    let mut carrier: InputsCarrier = postcard::from_bytes(&valid_bytes)?;
    // Duplicate slot 0 so compute_input_digest detects duplicate.
    carrier.inputs.push(carrier.inputs[0].clone());
    let hostile_bytes = postcard::to_allocvec(&carrier)?;
    let hostile: RunInputsRecord = postcard::from_bytes(&hostile_bytes)?;
    Ok(hostile)
}

#[test]
fn a_hostile_decoded_record_carries_mismatched_input_digest() -> TestResult {
    let valid = record(801, Box::from([(SlotIdx::new(0), SlotValue::I64(11))]))?;
    let hostile = hostile_record(&valid, SlotValue::I64(22))?;

    // All identity digests match; only the inputs mapping was mutated.
    assert_eq!(hostile.run(), valid.run());
    assert_eq!(hostile.artifact_digest(), valid.artifact_digest());
    assert_eq!(hostile.workflow_digest(), valid.workflow_digest());
    assert_eq!(hostile.source_digest(), valid.source_digest());
    assert_eq!(hostile.input_digest(), valid.input_digest());
    assert_eq!(hostile.inputs(), &[(SlotIdx::new(0), SlotValue::I64(22))]);

    // Recomputing the digest over the mutated mapping differs from the stored digest.
    let recomputed = compute_input_digest(hostile.inputs())?;
    assert_ne!(recomputed, hostile.input_digest());
    assert_ne!(recomputed, valid.input_digest());
    Ok(())
}

#[test]
fn b_direct_fjall_append_rejects_self_inconsistent_digest_with_no_writes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;

    let valid = record(802, Box::from([(SlotIdx::new(0), SlotValue::I64(11))]))?;
    let hostile = hostile_record(&valid, SlotValue::I64(22))?;

    // Otherwise fully matching admission events; only the rehash can reject.
    let events = vec![
        accepted(&hostile, 0, hostile.workflow_digest()),
        receipt(&hostile, 1)?,
    ];
    let result = journal.append_runtime_admission_batch(
        &events,
        &hostile,
        crate::types::DurabilityProfile::Strict,
    );
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "{result:?}"
    );

    // No journal events written.
    assert!(journal.events_for_run(hostile.run())?.is_empty());
    // No retained row written.
    assert_eq!(journal.run_inputs(hostile.run())?, None);

    // Raw key inspection: no row under the run header key.
    let key = crate::keys::run_header_key(hostile.run())?;
    let raw = journal.run_inputs.get(key)?;
    assert!(raw.is_none(), "raw row unexpectedly written");

    // Verify the run event keyspace has no entries for this run.
    let seq_key = crate::keys::run_event_key(hostile.run(), EventSeq::new(0))?;
    let raw_event = journal.events.get(seq_key.as_slice())?;
    assert!(raw_event.is_none(), "raw event unexpectedly written");
    Ok(())
}

#[test]
fn c_queued_enqueue_rejects_self_inconsistent_digest_before_queue_mutation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT)?;

    let valid = record(803, Box::from([(SlotIdx::new(0), SlotValue::I64(11))]))?;
    let hostile = hostile_record(&valid, SlotValue::I64(22))?;

    // Matching admission events; only rehash can reject.
    let events = vec![
        accepted(&hostile, 0, hostile.workflow_digest()),
        receipt(&hostile, 1)?,
    ];
    let counts_before = queue.pending_profile_counts()?;
    let state_before = format!("{queue:?}");
    let result = queue.enqueue_journaled_admission_batch(events, hostile.clone());
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "{result:?}"
    );

    // Queue unchanged.
    let counts_after = queue.pending_profile_counts()?;
    assert_eq!(counts_before, counts_after);
    let state_after = format!("{queue:?}");
    assert_eq!(state_before, state_after);

    // Attempt flush: Fjall unchanged, hostile run has no row or events.
    let flush_result = queue.flush_batch_durable(&journal)?;
    assert_eq!((flush_result.drained, flush_result.written), (0, 0));
    assert!(journal.events_for_run(hostile.run())?.is_empty());
    assert_eq!(journal.run_inputs(hostile.run())?, None);
    let key = crate::keys::run_header_key(hostile.run())?;
    let raw = journal.run_inputs.get(key)?;
    assert!(raw.is_none());
    Ok(())
}

#[test]
fn c2_queued_enqueue_with_preloaded_valid_batch_rejects_hostile() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT)?;

    let valid = record(804, Box::from([(SlotIdx::new(0), SlotValue::I64(11))]))?;
    let valid_events = vec![
        accepted(&valid, 0, valid.workflow_digest()),
        receipt(&valid, 1)?,
    ];
    queue.enqueue_journaled_admission_batch(valid_events.clone(), valid.clone())?;

    let hostile = hostile_record(
        &record(805, Box::from([(SlotIdx::new(0), SlotValue::I64(11))]))?,
        SlotValue::I64(22),
    )?;
    let hostile_events = vec![
        accepted(&hostile, 0, hostile.workflow_digest()),
        receipt(&hostile, 1)?,
    ];
    let counts_before = queue.pending_profile_counts()?;
    let state_before = format!("{queue:?}");
    let result = queue.enqueue_journaled_admission_batch(hostile_events, hostile.clone());
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "{result:?}"
    );

    // Pre-loaded valid batch still pending; hostile never entered queue.
    let counts_after = queue.pending_profile_counts()?;
    assert_eq!(counts_before, counts_after);
    let state_after = format!("{queue:?}");
    assert_eq!(state_before, state_after);

    // Flush: only the valid batch is written; hostile run has no row or events.
    let report = queue.flush_batch_durable(&journal)?;
    assert_eq!((report.drained, report.written), (2, 2));
    assert_eq!(journal.events_for_run(valid.run())?, valid_events);
    assert_eq!(journal.run_inputs(valid.run())?, Some(valid));
    assert!(journal.events_for_run(hostile.run())?.is_empty());
    assert_eq!(journal.run_inputs(hostile.run())?, None);
    let hostile_key = crate::keys::run_header_key(hostile.run())?;
    let raw = journal.run_inputs.get(hostile_key)?;
    assert!(raw.is_none());
    Ok(())
}

#[test]
fn d_valid_record_commits_through_direct_api() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;

    let valid = record(806, Box::from([(SlotIdx::new(0), SlotValue::I64(42))]))?;
    let events = vec![
        accepted(&valid, 0, valid.workflow_digest()),
        receipt(&valid, 1)?,
    ];
    journal.append_runtime_admission_batch(
        &events,
        &valid,
        crate::types::DurabilityProfile::Strict,
    )?;

    // Atomic event+row persisted and readable.
    assert_eq!(journal.events_for_run(valid.run())?, events);
    assert_eq!(journal.run_inputs(valid.run())?, Some(valid));
    Ok(())
}

#[test]
fn d2_valid_record_commits_through_queued_api() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT)?;

    let valid = record(807, Box::from([(SlotIdx::new(0), SlotValue::I64(42))]))?;
    let events = vec![
        accepted(&valid, 0, valid.workflow_digest()),
        receipt(&valid, 1)?,
    ];
    queue.enqueue_journaled_admission_batch(events, valid.clone())?;
    let report = queue.flush_batch_durable(&journal)?;
    assert_eq!((report.drained, report.written), (2, 2));

    assert_eq!(
        journal.events_for_run(valid.run())?,
        vec![
            accepted(&valid, 0, valid.workflow_digest()),
            receipt(&valid, 1)?,
        ]
    );
    assert_eq!(journal.run_inputs(valid.run())?, Some(valid));
    Ok(())
}

#[test]
fn e_duplicate_slots_surfaces_input_schema_mismatch() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;

    let valid = record(808, Box::from([(SlotIdx::new(0), SlotValue::I64(11))]))?;
    let duplicate = duplicate_slot_record(&valid)?;

    // The admission event still binds the original digest.
    let events = vec![
        accepted(&duplicate, 0, duplicate.workflow_digest()),
        receipt(&duplicate, 1)?,
    ];
    let result = journal.append_runtime_admission_batch(
        &events,
        &duplicate,
        crate::types::DurabilityProfile::Strict,
    );
    assert!(
        matches!(result, Err(JournalError::InputSchemaMismatch)),
        "{result:?}"
    );
    assert!(journal.events_for_run(duplicate.run())?.is_empty());
    assert_eq!(journal.run_inputs(duplicate.run())?, None);
    Ok(())
}

#[test]
fn g_atomic_apis_reject_unbudgeted_unbound_and_unverified_authority_without_mutation() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT)?;
    let artifact = artifact()?;
    for variant in 0u8..3 {
        let index = usize::from(variant);
        let run = RunId::new(810 + u64::try_from(index)?);
        let inputs = record(
            run.get(),
            Box::from([(SlotIdx::new(0), SlotValue::I64(42))]),
        )?;
        let admission = match variant {
            0 => unbudgeted_receipt(&inputs, 1),
            1 => deficient_budget_receipt(
                &inputs,
                1,
                RunInputBinding::LegacyUnbound,
                RunPolicyPin::Pinned(artifact.policy_digest),
            )?,
            _ => deficient_budget_receipt(
                &inputs,
                1,
                RunInputBinding::Digest(inputs.input_digest()),
                RunPolicyPin::Unverified,
            )?,
        };
        let events = vec![accepted(&inputs, 0, inputs.workflow_digest()), admission];
        let result = journal.append_runtime_admission_batch(
            &events,
            &inputs,
            crate::types::DurabilityProfile::Strict,
        );
        assert!(
            matches!(result, Err(JournalError::AdmissionRequired)),
            "{result:?}"
        );
        assert!(journal.events_for_run(inputs.run())?.is_empty());
        assert_eq!(journal.run_inputs(inputs.run())?, None);

        let queue_before = format!("{queue:?}");
        let queued = queue.enqueue_journaled_admission_batch(events, inputs.clone());
        assert!(
            matches!(queued, Err(JournalError::AdmissionRequired)),
            "{queued:?}"
        );
        assert_eq!(queue.pending_profile_counts()?.journaled, 0);
        assert_eq!(format!("{queue:?}"), queue_before);
        assert!(journal.events_for_run(inputs.run())?.is_empty());
        assert_eq!(journal.run_inputs(inputs.run())?, None);
    }
    Ok(())
}

#[test]
fn f_cold_reopen_recovers_valid_admitted_run_inputs() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let path = dir.path().to_owned();

    let valid = record(809, Box::from([(SlotIdx::new(0), SlotValue::I64(42))]))?;
    let submission = crate::RunSubmission::new(
        crate::WorkflowSourceRecord {
            digest: valid.source_digest(),
            source: b"atomic admission retained source".to_vec(),
        },
        artifact()?,
        valid.clone(),
        crate::RunHeaderRecord {
            run: valid.run(),
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: valid.workflow_digest(),
            status: 0,
            accepted_at_ms: 123,
        },
    )?;
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;

    // Close and reopen the same directory (cold recovery).
    drop(journal);
    let reopened = FjallJournal::open(&path, None)?;

    // Recover from durable state only.
    let mut issued = artifact()?;
    issued.accepted_at_seq = EventSeq::new(1);
    crate::admission::bind_artifact_digest(&mut issued)?;
    let valid = RunInputsRecord::new(
        valid.run(),
        issued.digest,
        valid.source_digest(),
        &workflow()?,
        valid.inputs().to_vec().into_boxed_slice(),
    )?;
    assert!(reopened.compiled_ir(valid.artifact_digest())?.is_some());
    let durable_events = reopened.events_for_run(valid.run())?;
    assert!(!durable_events.is_empty());

    let admission = crate::recovery::replay::recover_run_admission_from_events(&durable_events)
        .ok_or("admission recovery failed")?;
    assert_eq!(
        admission.input_digest,
        RunInputBinding::Digest(valid.input_digest())
    );
    assert_eq!(admission.artifact_digest, valid.artifact_digest());
    assert_eq!(admission.run_id, valid.run());

    let workflow = workflow()?;
    let retained = reopened.retained_run_inputs_for_recovery(
        valid.run(),
        valid.artifact_digest(),
        &workflow,
    )?;
    match retained {
        Some(RetainedRunInputs::Scalar(record)) => {
            assert_eq!(record, valid);
            // Validate the durable admission receipt against the retained record.
            let admission_event = durable_events
                .iter()
                .find(|e| matches!(e, JournalEvent::RunAdmissionBudget { .. }))
                .ok_or("no admission event in durable state")?;
            record.validate_admission(admission_event)?;
            record.validate_workflow(&workflow)?;

            // Full admitted-run-inputs read with admission binding.
            let admitted = reopened.admitted_run_inputs(
                valid.run(),
                valid.artifact_digest(),
                &workflow,
                admission_event,
            )?;
            match admitted {
                Some(RetainedRunInputs::Scalar(admitted_record)) => {
                    assert_eq!(admitted_record, valid);
                }
                other => panic!("expected scalar retained inputs, got {other:?}"),
            }
        }
        other => panic!("expected scalar retained inputs, got {other:?}"),
    }
    Ok(())
}
