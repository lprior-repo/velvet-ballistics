#![cfg(test)]

use super::{oracles, wire};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use vb_core::{RunId, RuntimePolicy};
use vb_storage::{
    AcceptedArtifact, EventSeq, FjallJournal,
    constants::KEYSPACE_RUN_EVENT,
    types::{KeyspaceProfile, keyspace_options_for},
};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) const SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: events_filters\nwhen: {manual: {}}\nsteps:\n  - id: first\n    set: {output: first_value, value: '7'}\n  - id: question\n    ask: {prompt: '$first'}\n  - id: done\n    finish: {result: '$question'}\n";

pub(super) struct Fixture {
    pub(super) path: PathBuf,
    directory: tempfile::TempDir,
}

impl Fixture {
    pub(super) fn new() -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let workflow = directory.path().join("workflow.yaml");
        let inputs = directory.path().join("inputs.bin");
        let path = directory.path().join("journal");
        std::fs::write(&workflow, SOURCE)?;
        std::fs::write(&inputs, [])?;
        let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
            .arg("run")
            .arg(&workflow)
            .arg("--input-bin")
            .arg(&inputs)
            .args(["--durability", "strict", "--db"])
            .arg(&path)
            .output()?;
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        assert!(
            String::from_utf8(output.stdout)?
                .contains("run accepted but not terminal after one runtime tick")
        );
        let fixture = Self { path, directory };
        fixture.assert_issued()?;
        Ok(fixture)
    }

    fn assert_issued(&self) -> TestResult {
        let journal = FjallJournal::open(&self.path, None)?;
        let inputs = journal
            .run_inputs(RunId::new(1))?
            .ok_or("retained inputs")?;
        let record = journal
            .compiled_ir(inputs.artifact_digest())?
            .ok_or("issued artifact")?;
        let actual: AcceptedArtifact = postcard::from_bytes(&record.ir)?;
        let workflow =
            vb_compile::compile_workflow(SOURCE).map_err(|error| format!("{error:?}"))?;
        let mut expected = vb_storage::admission::prepare_compiled_artifact(
            &workflow,
            RuntimePolicy::Strict,
            &[],
        )?;
        expected.accepted_at_seq = EventSeq::new(1);
        expected.digest = vb_storage::admission::accepted_artifact_digest(&expected)?;
        expected.verification.digest = expected.digest;
        assert_eq!(actual, expected);
        assert_eq!(inputs.artifact_digest(), actual.digest);
        assert_eq!(journal.events_for_run_full(RunId::new(1))?.len(), 9);
        Ok(())
    }

    pub(super) fn records(&self) -> TestResult<Vec<u8>> {
        let snapshot = {
            let journal = FjallJournal::open(&self.path, None)?;
            let run = RunId::new(1);
            let events = (0..12)
                .map(|seq| journal.get_event_bytes(run, EventSeq::new(seq)))
                .collect::<Result<Vec<_>, _>>()?;
            let inputs = journal.run_inputs(run)?.ok_or("retained inputs")?;
            let source = journal.workflow_source(inputs.source_digest())?;
            let artifact = journal.compiled_ir(inputs.artifact_digest())?;
            postcard::to_allocvec(&(events, journal.run_header(run)?, inputs, source, artifact))?
        };
        Ok(postcard::to_allocvec(&(
            snapshot,
            raw_event_inventory(&self.path)?,
        ))?)
    }

    pub(super) fn assert_selection(&self, filters: &[&str], selected: &[usize]) -> TestResult {
        let before = self.records()?;
        let expected = FjallJournal::open(&self.path, None)?.events_for_run_full(RunId::new(1))?;
        for emit in ["text", "yaml", "postcard"] {
            let output = events(&self.path, "1", emit, filters)?;
            assert_eq!(
                output.status.code(),
                Some(0),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(output.stderr.is_empty());
            if emit == "text" {
                assert_eq!(String::from_utf8(output.stdout)?, oracles::text(selected)?);
            } else {
                assert_eq!(
                    wire::decode(&output.stdout, emit)?,
                    oracles::report(&expected, selected)?
                );
            }
            assert_eq!(self.records()?, before);
        }
        assert!(self.directory.path().join("workflow.yaml").is_file());
        Ok(())
    }
}

fn raw_event_inventory(path: &Path) -> TestResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let database = fjall::Database::builder(path).open()?;
    let events = database.keyspace(KEYSPACE_RUN_EVENT, || {
        keyspace_options_for(KeyspaceProfile::Hot)
    })?;
    Ok(events
        .iter()
        .map(|guard| {
            let (key, value) = guard.into_inner()?;
            Ok((key.to_vec(), value.to_vec()))
        })
        .collect::<Result<Vec<_>, fjall::Error>>()?)
}

pub(super) fn events(path: &Path, run: &str, emit: &str, filters: &[&str]) -> TestResult<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["events", run, "--db"])
        .arg(path)
        .args(["--emit", emit])
        .args(filters)
        .output()?)
}
