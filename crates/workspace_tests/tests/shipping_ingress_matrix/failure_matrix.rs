//! Admission and recovery failure matrix: queue overflow, duplicate delivery,
//! and durable digest mismatch.

#![forbid(unsafe_code)]

use vb_core::value::Taint;
use vb_core::{CapabilitySet, CompiledWorkflow, RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx};
use vb_runtime::shard::ShardConfig;
use vb_storage::slot_extra::encode_slot_written_extra;
use vb_storage::{EventSeq, JournalEvent, RunInputBinding, RunPolicyPin};

use super::support::{
    Durability, TestResult, compile, events, open_runtime, open_runtime_with_config,
    persist_artifact,
};

const HEADER: &str = "version: velvet-ballistics/v1\nname: shipping_failure\nwhen: {manual: {}}\n";

fn wait_source() -> String {
    format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: deadline\n    set: {{output: delay, value: '60000'}}\n  - id: gate\n    wait: {{timeout: '$step.deadline'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
    )
}

fn finish_source() -> String {
    format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
    )
}

#[test]
fn queue_full_rejects_second_submit_and_preserves_first() -> TestResult {
    let workflow = compile(&wait_source())?;
    let dir = tempfile::tempdir()?;
    let mut config = ShardConfig::default();
    config.command_queue_capacity = 1;
    config.policy = RuntimePolicy::Strict;
    let mut open = open_runtime_with_config(dir.path(), config, Durability::Strict)?;
    persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &[])?;
    open.runtime
        .submit_compiled(RunId::new(201), workflow.clone())?;
    let error = open
        .runtime
        .submit_compiled(RunId::new(202), workflow)
        .expect_err("second submit must overflow the command queue");
    assert_eq!(error, vb_runtime::RuntimeError::QueueFull);
    assert!(matches!(
        open.runtime.snapshot_run(RunId::new(202), 0)?,
        vb_runtime::shard::InspectResponse::NotFound { .. }
    ));
    open.runtime.tick_all()?;
    assert!(matches!(
        open.runtime.snapshot_run(RunId::new(201), 0)?,
        vb_runtime::shard::InspectResponse::Found(_)
    ));
    assert_eq!(events(&open.journal, RunId::new(202))?, Vec::new());
    Ok(())
}

#[test]
fn duplicate_submit_is_rejected_by_the_owning_shard() -> TestResult {
    let workflow = compile(&wait_source())?;
    let dir = tempfile::tempdir()?;
    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &[])?;
    let run = RunId::new(203);
    open.runtime.submit_compiled(run, workflow.clone())?;
    open.runtime.submit_compiled(run, workflow)?;
    assert!(open.runtime.tick_all()?);
    let error = open
        .runtime
        .tick_all()
        .expect_err("duplicate submit must be rejected");
    assert_eq!(error, vb_runtime::RuntimeError::RunAlreadyExists);
    let durable = events(&open.journal, run)?;
    assert_eq!(
        super::support::count_where(&durable, |event| matches!(
            event,
            JournalEvent::RunAccepted { run: actual, .. } if *actual == run
        )),
        1
    );
    Ok(())
}

#[test]
fn recovery_rejects_mismatched_workflow_digest_without_mutation() -> TestResult {
    let accepted = compile(&finish_source())?;
    let mismatched = compile(&wait_source())?;
    assert_ne!(accepted.digest(), mismatched.digest());
    let dir = tempfile::tempdir()?;
    let run = RunId::new(204);
    let open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    let artifact = persist_artifact(&open.journal, &accepted, RuntimePolicy::Strict, &[])?;
    let extra = encode_slot_written_extra(Taint::Clean, None)
        .map_err(|error| format!("slot extra encode failed: {error:?}"))?;
    let bytes = postcard::to_allocvec(&SlotValue::I64(1))?;
    let appended = [
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: mismatched.digest(),
        },
        JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest: artifact.digest,
            input_digest: RunInputBinding::LegacyUnbound,
            policy_pin: RunPolicyPin::Unverified,
            granted_capabilities: CapabilitySet::empty(),
            policy: RuntimePolicy::Strict,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(3),
            slot: SlotIdx::ZERO,
            value: Some(bytes),
            extra: Some(extra),
            attempt: 1,
        },
    ];
    for event in &appended {
        open.journal.append_strict(event)?;
    }
    let before = events(&open.journal, run)?;
    let error = open
        .runtime
        .recover_product(run)
        .expect_err("digest mismatch must fail closed");
    assert!(
        matches!(
            &error,
            vb_runtime::RuntimeError::RecoveryCannotResume { reason }
                if reason == "workflow_digest_mismatch"
        ),
        "unexpected recovery error: {error:?}"
    );
    assert_eq!(events(&open.journal, run)?, before);
    assert_eq!(
        open.runtime.recover_and_resume(run),
        Err(vb_runtime::RuntimeError::RecoveryCannotResume {
            reason: String::from("workflow_digest_mismatch"),
        })
    );
    assert_eq!(events(&open.journal, run)?, before);
    Ok(())
}

#[test]
fn unknown_artifact_recovery_is_typed() -> TestResult {
    let workflow: CompiledWorkflow = compile(&finish_source())?;
    let dir = tempfile::tempdir()?;
    let run = RunId::new(205);
    let open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    let missing = vb_core::WorkflowDigest::from_bytes([0x5A; 32]);
    let extra = encode_slot_written_extra(Taint::Clean, None)
        .map_err(|error| format!("slot extra encode failed: {error:?}"))?;
    let bytes = postcard::to_allocvec(&SlotValue::I64(1))?;
    for event in [
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: workflow.digest(),
        },
        JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest: missing,
            input_digest: RunInputBinding::LegacyUnbound,
            policy_pin: RunPolicyPin::Unverified,
            granted_capabilities: CapabilitySet::empty(),
            policy: RuntimePolicy::Strict,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::ZERO,
            value: Some(bytes),
            extra: Some(extra),
            attempt: 1,
        },
    ] {
        open.journal.append_strict(&event)?;
    }
    let error = open
        .runtime
        .recover_product(run)
        .expect_err("missing artifact must fail closed");
    assert!(
        matches!(
            &error,
            vb_runtime::RuntimeError::RecoveryCannotResume { reason }
                if reason == "artifact_missing"
        ),
        "unexpected recovery error: {error:?}"
    );
    Ok(())
}
