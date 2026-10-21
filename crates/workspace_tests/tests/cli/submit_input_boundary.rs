use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;
use vb_core::SlotValue;

const NO_PORTS_YAML: &str = "version: velvet-ballistics/v1\nname: submit_boundary\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      output: saved\n      value: '42'\n  - id: done\n    finish:\n      result: saved\n";

const NUMBER_PORT_YAML: &str = "version: velvet-ballistics/v1\nname: submit_number\nwhen:\n  manual: {}\ninputs:\n  count: number\nsteps:\n  - id: build_result\n    save:\n      output: saved\n      value: '42'\n  - id: done\n    finish:\n      result: saved\n";

fn subprocess(wf: &str, inp: &str, db: &str, mode: &str) -> std::io::Result<std::process::Output> {
    Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args([
            "submit",
            wf,
            "--input-bin",
            inp,
            "--db",
            db,
            "--durability",
            mode,
        ])
        .output()
}

fn write_file(dir: &tempfile::TempDir, name: &str, data: &[u8]) -> std::io::Result<String> {
    let path = dir.path().join(name);
    let mut f = fs::File::create(&path)?;
    f.write_all(data)?;
    Ok(path.to_string_lossy().to_string())
}

fn db_path(dir: &tempfile::TempDir) -> String {
    dir.path().join("db").to_string_lossy().to_string()
}

#[test]
fn malformed_decode_fails_all_durabilities() -> Result<(), Box<dyn std::error::Error>> {
    for mode in ["none", "journaled", "strict"] {
        let dir = tempdir()?;
        let wf = write_file(&dir, "wf.yaml", NO_PORTS_YAML.as_bytes())?;
        let inp = write_file(&dir, "in.bin", &[255])?;
        let db = db_path(&dir);
        let out = subprocess(&wf, &inp, &db, mode)?;
        assert_eq!(out.status.code(), Some(2));
        assert_eq!(out.stdout, b"");
        assert_eq!(
            out.stderr,
            b"INPUT_MAPPING_FAILED: input-bin decode failed\n"
        );
        assert!(!Path::new(&db).try_exists()?);
    }
    Ok(())
}

#[test]
fn encoded_empty_plus_garbage_fails_all_durabilities() -> Result<(), Box<dyn std::error::Error>> {
    for mode in ["none", "journaled", "strict"] {
        let dir = tempdir()?;
        let wf = write_file(&dir, "wf.yaml", NO_PORTS_YAML.as_bytes())?;
        let inp = write_file(&dir, "in.bin", &[0, 255])?;
        let db = db_path(&dir);
        let out = subprocess(&wf, &inp, &db, mode)?;
        assert_eq!(out.status.code(), Some(2));
        assert_eq!(out.stdout, b"");
        assert_eq!(
            out.stderr,
            b"INPUT_MAPPING_FAILED: input-bin decode failed\n"
        );
        assert!(!Path::new(&db).try_exists()?);
    }
    Ok(())
}

#[test]
fn excess_slots_fails_all_durabilities() -> Result<(), Box<dyn std::error::Error>> {
    let encoded = postcard::to_allocvec(&Box::<[SlotValue]>::from([SlotValue::I64(7)]))?;
    for mode in ["none", "journaled", "strict"] {
        let dir = tempdir()?;
        let wf = write_file(&dir, "wf.yaml", NO_PORTS_YAML.as_bytes())?;
        let inp = write_file(&dir, "in.bin", &encoded)?;
        let db = db_path(&dir);
        let out = subprocess(&wf, &inp, &db, mode)?;
        assert_eq!(out.status.code(), Some(2));
        assert_eq!(out.stdout, b"");
        assert_eq!(
            out.stderr,
            b"INPUT_MAPPING_FAILED: input slot count exceeds workflow slot count\n"
        );
        assert!(!Path::new(&db).try_exists()?);
    }
    Ok(())
}

#[test]
fn bool_kind_mismatch_fails_all_durabilities() -> Result<(), Box<dyn std::error::Error>> {
    let compiled = vb_compile::compile_workflow(NUMBER_PORT_YAML.as_bytes())?;
    let slot = compiled.input_slots().first().ok_or("no input slot")?;
    let encoded = postcard::to_allocvec(&Box::<[SlotValue]>::from([SlotValue::Bool(true)]))?;
    for mode in ["none", "journaled", "strict"] {
        let dir = tempdir()?;
        let wf = write_file(&dir, "wf.yaml", NUMBER_PORT_YAML.as_bytes())?;
        let inp = write_file(&dir, "in.bin", &encoded)?;
        let db = db_path(&dir);
        let out = subprocess(&wf, &inp, &db, mode)?;
        assert_eq!(out.status.code(), Some(2));
        assert_eq!(out.stdout, b"");
        let expected = format!(
            "INPUT_MAPPING_FAILED: slot {:?} value kind mismatch, expected I64\n",
            slot.slot
        );
        assert_eq!(out.stderr, expected.as_bytes());
        assert!(!Path::new(&db).try_exists()?);
    }
    Ok(())
}

#[test]
fn test_submit_controls() -> Result<(), Box<dyn std::error::Error>> {
    use vb_compile::compile_workflow;
    use vb_storage::{EventSeq, FjallJournal};

    let modes = ["none", "journaled", "strict"];
    let cases = [
        (NO_PORTS_YAML, vec![]),
        (
            NO_PORTS_YAML,
            postcard::to_allocvec(&Box::<[SlotValue]>::from([]))?,
        ),
        (
            NUMBER_PORT_YAML,
            postcard::to_allocvec(&Box::<[SlotValue]>::from([SlotValue::I64(42)]))?,
        ),
    ];

    for mode in &modes {
        for (yaml, input) in &cases {
            let dir = tempdir()?;
            let wf_path = write_file(&dir, "wf.yaml", yaml.as_bytes())?;
            let inp_path = write_file(&dir, "inp.bin", input)?;
            let db = db_path(&dir);

            let out = subprocess(&wf_path, &inp_path, &db, mode)?;
            assert_eq!(out.status.code(), Some(0));
            assert!(out.stderr.is_empty());

            let compiled = compile_workflow(yaml.as_bytes())?;
            let journal = FjallJournal::open(&db, None)?;
            let headers = journal.run_headers()?;
            assert_eq!(headers.len(), 1);
            let header = headers.first().ok_or("missing header")?;
            assert_eq!(header.compiled_digest, compiled.digest());
            assert_eq!(header.status, 0);

            let digest_hex = compiled
                .digest()
                .as_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            let expected = format!(
                "submitted run {}\n  digest:     {}\n  steps:      {}\n  durability: {}\n  status:     submitted\n",
                header.run.get(),
                digest_hex,
                compiled.node_count(),
                mode
            );
            assert_eq!(String::from_utf8_lossy(&out.stdout), expected);

            let retained = journal
                .run_inputs(header.run)?
                .ok_or("missing retained inputs")?;
            assert_eq!(retained.workflow_digest(), compiled.digest());
            let expected_values: Box<[SlotValue]> = if input.is_empty() {
                Box::new([])
            } else {
                postcard::from_bytes(input)?
            };
            let expected_slots = compiled
                .input_slots()
                .iter()
                .zip(expected_values.iter())
                .map(|(slot, value)| (slot.slot, *value))
                .collect::<Vec<_>>();
            assert_eq!(retained.inputs(), expected_slots);
            let record = journal
                .compiled_ir(retained.artifact_digest())?
                .ok_or("missing artifact")?;
            let artifact: vb_storage::AcceptedArtifact = postcard::from_bytes(&record.ir)?;
            assert_eq!(artifact.digest, retained.artifact_digest());
            assert_eq!(artifact.source_digest, compiled.digest());
            assert_eq!(
                retained.source_digest(),
                vb_core::WorkflowDigest::from_bytes(blake3::hash(yaml.as_bytes()).into())
            );
            assert_eq!(
                journal
                    .workflow_source(retained.source_digest())?
                    .ok_or("missing source")?
                    .source,
                yaml.as_bytes()
            );
            let events = journal.events_for_run(header.run)?;
            if *mode == "none" {
                assert!(events.is_empty());
            } else {
                assert_eq!(
                    events,
                    vec![
                        vb_storage::JournalEvent::RunAccepted {
                            run: header.run,
                            seq: EventSeq::new(0),
                            workflow: compiled.digest()
                        },
                        vb_storage::JournalEvent::RunAdmission {
                            input_digest: vb_storage::RunInputBinding::Digest(
                                retained.input_digest()
                            ),
                            run: header.run,
                            seq: EventSeq::new(1),
                            artifact_digest: retained.artifact_digest(),
                            granted_capabilities: vb_core::CapabilitySet::empty(),
                            policy: if *mode == "strict" {
                                vb_core::RuntimePolicy::Strict
                            } else {
                                vb_core::RuntimePolicy::Journaled
                            },
                        }
                    ]
                );
            }
        }
    }
    Ok(())
}

#[test]
fn validation_precedes_journal_open() -> Result<(), Box<dyn std::error::Error>> {
    for mode in ["none", "journaled", "strict"] {
        let dir = tempdir()?;
        let db = db_path(&dir);
        let wf = write_file(&dir, "wf.yaml", NO_PORTS_YAML.as_bytes())?;
        let inp = write_file(&dir, "input.bin", &[255])?;
        let journal = vb_storage::FjallJournal::open(&db, None)?;
        let compiled = vb_compile::compile_workflow(NO_PORTS_YAML.as_bytes())?;
        let run = vb_core::RunId::new(77);
        let header = vb_storage::RunHeaderRecord {
            run,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: compiled.digest(),
            status: 0,
            accepted_at_ms: 1,
        };
        vb_storage::put_run_header(&journal, &header)?;
        journal.append_strict_batch(&[vb_storage::JournalEvent::RunAccepted {
            run,
            seq: vb_storage::EventSeq::new(0),
            workflow: compiled.digest(),
        }])?;
        let before_headers = journal.run_headers()?;
        let before_events = journal.events_for_run(run)?;
        let result = subprocess(&wf, &inp, &db, mode)?;
        assert_eq!(result.status.code(), Some(2));
        assert_eq!(result.stdout, b"");
        assert_eq!(
            result.stderr,
            b"INPUT_MAPPING_FAILED: input-bin decode failed\n"
        );
        let after_headers = journal.run_headers()?;
        let after_events = journal.events_for_run(run)?;
        assert_eq!(before_headers, after_headers);
        assert_eq!(before_events, after_events);
        drop(journal);
    }
    Ok(())
}
