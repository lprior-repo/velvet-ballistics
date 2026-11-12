#![forbid(unsafe_code)]
use std::path::{Path, PathBuf};
use std::process::Command;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const KEY: &str = "VB_F1_FAKE_SECRET_PRESENCE_20261110";
const VALUE: &str = "fixture-only-value-32bc912a-891e-4ac9-9ae6-fd16c19d1770";

struct Fixture {
    directory: tempfile::TempDir,
    source: PathBuf,
    ir: PathBuf,
    inputs: PathBuf,
    workflow: vb_core::CompiledWorkflow,
}

impl Fixture {
    fn new(key: &str) -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let source = directory.path().join("secret.yaml");
        let inputs = directory.path().join("inputs.bin");
        let ir = directory.path().join("secret.vbir");
        let text = format!(
            "version: velvet-ballistics/v1\nname: opaque_secret\nwhen: {{manual: {{}}}}\nsecrets: {{token: {key:?}}}\nsteps:\n  - id: done\n    finish: {{result: '$secrets.token'}}\n"
        );
        std::fs::write(&source, text.as_bytes())?;
        std::fs::write(&inputs, b"")?;
        let workflow = vb_compile::compile_workflow(text.as_bytes())?;
        std::fs::write(&ir, postcard::to_allocvec(&workflow.to_parts())?)?;
        Ok(Self {
            directory,
            source,
            ir,
            inputs,
            workflow,
        })
    }

    fn command(&self, name: &str, path: &Path, db: &Path) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"));
        command
            .arg(name)
            .arg(path)
            .arg("--input-bin")
            .arg(&self.inputs)
            .args(["--durability", "journaled", "--db"])
            .arg(db)
            .env_remove(KEY);
        command
    }
}

fn assert_no_value(bytes: &[u8]) {
    assert!(
        !bytes
            .windows(VALUE.len())
            .any(|window| window == VALUE.as_bytes()),
        "fake value crossed the host presence boundary"
    );
}

#[test]
fn cli_runs_yaml_and_ir_from_environment_presence_without_persisting_the_value() -> TestResult {
    let fixture = Fixture::new(KEY)?;
    let metadata = fixture.workflow.source_metadata().ok_or("metadata")?;
    let binding = metadata.secret_requirements().first().ok_or("binding")?;
    assert_eq!(metadata.symbol(binding.requirement()), Some(KEY));
    assert_eq!(
        metadata
            .symbols()
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<&str>>(),
        vec!["token", KEY]
    );
    for (name, path) in [("run", &fixture.source), ("run-compiled", &fixture.ir)] {
        let db = fixture.directory.path().join(name);
        let output = fixture.command(name, path, &db).env(KEY, VALUE).output()?;
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("run completed"));
        assert_eq!(output.stderr, b"");
        assert_no_value(&output.stdout);
        assert_no_value(&std::fs::read(&fixture.ir)?);
        let journal = vb_storage::FjallJournal::open(&db, None)?;
        let events = journal.events_for_run_full(vb_core::RunId::new(1))?;
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
        assert_no_value(&postcard::to_allocvec(&events)?);
        let artifact = journal
            .compiled_ir_for_source_digest(fixture.workflow.digest())?
            .ok_or("persisted artifact")?;
        assert_no_value(&postcard::to_allocvec(&artifact)?);
    }
    Ok(())
}

#[test]
fn cli_missing_or_invalid_host_keys_reject_before_run_admission() -> TestResult {
    for key in [KEY, "invalid=key"] {
        let fixture = Fixture::new(key)?;
        for (name, path) in [("run", &fixture.source), ("run-compiled", &fixture.ir)] {
            let db = fixture.directory.path().join(name);
            let output = fixture.command(name, path, &db).output()?;
            assert_eq!(output.status.code(), Some(1));
            assert_eq!(output.stdout, b"");
            assert_eq!(
                String::from_utf8(output.stderr)?,
                "runtime submit error: SECRET_UNAVAILABLE: secret requirement unavailable\n"
            );
            let journal = vb_storage::FjallJournal::open(&db, None)?;
            assert_eq!(journal.events_for_run_full(vb_core::RunId::new(1))?, vec![]);
            assert_eq!(journal.run_header(vb_core::RunId::new(1))?, None);
        }
    }
    Ok(())
}

#[test]
fn cli_compiled_ir_rejects_a_nul_host_key_without_calling_var_os() -> TestResult {
    let fixture = Fixture::new(KEY)?;
    let mut parts = fixture.workflow.to_parts();
    let metadata = parts.source_metadata.take().ok_or("metadata")?;
    let binding = *metadata.secret_requirements().first().ok_or("binding")?;
    let mut symbols = metadata.symbols().to_vec();
    *symbols
        .get_mut(usize::try_from(binding.requirement().get())?)
        .ok_or("requirement")? = "invalid\0key".into();
    parts.source_metadata = Some(Box::new(
        vb_core::workflow::source_metadata::WorkflowSourceMetadata::try_new(
            symbols.into_boxed_slice(),
            metadata.secret_requirements().into(),
        )?,
    ));
    parts
        .input_slots
        .iter_mut()
        .find(|input| Some(input.slot) == binding.slot())
        .ok_or("input")?
        .constraint
        .default_cold = Some(vb_core::InputDefaultValue::Text("invalid\0key".into()));
    let workflow = vb_core::CompiledWorkflow::try_from_parts(parts)?;
    std::fs::write(&fixture.ir, postcard::to_allocvec(&workflow.to_parts())?)?;
    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .arg("run-compiled")
        .arg(&fixture.ir)
        .arg("--input-bin")
        .arg(&fixture.inputs)
        .args(["--durability", "none"])
        .env_remove(KEY)
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"");
    assert_eq!(
        String::from_utf8(output.stderr)?,
        "runtime submit error: SECRET_UNAVAILABLE: secret requirement unavailable\n"
    );
    Ok(())
}
