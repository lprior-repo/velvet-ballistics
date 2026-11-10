use super::support::*;
use vb_core::{SlotValue, Taint};

#[test]
fn cold_submit_publishes_canonical_secret_and_recursive_defaults_without_host_values() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let source = format!(
        "version: velvet-ballistics/v1\nname: cold_secret\nwhen: {{manual: {{}}}}\nsecrets: {{token: {KEY_A}}}\ninputs:\n  config: {{is: object, default: {{}}, fields: {{label: {{is: text, default: guest}}, count: {{is: number, default: 3}}}}}}\nsteps:\n  - id: done\n    finish: {{result: '$secrets.token'}}\n"
    );
    let compiled = vb_compile::compile_workflow(source.as_bytes())?;
    assert!(matches!(
        vb_storage::RunInputsRecord::new(
            vb_core::RunId::new(92),
            compiled.digest(),
            compiled.digest(),
            &compiled,
            Box::new([])
        ),
        Err(vb_storage::JournalError::InputSchemaMismatch)
    ));
    let yaml = directory.path().join("source.yaml");
    let inputs = directory.path().join("empty.bin");
    let db = directory.path().join("journal");
    std::fs::write(&yaml, source)?;
    std::fs::write(&inputs, b"")?;
    let output = command()
        .arg("submit")
        .arg(&yaml)
        .arg("--input-bin")
        .arg(&inputs)
        .args(["--db"])
        .arg(&db)
        .args(["--durability", "strict", "--emit", "yaml"])
        .env(KEY_A, VALUE)
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
    assert_eq!(
        receipt.get("status").and_then(serde_json::Value::as_str),
        Some("submitted")
    );
    let run = vb_core::RunId::new(
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
    let digest =
        vb_core::WorkflowDigest::from_bytes(blake3::hash(std::fs::read(&yaml)?.as_slice()).into());
    let record = journal
        .owned_run_inputs(run, artifact.digest, digest, &compiled)?
        .ok_or("owned roots")?;
    let canonical = vb_core::root_bundle::OwnedRootBundle::new(
        Box::new([]),
        vb_core::value_store::ValueStore::with_max_slots(compiled.resource_contract().max_slots)
            .checkpoint_v1(),
        &compiled,
    )?;
    assert_eq!(record.roots(), canonical.roots());
    assert_eq!(record.checkpoint(), canonical.checkpoint());
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
    let store = vb_core::value_store::ValueStore::from_checkpoint_v1(record.checkpoint().clone())?;
    assert_eq!(store.symbol(binding.requirement())?, KEY_A);
    let object = record
        .roots()
        .iter()
        .find_map(|(_, value, _)| {
            if let SlotValue::Object(object) = value {
                Some(*object)
            } else {
                None
            }
        })
        .ok_or("config")?;
    let fields = store
        .object(object)?
        .iter()
        .map(|field| Ok((store.symbol(field.key)?, field.value, field.taint)))
        .collect::<TestResult<Vec<_>>>()?;
    assert!(fields.iter().any(|(key, value, taint)| *key == "count"
        && *value == SlotValue::I64(3)
        && *taint == Taint::Clean));
    let label = fields
        .iter()
        .find_map(|(key, value, _)| {
            if *key == "label" {
                if let SlotValue::Symbol(id) = value {
                    Some(*id)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .ok_or("label")?;
    assert_eq!(store.symbol(label)?, "guest");
    assert_eq!(fields.len(), 2);
    no_value(&postcard::to_allocvec(&record)?);
    no_value(&postcard::to_allocvec(&artifact)?);
    assert_eq!(journal.events_for_run_full(run)?.len(), 3);
    Ok(())
}

#[test]
fn cold_secret_submit_then_resume_uses_only_persisted_artifacts_and_rechecks_presence() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let yaml = directory.path().join("source.yaml");
    let inputs = directory.path().join("empty.bin");
    let db = directory.path().join("journal");
    std::fs::write(
        &yaml,
        format!(
            "version: velvet-ballistics/v1\nname: cold_resume_secret\nwhen: {{manual: {{}}}}\nsecrets: {{token: {KEY_A}}}\nsteps:\n  - id: done\n    finish: {{result: '$secrets.token'}}\n"
        ),
    )?;
    std::fs::write(&inputs, b"")?;
    let compiled = vb_compile::compile_workflow(&std::fs::read(&yaml)?)?;
    let output = command()
        .arg("submit")
        .arg(&yaml)
        .arg("--input-bin")
        .arg(&inputs)
        .arg("--db")
        .arg(&db)
        .args(["--durability", "strict", "--emit", "yaml"])
        .env(KEY_A, VALUE)
        .output()?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    no_value(&output.stdout);
    assert_eq!(output.stderr, b"");
    let receipt: serde_json::Value = serde_saphyr::from_str(std::str::from_utf8(&output.stdout)?)?;
    let run = vb_core::RunId::new(
        receipt
            .get("run_id")
            .and_then(serde_json::Value::as_u64)
            .ok_or("run")?,
    );
    let before = journal_events(&db, run)?;
    std::fs::remove_file(yaml)?;
    std::fs::remove_file(inputs)?;
    let missing = command()
        .args(["resume", &run.get().to_string(), "--db"])
        .arg(&db)
        .output()?;
    assert_eq!(missing.status.code(), Some(1));
    assert_eq!(missing.stdout, b"");
    assert_eq!(
        String::from_utf8(missing.stderr)?,
        "resume runtime: recovery: SECRET_UNAVAILABLE: secret requirement unavailable\n"
    );
    assert_eq!(journal_events(&db, run)?, before);
    let available = command()
        .args(["resume", &run.get().to_string(), "--db"])
        .arg(&db)
        .env(KEY_A, VALUE)
        .output()?;
    assert_eq!(
        available.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&available.stderr)
    );
    assert_eq!(available.stderr, b"");
    no_value(&available.stdout);
    let stdout = String::from_utf8(available.stdout)?;
    assert!(stdout.contains("run result: restricted\n"));
    assert!(stdout.ends_with("run completed\n"));
    let events = journal_events(&db, run)?;
    assert_eq!(events.get(..before.len()), Some(before.as_slice()));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunFinished { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunFailedEvent { .. }))
            .count(),
        0
    );
    assert_artifact(&db, &compiled)?;
    Ok(())
}

#[test]
fn cold_submit_does_not_invent_required_caller_input() -> TestResult {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("source.yaml");
    let inputs = directory.path().join("empty.bin");
    let db = directory.path().join("journal");
    std::fs::write(
        &source,
        format!(
            "version: velvet-ballistics/v1\nname: required_secret\nwhen: {{manual: {{}}}}\nsecrets: {{token: {KEY_A}}}\ninputs: {{count: number}}\nsteps:\n  - id: done\n    finish: {{result: '$input.count'}}\n"
        ),
    )?;
    std::fs::write(&inputs, b"")?;
    let output = command()
        .arg("submit")
        .arg(&source)
        .arg("--input-bin")
        .arg(&inputs)
        .arg("--db")
        .arg(&db)
        .args(["--durability", "strict"])
        .env(KEY_A, VALUE)
        .output()?;
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, b"");
    assert_eq!(
        String::from_utf8(output.stderr)?,
        "INPUT_MAPPING_FAILED: missing required input for slot SlotIdx(0)\n"
    );
    assert!(!db.exists());
    Ok(())
}
