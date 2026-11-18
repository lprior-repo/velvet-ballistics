#![forbid(unsafe_code)]
use std::process::{Command, Output};

const SOURCE: &str = "version: velvet-ballistics/v1\nname: semantic_diff\nwhen:\n  manual: {}\nsteps:\n  - id: saved\n    save:\n      output: value\n      value: '42'\n  - id: done\n    finish:\n      result: value\n";
fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(arguments)
        .output()
        .expect("CLI")
}
fn value(output: &Output) -> serde_json::Value {
    serde_saphyr::from_str(std::str::from_utf8(&output.stdout).expect("UTF8")).expect("YAML")
}
#[test]
fn workflow_diff_distinguishes_text_only_and_semantic_edits() {
    let dir = tempfile::tempdir().expect("directory");
    let before = dir.path().join("old.yaml");
    let after = dir.path().join("new.yaml");
    std::fs::write(&before, SOURCE).expect("source");
    std::fs::write(&after, format!("# formatting only\n{SOURCE}")).expect("source");
    let args = [
        "diff",
        after.to_str().expect("path"),
        "--against",
        before.to_str().expect("path"),
        "--emit",
        "yaml",
    ];
    let same = run(&args);
    assert_eq!(
        same.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&same.stderr)
    );
    let report = value(&same);
    assert_eq!(report["source_diff"]["changed"], true);
    assert_eq!(report["semantic_diff"]["changed"], false);
    assert_eq!(report["digest_comparison"]["equal"], true);
    std::fs::write(&after, SOURCE.replace("'42'", "'43'")).expect("source");
    let changed = run(&args);
    assert_eq!(changed.status.code(), Some(1));
    assert_eq!(value(&changed)["semantic_diff"]["changed"], true);
    std::fs::write(&after, "malformed: [").expect("bad source");
    assert_eq!(run(&args).status.code(), Some(2));
}
#[test]
fn workflow_diff_refuses_absent_baseline_without_creating_database() {
    let dir = tempfile::tempdir().expect("directory");
    let workflow = dir.path().join("workflow.yaml");
    let db = dir.path().join("absent-db");
    std::fs::write(&workflow, SOURCE).expect("source");
    let output = run(&[
        "diff",
        workflow.to_str().expect("path"),
        "--db",
        db.to_str().expect("path"),
        "--emit",
        "yaml",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(!db.exists());
}

#[test]
fn workflow_diff_uses_latest_issued_real_artifact_without_changing_records() {
    let dir = tempfile::tempdir().expect("directory");
    let db = dir.path().join("db");
    let workflow_path = dir.path().join("new.yaml");
    std::fs::write(&workflow_path, SOURCE).expect("source");
    let current = vb_compile::compile_workflow(SOURCE.as_bytes()).expect("workflow");
    let previous =
        vb_compile::compile_workflow(SOURCE.replace("'42'", "'41'").as_bytes()).expect("workflow");
    let journal = vb_storage::FjallJournal::open(&db, None).expect("journal");
    let old = vb_storage::submit_artifact(&journal, &previous, vb_core::RuntimePolicy::Strict)
        .expect("old issued");
    let new = vb_storage::submit_artifact(&journal, &current, vb_core::RuntimePolicy::Strict)
        .expect("new issued");
    assert!(new.accepted_at_seq > old.accepted_at_seq);
    let before = journal.list_artifacts().expect("artifacts");
    drop(journal);
    let args = [
        "diff",
        workflow_path.to_str().expect("path"),
        "--db",
        db.to_str().expect("path"),
        "--emit",
        "yaml",
    ];
    let unchanged = run(&args);
    assert_eq!(
        unchanged.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&unchanged.stderr)
    );
    assert_eq!(value(&unchanged)["source_diff"]["status"], "unavailable");
    assert_eq!(value(&unchanged)["digest_comparison"]["equal"], true);
    std::fs::write(&workflow_path, SOURCE.replace("'42'", "'43'")).expect("source");
    assert_eq!(run(&args).status.code(), Some(1));
    let journal = vb_storage::FjallJournal::open(&db, None).expect("journal");
    assert_eq!(journal.list_artifacts().expect("artifacts"), before);
    assert_eq!(
        journal
            .compiled_ir(new.digest)
            .expect("read")
            .expect("row")
            .digest,
        new.digest
    );
}

#[test]
fn workflow_diff_detects_secret_action_retry_and_nested_graph_changes() {
    let dir = tempfile::tempdir().expect("directory");
    let before = dir.path().join("old.yaml");
    let after = dir.path().join("new.yaml");
    let source = "version: velvet-ballistics/v1\nname: semantic_cases\nwhen: {manual: {}}\nsecrets: {token: OLD_OPAQUE_KEY}\nsteps:\n  - id: seed\n    set: {output: value, value: '42'}\n  - id: call\n    try_again: {max_attempts: 2, delay: '100ms'}\n    do: {action: '1', input: '$secrets.token'}\n  - id: done\n    finish: {result: '$step.seed'}\n";
    std::fs::write(&before, source).expect("source");
    for changed in [
        source.replace("OLD_OPAQUE_KEY", "NEW_OPAQUE_KEY"),
        source.replace("max_attempts: 2", "max_attempts: 3"),
        source.replace("action: '1'", "action: '3'"),
    ] {
        std::fs::write(&after, changed).expect("source");
        let output = run(&[
            "diff",
            after.to_str().expect("path"),
            "--against",
            before.to_str().expect("path"),
            "--emit",
            "yaml",
        ]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(value(&output)["semantic_diff"]["changed"], true);
        assert_eq!(value(&output)["digest_comparison"]["equal"], false);
    }
}

#[path = "../../../velvet_ballistics/src/cli_postcard.rs"]
mod cli_postcard;
#[test]
fn workflow_diff_postcard_matches_yaml_semantics() {
    let dir = tempfile::tempdir().expect("directory");
    let old = dir.path().join("old.yaml");
    let new = dir.path().join("new.yaml");
    std::fs::write(&old, SOURCE).expect("source");
    std::fs::write(&new, SOURCE.replace("'42'", "'43'")).expect("source");
    let output = run(&[
        "diff",
        new.to_str().expect("path"),
        "--against",
        old.to_str().expect("path"),
        "--emit",
        "postcard",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let (_, packet) = cli_postcard::decode_postcard_json(&output.stdout).expect("Postcard");
    assert_eq!(packet["kind"], "workflow_diff_report");
    assert_eq!(packet["semantic_diff"]["changed"], true);
    assert_eq!(packet["digest_comparison"]["equal"], false);
}

#[test]
fn workflow_diff_rejects_unissued_and_missing_matching_artifacts() {
    let dir = tempfile::tempdir().expect("directory");
    let db = dir.path().join("db");
    let workflow_path = dir.path().join("workflow.yaml");
    std::fs::write(&workflow_path, SOURCE).expect("source");
    let workflow = vb_compile::compile_workflow(SOURCE.as_bytes()).expect("workflow");
    let journal = vb_storage::FjallJournal::open(&db, None).expect("journal");
    let unissued = vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        vb_core::RuntimePolicy::Strict,
        &[],
    )
    .expect("artifact");
    journal
        .put_compiled_ir(&vb_storage::CompiledIrRecord {
            digest: unissued.digest,
            ir: postcard::to_allocvec(&unissued).expect("encode"),
        })
        .expect("unissued persisted as raw artifact");
    drop(journal);
    let args = [
        "diff",
        workflow_path.to_str().expect("path"),
        "--db",
        db.to_str().expect("path"),
        "--emit",
        "yaml",
    ];
    let unissued = run(&args);
    assert_eq!(unissued.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&unissued.stderr).contains("no issued acceptance sequence"));
    std::fs::write(
        &workflow_path,
        SOURCE.replace("semantic_diff", "other_workflow"),
    )
    .expect("source");
    let missing = run(&args);
    assert_eq!(missing.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&missing.stderr).contains("no prior accepted artifact"));
}

#[test]
fn review_line_insertions_and_deletions_are_aligned() {
    let dir = tempfile::tempdir().expect("directory");
    let before = dir.path().join("old.yaml");
    let after = dir.path().join("new.yaml");
    std::fs::write(&before, SOURCE).expect("source");
    std::fs::write(&after, format!("# inserted\n{SOURCE}")).expect("source");
    let report = value(&run(&[
        "diff",
        after.to_str().expect("path"),
        "--against",
        before.to_str().expect("path"),
        "--emit",
        "yaml",
    ]));
    assert_eq!(
        report["source_diff"]["lines"],
        serde_json::json!([{"kind": "insert", "before_line": null, "after_line": 1, "before": null, "after": "# inserted"}])
    );
    let report = value(&run(&[
        "diff",
        before.to_str().expect("path"),
        "--against",
        after.to_str().expect("path"),
        "--emit",
        "yaml",
    ]));
    assert_eq!(
        report["source_diff"]["lines"],
        serde_json::json!([{"kind": "delete", "before_line": 1, "after_line": null, "before": "# inserted", "after": null}])
    );
}

#[test]
fn review_conflicting_baseline_modes_are_rejected() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("workflow.yaml");
    std::fs::write(&path, SOURCE).expect("source");
    let output = run(&[
        "diff",
        path.to_str().expect("path"),
        "--against",
        path.to_str().expect("path"),
        "--db",
        dir.path().to_str().expect("path"),
        "--emit",
        "yaml",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--against and --db"));
}

#[test]
fn review_unissued_artifact_does_not_poison_positive_baseline() {
    let dir = tempfile::tempdir().expect("directory");
    let db = dir.path().join("db");
    let path = dir.path().join("workflow.yaml");
    std::fs::write(&path, SOURCE).expect("source");
    let workflow = vb_compile::compile_workflow(SOURCE.as_bytes()).expect("workflow");
    let journal = vb_storage::FjallJournal::open(&db, None).expect("journal");
    vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict)
        .expect("issued");
    let unissued = vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        vb_core::RuntimePolicy::Strict,
        &[],
    )
    .expect("prepare");
    journal
        .put_compiled_ir(&vb_storage::CompiledIrRecord {
            digest: unissued.digest,
            ir: postcard::to_allocvec(&unissued).expect("encode"),
        })
        .expect("unissued");
    drop(journal);
    let output = run(&[
        "diff",
        path.to_str().expect("path"),
        "--db",
        db.to_str().expect("path"),
        "--emit",
        "yaml",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(value(&output)["digest_comparison"]["equal"], true);
}

fn contract(id: u16, timeout_ms: u64) -> vb_core::ActionContract {
    vb_core::ActionContract {
        id: vb_core::ActionId::new(id),
        name: vb_core::action::ActionName::new("noop").expect("name"),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 65536,
        max_output_bytes: 65536,
        timeout_ms,
        idempotency: vb_core::Idempotency::DeterministicPure,
        side_effect: vb_core::SideEffect::Pure,
        retry_safety: vb_core::RetrySafety::Idempotent,
        required_capabilities: Box::default(),
    }
}
const ACTION_SOURCE: &str = "version: velvet-ballistics/v1\nname: action_diff\nwhen: {manual: {}}\nsteps:\n  - id: seed\n    set: {output: value, value: '42'}\n  - id: before_call\n    do: {action: '0', input: '$step.seed'}\n  - id: call\n    do: {action: '1', input: '$step.seed'}\n  - id: done\n    finish: {result: '$step.seed'}\n";

#[test]
fn workflow_diff_projects_current_external_read_declarations_without_changing_records()
-> Result<(), Box<dyn std::error::Error>> {
    for safety in [
        vb_core::RetrySafety::Idempotent,
        vb_core::RetrySafety::RequiresIdempotencyKey,
    ] {
        let dir = tempfile::tempdir()?;
        let db = dir.path().join("db");
        let path = dir.path().join("workflow.yaml");
        std::fs::write(&path, ACTION_SOURCE)?;
        let workflow = vb_compile::compile_workflow(ACTION_SOURCE.as_bytes())?;
        let mut read = contract(1, 1000);
        read.idempotency = vb_core::Idempotency::IdempotentExternal;
        read.side_effect = vb_core::SideEffect::ExternalRead;
        read.retry_safety = safety;
        let journal = vb_storage::FjallJournal::open(&db, None)?;
        let issued = vb_storage::submit_artifact_with_contracts(
            &journal,
            &workflow,
            vb_core::RuntimePolicy::Strict,
            &[contract(0, 1000), read.clone()],
        )?;
        assert!(issued.accepted_at_seq.get() > 0);
        let before = journal.compiled_ir(issued.digest)?.ok_or("issued record")?;
        let artifacts = journal.list_artifacts()?;
        drop(journal);
        let output = run(&[
            "diff",
            path.to_str().ok_or("workflow path")?,
            "--db",
            db.to_str().ok_or("database path")?,
            "--emit",
            "yaml",
        ]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report = value(&output);
        assert_eq!(
            report.pointer("/action_contracts/comparisons/1/before"),
            Some(&serde_json::to_value(&read)?)
        );
        assert_eq!(
            report.pointer("/action_contracts/changed"),
            Some(&true.into())
        );
        let journal = vb_storage::FjallJournal::open(&db, None)?;
        assert_eq!(journal.list_artifacts()?, artifacts);
        assert_eq!(journal.compiled_ir(issued.digest)?, Some(before));
    }
    Ok(())
}

#[test]
fn review_same_action_id_compares_persisted_contract_metadata() {
    let dir = tempfile::tempdir().expect("directory");
    let db = dir.path().join("db");
    let path = dir.path().join("workflow.yaml");
    std::fs::write(&path, ACTION_SOURCE).expect("source");
    let workflow = vb_compile::compile_workflow(ACTION_SOURCE.as_bytes()).expect("workflow");
    let journal = vb_storage::FjallJournal::open(&db, None).expect("journal");
    vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        vb_core::RuntimePolicy::Strict,
        &[contract(0, 1000), contract(1, 999)],
    )
    .expect("issued old contract");
    drop(journal);
    let output = run(&[
        "diff",
        path.to_str().expect("path"),
        "--db",
        db.to_str().expect("path"),
        "--emit",
        "yaml",
    ]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = value(&output);
    assert_eq!(report["digest_comparison"]["equal"], true);
    assert_eq!(report["action_contracts"]["status"], "partial");
    assert_eq!(report["action_contracts"]["changed"], true);
    assert_eq!(
        report["action_contracts"]["comparisons"][1]["before"]["timeout_ms"],
        999
    );
    assert_eq!(
        report["action_contracts"]["comparisons"][1]["after"]["timeout_ms"],
        1000
    );
    assert!(
        report["semantic_diff"]["changes"]
            .as_array()
            .expect("changes")
            .contains(&serde_json::json!("action_contracts"))
    );
    let unavailable = run(&[
        "diff",
        path.to_str().expect("path"),
        "--against",
        path.to_str().expect("path"),
        "--emit",
        "yaml",
    ]);
    assert_eq!(unavailable.status.code(), Some(2));
    assert_eq!(
        value(&unavailable)["action_contracts"]["status"],
        "unavailable"
    );
    assert_eq!(
        value(&unavailable)["semantic_diff"]["changed"],
        serde_json::Value::Null
    );
}

#[test]
fn review_missing_retained_source_only_disables_text_comparison() {
    let dir = tempfile::tempdir().expect("directory");
    let db = dir.path().join("db");
    let path = dir.path().join("workflow.yaml");
    std::fs::write(&path, SOURCE).expect("source");
    let workflow = vb_compile::compile_workflow(SOURCE.as_bytes()).expect("workflow");
    let journal = vb_storage::FjallJournal::open(&db, None).expect("journal");
    let artifact = vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict)
        .expect("issued");
    let run_id = vb_core::RunId::new(807);
    let source_digest =
        vb_core::WorkflowDigest::from_bytes(*blake3::hash(SOURCE.as_bytes()).as_bytes());
    let inputs = vb_storage::RunInputsRecord::new(
        run_id,
        artifact.digest,
        source_digest,
        &workflow,
        Box::default(),
    )
    .expect("inputs");
    journal
        .put_run_header(&vb_storage::RunHeaderRecord {
            run: run_id,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: 123,
        })
        .expect("header");
    journal
        .append_runtime_admission_batch(
            &[
                vb_storage::JournalEvent::RunAccepted {
                    run: run_id,
                    seq: vb_storage::EventSeq::ZERO,
                    workflow: workflow.digest(),
                },
                vb_storage::JournalEvent::RunAdmissionBudget {
                    run: run_id,
                    seq: vb_storage::EventSeq::new(1),
                    artifact_digest: artifact.digest,
                    input_digest: vb_storage::RunInputBinding::Digest(inputs.input_digest()),
                    policy_pin: vb_storage::RunPolicyPin::Pinned(artifact.policy_digest),
                    granted_capabilities: vb_core::CapabilitySet::empty(),
                    policy: vb_core::RuntimePolicy::Strict,
                    budget: vb_core::AggregateResourceBudget::from_workflow(&workflow)
                        .expect("budget"),
                },
            ],
            &inputs,
            vb_storage::DurabilityProfile::Strict,
        )
        .expect("current admission");
    drop(journal);
    let output = run(&[
        "diff",
        path.to_str().expect("path"),
        "--db",
        db.to_str().expect("path"),
        "--emit",
        "yaml",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(value(&output)["source_diff"]["status"], "unavailable");
    assert_eq!(value(&output)["semantic_diff"]["changed"], false);
}

fn padded_source(line_count: usize, label: &str) -> String {
    let padding = line_count
        .checked_sub(SOURCE.lines().count())
        .expect("padding");
    let source = format!("{}{}", format!("# {label}\n").repeat(padding), SOURCE);
    assert_eq!(source.lines().count(), line_count);
    source
}

#[test]
fn review_line_diff_scratch_boundary_preserves_unchanged_semantics() {
    let dir = tempfile::tempdir().expect("directory");
    let before = dir.path().join("before.yaml");
    let after = dir.path().join("after.yaml");
    for line_count in [1023, 1024] {
        std::fs::write(&before, padded_source(line_count, "before")).expect("before");
        std::fs::write(&after, padded_source(line_count, "after")).expect("after");
        let output = run(&[
            "diff",
            after.to_str().expect("path"),
            "--against",
            before.to_str().expect("path"),
            "--emit",
            "yaml",
        ]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report = value(&output);
        assert_eq!(report["semantic_diff"]["changed"], false);
        assert_eq!(report["digest_comparison"]["equal"], true);
        assert_eq!(report["source_diff"]["changed"], true);
        if line_count == 1023 {
            assert_eq!(report["source_diff"]["status"], "available");
            assert!(
                !report["source_diff"]["lines"]
                    .as_array()
                    .expect("edits")
                    .is_empty()
            );
        } else {
            assert_eq!(
                report["source_diff"],
                serde_json::json!({"status": "unavailable", "changed": true, "reason": "line diff exceeds bounded scratch capacity"})
            );
        }
    }
}

#[test]
fn review_metadata_without_admission_keeps_source_unavailable() {
    let dir = tempfile::tempdir().expect("directory");
    let db = dir.path().join("db");
    let path = dir.path().join("workflow.yaml");
    std::fs::write(&path, SOURCE).expect("source");
    let workflow = vb_compile::compile_workflow(SOURCE.as_bytes()).expect("workflow");
    let journal = vb_storage::FjallJournal::open(&db, None).expect("journal");
    let artifact = vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict)
        .expect("issued");
    let run_id = vb_core::RunId::new(908);
    let source_digest =
        vb_core::WorkflowDigest::from_bytes(*blake3::hash(SOURCE.as_bytes()).as_bytes());
    journal
        .put_run_header(&vb_storage::RunHeaderRecord {
            run: run_id,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: 123,
        })
        .expect("header");
    journal
        .put_workflow_source(&vb_storage::WorkflowSourceRecord {
            digest: source_digest,
            source: SOURCE.as_bytes().to_vec(),
        })
        .expect("source row");
    assert_eq!(journal.events_for_run(run_id).expect("events"), vec![]);
    assert_eq!(journal.run_inputs(run_id).expect("inputs"), None);
    let before = journal.list_artifacts().expect("artifacts");
    drop(journal);
    let arguments = [
        "diff",
        path.to_str().expect("path"),
        "--db",
        db.to_str().expect("path"),
        "--emit",
        "yaml",
    ];
    let output = run(&arguments);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        value(&output)["source_diff"],
        serde_json::json!({"status": "unavailable", "reason": "retained source unavailable"})
    );
    assert_eq!(value(&output)["semantic_diff"]["changed"], false);
    assert_eq!(value(&output)["digest_comparison"]["equal"], true);
    let journal = vb_storage::FjallJournal::open(&db, None).expect("journal");
    assert_eq!(journal.list_artifacts().expect("artifacts"), before);
    assert_eq!(journal.events_for_run(run_id).expect("events"), vec![]);
    assert_eq!(
        journal
            .workflow_source(source_digest)
            .expect("source row")
            .expect("retained")
            .source,
        SOURCE.as_bytes()
    );
    let inputs = vb_storage::RunInputsRecord::new(
        run_id,
        artifact.digest,
        source_digest,
        &workflow,
        Box::default(),
    )
    .expect("inputs");
    journal
        .append_runtime_admission_batch(
            &[
                vb_storage::JournalEvent::RunAccepted {
                    run: run_id,
                    seq: vb_storage::EventSeq::ZERO,
                    workflow: workflow.digest(),
                },
                vb_storage::JournalEvent::RunAdmissionBudget {
                    run: run_id,
                    seq: vb_storage::EventSeq::new(1),
                    artifact_digest: artifact.digest,
                    input_digest: vb_storage::RunInputBinding::Digest(inputs.input_digest()),
                    policy_pin: vb_storage::RunPolicyPin::Pinned(artifact.policy_digest),
                    granted_capabilities: vb_core::CapabilitySet::empty(),
                    policy: vb_core::RuntimePolicy::Strict,
                    budget: vb_core::AggregateResourceBudget::from_workflow(&workflow)
                        .expect("budget"),
                },
            ],
            &inputs,
            vb_storage::DurabilityProfile::Strict,
        )
        .expect("current admission");
    drop(journal);
    let output = run(&arguments);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        value(&output)["source_diff"],
        serde_json::json!({"status": "available", "changed": false, "lines": []})
    );
    assert_eq!(value(&output)["semantic_diff"]["changed"], false);
}

#[test]
fn review_current_cli_submission_supplies_issued_bound_source_without_changes() {
    let directory = tempfile::tempdir().expect("directory");
    let source = directory.path().join("workflow.yaml");
    let db = directory.path().join("db");
    let input = directory.path().join("input.bin");
    std::fs::write(&source, SOURCE).expect("source");
    std::fs::write(
        &input,
        postcard::to_allocvec(&Vec::<vb_core::SlotValue>::new()).expect("input"),
    )
    .expect("input file");
    let submitted = run(&[
        "submit",
        source.to_str().expect("path"),
        "--input-bin",
        input.to_str().expect("path"),
        "--db",
        db.to_str().expect("path"),
        "--durability",
        "strict",
        "--emit",
        "yaml",
    ]);
    assert_eq!(submitted.status.code(), Some(0), "{:?}", submitted.stderr);
    let run_id = vb_core::RunId::new(value(&submitted)["run_id"].as_u64().expect("run"));
    let journal = vb_storage::FjallJournal::open(&db, None).expect("journal");
    let inputs = journal
        .run_inputs(run_id)
        .expect("inputs")
        .expect("retained");
    let record = journal
        .compiled_ir(inputs.artifact_digest())
        .expect("artifact")
        .expect("issued");
    let artifact: vb_storage::AcceptedArtifact = postcard::from_bytes(&record.ir).expect("decode");
    assert_eq!(artifact.accepted_at_seq, vb_storage::EventSeq::new(1));
    assert_eq!(artifact.digest, inputs.artifact_digest());
    let before = journal
        .get_event_bytes(run_id, vb_storage::EventSeq::new(1))
        .expect("receipt")
        .expect("bytes");
    drop(journal);
    let output = run(&[
        "diff",
        source.to_str().expect("path"),
        "--db",
        db.to_str().expect("path"),
        "--emit",
        "yaml",
    ]);
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    let report = value(&output);
    assert_eq!(
        report["source_diff"],
        serde_json::json!({"status":"available", "changed":false, "lines":[]})
    );
    assert_eq!(report["semantic_diff"]["changed"], false);
    assert_eq!(report["digest_comparison"]["equal"], true);
    let journal = vb_storage::FjallJournal::open(&db, None).expect("journal");
    assert_eq!(
        journal
            .get_event_bytes(run_id, vb_storage::EventSeq::new(1))
            .expect("receipt")
            .expect("bytes"),
        before
    );
    assert_eq!(journal.run_inputs(run_id).expect("inputs"), Some(inputs));
}

#[test]
fn workflow_diff_without_either_baseline_has_explicit_public_refusal() {
    let directory = tempfile::tempdir().expect("directory");
    let source = directory.path().join("workflow.yaml");
    std::fs::write(&source, SOURCE).expect("source");
    let output = run(&["diff", source.to_str().expect("path")]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, b"");
    assert_eq!(
        output.stderr,
        b"previous-artifact comparison requires --db or --against\n"
    );
    assert_eq!(
        std::fs::read(&source).expect("source unchanged"),
        SOURCE.as_bytes()
    );
    assert_eq!(
        std::fs::read_dir(directory.path())
            .expect("directory")
            .count(),
        1
    );
}
