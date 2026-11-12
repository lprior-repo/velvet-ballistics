use std::{fs, path::Path, process::Command};
use vb_core::{RunId, SlotValue, Taint};
use vb_storage::{AcceptedArtifact, FjallJournal};

const INPUT_FINISH: &str = "version: velvet-ballistics/v1\nname: retained_finish\nwhen:\n  manual: {}\ninputs:\n  count: number\nsteps:\n  - id: done\n    finish:\n      result: $input.count\n";

type TestResult = Result<(), Box<dyn std::error::Error>>;

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
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stderr.is_empty());
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

fn execute_retained(
    journal: &FjallJournal,
    run: RunId,
) -> Result<vb_core::engine::EngineSignal, Box<dyn std::error::Error>> {
    let retained = journal.run_inputs(run)?.ok_or("missing retained inputs")?;
    let artifact_record = journal
        .compiled_ir(retained.artifact_digest())?
        .ok_or("missing artifact")?;
    let artifact: AcceptedArtifact = postcard::from_bytes(&artifact_record.ir)?;
    assert_eq!(
        vb_storage::admission::accepted_artifact_digest(&artifact)?,
        retained.artifact_digest()
    );
    let compiled = vb_storage::admission::verify_accepted_artifact(&artifact)?;
    retained.validate_workflow(&compiled)?;
    let mut frame = vb_core::engine::new_run_frame(run, &compiled)?;
    vb_runtime::shard::helpers::seed_input_slots(&mut frame, retained.inputs(), &compiled)?;
    Ok(vb_core::engine::run_until_blocked(
        &compiled,
        &mut frame,
        vb_core::engine::StepBudget::MAX,
        &mut vb_core::value_store::ValueStore::new(),
    )?)
}

#[test]
fn submitted_values_survive_process_exit_and_reopen_and_drive_numeric_ir() -> TestResult {
    for mode in ["none", "journaled", "strict"] {
        for value in [i64::MIN, -7, 0, 41, i64::MAX] {
            let directory = tempfile::tempdir()?;
            let yaml = directory.path().join("workflow.yaml");
            let input = directory.path().join("input.bin");
            let db = directory.path().join("journal");
            fs::write(&yaml, INPUT_FINISH)?;
            fs::write(&input, postcard::to_allocvec(&vec![SlotValue::I64(value)])?)?;
            let run = submit(&yaml, &input, &db, mode)?;
            fs::remove_file(&yaml)?;
            fs::remove_file(&input)?;
            for _reopen in 0..2 {
                let journal = FjallJournal::open(&db, None)?;
                let retained = journal
                    .run_inputs(run)?
                    .ok_or("missing inputs after reopen")?;
                assert_eq!(
                    retained
                        .inputs()
                        .iter()
                        .map(|(_, value)| *value)
                        .collect::<Vec<_>>(),
                    [SlotValue::I64(value)]
                );
                let events = journal.events_for_run(run)?;
                if mode == "none" {
                    assert!(events.is_empty());
                } else {
                    let admission = events
                        .iter()
                        .find(|event| {
                            matches!(event, vb_storage::JournalEvent::RunAdmission { .. })
                        })
                        .ok_or("missing admission")?;
                    retained.validate_admission(admission)?;
                }
                assert_eq!(
                    execute_retained(&journal, run)?,
                    vb_core::engine::EngineSignal::Finished(SlotValue::I64(value), Taint::Clean)
                );
            }
        }
    }
    Ok(())
}

#[test]
fn dangling_arena_input_is_rejected_before_creating_database() -> TestResult {
    let yaml = INPUT_FINISH.replace("count: number", "count: list<number>");
    for mode in ["none", "journaled", "strict"] {
        let directory = tempfile::tempdir()?;
        let workflow = directory.path().join("workflow.yaml");
        let input = directory.path().join("input.bin");
        let db = directory.path().join("journal");
        fs::write(&workflow, &yaml)?;
        fs::write(
            &input,
            postcard::to_allocvec(&vec![SlotValue::List(vb_core::ListId::new(0))])?,
        )?;
        let result = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
            .arg("submit")
            .arg(workflow)
            .arg("--input-bin")
            .arg(input)
            .arg("--db")
            .arg(&db)
            .args(["--durability", mode])
            .output()?;
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&result.stderr),
            "runtime input schema mismatch\n"
        );
        assert!(!db.try_exists()?);
    }
    Ok(())
}
