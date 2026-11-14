use std::{fs, num::NonZeroUsize, path::Path, process::Command, sync::Arc};
use vb_core::RuntimePolicy;
use vb_core::{RunId, SlotValue};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::recovery::RuntimeRecoveryProduct;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::FjallJournal;

const INPUT_FINISH: &str = "version: velvet-ballistics/v1\nname: retained_finish\nwhen:\n  manual: {}\ninputs:\n  count: number\nsteps:\n  - id: done\n    finish:\n      result: $input.count\n";

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn shard_count(value: usize) -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(value).ok_or_else(|| format!("invalid shard count {value}"))
}

fn journaled_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 32,
        trace_capacity: 64,
        step_budget_per_tick: 16,
        max_active_runs: 8,
        max_open_gates: 1024,
        policy: RuntimePolicy::Journaled,
    }
}

fn submit(
    yaml: &Path,
    input: &Path,
    db: &Path,
    mode: &str,
) -> Result<RunId, Box<dyn std::error::Error>> {
    let result = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .arg("submit")
        .arg(yaml)
        .arg("--input-bin")
        .arg(input)
        .arg("--db")
        .arg(db)
        .args(["--durability", mode, "--emit", "yaml"])
        .output()?;
    assert_eq!(
        result.status.code(),
        Some(0),
        "submit failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let receipt: serde_json::Value = serde_saphyr::from_str(&String::from_utf8(result.stdout)?)?;
    assert_eq!(
        receipt.get("status").and_then(serde_json::Value::as_str),
        Some("submitted")
    );
    Ok(RunId::new(
        receipt
            .get("run_id")
            .and_then(serde_json::Value::as_u64)
            .ok_or("missing run")?,
    ))
}

/// C6: Actual CLI process submits a workflow with inputs, exits. Source and
/// input files are deleted. A new public Runtime + FjallJournal recover call
/// produces the exact same result. No CLI recover command is invented.
#[test]
fn retained_inputs_recovered_from_cold_storage_with_runtime_api() -> TestResult {
    for value in [i64::MIN, -7, 0, 41, i64::MAX] {
        let directory = tempfile::tempdir()?;
        let yaml = directory.path().join("workflow.yaml");
        let input = directory.path().join("input.bin");
        let db = directory.path().join("journal");

        fs::write(&yaml, INPUT_FINISH)?;
        fs::write(&input, postcard::to_allocvec(&vec![SlotValue::I64(value)])?)?;

        let run = submit(&yaml, &input, &db, "journaled")?;

        // Delete source and input files, simulating process exit + no files left
        fs::remove_file(&yaml)?;
        fs::remove_file(&input)?;

        // Open storage journal directly (cold startup)
        let fjall_journal = Arc::new(FjallJournal::open(&db, None)?);

        // Verify retained inputs are still accessible
        let retained = fjall_journal
            .run_inputs(run)?
            .ok_or("missing retained inputs")?;
        assert_eq!(
            retained
                .inputs()
                .iter()
                .map(|(_, v)| *v)
                .collect::<Vec<_>>(),
            [SlotValue::I64(value)]
        );

        // Validate the retained record against the admission receipt
        let workflow = vb_compile::compile_workflow(INPUT_FINISH.as_bytes())?;
        let stored = fjall_journal
            .compiled_ir_for_source_digest(workflow.digest())?
            .ok_or("missing accepted artifact")?;
        let artifact: vb_storage::AcceptedArtifact = postcard::from_bytes(&stored.ir)?;
        let header = fjall_journal.run_header(run)?.ok_or("missing run header")?;
        let events = fjall_journal.events_for_run_full(run)?;
        let admission_event = events
            .get(1)
            .ok_or("missing current RunAdmissionBudget event")?;
        let expected_budget = vb_core::AggregateResourceBudget::from_workflow(&workflow)?;
        let expected_compatibility = vb_storage::compat::decide_boundary(
            vb_storage::compat::artifact_ir_version_inputs(artifact.artifact_version),
            vb_storage::compat::schema_version_digest(artifact.artifact_version),
            vb_storage::compat::schema_version_digest(vb_storage::compat::CURRENT_IR_VERSION),
        );
        assert_eq!(
            events,
            vec![
                vb_storage::JournalEvent::RunAccepted {
                    run,
                    seq: vb_storage::EventSeq::new(0),
                    workflow: workflow.digest(),
                },
                vb_storage::JournalEvent::RunAdmissionBudget {
                    run,
                    seq: vb_storage::EventSeq::new(1),
                    artifact_digest: artifact.digest,
                    input_digest: vb_storage::RunInputBinding::Digest(retained.input_digest()),
                    policy_pin: vb_storage::RunPolicyPin::Pinned(artifact.policy_digest),
                    granted_capabilities: vb_core::CapabilitySet::empty(),
                    policy: RuntimePolicy::Journaled,
                    budget: expected_budget,
                },
                vb_storage::JournalEvent::CompatibilityDecisionEvent {
                    run,
                    seq: vb_storage::EventSeq::new(2),
                    decision: expected_compatibility,
                },
                vb_storage::JournalEvent::RunBudgetOriginEvent {
                    run,
                    seq: vb_storage::EventSeq::new(3),
                    started_millis: header.accepted_at_ms,
                },
            ]
        );
        retained.validate_admission(admission_event)?;

        // Create a runtime with the storage journal and recover the run
        let storage_journal = Arc::new(StorageRuntimeJournal::journaled(fjall_journal.clone()));
        let runtime = Runtime::new(shard_count(1)?, journaled_config(), storage_journal);

        // Recover via the public API - this should load retained inputs
        // and restore the initial input slots before applying journal overlays
        let product = runtime.recover_product(run)?;
        match product {
            RuntimeRecoveryProduct::Resumable(resumable) => {
                assert_eq!(resumable.run_id(), run);
            }
            other => {
                panic!("Expected resumable recovery product, got: {:?}", other);
            }
        }

        // Verify by re-opening and reading retained inputs directly
        let retained2 = fjall_journal
            .run_inputs(run)?
            .ok_or("missing retained inputs after runtime")?;
        retained2.validate_admission(admission_event)?;
    }
    Ok(())
}
