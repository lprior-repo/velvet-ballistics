// ---------------------------------------------------------------------------
// Phase 10: Taint propagation via action ABI
// ---------------------------------------------------------------------------

#[test]
fn taint_secret_propagates_through_deterministic_action() {
    use vb_core::action::{Idempotency, propagate_action_taint};
    use vb_core::value::Taint;

    let result = propagate_action_taint(Idempotency::DeterministicPure, Taint::Secret);
    assert_eq!(result, Taint::Secret, "Secret input should propagate");
}

#[test]
fn taint_clean_stays_clean_for_pure_actions() {
    use vb_core::action::{Idempotency, propagate_action_taint};
    use vb_core::value::Taint;

    let result = propagate_action_taint(Idempotency::DeterministicPure, Taint::Clean);
    assert_eq!(result, Taint::Clean, "Clean input stays clean");
}

#[test]
fn taint_derived_propagates() {
    use vb_core::action::{Idempotency, propagate_action_taint};
    use vb_core::value::Taint;

    let result = propagate_action_taint(Idempotency::IdempotentExternal, Taint::DerivedFromSecret);
    assert_eq!(
        result,
        Taint::DerivedFromSecret,
        "DerivedFromSecret propagates"
    );
}

// ---------------------------------------------------------------------------
// Phase 11: IPC command enum completeness
// ---------------------------------------------------------------------------

#[test]
fn ipc_all_commands_have_distinct_codes() {
    use std::collections::HashSet;
    let commands = [
        vb_ipc::IpcCommand::Health,
        vb_ipc::IpcCommand::Shutdown,
        vb_ipc::IpcCommand::SubmitRun,
        vb_ipc::IpcCommand::SubmitRunInline,
        vb_ipc::IpcCommand::CancelRun,
        vb_ipc::IpcCommand::InspectRun,
        vb_ipc::IpcCommand::ListEvents,
        vb_ipc::IpcCommand::AnswerAsk,
        vb_ipc::IpcCommand::CompleteAction,
        vb_ipc::IpcCommand::FailAction,
        vb_ipc::IpcCommand::DrainTrace,
    ];
    let codes: HashSet<u16> = commands.iter().map(|c| c.as_u16()).collect();
    assert_eq!(
        codes.len(),
        commands.len(),
        "all commands must have unique codes"
    );
}

// ---------------------------------------------------------------------------
// Phase 12: Limits are enforced
// ---------------------------------------------------------------------------

#[test]
fn limits_max_expression_stack_is_bounded() {
    let max = vb_core::limits::MAX_EXPRESSION_STACK;
    assert!(
        max <= 64,
        "expression stack must be bounded to 64: got {max}"
    );
}

#[test]
fn limits_max_steps_per_workflow_is_bounded() {
    let max = vb_core::limits::MAX_STEPS_PER_WORKFLOW;
    assert!(max <= 65535, "max steps must be bounded: got {max}");
}

// ---------------------------------------------------------------------------
// Phase 13: CLI validate subcommand
// ---------------------------------------------------------------------------

#[test]
fn cli_validate_valid_minimal_workflow_succeeds() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("valid.yaml");
    let workflow = "version: velvet-ballistics/v1
name: validate_test
when:
  manual: {}
steps:
  - id: greet
    save:
      output: greeting
      value: '42'
  - id: done
    finish:
      result: greeting
";
    if !write_test_file(&workflow_path, workflow.as_bytes()) {
        return;
    }

    let output = match run_cli(&[std::ffi::OsStr::new("validate"), workflow_path.as_os_str()]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "validate valid workflow");
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("valid"),
        "validate should print 'valid': {stdout}"
    );
}

#[test]
fn cli_validate_invalid_yaml_returns_parse_error() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("broken.yaml");
    if !write_test_file(&workflow_path, b"{{{not-yaml") {
        return;
    }

    let output = match run_cli(&[std::ffi::OsStr::new("validate"), workflow_path.as_os_str()]) {
        Some(output) => output,
        None => return,
    };
    assert!(
        !output.status.success(),
        "validate should fail on broken YAML"
    );
    let stderr = output_stderr(&output);
    assert!(
        stderr.contains("YAML parse error") || stderr.contains("YAML parse failed"),
        "validate should report parse error: {stderr}"
    );
}

#[test]
fn cli_validate_undefined_step_reference_returns_validation_error() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("bad-ref.yaml");
    let workflow = "version: velvet-ballistics/v1
name: bad_ref_test
when:
  manual: {}
steps:
  - id: greet
    save:
      output: greeting
      value: $steps.nonexistent
  - id: done
    finish:
      result: greeting
";
    if !write_test_file(&workflow_path, workflow.as_bytes()) {
        return;
    }

    let output = match run_cli(&[std::ffi::OsStr::new("validate"), workflow_path.as_os_str()]) {
        Some(output) => output,
        None => return,
    };
    assert!(
        !output.status.success(),
        "validate should fail on undefined step reference"
    );
    let stderr = output_stderr(&output);
    assert!(
        stderr.contains("compile error"),
        "validate should report compile error for undefined step reference: {stderr}"
    );
}

#[test]
fn cli_validate_type_mismatch_returns_typed_error() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("type-mismatch.yaml");
    let workflow = "version: velvet-ballistics/v1
name: type_mismatch_test
when:
  manual: {}
steps:
  - id: greet
    save:
      output: message
      value: \"hello\"
    then: done
  - id: done
    finish:
      result: 1 + \"not_a_number\"
";
    if !write_test_file(&workflow_path, workflow.as_bytes()) {
        return;
    }

    let output = match run_cli(&[std::ffi::OsStr::new("validate"), workflow_path.as_os_str()]) {
        Some(output) => output,
        None => return,
    };
    assert!(
        !output.status.success(),
        "validate should fail on type mismatch"
    );
    let stderr = output_stderr(&output);
    assert!(
        stderr.contains("compile error"),
        "validate should report compile error: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// Phase 14: CLI compile subcommand
// ---------------------------------------------------------------------------

#[test]
fn cli_compile_valid_workflow_produces_ir() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let ir_path = dir.path().join("workflow.vbir");

    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("compile"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("ir"),
        std::ffi::OsStr::new("--out"),
        ir_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "compile --emit ir");
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("compiled IR written"),
        "compile should report IR written: {stdout}"
    );

    let ir_bytes = match std::fs::read(&ir_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "failed to read compiled IR: {err}"
            );
            return;
        }
    };
    assert!(!ir_bytes.is_empty(), "compiled IR file should not be empty");

    let parts_result = postcard::from_bytes::<vb_core::workflow::WorkflowParts>(&ir_bytes);
    assert!(
        parts_result.is_ok(),
        "compiled IR should be valid postcard-encoded WorkflowParts: {parts_result:?}"
    );
}

#[test]
fn cli_compile_invalid_syntax_fails_with_clear_error() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("bad.yaml");
    let ir_path = dir.path().join("bad.vbir");

    if !write_test_file(&workflow_path, b"version: not-the-right-version\nsteps: []") {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("compile"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("ir"),
        std::ffi::OsStr::new("--out"),
        ir_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert!(
        !output.status.success(),
        "compile should fail on invalid workflow"
    );
    let stderr = output_stderr(&output);
    assert!(
        stderr.contains("compile error"),
        "compile should report compile error: {stderr}"
    );
}

#[test]
fn cli_compile_preserves_workflow_digest() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let ir_path = dir.path().join("workflow.vbir");

    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("compile"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("ir"),
        std::ffi::OsStr::new("--out"),
        ir_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "compile --emit ir");

    let ir_bytes = match std::fs::read(&ir_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "failed to read compiled IR: {err}"
            );
            return;
        }
    };
    let parts = match postcard::from_bytes::<vb_core::workflow::WorkflowParts>(&ir_bytes) {
        Ok(parts) => parts,
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "failed to decode WorkflowParts: {err}"
            );
            return;
        }
    };

    let compile_result = vb_compile::compile_workflow(CLI_WORKFLOW.as_bytes());
    match compile_result {
        Ok(compiled) => {
            assert_eq!(
                parts.digest,
                compiled.digest(),
                "compiled IR digest should match in-memory compile digest"
            );
        }
        Err(err) => assert!(
            forced_assertion_failure(),
            "in-memory compile should succeed: {err:?}"
        ),
    }
}

// ---------------------------------------------------------------------------
// Phase 15: CLI run subcommand
// ---------------------------------------------------------------------------

#[test]
fn cli_run_minimal_workflow_completes() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "run --durability none");
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("run completed"),
        "run should report completion: {stdout}"
    );
}

#[test]
fn cli_run_strict_durability_writes_journal_events() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");
    let db_path = dir.path().join("strict-db");

    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

    let run_output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("strict"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&run_output, "run --durability strict");
    let stdout = output_stdout(&run_output);
    assert!(
        stdout.contains("run completed"),
        "strict run should complete: {stdout}"
    );

    let events_output = match run_cli(&[
        std::ffi::OsStr::new("events"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&events_output, "events after strict run");
    let events_stdout = output_stdout(&events_output);
    assert!(
        events_stdout.contains("RunAccepted"),
        "strict run should produce RunAccepted event: {events_stdout}"
    );
    assert!(
        events_stdout.contains("RunFinished"),
        "strict run should produce RunFinished event: {events_stdout}"
    );
}

#[test]
fn cli_run_invalid_workflow_returns_error_exit_code() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("invalid.yaml");
    let input_path = dir.path().join("input.bin");

    if !write_test_file(&workflow_path, b"not-a-workflow-at-all") {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert!(
        !output.status.success(),
        "run should fail on invalid workflow"
    );
    let stderr = output_stderr(&output);
    assert!(
        !stderr.is_empty(),
        "run should produce error output for invalid workflow"
    );
}

// ---------------------------------------------------------------------------
// Phase 16: CLI inspect subcommand
// ---------------------------------------------------------------------------

#[test]
fn cli_inspect_compiled_run_shows_status_and_event_count() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");
    let db_path = dir.path().join("inspect-db");

    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

    let run_output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("journaled"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&run_output, "run for inspect setup");

    let inspect_output = match run_cli(&[
        std::ffi::OsStr::new("inspect"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&inspect_output, "inspect 1");
    let stdout = output_stdout(&inspect_output);
    assert!(
        stdout.contains("status=finished"),
        "inspect should show finished status: {stdout}"
    );
    assert!(
        stdout.contains("events="),
        "inspect should show event count: {stdout}"
    );
}

#[test]
fn cli_inspect_nonexistent_run_shows_no_events() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    // An existing but empty database: a missing run must report "no events
    // found". A missing database directory is a distinct error covered by
    // qa_findings_regression::read_commands_reject_missing_db_directory.
    let db_path = dir.path().join("empty-db");
    if std::fs::create_dir_all(&db_path).is_err() {
        assert!(forced_assertion_failure(), "db dir setup failed");
        return;
    }

    let inspect_output = match run_cli(&[
        std::ffi::OsStr::new("inspect"),
        std::ffi::OsStr::new("999"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_failure_contains(
        &inspect_output,
        "inspect nonexistent run",
        "no events found",
    );
}

// ---------------------------------------------------------------------------
// Doctor trim eligibility integration tests (vb-zo9d)
// ---------------------------------------------------------------------------

#[test]
fn cli_doctor_json_includes_trim_eligibility_check() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let db_path = dir.path().join("doctor-trim-db");

    // Set up journal with one run that has a snapshot
    let journal = match vb_storage::FjallJournal::open(&db_path, None) {
        Ok(j) => j,
        Err(err) => {
            assert!(forced_assertion_failure(), "failed to open journal: {err}");
            return;
        }
    };

    let run = vb_core::RunId::new(100_000);
    let digest = vb_core::WorkflowDigest::from_bytes([0xCC; 32]);
    let events: Vec<vb_storage::JournalEvent> = (0..6u64)
        .map(|i| {
            if i == 0 {
                vb_storage::JournalEvent::RunAccepted {
                    run,
                    seq: vb_storage::EventSeq::new(i),
                    workflow: digest,
                }
            } else {
                vb_storage::JournalEvent::StepStarted {
                    run,
                    seq: vb_storage::EventSeq::new(i),
                    step: vb_core::StepIdx::new(
                        u16::try_from(i).unwrap_or_default().saturating_sub(1),
                    ),
                    attempt: 1,
                }
            }
        })
        .collect();
    if let Err(err) = journal.append_strict_batch(&events) {
        assert!(forced_assertion_failure(), "failed to append events: {err}");
        return;
    }

    let header = vb_storage::RunHeaderRecord {
        run,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: digest,
        status: 0,
        accepted_at_ms: 100_000,
    };
    if let Err(err) = journal.put_run_header(&header) {
        assert!(forced_assertion_failure(), "failed to write header: {err}");
        return;
    }

    let snapshot = vb_storage::RunSnapshot {
        run,
        seq: vb_storage::EventSeq::new(3),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    if let Err(err) = journal.put_snapshot(&snapshot) {
        assert!(
            forced_assertion_failure(),
            "failed to write snapshot: {err}"
        );
        return;
    }
    drop(journal);

    let output = match run_cli(&[
        std::ffi::OsStr::new("doctor"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "doctor --db <path> --emit yaml");

    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("trim_eligibility"),
        "YAML output should contain trim_eligibility; got: {stdout}"
    );
    assert!(
        stdout.contains("status: pass"),
        "YAML output should contain status: pass; got: {stdout}"
    );
    assert!(
        stdout.contains("total_runs: 1"),
        "YAML output should contain total_runs: 1; got: {stdout}"
    );
    assert!(
        stdout.contains("eligible_runs: 1"),
        "YAML output should contain eligible_runs: 1; got: {stdout}"
    );
    assert!(
        stdout.contains("blocked_runs: 0"),
        "YAML output should contain blocked_runs: 0; got: {stdout}"
    );
    assert!(
        stdout.contains("total_events_trimmable: 3"),
        "YAML output should contain total_events_trimmable: 3; got: {stdout}"
    );
}

#[test]
fn cli_doctor_text_reports_trim_eligibility() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let db_path = dir.path().join("doctor-trim-text-db");

    let journal = match vb_storage::FjallJournal::open(&db_path, None) {
        Ok(j) => j,
        Err(err) => {
            assert!(forced_assertion_failure(), "failed to open journal: {err}");
            return;
        }
    };

    let run = vb_core::RunId::new(100_001);
    let digest = vb_core::WorkflowDigest::from_bytes([0xDD; 32]);
    let events = [
        vb_storage::JournalEvent::RunAccepted {
            run,
            seq: vb_storage::EventSeq::new(0),
            workflow: digest,
        },
        vb_storage::JournalEvent::StepStarted {
            run,
            seq: vb_storage::EventSeq::new(1),
            step: vb_core::StepIdx::new(0),
            attempt: 1,
        },
    ];
    if let Err(err) = journal.append_strict_batch(&events) {
        assert!(forced_assertion_failure(), "failed to append events: {err}");
        return;
    }

    let header = vb_storage::RunHeaderRecord {
        run,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: digest,
        status: 0,
        accepted_at_ms: 100_001,
    };
    if let Err(err) = journal.put_run_header(&header) {
        assert!(forced_assertion_failure(), "failed to write header: {err}");
        return;
    }

    // No snapshot — run should be blocked
    drop(journal);

    let output = match run_cli(&[
        std::ffi::OsStr::new("doctor"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "doctor --db <path>");

    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("trim eligibility"),
        "doctor text output should mention trim eligibility: {stdout}"
    );
    assert!(
        stdout.contains("1 total"),
        "doctor text should report 1 total run: {stdout}"
    );
    assert!(
        stdout.contains("0 eligible"),
        "doctor text should report 0 eligible: {stdout}"
    );
    assert!(
        stdout.contains("1 blocked"),
        "doctor text should report 1 blocked: {stdout}"
    );
}

#[test]
fn cli_doctor_returns_success_for_healthy_journal_with_trim_recommended() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let db_path = dir.path().join("doctor-trim-success-db");

    let journal = match vb_storage::FjallJournal::open(&db_path, None) {
        Ok(j) => j,
        Err(err) => {
            assert!(forced_assertion_failure(), "failed to open journal: {err}");
            return;
        }
    };

    let run = vb_core::RunId::new(100_002);
    let digest = vb_core::WorkflowDigest::from_bytes([0xEE; 32]);
    let events: Vec<vb_storage::JournalEvent> = (0..4u64)
        .map(|i| {
            if i == 0 {
                vb_storage::JournalEvent::RunAccepted {
                    run,
                    seq: vb_storage::EventSeq::new(i),
                    workflow: digest,
                }
            } else {
                vb_storage::JournalEvent::StepStarted {
                    run,
                    seq: vb_storage::EventSeq::new(i),
                    step: vb_core::StepIdx::new(
                        u16::try_from(i).unwrap_or_default().saturating_sub(1),
                    ),
                    attempt: 1,
                }
            }
        })
        .collect();
    if let Err(err) = journal.append_strict_batch(&events) {
        assert!(forced_assertion_failure(), "failed to append events: {err}");
        return;
    }

    let header = vb_storage::RunHeaderRecord {
        run,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: digest,
        status: 0,
        accepted_at_ms: 100_002,
    };
    if let Err(err) = journal.put_run_header(&header) {
        assert!(forced_assertion_failure(), "failed to write header: {err}");
        return;
    }

    let snapshot = vb_storage::RunSnapshot {
        run,
        seq: vb_storage::EventSeq::new(2),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    if let Err(err) = journal.put_snapshot(&snapshot) {
        assert!(
            forced_assertion_failure(),
            "failed to write snapshot: {err}"
        );
        return;
    }
    drop(journal);

    let output = match run_cli(&[
        std::ffi::OsStr::new("doctor"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "doctor --db <path>");
}

#[test]
fn cli_doctor_returns_storage_error_for_unreadable_path() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    // A regular file where the journal directory should be: opening a
    // journal below it fails with ENOTDIR for every user including root,
    // while a merely nonexistent path is created and opens fine as root.
    let blocker = dir.path().join("blocker");
    if !write_test_file(&blocker, b"not a directory") {
        return;
    }
    let db = blocker.join("db");

    let output = match run_cli(&[
        std::ffi::OsStr::new("doctor"),
        std::ffi::OsStr::new("--db"),
        db.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert!(
        !output.status.success(),
        "doctor should fail for unreadable path"
    );
}
