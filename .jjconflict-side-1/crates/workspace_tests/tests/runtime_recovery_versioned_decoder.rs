#![forbid(unsafe_code)]

use std::{num::NonZeroUsize, sync::Arc};
use vb_core::{CompiledWorkflow, RunId, RuntimePolicy, WorkflowDigest};
use vb_runtime::{
    RuntimeError, journal::StorageRuntimeJournal, runtime::Runtime, shard::ShardConfig,
};
use vb_storage::compat::{CURRENT_IR_VERSION, CompatibilityClass, SchemaVersion};
use vb_storage::{
    AcceptedArtifact, CompiledIrRecord, EventSeq, FjallJournal, JournalError, JournalEvent,
    RunInputBinding, RunPolicyPin,
};

#[path = "artifact_evidence_support/mod.rs"]
mod fixture;
#[path = "runtime_recovery_versioned_decoder/positive.rs"]
mod positive;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const RUN: RunId = RunId::new(816_009);
const V2_SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: computed_recovery\nwhen: {manual: {}}\nsteps:\n  - id: invoke\n    do: {action: '0', with: {auth: 'ordinary', items: [3, 5]}}\n  - id: done\n    finish: {result: '$step.invoke'}\n";

#[derive(Clone, Copy, Debug)]
enum Hostile {
    Future,
    V1WithV2,
    V2WithV1,
    BadPrefix,
    LegacyBlob,
    ShortEscape,
}

fn open(journal: Arc<FjallJournal>) -> Runtime {
    Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Journaled,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal),
    )
}

fn contract() -> TestResult<vb_core::ActionContract> {
    let mut contract = fixture::contract()?;
    contract.side_effect = vb_core::SideEffect::None;
    contract.idempotency = vb_core::Idempotency::IdempotentExternal;
    contract.retry_safety = vb_core::RetrySafety::Safe;
    Ok(contract)
}

fn workflow(kind: Hostile) -> TestResult<CompiledWorkflow> {
    if matches!(kind, Hostile::V2WithV1 | Hostile::LegacyBlob) {
        let workflow = fixture::action_workflow()?;
        assert_eq!(workflow.symbol_payloads(), None);
        Ok(workflow)
    } else {
        let workflow = vb_compile::compile_workflow(V2_SOURCE)?;
        match workflow.symbol_payloads() {
            Some(_) => Ok(workflow),
            None => Err("expected a bound V2 fixture".into()),
        }
    }
}

fn tamper(artifact: &mut AcceptedArtifact, kind: Hostile) -> TestResult {
    match kind {
        Hostile::Future => artifact.artifact_version = SchemaVersion::new(99, 0, 0),
        Hostile::V1WithV2 => artifact.artifact_version = SchemaVersion::new(1, 0, 0),
        Hostile::V2WithV1 => artifact.artifact_version = SchemaVersion::new(2, 0, 0),
        Hostile::BadPrefix => *artifact.ir.get_mut(3).ok_or("missing V2 prefix")? = b'C',
        Hostile::ShortEscape => artifact.ir = vec![1, 255, 86],
        Hostile::LegacyBlob => return Err("raw IR is not an artifact envelope".into()),
    }
    fixture::rebind(artifact)?;
    Ok(())
}

fn reject_future(result: Result<CompiledWorkflow, JournalError>) -> TestResult {
    match result {
        Err(JournalError::ArtifactVersionIncompatible {
            held,
            required,
            class,
        }) => {
            assert_eq!(held, SchemaVersion::new(99, 0, 0));
            assert_eq!(required, SchemaVersion::new(2, 0, 0));
            assert_eq!(class, CompatibilityClass::Rejected);
            Ok(())
        }
        other => Err(format!("expected exact future-version rejection, got {other:?}").into()),
    }
}

fn reject_outer1(result: Result<CompiledWorkflow, JournalError>) -> TestResult {
    match result {
        Err(JournalError::ArtifactVersionIncompatible {
            held,
            required,
            class,
        }) => {
            assert_eq!(held, SchemaVersion::new(1, 0, 0));
            assert_eq!(required, SchemaVersion::new(2, 0, 0));
            assert_eq!(class, CompatibilityClass::Rejected);
            Ok(())
        }
        other => Err(format!("expected exact outer-1 rejection, got {other:?}").into()),
    }
}

fn check_codec_rejection(artifact: &AcceptedArtifact, kind: Hostile) -> TestResult {
    let result = vb_storage::decode_compiled_ir(&artifact.ir, Some(artifact.artifact_version));
    if matches!(kind, Hostile::Future) {
        return reject_future(result);
    }
    if matches!(kind, Hostile::V1WithV2) {
        return reject_outer1(result);
    }
    match result {
        Err(JournalError::ArtifactMalformed) => Ok(()),
        other => Err(format!("expected ArtifactMalformed, got {other:?}").into()),
    }
}

fn legacy_record(workflow: &CompiledWorkflow) -> TestResult<CompiledIrRecord> {
    let ir = vb_storage::encode_compiled_ir(workflow)?.into_vec();
    assert_eq!(
        vb_storage::decode_compiled_ir(&ir, Some(CURRENT_IR_VERSION))?,
        *workflow
    );
    Ok(CompiledIrRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(&ir).into()),
        ir,
    })
}

fn hostile_record(workflow: &CompiledWorkflow, kind: Hostile) -> TestResult<CompiledIrRecord> {
    if matches!(kind, Hostile::LegacyBlob) {
        return legacy_record(workflow);
    }
    let mut artifact = vb_storage::admission::prepare_compiled_artifact(
        workflow,
        RuntimePolicy::Journaled,
        &[contract()?],
    )?;
    assert_eq!(
        vb_storage::admission::verify_accepted_artifact(&artifact)?,
        *workflow
    );
    tamper(&mut artifact, kind)?;
    check_codec_rejection(&artifact, kind)?;
    Ok(CompiledIrRecord {
        digest: artifact.digest,
        ir: postcard::to_allocvec(&artifact)?,
    })
}

fn seed_prefix(
    journal: &FjallJournal,
    workflow: &CompiledWorkflow,
    artifact: WorkflowDigest,
) -> TestResult {
    journal.append_journaled(&JournalEvent::RunAccepted {
        run: RUN,
        seq: EventSeq::new(0),
        workflow: workflow.digest(),
    })?;
    journal.append_journaled(&JournalEvent::RunAdmission {
        run: RUN,
        seq: EventSeq::new(1),
        artifact_digest: artifact,
        input_digest: RunInputBinding::LegacyUnbound,
        policy_pin: RunPolicyPin::Unverified,
        granted_capabilities: vb_core::CapabilitySet::empty(),
        policy: RuntimePolicy::Journaled,
    })?;
    Ok(())
}

fn reject(kind: Hostile) -> TestResult {
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let workflow = workflow(kind)?;
    let record = hostile_record(&workflow, kind)?;
    journal.put_compiled_ir(&record)?;
    seed_prefix(&journal, &workflow, record.digest)?;
    let runtime = open(journal.clone());
    let before = postcard::to_allocvec(&journal.events_for_run_full(RUN)?)?;
    assert_eq!(
        runtime.recover_and_resume(RUN),
        Err(RuntimeError::RecoveryCannotResume {
            reason: "artifact_decode_failed".into(),
        })
    );
    assert_eq!(
        postcard::to_allocvec(&journal.events_for_run_full(RUN)?)?,
        before
    );
    assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);
    Ok(())
}

fn assert_current_raw(workflow: &CompiledWorkflow) -> TestResult {
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        workflow,
        RuntimePolicy::Journaled,
        &[contract()?],
    )?;
    assert_eq!(artifact.artifact_version, CURRENT_IR_VERSION);
    assert_eq!(artifact.artifact_version, SchemaVersion::new(2, 0, 0));
    assert_eq!(artifact.ir, postcard::to_allocvec(&workflow.to_parts())?);
    let decoded = vb_storage::decode_compiled_ir(&artifact.ir, Some(artifact.artifact_version))?;
    assert_eq!(&decoded, workflow);
    assert_eq!(decoded.symbol_payloads(), None);
    Ok(())
}

#[test]
fn future_version_rejected() -> TestResult {
    reject(Hostile::Future)
}

#[test]
fn v1_with_v2_rejected() -> TestResult {
    reject(Hostile::V1WithV2)
}

#[test]
fn v2_with_v1_current_outer_recovers() -> TestResult {
    let workflow = workflow(Hostile::V2WithV1)?;
    assert_current_raw(&workflow)?;
    positive::check(workflow)
}

#[test]
fn bad_prefix_rejected() -> TestResult {
    reject(Hostile::BadPrefix)
}

#[test]
fn legacy_blob_rejected() -> TestResult {
    reject(Hostile::LegacyBlob)
}

#[test]
fn truncated_reserved_escape_rejected() -> TestResult {
    reject(Hostile::ShortEscape)
}
