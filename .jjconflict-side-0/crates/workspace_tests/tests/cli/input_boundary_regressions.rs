use std::{fs, process::Command};
use vb_core::{RunId, SlotValue, Taint};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn report(output: &std::process::Output) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stderr, b"");
    Ok(serde_saphyr::from_str(std::str::from_utf8(
        &output.stdout,
    )?)?)
}

fn source(schema: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: nullable_cli\nwhen: {{manual: {{}}}}\ninputs: {{value: {schema}}}\nsteps:\n  - id: copied\n    set: {{output: copied, value: '$input.value'}}\n  - id: done\n    finish: {{result: '$steps.copied'}}\n"
    )
}

const NULLABLE: [&str; 5] = [
    "{is: number, nullable: true, default: 99}",
    "{is: boolean, nullable: true}",
    "{is: text, nullable: true}",
    "{is: list, of: number, nullable: true, max: 2}",
    "{is: object, nullable: true, fields: {count: number}}",
];

#[test]
fn examples_bind_ordinary_inputs_before_one_or_more_secret_roots() -> TestResult {
    for secrets in [
        "{token: VB_CLI_EXAMPLE_SECRET_A}",
        "{token: VB_CLI_EXAMPLE_SECRET_A, other: VB_CLI_EXAMPLE_SECRET_B}",
    ] {
        let dir = tempfile::tempdir()?;
        let yaml = dir.path().join("examples.yaml");
        fs::write(
            &yaml,
            format!(
                "version: velvet-ballistics/v1\nname: secret_examples\nwhen: {{manual: {{}}}}\ninputs:\n  first: {{is: number, default: 40}}\n  second: {{is: number, default: 2}}\nsecrets: {secrets}\nvars: {{marker: retained_source_payload}}\nsteps:\n  - id: answer\n    set: {{output: answer, value: '$input.first + $input.second'}}\nresult: {{value: '$steps.answer'}}\nexamples:\n  - input: {{first: 7, second: 9}}\n    expected: {{value: 16}}\n  - input: {{first: 5}}\n    expected: {{value: 7}}\n  - input: {{}}\n    expected: {{value: 42}}\n"
            ),
        )?;
        for profile in ["quick", "standard", "full"] {
            let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
                .arg("verify")
                .arg(&yaml)
                .args(["--profile", profile, "--emit", "yaml"])
                .env_remove("VB_CLI_EXAMPLE_SECRET_A")
                .env_remove("VB_CLI_EXAMPLE_SECRET_B")
                .output()?;
            let doc = report(&output)?;
            assert_eq!(doc.get("success"), Some(&serde_json::Value::Bool(true)));
            assert_eq!(doc["profile"], profile);
            let mut checks = vec!["yaml_parse", "compilation", "ir_validation"];
            if profile != "quick" {
                checks.extend(["budget_computation", "boundedness_policy"]);
            }
            checks.push("examples");
            assert_eq!(doc["checks"], serde_json::to_value(checks)?);
        }
    }
    Ok(())
}

#[test]
fn explicit_nullable_null_executes_both_run_commands() -> TestResult {
    for schema in NULLABLE {
        let dir = tempfile::tempdir()?;
        let yaml = dir.path().join("workflow.yaml");
        let ir = dir.path().join("workflow.ir");
        let input = dir.path().join("input.bin");
        let text = source(schema);
        let workflow = vb_compile::compile_workflow(text.as_bytes())?;
        fs::write(&yaml, text)?;
        fs::write(&ir, postcard::to_allocvec(&workflow.to_parts())?)?;
        fs::write(&input, postcard::to_allocvec(&vec![SlotValue::Null])?)?;
        for (command, target) in [("run", &yaml), ("run-compiled", &ir)] {
            let db = dir.path().join(command);
            let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
                .arg(command)
                .arg(target)
                .arg("--input-bin")
                .arg(&input)
                .args(["--durability", "strict", "--emit", "yaml", "--db"])
                .arg(&db)
                .output()?;
            let doc = report(&output)?;
            assert_eq!(doc["status"], "completed");
            assert_eq!(
                doc["runtime"],
                serde_json::json!({"submitted": 1, "completed": 1, "failed": 0, "steps": 2})
            );
            let events =
                vb_storage::FjallJournal::open(&db, None)?.events_for_run_full(RunId::new(1))?;
            let results = events
                .iter()
                .filter_map(|event| match event {
                    vb_storage::JournalEvent::RunFinished { result, .. } => Some(*result),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(results, vec![vb_core::SlotIdx::new(0)]);
            let result_slot = results.first().ok_or("result")?;
            let payload = events
                .iter()
                .rev()
                .find_map(|event| match event {
                    vb_storage::JournalEvent::SlotWrittenEvent {
                        slot,
                        value: Some(bytes),
                        ..
                    } if slot == result_slot => Some(bytes),
                    _ => None,
                })
                .ok_or("missing result value evidence")?;
            assert_eq!(postcard::from_bytes::<SlotValue>(payload)?, SlotValue::Null);
        }
    }
    Ok(())
}

#[test]
fn explicit_nullable_null_survives_public_submit_admission() -> TestResult {
    for schema in NULLABLE {
        for mode in ["none", "journaled", "strict"] {
            let dir = tempfile::tempdir()?;
            let yaml = dir.path().join("workflow.yaml");
            let input = dir.path().join("input.bin");
            let db = dir.path().join("database");
            let text = source(schema);
            let workflow = vb_compile::compile_workflow(text.as_bytes())?;
            fs::write(&yaml, text)?;
            fs::write(&input, postcard::to_allocvec(&vec![SlotValue::Null])?)?;
            let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
                .arg("submit")
                .arg(&yaml)
                .arg("--input-bin")
                .arg(&input)
                .arg("--db")
                .arg(&db)
                .args(["--durability", mode, "--emit", "yaml"])
                .output()?;
            let doc = report(&output)?;
            assert_eq!(doc["status"], "submitted");
            let run = RunId::new(doc["run_id"].as_u64().ok_or("run id")?);
            let journal = vb_storage::FjallJournal::open(&db, None)?;
            let retained = journal.run_inputs(run)?.ok_or("retained inputs")?;
            let slot = workflow.input_slots().first().ok_or("input")?.slot;
            assert_eq!(retained.inputs(), &[(slot, SlotValue::Null)]);
            retained.validate_workflow(&workflow)?;
            let mut frame = vb_core::new_run_frame(run, &workflow)?;
            vb_runtime::shard::helpers::seed_input_slots(&mut frame, retained.inputs(), &workflow)?;
            assert_eq!(
                vb_core::engine::run_until_blocked(
                    &workflow,
                    &mut frame,
                    vb_core::StepBudget::MAX,
                    &mut vb_core::ValueStore::new()
                )?,
                vb_core::EngineSignal::Finished(SlotValue::Null, Taint::Clean)
            );
        }
    }
    Ok(())
}

#[test]
fn nonnullable_null_remains_rejected_by_every_cli_caller() -> TestResult {
    let dir = tempfile::tempdir()?;
    let yaml = dir.path().join("workflow.yaml");
    let ir = dir.path().join("workflow.ir");
    let input = dir.path().join("input.bin");
    let text = source("{is: number}");
    let workflow = vb_compile::compile_workflow(text.as_bytes())?;
    fs::write(&yaml, text)?;
    fs::write(&ir, postcard::to_allocvec(&workflow.to_parts())?)?;
    fs::write(&input, postcard::to_allocvec(&vec![SlotValue::Null])?)?;
    for (command, target, code) in [
        ("run", &yaml, 1),
        ("run-compiled", &ir, 3),
        ("submit", &yaml, 2),
    ] {
        let db = dir.path().join(command);
        let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
            .arg(command)
            .arg(target)
            .arg("--input-bin")
            .arg(&input)
            .args(["--durability", "strict", "--db"])
            .arg(&db)
            .output()?;
        assert_eq!(output.status.code(), Some(code));
        assert_eq!(output.stdout, b"");
        assert_eq!(
            String::from_utf8(output.stderr)?,
            format!(
                "INPUT_MAPPING_FAILED: slot {:?} value kind mismatch, expected I64\n",
                workflow.input_slots().first().ok_or("input")?.slot
            )
        );
        assert!(!db.try_exists()?);
    }
    Ok(())
}
