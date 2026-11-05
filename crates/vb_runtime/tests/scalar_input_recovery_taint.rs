#![forbid(unsafe_code)]
//! Constructed-prefix scalar recovery, not normal submission or crash acceptance.
//!
//! Matching Relaxed artifacts/records are staged through public RunSubmission.
//! Its Relaxed mode emits no admission events. This fixture then constructs a
//! bound positive-budget prefix and persists it with append_strict_batch/close.
//! Storage synchronization does not turn Relaxed admission into Strict admission.
//! No owned arenas, timed Ask, YAML compilation or full replay is qualified.

use std::error::Error;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;

use vb_core::budget::{AggregateResourceBudget, BoundednessPolicy, validate_aggregate_budget};
use vb_core::frame::RunFrame;
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNodeKind, CompiledWorkflow, InputSlotKind,
    ResourceContract, WorkflowParts, node,
};
use vb_core::{
    CapabilitySet, CoreError, RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx, Taint,
    WorkflowDigest, WorkflowId,
};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::recovery::hydrate_run_admission_from_events;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::admission::{AcceptedArtifact, prepare_compiled_artifact};
use vb_storage::{
    EventSeq, FjallConfig, FjallJournal, JournalEvent, RunHeaderRecord, RunInputBinding,
    RunInputsRecord, RunPolicyPin, RunSubmission, WorkflowSourceRecord,
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const POLICY: RuntimePolicy = RuntimePolicy::Relaxed;
const PROMPT: SlotIdx = SlotIdx::new(0);
const CLEAN: SlotIdx = SlotIdx::new(1);
const SECRET: SlotIdx = SlotIdx::new(2);
const DEFAULTED: SlotIdx = SlotIdx::new(3);
const NULLABLE: SlotIdx = SlotIdx::new(4);
const OPTIONAL: SlotIdx = SlotIdx::new(5);
const ANSWER: SlotIdx = SlotIdx::new(6);
const SOURCE: &[u8] = b"constructed scalar-input recovery prefix; no compiler invoked";

fn workflow() -> TestResult<CompiledWorkflow> {
    let clean = CompiledInputConstraint::required();
    let mut secret = clean;
    secret.secret = true;
    let mut defaulted = secret;
    defaulted.default_value = Some(SlotValue::I64(17));
    let mut nullable = secret;
    nullable.nullable = true;
    let mut optional = secret;
    optional.optional = true;
    let declaration = |slot, constraint| CompiledInputSlot {
        slot,
        kind: InputSlotKind::I64,
        constraint,
    };
    let mut parts = WorkflowParts {
        semantic_source_identity: None,
        name: "scalar-input-recovery-taint".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::new([
            node(
                StepIdx::ZERO,
                CompiledNodeKind::Ask {
                    prompt: PROMPT,
                    timeout_slot: None,
                },
                Some(ANSWER),
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::AskResume { answer: ANSWER },
                Some(ANSWER),
                Some(StepIdx::new(2)),
            ),
            node(
                StepIdx::new(2),
                CompiledNodeKind::Finish { result: ANSWER },
                None,
                None,
            ),
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 7,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::new([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::new([
            declaration(PROMPT, clean),
            declaration(CLEAN, clean),
            declaration(SECRET, secret),
            declaration(DEFAULTED, defaulted),
            declaration(NULLABLE, nullable),
            declaration(OPTIONAL, optional),
        ]),
    };
    let canonical = postcard::to_allocvec(&parts)?;
    parts.digest = WorkflowDigest::from_bytes(*blake3::hash(&canonical).as_bytes());
    Ok(CompiledWorkflow::try_from_parts(parts)?)
}

struct Fixture {
    run: RunId,
    workflow: CompiledWorkflow,
    artifact: AcceptedArtifact,
    inputs: RunInputsRecord,
    required: AggregateResourceBudget,
    recorded: AggregateResourceBudget,
    admission: JournalEvent,
}

impl Fixture {
    fn new(run: RunId) -> TestResult<Self> {
        let workflow = workflow()?;
        let artifact = prepare_compiled_artifact(&workflow, POLICY, &[])?;
        let source_digest = WorkflowDigest::from_bytes(*blake3::hash(SOURCE).as_bytes());
        let inputs = RunInputsRecord::new(
            run,
            artifact.digest,
            source_digest,
            &workflow,
            Box::new([
                (SECRET, SlotValue::I64(99)),
                (NULLABLE, SlotValue::Null),
                (CLEAN, SlotValue::I64(11)),
                (PROMPT, SlotValue::I64(7)),
            ]),
        )?;
        assert_eq!(
            inputs.inputs(),
            &[
                (SECRET, SlotValue::I64(99)),
                (NULLABLE, SlotValue::Null),
                (CLEAN, SlotValue::I64(11)),
                (PROMPT, SlotValue::I64(7)),
                (DEFAULTED, SlotValue::I64(17)),
            ]
        );
        let required = AggregateResourceBudget::from_workflow(&workflow)
            .map_err(|error| format!("required budget: {error:?}"))?;
        validate_aggregate_budget(&required, &BoundednessPolicy::DEFAULT)
            .map_err(|error| format!("required budget policy: {error:?}"))?;
        assert!(required.max_steps_executable > 0);
        assert!(required.max_transitions > 0);
        assert!(required.max_journal_bytes_total > 0);
        let mut recorded = required;
        recorded.max_cost_units = required
            .max_cost_units
            .checked_add(7)
            .ok_or("recorded cost headroom overflow")?;
        validate_aggregate_budget(&recorded, &BoundednessPolicy::DEFAULT)
            .map_err(|error| format!("recorded budget policy: {error:?}"))?;
        assert_ne!(recorded, required);
        let admission = JournalEvent::RunAdmissionBudget {
            run,
            seq: EventSeq::new(1),
            artifact_digest: artifact.digest,
            input_digest: RunInputBinding::Digest(inputs.input_digest()),
            policy_pin: RunPolicyPin::Pinned(artifact.policy_digest),
            granted_capabilities: CapabilitySet::empty(),
            policy: POLICY,
            budget: recorded,
        };
        inputs.validate_admission(&admission)?;
        Ok(Self {
            run,
            workflow,
            artifact,
            inputs,
            required,
            recorded,
            admission,
        })
    }
}

fn open_journal(path: &Path) -> TestResult<FjallJournal> {
    let config = FjallConfig::try_new(FjallConfig::MIN_CACHE_SIZE_BYTES)
        .ok_or("minimum Fjall cache configuration")?;
    Ok(FjallJournal::open(path, Some(config))?)
}

#[derive(Clone, Copy)]
struct Write {
    slot: SlotIdx,
    value: SlotValue,
    taint: Taint,
}

struct DurablePrefix {
    artifact_bytes: Vec<u8>,
    raw_admission: Vec<u8>,
    complete_admission: Vec<u8>,
}

fn persist_prefix(path: &Path, fixture: &Fixture, writes: &[Write]) -> TestResult<DurablePrefix> {
    assert!(
        writes.len() <= 1,
        "this constructed fixture has at most one later write"
    );
    let mut journal = open_journal(path)?;
    let submission = RunSubmission::new(
        WorkflowSourceRecord {
            digest: fixture.inputs.source_digest(),
            source: SOURCE.to_vec(),
        },
        fixture.artifact.clone(),
        fixture.inputs.clone(),
        RunHeaderRecord {
            run: fixture.run,
            workflow_id: WorkflowId::new(23),
            compiled_digest: fixture.workflow.digest(),
            status: 0,
            accepted_at_ms: 123,
        },
    )?;
    journal.commit_run_submission(&submission, POLICY)?;
    assert!(journal.events_for_run_full(fixture.run)?.is_empty());
    let mut events = vec![
        JournalEvent::RunAccepted {
            run: fixture.run,
            seq: EventSeq::new(0),
            workflow: fixture.workflow.digest(),
        },
        fixture.admission.clone(),
        JournalEvent::StepStarted {
            run: fixture.run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            attempt: 1,
        },
    ];
    for write in writes {
        let extra = vb_storage::slot_extra::encode_slot_written_extra(write.taint, None)
            .map_err(|error| format!("slot extra encoding: {error:?}"))?;
        events.push(JournalEvent::SlotWrittenEvent {
            run: fixture.run,
            seq: EventSeq::new(u64::try_from(events.len())?),
            slot: write.slot,
            value: Some(postcard::to_allocvec(&write.value)?),
            extra: Some(extra),
            attempt: 1,
        });
    }
    events.push(JournalEvent::AskScheduledEvent {
        run: fixture.run,
        seq: EventSeq::new(u64::try_from(events.len())?),
        step: StepIdx::ZERO,
        attempt: 1,
    });
    journal.append_strict_batch(&events)?;
    assert_eq!(journal.events_for_run_full(fixture.run)?, events);
    let artifact_bytes = journal
        .compiled_ir(fixture.artifact.digest)?
        .ok_or("missing fixture artifact")?
        .ir;
    assert_eq!(artifact_bytes, postcard::to_allocvec(&fixture.artifact)?);
    let raw_admission = journal
        .get_event_bytes(fixture.run, EventSeq::new(1))?
        .ok_or("missing raw admission")?;
    let original = hydrate_run_admission_from_events(std::slice::from_ref(&fixture.admission))
        .ok_or("missing typed fixture admission")?;
    assert_eq!(original.budget(), Some(fixture.recorded));
    let complete_admission = postcard::to_allocvec(&original)?;
    journal.close()?;
    Ok(DurablePrefix {
        artifact_bytes,
        raw_admission,
        complete_admission,
    })
}

fn assert_slot(frame: &RunFrame, slot: SlotIdx, value: SlotValue, taint: Taint) -> TestResult {
    assert_eq!(frame.read_slot(slot)?, &value);
    assert_eq!(frame.read_taint(slot)?, taint);
    Ok(())
}

fn assert_restored(
    runtime: &Runtime,
    fixture: &Fixture,
    prefix: &DurablePrefix,
    clean: (SlotValue, Taint),
    secret: (SlotValue, Taint),
) -> TestResult {
    assert!(
        runtime
            .lookup_pending_ask_ticket(fixture.run, StepIdx::ZERO)
            .is_some()
    );
    let state = runtime.inspect_run_state(fixture.run)?;
    let admission = state
        .admission
        .as_ref()
        .ok_or("missing restored admission")?;
    assert_eq!(admission.run_id(), fixture.run);
    assert_eq!(admission.artifact_digest(), fixture.artifact.digest);
    assert_eq!(admission.policy(), POLICY);
    assert_eq!(
        admission.policy_pin(),
        RunPolicyPin::Pinned(fixture.artifact.policy_digest)
    );
    assert_eq!(admission.budget(), Some(fixture.recorded));
    assert_ne!(admission.budget(), Some(fixture.required));
    assert_eq!(postcard::to_allocvec(admission)?, prefix.complete_admission);
    assert_slot(&state.frame, PROMPT, SlotValue::I64(7), Taint::Clean)?;
    assert_slot(&state.frame, CLEAN, clean.0, clean.1)?;
    assert_slot(&state.frame, SECRET, secret.0, secret.1)?;
    assert_slot(&state.frame, DEFAULTED, SlotValue::I64(17), Taint::Secret)?;
    assert_slot(&state.frame, NULLABLE, SlotValue::Null, Taint::Secret)?;
    assert!(matches!(state.frame.read_slot(OPTIONAL),
        Err(CoreError::SlotUninitialized { slot }) if slot == OPTIONAL));
    Ok(())
}

fn reopen_case(
    run: RunId,
    writes: &[Write],
    clean: (SlotValue, Taint),
    secret: (SlotValue, Taint),
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let fixture = Fixture::new(run)?;
    let prefix = persist_prefix(directory.path(), &fixture, writes)?;
    // All producer handles were closed/dropped before this independent open.
    let journal = Arc::new(open_journal(directory.path())?);
    assert_eq!(journal.run_inputs(run)?.as_ref(), Some(&fixture.inputs));
    assert_eq!(
        journal
            .compiled_ir(fixture.artifact.digest)?
            .ok_or("missing reopened artifact")?
            .ir,
        prefix.artifact_bytes
    );
    assert_eq!(
        journal.get_event_bytes(run, EventSeq::new(1))?.as_ref(),
        Some(&prefix.raw_admission)
    );
    let shared = StorageRuntimeJournal::shared_strict(Arc::clone(&journal));
    let config = ShardConfig::new(4, 16, 16, 1, POLICY)?;
    let mut runtime = Runtime::new(NonZeroUsize::new(1).ok_or("one shard")?, config, shared);
    runtime.recover_and_resume(run)?;
    assert!(runtime.tick_all()?);
    assert_restored(&runtime, &fixture, &prefix, clean, secret)?;
    assert_eq!(
        journal.get_event_bytes(run, EventSeq::new(1))?.as_ref(),
        Some(&prefix.raw_admission)
    );
    assert_eq!(journal.run_inputs(run)?.as_ref(), Some(&fixture.inputs));
    assert_eq!(
        journal
            .compiled_ir(fixture.artifact.digest)?
            .ok_or("missing retained artifact")?
            .ir,
        prefix.artifact_bytes
    );
    drop(runtime);
    let mut journal = Arc::try_unwrap(journal).map_err(|_| "recovery retained a journal owner")?;
    journal.close()?;
    Ok(())
}

#[test]
fn scalar_taints_defaults_nullable_null_and_optional_absence_survive_reopen() -> TestResult {
    reopen_case(
        RunId::new(8_301),
        &[],
        (SlotValue::I64(11), Taint::Clean),
        (SlotValue::I64(99), Taint::Secret),
    )
}

#[test]
fn later_clean_journal_write_preserves_its_value_and_taint_over_secret_input() -> TestResult {
    reopen_case(
        RunId::new(8_302),
        &[Write {
            slot: SECRET,
            value: SlotValue::I64(401),
            taint: Taint::Clean,
        }],
        (SlotValue::I64(11), Taint::Clean),
        (SlotValue::I64(401), Taint::Clean),
    )
}

#[test]
fn later_secret_journal_write_preserves_its_value_and_taint_over_clean_input() -> TestResult {
    reopen_case(
        RunId::new(8_303),
        &[Write {
            slot: CLEAN,
            value: SlotValue::I64(402),
            taint: Taint::Secret,
        }],
        (SlotValue::I64(402), Taint::Secret),
        (SlotValue::I64(99), Taint::Secret),
    )
}
