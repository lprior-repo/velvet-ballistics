use super::support::*;
use vb_core::{RunId, SlotValue, Taint};

fn source() -> String {
    format!(
        "version: velvet-ballistics/v1\nname: caller_inputs\nwhen: {{manual: {{}}}}\nsecrets: {{token: {KEY_A}}}\ninputs: {{count: number}}\nsteps:\n  - id: calculate\n    set: {{output: answer, value: '$input.count + 1'}}\n  - id: done\n    finish: {{result: answer}}\n"
    )
}

#[test]
fn socket_preserves_required_caller_input_beside_omitted_secret_root() -> TestResult {
    let directory = tempfile::tempdir()?;
    let workflow = vb_compile::compile_workflow(source().as_bytes())?;
    store(directory.path(), &[&workflow])?;
    let server = Server::start(directory.path(), &[KEY_A])?;
    let mut stream = server.connect()?;
    for inline in [false, true] {
        let run = RunId::new(if inline { 94 } else { 93 });
        let payload = vb_ipc::SubmitRunPayload {
            run_id: run,
            workflow: workflow.digest(),
            input: postcard::to_allocvec(&vec![SlotValue::I64(41)])?,
        };
        let (command, payload) = if inline {
            (
                vb_ipc::IpcCommand::SubmitRunInline,
                vb_ipc::IpcPayload::SubmitRunInline(payload),
            )
        } else {
            (
                vb_ipc::IpcCommand::SubmitRun,
                vb_ipc::IpcPayload::SubmitRun(payload),
            )
        };
        assert_eq!(
            request(&mut stream, command, payload)?,
            vb_ipc::server::IpcResponse::AcceptedRun { run_id: run.get() }
        );
    }
    server.stop()?;
    for run in [93, 94] {
        assert_answer_result(&journal_events(directory.path(), RunId::new(run))?, 42)?;
    }
    Ok(())
}

#[test]
fn cold_submit_keeps_caller_value_and_rejects_missing_host_before_publication() -> TestResult {
    let directory = tempfile::tempdir()?;
    let yaml = directory.path().join("source.yaml");
    let inputs = directory.path().join("inputs.bin");
    let db = directory.path().join("journal");
    let text = source();
    let compiled = vb_compile::compile_workflow(text.as_bytes())?;
    std::fs::write(&yaml, &text)?;
    std::fs::write(&inputs, postcard::to_allocvec(&vec![SlotValue::I64(41)])?)?;
    let mut missing = command();
    missing
        .arg("submit")
        .arg(&yaml)
        .arg("--input-bin")
        .arg(&inputs)
        .arg("--db")
        .arg(&db)
        .args(["--durability", "strict"]);
    let output = missing.output()?;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"");
    assert_eq!(
        String::from_utf8(output.stderr)?,
        "SECRET_UNAVAILABLE: secret requirement unavailable\n"
    );
    assert!(!db.exists());
    let output = missing
        .env(KEY_A, VALUE)
        .args(["--emit", "yaml"])
        .output()?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stderr, b"");
    no_value(&output.stdout);
    let receipt: serde_json::Value = serde_saphyr::from_str(std::str::from_utf8(&output.stdout)?)?;
    let run = RunId::new(
        receipt
            .get("run_id")
            .and_then(serde_json::Value::as_u64)
            .ok_or("run")?,
    );
    let journal = vb_storage::FjallJournal::open(&db, None)?;
    let artifact = journal
        .compiled_ir_for_source_digest(compiled.digest())?
        .ok_or("artifact")?;
    let artifact: vb_storage::AcceptedArtifact = postcard::from_bytes(&artifact.ir)?;
    let source_digest = vb_core::WorkflowDigest::from_bytes(blake3::hash(text.as_bytes()).into());
    let record = journal
        .owned_run_inputs(run, artifact.digest, source_digest, &compiled)?
        .ok_or("owned roots")?;
    assert_eq!(record.roots().len(), 2);
    let input = compiled
        .input_slots()
        .iter()
        .find(|input| {
            input.kind == vb_core::InputSlotKind::I64 && input.constraint.requires_value()
        })
        .ok_or("count input")?
        .slot;
    assert!(
        record
            .roots()
            .contains(&(input, SlotValue::I64(41), Taint::Clean))
    );
    let binding = compiled
        .source_metadata()
        .ok_or("metadata")?
        .secret_requirements()
        .first()
        .ok_or("binding")?;
    assert!(record.roots().contains(&(
        binding.slot().ok_or("slot")?,
        SlotValue::Symbol(binding.requirement()),
        Taint::Secret
    )));
    no_value(&postcard::to_allocvec(&record)?);
    Ok(())
}
